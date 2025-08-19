use bytes::{Buf, BytesMut};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug, PartialEq, Eq)]
pub enum MessageTag {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield,
    Request,
    Piece,
    Cancel,
    Extension,
}

impl MessageTag {
    pub fn type_to_tag(&self) -> u8 {
        match self {
            Self::Choke => 0,
            Self::Unchoke => 1,
            Self::Interested => 2,
            Self::NotInterested => 3,
            Self::Have => 4,
            Self::Bitfield => 5,
            Self::Request => 6,
            Self::Piece => 7,
            Self::Cancel => 8,
            Self::Extension => 20,
        }
    }
    pub fn tag_to_type(tag: &u8) -> Option<MessageTag> {
        match tag {
            0 => Some(MessageTag::Choke),
            1 => Some(MessageTag::Unchoke),
            2 => Some(MessageTag::Interested),
            3 => Some(MessageTag::NotInterested),
            4 => Some(MessageTag::Have),
            5 => Some(MessageTag::Bitfield),
            6 => Some(MessageTag::Request),
            7 => Some(MessageTag::Piece),
            8 => Some(MessageTag::Cancel),
            20 => Some(MessageTag::Extension),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub message_tag: MessageTag,
    pub payload: Payload,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Payload {
    SimplePayload(Vec<u8>),
    //// Used for extension messages
    ExtendedPayload(ExtensionPayload),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionPayload {
    pub extension_id: u8,
    /// Dictionary of supported extension messages which maps names of extensions to an extended message ID for each extension message.
    pub m: M,

    /// Local TCP listen port
    #[serde(default)]
    pub p: u8,

    /// Client name and version (as utf8)
    #[serde(default)]
    pub v: String,

    /// ip address of this peer (maybe IPV4 or IPV6)
    #[serde(default = "default_peer")]
    pub yourip: PeerIP,

    /// If this peer has an IPv6 interface, this is the compact representation of that address (16 bytes)
    #[serde(default = "ipv6_default")]
    pub ipv6: Ipv6Addr,

    /// If extend_from_slices peer has an IPv4 interface, this is the compact representation of that address (4 bytes).
    #[serde(default = "ipv4_default")]
    pub ipv4: Ipv4Addr,

    /// An integer, the number of outstanding request messages this client supports without dropping any. The default in in libtorrent is 250.
    #[serde(default)]
    pub reqq: u8,
}

pub fn ipv6_default() -> Ipv6Addr {
    Ipv6Addr::UNSPECIFIED
}

pub fn ipv4_default() -> Ipv4Addr {
    Ipv4Addr::UNSPECIFIED
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct M {
    #[serde(default)]
    pub ut_metadata: u8,
    #[serde(default)]
    pub ut_pex: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeerIP {
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

pub fn default_peer() -> PeerIP {
    PeerIP::Ipv4(ipv4_default())
}

// impl ExtensionPayload {
//     pub fn to_vector(&self) -> Vec<u8> {
//         let mut single_slice = Vec::new();
//         single_slice.extend(&self.extension_id.to_be_bytes());
//         let str_reference = self
//             .dict
//             .as_str()
//             .expect("Conversion from Value to String failed");
//         single_slice.extend(str_reference.as_bytes());
//         single_slice
//     }
//
//     pub fn from_utf8(v: &[u8]) -> Self {
//         let extension_message_id: u8 = u8::from_be(v[0]);
//         assert_eq!(extension_message_id, 0, "Extension Message id has to be 0");
//         let bencoded_dictionary: String;
//         unsafe {
//             bencoded_dictionary = String::from_utf8_unchecked(v[1..].into());
//             // .expect(&format!("Parsing utf8 to string failed: {:?}", &v[93]));
//         }
//
//         ExtensionPayload {
//             extension_id: extension_message_id,
//             dict: bencoded_dictionary.into(),
//         }
//     }
// }
//
// #[cfg(test)]
// mod extension_payload_test {
//     use crate::message::ExtensionPayload;
//
//     #[test]
//     fn test_serialize_and_deserialized() {
//         let bencoded_dict = "d1:md11:ut_metadatai13eee";
//         // eprintln!("{:?}", bencoded_dict);
//         let my_struct = ExtensionPayload {
//             extension_id: 0,
//             dict: bencoded_dict.into(),
//         };
//         let seriazlied_struct = my_struct.to_vector();
//         let deserialized_struct = ExtensionPayload::from_utf8(&seriazlied_struct);
//         println!("{:?}", deserialized_struct);
//         assert_eq!(
//             my_struct.extension_id, deserialized_struct.extension_id,
//             "Should be equal"
//         );
//         assert_eq!(my_struct.dict, deserialized_struct.dict, "Should be equal");
//     }
// }

pub struct MessageFramer;
const MAX: usize = 2 * 16 * 1024; // 2^15

/// The 1st 4 bytes gives playload length + message_type
impl Decoder for MessageFramer {
    type Item = Message;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // println!("DEcode lenght {}", src.len());
        if src.len() < 5 {
            // Not enough data to read length and tag marker.
            return Ok(None);
        }

        // Read length marker.
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;
        // println!("Payload lenght {}", length);

        // Check that the length is not too large to avoid a denial of
        // service attack where the server runs out of memory.
        if length > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", length),
            ));
        }

        if src.len() < 4 + length {
            // The full string has not yet arrived.
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(4 + length - src.len());

            // We inform the Framed that we need more bytes to form the next
            // frame.
            return Ok(None);
        }

        // Use advance to modify src such that it no longer contains
        // this frame.
        let data = if src.len() > 5 {
            src[5..5 + length - 1].to_vec()
        } else {
            Vec::new()
        };
        // Convert the data to the appropriate Message
        let message_tag = MessageTag::tag_to_type(&src[4]).expect("Invalid msg type");
        src.advance(4 + length);
        let mut payload = Payload::SimplePayload(data.clone());
        if message_tag == MessageTag::Extension {
            let extension_payload: ExtensionPayload =
                serde_bencode::from_bytes(&data).expect("Serde bencode failed");
            payload = Payload::ExtendedPayload(extension_payload);
        }
        Ok(Some(Message {
            message_tag,
            payload,
        }))
    }
}

impl Encoder<Message> for MessageFramer {
    type Error = std::io::Error;

    fn encode(&mut self, message: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Don't send the message if it is longer than the other end will
        // accept.
        let payload = match &message.payload {
            Payload::SimplePayload(vector) => vector,
            Payload::ExtendedPayload(extension_payload_struct) => {
                &extension_payload_struct.to_vector()
            }
        };
        let payload_length = payload.len();
        if payload_length + 1 > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Payload of length {} is too large.", payload_length),
            ));
        }

        // Convert the length into a byte array.
        // The cast to u32 cannot overflow due to the length check above.
        let len_slice = u32::to_be_bytes(payload_length as u32 + 1);

        // Reserve space in the buffer.
        dst.reserve(5 + payload_length);

        // Write the length and string to the buffer.
        dst.extend_from_slice(&len_slice);
        dst.extend_from_slice(&[MessageTag::type_to_tag(&message.message_tag)]);
        dst.extend_from_slice(payload);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use tokio_util::codec::Decoder;

    #[test]
    fn test_my_message_decoder() {
        // Simulate a Bitfield message with payload [0xDE, 0xAD, 0xBE, 0xEF]
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xDE, 0xAD, 0xBE, 0xEF];
        let length = payload.len() as u32 + 1;

        let mut buf = BytesMut::new();

        // Write length (4 bytes, BE)
        buf.extend_from_slice(&length.to_be_bytes());

        // Write message tag byte (e.g., 5 for Bitfield)
        buf.extend_from_slice(&[5]);

        // Write payload
        buf.extend_from_slice(&payload);

        // Run decoder
        let mut decoder = MessageFramer {};
        let result = decoder.decode(&mut buf).expect("Decoding failed");

        // Assert decoded message
        match result {
            Some(msg) => {
                assert_eq!(msg.message_tag, MessageTag::Bitfield);
                assert_eq!(msg.payload, Payload::SimplePayload(payload));
            }
            None => panic!("Expected a decoded message, got None"),
        }
    }

