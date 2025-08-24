use core::panic;

use bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use crate::{
    constant, 
    torrent::Info,
    extension::{
        extensionhandshake::ExtensionHandshake, 
        extensionmetadata::{DataMetaData, ExtensionMetadata, MetaData}, 
        extensionpayload::{ExtensionPayload, ExtensionType}
    }
};

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

pub struct MessageFramer;
const MAX: usize = 2 * 16 * 1024; // 2^15
const EXTNSION_ID: u8 = constant::get_extension_id();

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
            match data[0] {
                0 => {
                    let extension_handshake: ExtensionHandshake =
                        serde_bencode::from_bytes(&data[1..]).expect("Serde bencode failed");
                        payload = Payload::ExtendedPayload(ExtensionPayload { 
                            extension_id: 0, 
                            payload: ExtensionType::ExtensionHandshakeMessage(extension_handshake)  
                    }); 
                },
                v if v == EXTNSION_ID => {
                    let extension_metadata_data: DataMetaData = serde_bencode::from_bytes(&data[1..]).expect("Serde bencode failed");                   
                    let i = &data[1..].len() - extension_metadata_data.total_size as usize + 1;
                    let info : Info = serde_bencode::from_bytes(&data[i..]).expect("Conversion to Info failed");
                    payload = Payload::ExtendedPayload(ExtensionPayload { 
                        extension_id: 0, 
                        payload: ExtensionType::MetaDataMessage(ExtensionMetadata::Data(extension_metadata_data, info))  
                    }); 
                },
                _ => panic!("Unexpected extension payload")
            };
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
                &extension_payload_struct.to_vec()
                // &serde_bencode::to_bytes(&extension_payload_struct).expect("Serialization failed")
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

pub mod requestpayload{
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

}


#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use tokio_util::codec::Decoder;
    use std::{fs, panic};

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

    #[test]
    fn test_my_message_decoder_4() {
        // test to see if the 1st bit of the encoded codec is 0 
        
        let content = fs::read("magnet.file").expect("Read file");
        let extension_handshake: ExtensionHandshake = serde_bencode::from_bytes(&content).expect("Convert file to a struct");        
        let extension_payload_payload = ExtensionPayload { 
                extension_id: 0, 
                payload: ExtensionType::ExtensionHandshakeMessage(extension_handshake) 
        };
        
        let bytes_payload = extension_payload_payload.to_vec();
        assert_eq!(bytes_payload[0],0,"payload length mimatch");
    }
}