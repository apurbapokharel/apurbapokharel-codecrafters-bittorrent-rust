use tokio_util::codec::{Decoder, Encoder};
use bytes::{BytesMut, Buf};

#[derive(Debug, PartialEq, Eq)]
pub enum MessageTag{
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield,
    Request,
    Piece,
    Cancel
}

impl MessageTag {
    pub fn type_to_tag(&self) -> u8{
        match self {
            Self::Choke                 => 0,
            Self::Unchoke               => 1,
            Self::Interested            => 2,
            Self::NotInterested         => 3,
            Self::Have                  => 4,
            Self::Bitfield              => 5,
            Self::Request               => 6,
            Self::Piece                 => 7,
            Self::Cancel                => 8,
        }
    }
    pub fn tag_to_type(tag:&u8) -> Option<MessageTag>{
        match tag  {
            0 => Some(MessageTag::Choke),
            1 => Some(MessageTag::Unchoke),
            2 => Some(MessageTag::Interested),
            3 => Some(MessageTag::NotInterested),
            4 => Some(MessageTag::Have),
            5 => Some(MessageTag::Bitfield),
            6 => Some(MessageTag::Request),
            7 => Some(MessageTag::Piece),
            8 => Some(MessageTag::Cancel),
            _ => None
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message{
    pub message_tag: MessageTag,
    pub payload: Vec<u8>
}
pub struct MessageFramer;
const MAX: usize = 16 * 1024;

impl Decoder for MessageFramer {
    type Item = Message;
    type Error = std::io::Error;

    fn decode(
        &mut self,
        src: &mut BytesMut
    ) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 5 {
            // Not enough data to read length and tag marker.
            return Ok(None);
        }

        // Read length marker.
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;
        // println!("Length {}", length);

        // Check that the length is not too large to avoid a denial of
        // service attack where the server runs out of memory.
        if length > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", length)
            ));
        }

        if src.len() < 4 + 1 + length {
            // The full string has not yet arrived.
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(4 + 1 + length - src.len());

            // We inform the Framed that we need more bytes to form the next
            // frame.
            return Ok(None);
        }

        // Use advance to modify src such that it no longer contains
        // this frame.
        let data = if src.len() > 5 {
            src[5..5 + length].to_vec()
        } else {
            Vec::new()
        };
        // Convert the data to the appropriate Message
        let message_tag = MessageTag::tag_to_type(&src[4]).expect("Invalid msg type");
        src.advance(5 + length);

        Ok(Some(
            Message{
                message_tag: message_tag,
                payload: data
            }
            )
        )
    }
}

impl Encoder<Message> for MessageFramer {
    type Error = std::io::Error;

    fn encode(&mut self, message: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Don't send the message if it is longer than the other end will
        // accept.
        if message.payload.len() > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Payload of length {} is too large.", message.payload.len())
            ));
        }

        // Convert the length into a byte array.
        // The cast to u32 cannot overflow due to the length check above.
        let len_slice = u32::to_be_bytes(message.payload.len() as u32);

        // Reserve space in the buffer.
        dst.reserve(5 + message.payload.len());

        // Write the length and string to the buffer.
        dst.extend_from_slice(&len_slice);
        dst.extend_from_slice(&[MessageTag::type_to_tag(&message.message_tag)]);
        dst.extend_from_slice(&message.payload);
        Ok(())
    }
}

// #[test]
// fn test_one(){
//     assert_eq!(1,1,"Not eq");

// }
#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use tokio_util::codec::Decoder;

    #[test]
    fn test_my_message_decoder() {
        // Simulate a Bitfield message with payload [0xDE, 0xAD, 0xBE, 0xEF]
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xDE, 0xAD, 0xBE, 0xEF];
        let length = payload.len() as u32;

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
                assert_eq!(msg.payload, payload);
            }
            None => panic!("Expected a decoded message, got None"),
        }
    }

    #[test]
    fn test_my_message_decoder_2() {
        // Simulate a incomplete Bitfield message with payload
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xDE, 0xAD, 0xBE];
        let length = 8 as u32;

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
