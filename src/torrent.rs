use serde::{Serialize, Deserialize};
pub use pieces::Pieces;
use sha1::{Digest, Sha1};

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    /// The URL of the tracker.
    pub announce: String,

    pub info: Info,
}

impl Torrent{
    pub fn info_hash(&self) -> [u8;20]{
        let info_bencoded_bytes = serde_bencode::to_bytes(&self.info)
            .expect("Info Bencode failed");
        let mut hasher = Sha1::new();
        hasher.update(info_bencoded_bytes);
        hasher.finalize().try_into().expect("Type conversion failed")
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

mod pieces{
    use serde::de::{ Deserialize};
    use serde::ser::{Serialize, Serializer};

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

                    let pieces = 
                        Pieces( 
                            v.chunks_exact(20)
                            .map(|chunk| chunk.try_into().unwrap())
                            .collect()
                        );
                    Ok(pieces)
        }
    }


    impl<'de> Deserialize<'de> for Pieces {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_bytes(IPieces)
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