    #[test]
    fn test_my_message_decoder_2() {
        // Simulate a incomplete Bitfield message with payload
        // we want the total payload to be of 8 bytes but we make it 7 to simulate incompleteness
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xDE, 0xAD, 0xBE];
        let length = 8 as u32 + 1;

        let mut buf = BytesMut::new();

        // Write length (4 bytes, BE)
        buf.extend_from_slice(&length.to_be_bytes());

        // Write message tag byte (e.g., 5 for Bitfield)
        buf.extend_from_slice(&[5]);

        // Write payload
        buf.extend_from_slice(&payload);

        // Run decoder
        let mut decoder = MessageFramer {};
        let result = decoder.decode(&mut buf).expect("Decoding failed");

        assert_eq!(result, None::<Message>);
    }

    #[test]
    fn test_my_message_decoder_3() {
        // Simulate a incomplete Bitfield message with just 4 bits
        let length = 4 as u32;

        let mut buf = BytesMut::new();

        // Write length (4 bytes, BE)
        buf.extend_from_slice(&length.to_be_bytes());

        // Run decoder
        let mut decoder = MessageFramer {};
        let result = decoder.decode(&mut buf).expect("Decoding failed");

        assert_eq!(result, None::<Message>)
    }
}

pub struct RequestPayload {
    /// the zero-based piece index
    pub index: u32,
    ///  the zero-based byte offset within the piece
    /// This'll be 0 for the first block, 2^14 for the second block, 2*2^14 for the third block etc.
    pub begin: u32,
    /// the length of the block in bytes
    /// This'll be 2^14 (16 * 1024) for all blocks except the last one.
    pub length: u32,
}

impl RequestPayload {
    pub fn to_vec(&self) -> Vec<u8> {
        // let mut vector: Vec<u8> = Vec::new();
        let mut buf: [u8; 12] = [0u8; 12];
        buf[0..4].copy_from_slice(&self.index.to_be_bytes());
        buf[4..8].copy_from_slice(&self.begin.to_be_bytes());
        buf[8..12].copy_from_slice(&self.length.to_be_bytes());
        return buf.into();
    }
}
pub struct ReceivePayload {
    /// the zero-based piece index
    pub index: u32,
    ///  the zero-based byte offset within the piece
    /// This'll be 0 for the first block, 2^14 for the second block, 2*2^14 for the third block etc.
    pub begin: u32,
    /// the data for the piece, usually 2^14 bytes long
    pub block: Vec<u8>,
}

impl ReceivePayload {
    pub fn new(payload: &mut Vec<u8>) -> Self {
        let index = u32::from_be_bytes(payload[0..4].try_into().expect("slice length not 4"));
        let begin = u32::from_be_bytes(payload[4..8].try_into().expect("slice length not 4"));
        Self {
            index: index,
            begin: begin,
            block: payload.split_off(8),
        }
    }
}
