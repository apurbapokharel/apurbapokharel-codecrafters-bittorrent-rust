use serde::{Deserialize};

#[derive(Debug, Deserialize)]
pub struct Handshake{
    pub protocol_length: u8,
    pub protocol_name: [u8;19],
    ///  Representation of A single [u8;8]     
    pub reserved: [u8;8],
    /// String representation of A single [u8;20]
    pub info_hash: [u8;20],
    /// String representation of A single [u8;20]
    pub peer_id: [u8;20]
}

impl Handshake {
    pub fn as_bytes(&self) -> [u8; 68] {
        let mut buf = [0u8; 68];
        buf[0] = self.protocol_length;
        buf[1..20].copy_from_slice(&self.protocol_name);
        buf[20..28].copy_from_slice(&self.reserved);
        buf[28..48].copy_from_slice(&self.info_hash);
        buf[48..68].copy_from_slice(&self.peer_id);
        buf
    }
}
