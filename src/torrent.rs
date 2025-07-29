use std:: {fs};
use serde::{Serialize, Deserialize, Deserializer, Serializer};
use anyhow::{Result};
use crate::{error::MyError};
use serde_bencode::value::Value;
pub use pieces::Pieces;

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    /// The URL of the tracker.
    pub announce: String,

    pub info: Info,
}

#[derive(Debug,Serialize, Deserialize)]
pub struct Info{
    pub length: usize,

    /// suggested name
    pub name: String,

    /// the # of bytes in each piece
    #[serde(rename = "piece length")]
    pub pieces_length: usize,

    // concatenated SHA-1 hashes of each piece
    pub pieces: Pieces,
}

impl Torrent {
    pub fn new(file_path: &str) -> Result<Self, MyError> {
        let content = fs::read(file_path)
            .map_err(|e| {
                eprintln!("Failed to read file: {e}");
                MyError::FileError
            })?;

        let torrent: Torrent = serde_bencode::from_bytes(&content)
            .map_err(|e| {
                eprintln!("Failed to parse bencode: {e}");
                MyError::InvalidInput
            })?;

        Ok(torrent)
    }
}

mod pieces{
    use serde::de::{self, Deserialize, Deserializer, Visitor};
    use serde::ser::{Serialize, Serializer};
    use std::fmt;

    #[derive(Debug)]
    pub struct Pieces(pub Vec<[u8;20]>);
    struct IPieces;

    impl<'de> serde::de::Visitor<'de> for IPieces {
        type Value = Pieces;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            // assert!(formatter % 20 == 0, "Length of pieces data mismatch");
            write!(formatter, "a byte string whose length is a multiple of 20")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error, {
                    if &v.len() % 20 != 0{
                        return Err(E::custom(format!("Not a multiple of 20")))
                    }

                    println!("after1 {:?}", v);
                    Ok(
                        Pieces( 
                        v.chunks_exact(20)
                        .map(|chunk| chunk.try_into().unwrap())
                        .collect()
                        )
                    )
        }
    }


    impl<'de> Deserialize<'de> for Pieces {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            // deserializer.deserialize_bytes(IPieces)
            let visitor = IPieces;
            print!("before");
            let docs = deserializer.deserialize_bytes(visitor)?;
            print!("after");
            Ok(docs)
        }
    }

    impl Serialize for Pieces {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let single_slice = self.0.concat();
            serializer.serialize_bytes(&single_slice)
        }
    }
}





