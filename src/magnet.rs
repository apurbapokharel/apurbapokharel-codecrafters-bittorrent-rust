use anyhow::Context;
use serde::{Deserialize};
use urlencoding::decode;
#[derive(Debug, Deserialize)]
pub struct Magnet{
    pub url: String,
    pub info_hash: String,
    pub magnet_name: String
}

impl Magnet{
    pub fn new(magnet_link: &String) -> anyhow::Result<Magnet>{
        let (left_string,url_string) = magnet_link
            .split_once("&tr=")
            .and_then(|(left,right)|{
                Some((left, decode(right).expect("UTF-8")))
            })
            .context("Splitting at url")?;

        let (info_hash, name )= left_string
            .split_once("&dn=")
            .and_then(|(left,name)|{
                left.split_once("btih:")
                .and_then(|(_,url)|{
                    Some((url, name))
                })
            }).context("Splitting for hash and name failed")?;     

        Ok(
            Self { 
                url: url_string.into_owned(), 
                info_hash: info_hash.into(),
                magnet_name: name.into()  
            }
        )
    }

    pub fn info_hash_to_slice(&self) -> [u8; 20] {
        let mut a: Vec<u8> = Vec::with_capacity(20);

        self.info_hash
            .as_bytes()
            .chunks_exact(2)
            .for_each(|chunk| {
                // Convert the two-byte chunk to a &str
                let hex_str = std::str::from_utf8(chunk).expect("Invalid UTF-8 in info_hash");
                // Parse it as a hex number (u8)
                let byte = u8::from_str_radix(hex_str, 16).expect("Invalid hex in info_hash");
                a.push(byte);
            });

        a.try_into().expect("Conversion to [u8; 20] failed")
    }

}