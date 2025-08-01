use clap::builder::styling;
use serde::{de, Serialize, Serializer, Deserialize};
use peers::Peers;
#[derive(Debug, Serialize, Deserialize)]
pub struct Request{
    pub peer_id: String,
    pub port: u16,
    pub uploaded: usize,
    pub downloaded: usize,
    pub left: usize,
    pub compact: u8
}

#[derive(Debug, Deserialize)]
pub struct Response{
    pub interval: usize,
    pub peers: Peers
}

mod peers{
    use serde::de::{ Deserialize};
    use serde::ser::{Serialize, Serializer};
    use std::net::Ipv4Addr;
    use std::path::Display;
    #[derive(Debug)]
    pub struct Peers(pub Vec<(Ipv4Addr, u16)>);
    struct IPeers;

    impl<'de> serde::de::Visitor<'de> for IPeers {
        type Value = Peers;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            // assert!(formatter % 20 == 0, "Length of pieces data mismatch");
            write!(formatter, "a byte representation of peer addresses, where each address is 6 bytes")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error, {
                    if &v.len() % 6 != 0{
                        return Err(E::custom(format!("Not a multiple of 6")))
                    }

                    let mut vector: Vec<(Ipv4Addr, u16)> = Vec::new();

                    v.chunks_exact(6).for_each(|chunk| {
                        let a: [u8;4] = chunk[0..4].try_into().unwrap();
                        let a: Ipv4Addr = a.try_into().unwrap();
                        let b: u16 = u16::from_be_bytes(chunk[4..].try_into().unwrap());
                        vector.push((a,b));
                    });
                            
                    Ok(Peers(vector))
        }
    }


    impl<'de> Deserialize<'de> for Peers {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_bytes(IPeers)
        }
    }

    // impl Display for Peers{

    // }

    // impl Serialize for Peers {
    //     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    //     where
    //         S: Serializer,
    //     {
    //         let single_slice = self.0.concat();
    //         serializer.serialize_bytes(&single_slice)
    //     }
    // }

}


