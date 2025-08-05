use codecrafters_bittorrent::{
        handshake::Handshake, 
        message::{Message, MessageFramer, MessageTag, ReceivePayload, RequestPayload}, 
        request::{Request, Response}, 
        torrent::{Pieces, Torrent}};
use serde::{Serialize, Deserialize};
use serde_json::{self};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpStream}};
use tokio_util::codec::Framed;
// use futures_util::{stream::StreamExt};
use futures_util::{stream::StreamExt, sink::SinkExt};
use std::{fs, net::{SocketAddrV4}};
use anyhow::{Context};
use hex;
use clap::{Parser, Subcommand};
use urlencoding::encode_binary;
use sha1::{Digest, Sha1};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    operation: Type
}

#[derive(Debug, Subcommand)]
#[clap(rename_all="snake_case")]
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
    },
    DownloadPiece{
        #[arg(short)]
        output: String,
        info: String,
        index: usize
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
            let peer_id   = hex::encode(&res[48..]);
            println!("Peer ID: {}", peer_id);
        },
        Type::DownloadPiece { output, info, index } => {
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
            let peer = &response.peers.0[0];
            let mut tcp_stream = TcpStream::connect(peer).await.context("TCP connection to peer")?;
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
            let peer_id   = hex::encode(&res[48..]);
            // println!("Peer ID: {}", peer_id);

            let codec = MessageFramer;
            let mut tcp_stream = Framed::new(tcp_stream, codec);
            let message_received = tcp_stream
                .next()
                .await
                .expect("Expexting a btifield")
                .context("Message was invalid")?;
            assert_eq!(message_received.message_tag, MessageTag::Bitfield);

            let message_to_send = Message{message_tag: MessageTag::Interested, payload: Vec::new()};
            let message_sent = tcp_stream
                .send(message_to_send)
                .await;
            // println!("Sent {:?}", message_sent);
            assert_eq!(message_sent.unwrap(), ());

            let message_received = tcp_stream
                .next()
                .await
                .expect("Expexting a unchoke")
                .context("Message was invalid")?;
            assert_eq!(message_received.message_tag, MessageTag::Unchoke);

            // fetching pieces sequentially
            // not using any pipelining
            
            let mut pieces : Vec<u8> = Vec::new(); 
            let num_of_pieces = *&tor.info.pieces.0.len();
            for _ in 0..=0{
                let piece = *index as usize;
                let piece_size = if piece < num_of_pieces - 1 {
                    *&tor.info.pieces_length
                } else {
                    &tor.info.length - (&tor.info.pieces_length * (num_of_pieces - 1))
                };
                println!("Piece index = {} and Piece size = {}", piece, piece_size);

                let num_of_blocks = piece_size.div_ceil(16*1024);
                // println!("Number of blocks{}",num_of_blocks);

                let mut blocks : Vec<u8> = Vec::new(); 
                let mut begin = 0;
                for block in 0..num_of_blocks{
                    let block_size = if block < num_of_blocks - 1 {
                        16 * 1024
                    } else {
                        piece_size - (16 * 1024 * (num_of_blocks - 1))
                    };
                    println!("Block index = {} and Block size = {}", block, block_size);
                    let request_message = RequestPayload{
                        index: piece as u32,
                        begin: begin,
                        length: block_size as u32
                    };
                    let payload = request_message.to_vec();
                    let message_to_send = Message{message_tag: MessageTag::Request, payload: payload};
                    let message_sent = tcp_stream
                        .send(message_to_send)
                        .await;
                    // println!("Sent {:?}", message_sent);
                    assert_eq!(message_sent.unwrap(), ());

                    let mut message_received = tcp_stream
                        .next()
                        .await
                        .expect("Expexting a unchoke")
                        .context("Message was invalid")?;
                    assert_eq!(message_received.message_tag, MessageTag::Piece);
                    assert!(!message_received.payload.is_empty());
                    
                    let received_payload = ReceivePayload::new(&mut message_received.payload);
                    assert_eq!(received_payload.index, piece as u32);
                    assert_eq!(received_payload.begin, begin);
                    assert!(!received_payload.block.is_empty());
                    begin += 16 * 1024;

                    // store each block
                    blocks.extend_from_slice(&received_payload.block);
                }
                // check hash 
                let mut hasher = Sha1::new();
                hasher.update(&blocks);
                let res  = hex::encode(hasher.finalize());
                assert_eq!(res, hex::encode(tor.info.pieces.0[piece]));
                // add to slice
                pieces.extend_from_slice(&blocks);
                blocks.clear();
            }
            tokio::fs::write(&output, pieces)
                .await
                .context("write out downloaded piece")?;
            println!("Piece {index} downloaded to {}.", output);
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


