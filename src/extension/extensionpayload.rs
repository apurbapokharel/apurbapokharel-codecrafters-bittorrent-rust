use serde::{Deserialize, Serialize};
use crate::extension::{
    extensionhandshake::ExtensionHandshake,
    extensionmetadata::ExtensionMetadata
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionPayload{
    #[serde(default)]
    // this will be 0 for extensionHandshake and the peer's extensionID for ExtensionMetadata
    pub extension_id: u8,
    pub payload: ExtensionType
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ExtensionType {
    ExtensionHandshakeMessage(ExtensionHandshake),
    MetaDataMessage(ExtensionMetadata)
}

impl ExtensionPayload{
    pub fn to_vec(&self) -> Vec<u8>{
        let mut payload_vec = serde_bencode::to_bytes(&self.payload).expect("Serialization failed");
        let mut a: Vec<u8> = Vec::new();
        a.append(&mut self.extension_id.to_be_bytes().to_vec());
        a.append(&mut payload_vec);
        return a
    }
}

