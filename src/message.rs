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
    #[serde(default)]
    pub extension_id: u8,
    /// Dictionary of supported extension messages which maps names of extensions to an extended message ID for each extension message.
    pub m: M,

    /// Local TCP listen port
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub p: u8,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub metadata_size: u8,

    /// Client name and version (as utf8)
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub v: String,

    /// ip address of the sending peer (maybe IPV4 or IPV6)
    #[serde(default = "default_peer")]
    #[serde(skip_serializing_if = "is_default")]
    pub yourip: PeerIP,

    /// If this peer has an IPv6 interface, this is the compact representation of that address (16 bytes)
    #[serde(default = "ipv6_default")]
    #[serde(skip_serializing_if = "is_ipv6_default")]
    pub ipv6: Ipv6Addr,

    /// If extend_from_slices peer has an IPv4 interface, this is the compact representation of that address (4 bytes).
    #[serde(default = "ipv4_default")]
    #[serde(skip_serializing_if = "is_ipv4_default")]
    pub ipv4: Ipv4Addr,

    /// An integer, the number of outstanding request messages this client supports without dropping any. The default in in libtorrent is 250.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub reqq: u8,
}

// pub struct ExtensionPayload {
//     #[serde(default)]
//     pub extension_id: u8,
//     /// Dictionary of supported extension messages which maps names of extensions to an extended message ID for each extension message.
//     pub m: M,
//     #[serde(default)]
//     pub metadata_size: u8
// }

// #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
// pub struct ExtensionPayload {
//     pub extension_id: u8,
//     pub m: M,
//     #[serde(default)]
//     #[serde(skip_serializing_if = "String::is_empty")]
//     pub string: String,
//     // #[serde(default = "ipv6_default")]
//     // pub ipv6: Ipv6Addr,  
//     // #[serde(default = "default_peer")]
//     // pub yourip: PeerIP,
//     // #[serde(default = "ipv4_default")]
//     // pub ipv4: Ipv4Addr,
// }

fn is_zero(x: &u8) -> bool {
    *x == 0
}

fn is_ipv4_default(ipv4: &Ipv4Addr) -> bool{
    ipv4.eq(&Ipv4Addr::UNSPECIFIED)
} 

fn is_ipv6_default(ipv6: &Ipv6Addr) -> bool{
    ipv6.eq(&Ipv6Addr::UNSPECIFIED)
} 

fn is_default(peer_ip: &PeerIP) -> bool {
    match peer_ip {
        PeerIP::Ipv4(ip)=> is_ipv4_default(ip),
        PeerIP::Ipv6(ip)=> is_ipv6_default(ip)
    }
}

pub fn ipv6_default() -> Ipv6Addr {
    Ipv6Addr::UNSPECIFIED
}

pub fn ipv4_default() -> Ipv4Addr {
    Ipv4Addr::UNSPECIFIED
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct M {
    pub ut_metadata: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub ut_pex: u8,
}


#[derive(Debug, PartialEq, Eq)]
pub enum PeerIP {
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

pub fn default_peer() -> PeerIP {
    PeerIP::Ipv4(ipv4_default())
}


mod peerip{
    use std::net::{Ipv4Addr, Ipv6Addr};

    use serde::de::{ Deserialize};
    use serde::ser::{Serialize, Serializer};
    use crate::message::PeerIP;

    struct IPeerIp;

    impl<'de> serde::de::Visitor<'de> for IPeerIp {
        type Value = PeerIP;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a byte string whose length is a multiple of 20")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error, {
                    if ! (v.len() == 4 as usize || v.len() == 16 as usize) {
                        return Err(E::custom(format!("Expecting length of 4 or 6")))
                    }

                    let peer = 
                        if v.len() == 4{
                            PeerIP::Ipv4(Ipv4Addr::new(v[0],v[1],v[2],v[3]))
                        } else {
                            let u16_vector: Vec<u16> = 
                                v.chunks_exact(2)
                                    .map(|chunk|{
                                       u16::from_be_bytes([chunk[0], chunk[1]])
                                    }).collect();
                            PeerIP::Ipv6(Ipv6Addr::new(
                                    u16_vector[0],
                                    u16_vector[1],
                                    u16_vector[2],
                                    u16_vector[3],
                                    u16_vector[4],
                                    u16_vector[5],
                                    u16_vector[6],
                                    u16_vector[7])
                                )
                        };
                    Ok(peer)
        }
    }


    impl<'de> Deserialize<'de> for PeerIP {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_bytes(IPeerIp)
        }
    }

    impl Serialize for PeerIP {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                PeerIP::Ipv4(v4) => {
                    let a : Vec<u8> = v4.octets().to_vec();
                    serializer.serialize_bytes(&a)
                },
                PeerIP::Ipv6(v6) => {
                    let a : Vec<u8> = v6.octets().to_vec();
                    serializer.serialize_bytes(&a)
                }
            }
        }
    }

}
// d1:md11:ut_metadatai1e6:ut_pexi2ee13:metadata_sizei132e4:reqqi250e1:v10:Rain 0.0.06:yourip4:/.e

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
            // println!("BYTES {:?}", data);
            // let data: &[u8] = &[100, 49, 58, 109, 100, 49, 49, 58, 117, 116, 95, 109, 101, 116, 97, 100, 97, 116, 97, 105, 49, 101, 54, 58, 117, 116, 95, 112, 101, 120, 105, 50, 101, 101, 49, 51, 58, 109, 101, 116, 97, 100, 97, 116, 97, 95, 115, 105, 122, 101, 105, 49, 51, 50, 101, 101];
            // let data: &[u8] = &[100, 49, 58, 109, 100, 49, 49, 58, 117, 116, 95, 109, 101, 116, 97, 100, 97, 116, 97, 105, 49, 101, 54, 58, 117, 116, 95, 112, 101, 120, 105, 50, 101, 101, 49, 51, 58, 109, 101, 116, 97, 100, 97, 116, 97, 95, 115, 105, 122, 101, 105, 49, 51, 50, 101, 52, 58, 114, 101, 113, 113, 105, 50, 53, 48, 101, 49, 58, 118, 49, 48, 58, 82, 97, 105, 110, 32, 48, 46, 48, 46, 48, 54, 58, 121, 111, 117, 114, 105, 112, 52, 58, 47, 4, 16, 46, 101];
            assert_eq!(*&data[0],0 as u8,"extension id should be 0");
            let extension_payload: ExtensionPayload =
                serde_bencode::from_bytes(&data[1..]).expect("Serde bencode failed");
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
                &serde_bencode::to_bytes(&extension_payload_struct).expect("Serialization failed")
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
