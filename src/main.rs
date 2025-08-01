#![allow(unused_imports)]
use codecrafters_bittorrent::{
        torrent::Torrent, 
        request::Request, 
        request::Response};
use reqwest::Client;
use serde::{de, Serialize, Serializer, Deserialize};
use serde_bencode::value;
use serde_json::{self, Number, Value};
use core::fmt;
use std::{collections::HashMap, env, fs, path::PathBuf, fmt::Display};
use anyhow::Context;
use hex;
use clap::{Arg, Parser, Subcommand};
use reqwest::Url;
use urlencoding::encode_binary;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    operation: Type
}

#[derive(Debug, Subcommand)]
enum Type {
    Decode{
        decode: String
    },
    Info{
        info: String
    },
    Peers{
        info: String
    }
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {  
    let arg = Args::parse();
    // println!("{:?}",arg);
    match &arg.operation {
        Type::Decode { decode } => {
            let decoded_value = decode_bencoded_value(&decode).0;
            println!("{decoded_value}");
        },
        Type::Info { info } => {
            let content = fs::read(info)
                .context("Read file")?;
            let tor: Torrent = serde_bencode::from_bytes(&content)
                .context("Convert file to a struct")?;

            let info_hash = tor.info_hash();
            let piece_length =  &tor.info.pieces_length;
            println!("Tracker URL: {}", &tor.announce);
            println!("Length: {}", &tor.info.length);
            println!("Info Hash: {}", hex::encode(info_hash));
            println!("Piece Length: {}", &piece_length);
            println!("Piece Hashes:");

            for piece in &tor.info.pieces.0 {
                println!("{}", hex::encode(piece));
            }
        },
        Type::Peers { info } => {
            let content = fs::read(info)
                .context("Reading file")?;
            let tor: Torrent = serde_bencode::from_bytes(&content)
                .context("Convert file to a struct")?;
            
            // let info_bencoded_bytes = serde_bencode::to_bytes(&tor.info).context("Info Bencode failed")?;
            // let mut hasher = Sha1::new();
            // hasher.update(info_bencoded_bytes);
            // let result = hasher.finalize();
            let info_hash = tor.info_hash();
            let left = tor.info.length.clone();
            let encoded_info_hash = encode_binary(&info_hash).into_owned();
            let request_body = Request{
                peer_id: "123456789abcdefghijk".to_string(),
                port: 6881,
                downloaded: 0,
                uploaded: 0,
                left: left,
                compact: 1
            };
            let header = serde_urlencoded::to_string(&request_body).context("Serder Url Encoding")?;
            // println!("TO {:?}", header);

            let url = format!(
                "{}?{}&info_hash={}",
                tor.announce,
                header,
                encoded_info_hash
            );
            let response = reqwest::get(url).await.context("Query Tracker")?;
            let response = response.bytes().await.context("Fetch tracker response")?;
            let response: Response = serde_bencode::from_bytes(&response).context("Decoding response to response struct")?;
            for (peer, port) in &response.peers.0{
                let a = format!(
                    "{}:{}",
                    peer,
                    port
                );
                println!("{}", a);
            }
        }
    }
    Ok(())
}

//string have 4:home
//numbers have i-3e
//list [a,b] = l1:a1:be
//dictionary {'cow': 'moo', 'spam': 'eggs'} = d3:cow3:moo4:spam4:eggse 
fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {
    match &encoded_value.chars().next() {
        Some('i') => {
            if let Some((digit, rest)) = 
                encoded_value.
                    split_at(1)
                    .1
                    .split_once('e')
                    .and_then(|(digits,rest)|{
                        let n = digits.parse::<i64>().ok()?;
                        return Some((n, rest))
                    }){
                        return (digit.into(), rest);
                    }
        }
        Some('l') => {
            let mut res = Vec::new();
            let mut rem  = encoded_value.split_at(1).1;
            while let Some(c) = rem.chars().next() {
                if c == 'e' {
                    break;
                }
                let (result, rest) = decode_bencoded_value(rem);
                res.push(result);
                rem = rest;
            }
            return (res.into(), &rem[1..])

        }
        Some('d') => {
            let mut dict = serde_json::Map::new();
            let mut rem = encoded_value.split_at(1).1;
            while !rem.is_empty() && !rem.starts_with('e'){
                let (key, rest) = decode_bencoded_value(rem);
                let (value, rest) = decode_bencoded_value(rest);
                if let serde_json::Value::String(key_string) = key{
                    dict.insert(key_string, value);
                } else{
                    panic!("Dict keys should be strings")
                }
                rem = rest;
            }
            return (dict.into(), &rem[1..])
        }
        Some('0'..='9') => {
            if let Some((len, rest)) = encoded_value.split_once(':'){
                if let Ok(len) = len.parse::<usize>() {
                    return (rest[..len].to_string().into(), &rest[len..]);  
                }
            }
        }
        _ => {}
    }    
    panic!("Unhandled encoded value: {}", encoded_value)
}


