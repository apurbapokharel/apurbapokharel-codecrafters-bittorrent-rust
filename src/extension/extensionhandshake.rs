use std::net::{Ipv4Addr, Ipv6Addr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionHandshake {
    /// Dictionary of supported extension messages which maps names of extensions to an extended message ID for each extension message.
    pub m: M,

    /// Local TCP listen port
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub p: u8,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub metadata_size: u8,

    /// Client name and version (as utf8)
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub v: String,

    /// ip address of the sending peer (maybe IPV4 or IPV6)
    #[serde(default = "default_peer")]
    #[serde(skip_serializing_if = "is_default")]
    pub yourip: PeerIP,

    /// If this peer has an IPv6 interface, this is the compact representation of that address (16 bytes)
    #[serde(default = "ipv6_default")]
    #[serde(skip_serializing_if = "is_ipv6_default")]
    pub ipv6: Ipv6Addr,

    /// If extend_from_slices peer has an IPv4 interface, this is the compact representation of that address (4 bytes).
    #[serde(default = "ipv4_default")]
    #[serde(skip_serializing_if = "is_ipv4_default")]
    pub ipv4: Ipv4Addr,

    /// An integer, the number of outstanding request messages this client supports without dropping any. The default in in libtorrent is 250.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub reqq: u8,
}

fn is_zero(x: &u8) -> bool {
    *x == 0
}

fn is_ipv4_default(ipv4: &Ipv4Addr) -> bool{
    ipv4.eq(&Ipv4Addr::UNSPECIFIED)
} 

fn is_ipv6_default(ipv6: &Ipv6Addr) -> bool{
    ipv6.eq(&Ipv6Addr::UNSPECIFIED)
} 

fn is_default(peer_ip: &PeerIP) -> bool {
    match peer_ip {
        PeerIP::Ipv4(ip)=> is_ipv4_default(ip),
        PeerIP::Ipv6(ip)=> is_ipv6_default(ip)
    }
}

pub fn ipv6_default() -> Ipv6Addr {
    Ipv6Addr::UNSPECIFIED
}

pub fn ipv4_default() -> Ipv4Addr {
    Ipv4Addr::UNSPECIFIED
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct M {
    pub ut_metadata: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub ut_pex: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PeerIP {
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

pub fn default_peer() -> PeerIP {
    PeerIP::Ipv4(ipv4_default())
}

mod peerip{
    use std::net::{Ipv4Addr, Ipv6Addr};

    use serde::de::{ Deserialize};
    use serde::ser::{Serialize, Serializer};
    use crate::extension::extensionhandshake::PeerIP;

    struct IPeerIp;

    impl<'de> serde::de::Visitor<'de> for IPeerIp {
        type Value = PeerIP;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a byte string whose length is a multiple of 20")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error, {
                    if ! (v.len() == 4 as usize || v.len() == 16 as usize) {
                        return Err(E::custom(format!("Expecting length of 4 or 6")))
                    }

                    let peer = 
                        if v.len() == 4{
                            PeerIP::Ipv4(Ipv4Addr::new(v[0],v[1],v[2],v[3]))
                        } else {
                            let u16_vector: Vec<u16> = 
                                v.chunks_exact(2)
                                    .map(|chunk|{
                                       u16::from_be_bytes([chunk[0], chunk[1]])
                                    }).collect();
                            PeerIP::Ipv6(Ipv6Addr::new(
                                    u16_vector[0],
                                    u16_vector[1],
                                    u16_vector[2],
                                    u16_vector[3],
                                    u16_vector[4],
                                    u16_vector[5],
                                    u16_vector[6],
                                    u16_vector[7])
                                )
                        };
                    Ok(peer)
        }
    }


    impl<'de> Deserialize<'de> for PeerIP {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_bytes(IPeerIp)
        }
    }

    impl Serialize for PeerIP {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                PeerIP::Ipv4(v4) => {
                    let a : Vec<u8> = v4.octets().to_vec();
                    serializer.serialize_bytes(&a)
                },
                PeerIP::Ipv6(v6) => {
                    let a : Vec<u8> = v6.octets().to_vec();
                    serializer.serialize_bytes(&a)
                }
            }
        }
    }

}
// d1:md11:ut_metadatai1e6:ut_pexi2ee13:metadata_sizei132e4:reqqi250e1:v10:Rain 0.0.06:yourip4:/.e

#[cfg(test)]
mod tests {
    use crate::{
        extension::{
            extensionhandshake::ExtensionHandshake,
            extensionpayload::{ExtensionPayload, ExtensionType},
        },
    };
    use std::{fs};

    #[test]
    fn test_extension_handshake_serialization() {
        let content = fs::read("magnet.file").expect("Read file");
        let extension_handshake: ExtensionHandshake = serde_bencode::from_bytes(&content).expect("Convert file to a struct");
        let payload_vec = serde_bencode::to_bytes(&extension_handshake).expect("Serialization failed");
        let utf_8 = String::from_utf8(payload_vec).unwrap();

        let extension_payload = ExtensionPayload { 
            extension_id: 0, 
            payload: ExtensionType::ExtensionHandshakeMessage(extension_handshake) 
        };

        let payload_vec_2 = serde_bencode::to_bytes(&extension_payload.payload).expect("Serialization failed");
        let utf_8_2 = String::from_utf8(payload_vec_2).unwrap();

        assert_eq!(utf_8, utf_8_2, "Should be equal");
    }
}