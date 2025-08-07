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
}