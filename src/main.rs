#![allow(unused_imports)]
use codecrafters_bittorrent::torrent::Torrent;
use serde::{de, Serialize, Serializer};
use serde_bencode::value;
use serde_json::{self, Number, Value};
use std::{collections::HashMap, env, fs, path::PathBuf};
use anyhow::Context;
use sha1::{Digest, Sha1};
use hex;

fn main() -> anyhow::Result<()> {  
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(&encoded_value).0;
        println!("{decoded_value}");
    } 
    else if command == "info" {
        let content = fs::read(&args[2])
            .context("Failed to read file")?;
        // println!("{:?}",content.len());

        let tor: Torrent = serde_bencode::from_bytes(&content)
            .context("Failed to convert file to a struct")?;
        
        let info_bencoded_bytes = serde_bencode::to_bytes(&tor.info)
            .context("Info Bencode failed")?;
        let mut hasher = Sha1::new();
        hasher.update(info_bencoded_bytes);
        let result = hasher.finalize();

        let piece_length =  &tor.info.pieces_length;
        println!("Tracker URL: {}", &tor.announce);
        println!("Length: {}", &tor.info.length);
        println!("Info Hash: {}", hex::encode(result));
        println!("Piece Length: {}", &piece_length);
        println!("Piece Hashes:");

        for piece in &tor.info.pieces.0 {
            println!("{}", hex::encode(piece));
        }

    } 
    else {
        panic!("unknown command: {}", args[1])
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


