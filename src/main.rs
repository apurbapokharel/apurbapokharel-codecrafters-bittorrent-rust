use codecrafters_bittorrent::{
        handshake::Handshake, request::{Request, Response}, torrent::Torrent};
use serde::{Serialize, Deserialize};
use serde_json::{self};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use std::{fs, net::{SocketAddrV4}};
use anyhow::Context;
use hex;
use clap::{Parser, Subcommand};
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
    },
    Handshake{
        info: String,
        peers: SocketAddrV4
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Dummy {
    pub name: String,
    pub num: u16
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
            let url = format!(
                "{}?{}&info_hash={}",
                tor.announce,
                header,
                encoded_info_hash
            );
            let response = reqwest::get(url).await.context("Query Tracker")?;
            let response = response.bytes().await.context("Fetch tracker response")?;
            let response: Response = serde_bencode::from_bytes(&response).context("Decoding response to response struct")?;
            for socket_address in &response.peers.0{
                let a = format!(
                    "{}:{}",
                    socket_address.ip(),
                    socket_address.port()
                );
                println!("{}", a);
            }
        },
        Type::Handshake { info, peers } => {
            let content = fs::read(info)
                .context("Reading file")?;
            let tor: Torrent = serde_bencode::from_bytes(&content)
                .context("Convert file to a struct")?;
            let info_hash = tor.info_hash();
            let mut tcp_stream = TcpStream::connect(peers).await.context("TCP connection to peer")?;
            let reserved: [u8; 8] = [0; 8];
            let peer_id: [u8; 20]  = *b"ABCDEFGHIJKLMNOPQRST"; // exactly 20 bytes
            let handshake_message = Handshake{
                protocol_name: *b"BitTorrent protocol",
                protocol_length: 19,
                reserved: reserved,
                info_hash: info_hash,
                peer_id: peer_id
            };
            tcp_stream.write_all(&handshake_message.as_bytes()).await.context("Sending Handshake")?;
            let mut res = [0u8; 68];
            tcp_stream.read_exact(&mut res).await.context("Read from peers")?;
            let peer_id  = hex::encode(&res[48..]);
            println!("Peer ID: {}", peer_id);
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


