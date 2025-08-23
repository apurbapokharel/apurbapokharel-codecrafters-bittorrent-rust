use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ExtensionMetadata {
    /// {'msg_type': 0, 'piece': 0}
    Request(MetaData),
    /// {'msg_type': 1, 'piece': 0, 'total_size': 3425}
    Data(DataMetaData),
    /// {'msg_type': 2, 'piece': 0}
    Reject(MetaData)
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetaData{
    pub msg_type: u8,
    pub piece: u8
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataMetaData{
    pub msg_type: u8,
    pub piece: u8,
    pub total_size: u32
}

// impl ExtensionMetadata {
    
// }
#[cfg(test)]
mod tests {
    use crate::extension::extensionmetadata::{ExtensionMetadata, MetaData};

    #[test]
    fn test_bencode_enum() {
        let extension_meta_data = ExtensionMetadata::Request(
            MetaData{
                msg_type: 0, 
                piece: 0
            }
        );

        let bencoded_bytes = serde_bencode::to_bytes(&extension_meta_data).expect("Serialization failed");
        let decoded_utf8 = String::from_utf8(bencoded_bytes).expect("Conversion to string failed");
        assert_eq!(decoded_utf8.contains("Request"), false, "Incorrect serialization");
    }
}
