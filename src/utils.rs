use crate::{
    handshake::Handshake,
    magnet::Magnet,
    message::{Message, MessageFramer, MessageTag, Payload, ReceivePayload, RequestPayload},
    request::{Request, Response},
    torrent::Torrent,
};
use anyhow::Context;
use futures_util::{sink::SinkExt, stream::StreamExt};
use sha1::{Digest, Sha1};
use std::{fs, net::SocketAddrV4};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::Framed;
use urlencoding::encode_binary;

//string have 4:home
//numbers have i-3e
//list [a,b] = l1:a1:be
//dictionary {'cow': 'moo', 'spam': 'eggs'} = d3:cow3:moo4:spam4:eggse
pub fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {
    match &encoded_value.chars().next() {
        Some('i') => {
            if let Some((digit, rest)) =
                encoded_value
                    .split_at(1)
                    .1
                    .split_once('e')
                    .and_then(|(digits, rest)| {
                        let n = digits.parse::<i64>().ok()?;
                        Some((n, rest))
                    })
            {
                return (digit.into(), rest);
            }
        }
        Some('l') => {
            let mut res = Vec::new();
            let mut rem = encoded_value.split_at(1).1;
            while let Some(c) = rem.chars().next() {
                if c == 'e' {
                    break;
                }
                let (result, rest) = decode_bencoded_value(rem);
                res.push(result);
                rem = rest;
            }
            return (res.into(), &rem[1..]);
        }
        Some('d') => {
            let mut dict = serde_json::Map::new();
            let mut rem = encoded_value.split_at(1).1;
            while !rem.is_empty() && !rem.starts_with('e') {
                let (key, rest) = decode_bencoded_value(rem);
                let (value, rest) = decode_bencoded_value(rest);
                if let serde_json::Value::String(key_string) = key {
                    dict.insert(key_string, value);
                } else {
                    panic!("Dict keys should be strings")
                }
                rem = rest;
            }
            return (dict.into(), &rem[1..]);
        }
        Some('0'..='9') => {
            if let Some((len, rest)) = encoded_value.split_once(':') {
                if let Ok(len) = len.parse::<usize>() {
                    return (rest[..len].to_string().into(), &rest[len..]);
                }
            }
        }
        _ => {}
    }
    panic!("Unhandled encoded value: {}", encoded_value)
}

/// This file will contain all the helper function used in main
pub fn read_and_deserialize_torrent(info: &String) -> anyhow::Result<Torrent> {
    let content = fs::read(info).context("Read file")?;
    let tor: Torrent = serde_bencode::from_bytes(&content).context("Convert file to a struct")?;
    Ok(tor)
}

pub async fn get_peers_from_tracker_url(info: &String) -> anyhow::Result<(Response, Torrent)> {
    let tor: Torrent =
        read_and_deserialize_torrent(info).context("Unable to read and deserialize")?;
    let info_hash = tor.info_hash();
    let left = tor.info.length;
    let encoded_info_hash = encode_binary(&info_hash).into_owned();
    let request_body = Request {
        peer_id: "123456789abcdefghijk".to_string(),
        port: 6881,
        downloaded: 0,
        uploaded: 0,
        left: left,
        compact: 1,
    };
    let header = serde_urlencoded::to_string(&request_body).context("Serder Url Encoding")?;
    let url = format!(
        "{}?{}&info_hash={}",
        tor.announce, header, encoded_info_hash
    );
    let response = reqwest::get(url).await.context("Query Tracker")?;
    let response = response.bytes().await.context("Fetch tracker response")?;
    let response: Response =
        serde_bencode::from_bytes(&response).context("Decoding response to response struct")?;
    Ok((response, tor))
}

pub async fn establish_handshake(
    info_hash: [u8; 20],
    peer: &SocketAddrV4,
    reserved: [u8; 8],
) -> anyhow::Result<(TcpStream, String)> {
    let mut tcp_stream = TcpStream::connect(peer)
        .await
        .context("TCP connection to peer")?;
    let peer_id: [u8; 20] = *b"ABCDEFGHIJKLMNOPQRST"; // exactly 20 bytes
    let handshake_message = Handshake {
        protocol_name: *b"BitTorrent protocol",
        protocol_length: 19,
        reserved: reserved,
        info_hash: info_hash,
        peer_id: peer_id,
    };
    tcp_stream
        .write_all(&handshake_message.as_bytes())
        .await
        .context("Sending Handshake")?;
    let mut res = [0u8; 68];
    tcp_stream
        .read_exact(&mut res)
        .await
        .context("Read from peers")?;
    let peer_id = hex::encode(&res[48..]);
    Ok((tcp_stream, peer_id))
}

