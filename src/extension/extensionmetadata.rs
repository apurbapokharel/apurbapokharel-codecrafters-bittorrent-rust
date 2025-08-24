use serde::{Deserialize, Serialize};

use crate::torrent::Info;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ExtensionMetadata {
    /// {'msg_type': 0, 'piece': 0}
    Request(MetaData),
    /// {'msg_type': 1, 'piece': 0, 'total_size': 3425}
    Data(DataMetaData, Info),
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

#[cfg(test)]
mod tests {
    use crate::extension::extensionmetadata::{DataMetaData, ExtensionMetadata, MetaData};

    #[test]
    fn test_bencode_enum() {
        let meta_data = MetaData{
                msg_type: 0, 
                piece: 0
        };
        let extension_meta_data = ExtensionMetadata::Request(
            meta_data
        );

        let bencoded_bytes = serde_bencode::to_bytes(&extension_meta_data).expect("Serialization failed");
        let decoded_utf8 = String::from_utf8(bencoded_bytes.clone()).expect("Conversion to string failed");
        let back_to_struct: MetaData = serde_bencode::from_bytes(&bencoded_bytes).expect("Conversion failed");
        assert_eq!(decoded_utf8.contains("Request"), false, "Incorrect serialization");
        // assert_eq!(meta_data, back_to_struct, "Struct mismatch");
        
        // let b: &[u8] = &[100, 56, 58, 109, 115, 103];
        // let bencoded_bytes: DataMetaData = serde_bencode::from_bytes(&b).expect("Serialization failed");
        // println!("{:?}", bencoded_bytes);
        
        
        // let meta_data = DataMetaData{
        //     msg_type: 0, 
        //     piece: 0,
        //     total_size: 1245
        // };
        // let extension_meta_data = ExtensionMetadata::Data(
        //     meta_data.clone()
        // );
        // let bencoded_bytes = serde_bencode::to_bytes(&meta_data).expect("Serialization failed");
        // let bencoded_bytess = serde_bencode::to_bytes(&extension_meta_data).expect("Serialization failed");
        // println!("{:?}", bencoded_bytes.len());
        // println!("{:?}", bencoded_bytess);

    }
}
