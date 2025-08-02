use serde::{Serialize, Deserialize};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Response{
    pub interval: usize,
    pub peers: Peers
}

mod peers{
    use serde::de::{ Deserialize};
    use serde::ser::{Serialize, Serializer};
    use std::net::{Ipv4Addr, SocketAddrV4};
    #[derive(Debug)]
    pub struct Peers(pub Vec<SocketAddrV4>);
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

                    let mut vector: Vec<SocketAddrV4> = Vec::new();

                    v.chunks_exact(6).for_each(|chunk| {
                        let a: [u8;4] = chunk[0..4].try_into().unwrap();
                        let a: Ipv4Addr = a.try_into().unwrap();
                        let b: u16 = u16::from_be_bytes(chunk[4..].try_into().unwrap());
                        let socket_address = SocketAddrV4::new(a,b);
                        vector.push(socket_address);
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

    impl Serialize for Peers {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut single_slice = Vec::with_capacity(6 * self.0.len());
            for socket_address in &self.0 {
                single_slice.extend(socket_address.ip().octets());
                single_slice.extend(socket_address.port().to_be_bytes());
            }
            serializer.serialize_bytes(&single_slice)
        }
    }

}