pub async fn establish_handshake_and_download(
    output: &String,
    info: &String,
    index: Option<usize>,
    reserved: [u8; 8],
) -> anyhow::Result<()> {
    let (response, tor) = get_peers_from_tracker_url(info)
        .await
        .context("Unable to get response")?;
    let peer = &response.peers.0[0];
    let info_hash = &tor.info_hash();
    // establish handshake
    let (tcp_stream, _peer_id) = establish_handshake(*info_hash, peer, reserved)
        .await
        .context("Unable to establish handhshake")?;

    // open up a bidirectional tcp socket for communication
    let codec = MessageFramer;
    let mut tcp_stream = Framed::new(tcp_stream, codec);
    let message_received = tcp_stream
        .next()
        .await
        .expect("Expecting a btifield")
        .context("Message was invalid")?;
    assert_eq!(message_received.message_tag, MessageTag::Bitfield);

    let message_to_send = Message {
        message_tag: MessageTag::Interested,
        payload: Payload::SimplePayload(Vec::new()),
    };
    let _message_sent = tcp_stream.send(message_to_send).await;
    //TODO Need a better way of checking this
    //assert_eq!(message_sent.unwrap(), ());

    let message_received = tcp_stream
        .next()
        .await
        .expect("Expecting a unchoke")
        .context("Message was invalid")?;
    assert_eq!(message_received.message_tag, MessageTag::Unchoke);

    // fetching pieces sequentially
    // not using any pipelining
    let mut res: Vec<u8> = Vec::new();
    if let Some(piece_index) = index {
        res = fetch_a_piece(&tor, &mut tcp_stream, piece_index)
            .await
            .context("Fetch a piece failed")?;
    } else {
        res = fetch_all_pieces(&tor, &mut tcp_stream)
            .await
            .context("Fetch all piece failed")?;
    }

    tokio::fs::write(&output, res)
        .await
        .context("write out downloaded piece")?;
    Ok(())
}

async fn fetch_a_piece(
    tor: &Torrent,
    tcp_stream: &mut Framed<TcpStream, MessageFramer>,
    piece_index: usize,
) -> anyhow::Result<Vec<u8>> {
    let num_of_pieces = *&tor.info.pieces.0.len();
    let piece_size = if piece_index < num_of_pieces - 1 {
        *&tor.info.pieces_length
    } else {
        &tor.info.length - (&tor.info.pieces_length * (num_of_pieces - 1))
    };
    println!(
        "Piece index = {} and Piece size = {}",
        piece_index, piece_size
    );
    let num_of_blocks = piece_size.div_ceil(16 * 1024);
    // println!("Number of blocks{}",num_of_blocks);
    let mut blocks: Vec<u8> = Vec::new();
    let mut begin = 0;
    for block in 0..num_of_blocks {
        let block_size = if block < num_of_blocks - 1 {
            16 * 1024
        } else {
            piece_size - (16 * 1024 * (num_of_blocks - 1))
        };
        println!("Block index = {} and Block size = {}", block, block_size);
        let request_message = RequestPayload {
            index: piece_index as u32,
            begin: begin,
            length: block_size as u32,
        };
        let payload = request_message.to_vec();
        let message_to_send = Message {
            message_tag: MessageTag::Request,
            payload: Payload::SimplePayload(payload),
        };
        let message_sent = tcp_stream.send(message_to_send).await;
        // println!("Sent {:?}", message_sent);
        //TODO need a better check
        //assert_eq!(message_sent.unwrap(), ());

        let message_received = tcp_stream
            .next()
            .await
            .expect("Expexting a unchoke")
            .context("Message was invalid")?;
        assert_eq!(message_received.message_tag, MessageTag::Piece);
        if let Payload::SimplePayload(mut vector) = message_received.payload {
            assert!(!&vector.is_empty());
            let received_payload = ReceivePayload::new(&mut vector);
            assert_eq!(received_payload.index, piece_index as u32);
            assert_eq!(received_payload.begin, begin);
            assert!(!received_payload.block.is_empty());
            begin += 16 * 1024;
            // store each block
            blocks.extend_from_slice(&received_payload.block);
        } else {
            println!("Extended payload received");
        }
    }
    // check hash
    let mut hasher = Sha1::new();
    hasher.update(&blocks);
    let res = hex::encode(hasher.finalize());
    assert_eq!(res, hex::encode(tor.info.pieces.0[piece_index]));
    // add to slice
    // pieces.extend_from_slice(&blocks);
    // blocks.clear();
    Ok(blocks)
}

async fn fetch_all_pieces(
    tor: &Torrent,
    tcp_stream: &mut Framed<TcpStream, MessageFramer>,
) -> anyhow::Result<Vec<u8>> {
    let mut pieces: Vec<u8> = Vec::new();
    let num_of_pieces = *&tor.info.pieces.0.len();
    println!("THe number of pices is {}", num_of_pieces);
    for piece in 0..num_of_pieces {
        let res = fetch_a_piece(&tor, tcp_stream, piece)
            .await
            .context("Fetch a piece failed for index")?;
        pieces.extend_from_slice(&res);
    }
    Ok(pieces)
}

pub async fn get_peers_from_magnet(magnet: &Magnet) -> anyhow::Result<Response> {
    let encoded_info_hash = encode_binary(&magnet.info_hash_to_slice()).into_owned();
    //TODO: the left is unknown so use a non zero value
    let request_body = Request {
        peer_id: "123456789abcdefghijk".to_string(),
        port: 6881,
        downloaded: 0,
        uploaded: 0,
        left: 1000,
        compact: 1,
    };
    let header = serde_urlencoded::to_string(&request_body).context("Serder Url Encoding")?;
    let url = format!("{}?info_hash={}&{}", &magnet.url, encoded_info_hash, header);
    let response = reqwest::get(url).await.context("Query Tracker")?;
    let response = response.bytes().await.context("Fetch tracker response")?;
    let response: Response =
        serde_bencode::from_bytes(&response).context("Decoding response to response struct")?;
    Ok(response)
}
