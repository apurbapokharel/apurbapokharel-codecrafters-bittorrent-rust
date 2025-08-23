use codecrafters_bittorrent::{
    extension::{
        extensionhandshake::ExtensionHandshake, extensionmetadata::{ExtensionMetadata, MetaData}, extensionpayload::{ExtensionPayload, ExtensionType}
    }, handshake::Handshake, magnet::Magnet, message::{ Message, MessageFramer, MessageTag, Payload}, torrent::Torrent, utils::{
        decode_bencoded_value, establish_handshake, establish_handshake_and_download,
        get_peers_from_magnet, get_peers_from_tracker_url, read_and_deserialize_torrent,
    }
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::Framed;
use std::{fs, net::SocketAddrV4};
use anyhow::Context;
use clap::{Parser, Subcommand};
use hex;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    operation: Type,
}

#[derive(Debug, Subcommand)]
#[clap(rename_all = "snake_case")]
enum Type {
    Decode {
        decode: String,
    },
    Info {
        info: String,
    },
    Peers {
        info: String,
        // #[arg(default_value_t = Reserved::default())]
        // reserved: Reserved
    },
    Handshake {
        info: String,
        peer: SocketAddrV4,
    },
    DownloadPiece {
        #[arg(short)]
        output: String,
        info: String,
        index: usize,
    },
    Download {
        #[arg(short)]
        output: String,
        info: String,
    },
    MagnetParse {
        magnet: String,
    },
    MagnetHandshake {
        magnet: String,
    },
    MagnetInfo{
        magnet: String
    }
}

// #[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
// pub struct Reserved {
//     pub bit: [u8;8],
// }

// impl fmt::Display for Reserved {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{:?}", self.bit)
//     }
// }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let arg = Args::parse();
    // println!("{:?}",arg);
    match &arg.operation {
        Type::Decode { decode } => {
            let decoded_value = decode_bencoded_value(&decode).0;
            println!("{decoded_value}");
        }
        Type::Info { info } => {
            let tor: Torrent =
                read_and_deserialize_torrent(info).context("Unable to read and deserialize")?;
            let info_hash = tor.info_hash();
            let piece_length = &tor.info.pieces_length;
            println!("Tracker URL: {}", &tor.announce);
            println!("Length: {}", &tor.info.length);
            println!("Info Hash: {}", hex::encode(info_hash));
            println!("Piece Length: {}", &piece_length);
            println!("Piece Hashes:");

            for piece in &tor.info.pieces.0 {
                println!("{}", hex::encode(piece));
            }
        }
        Type::Peers { info } => {
            // Type::Peers { info, reserved } => {
            // println!("{}", reserved);
            let (response, _) = get_peers_from_tracker_url(info)
                .await
                .context("Unable to get response")?;
            for socket_address in &response.peers.0 {
                let a = format!("{}:{}", socket_address.ip(), socket_address.port());
                println!("{}", a);
            }
        }
        Type::Handshake { info, peer } => {
            let tor: Torrent =
                read_and_deserialize_torrent(info).context("Unable to read and deserialize")?;
            let info_hash = tor.info_hash();
            let reserved: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
            let (_, peer_id) = establish_handshake(info_hash, peer, reserved)
                .await
                .context("Unable to establish handhshake")?;
            println!("Peer ID: {}", peer_id);
        }
        Type::DownloadPiece {
            output,
            info,
            index,
        } => {
            let reserved: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
            let _res = establish_handshake_and_download(&output, &info, Some(*index), reserved)
                .await
                .context("Downloading a single piece");
        }
        Type::Download { output, info } => {
            let reserved: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
            let _res = establish_handshake_and_download(&output, &info, None, reserved)
                .await
                .context("Downloading all pieces");
        }
        Type::MagnetParse { magnet } => {
            let magnet: Magnet = Magnet::new(&magnet).context("Parsing failed")?;
            println!("Tracker URL: {}", &magnet.url);
            println!("Info Hash: {}", &magnet.info_hash);
        }
        Type::MagnetHandshake { magnet } => {
            let magnet: Magnet = Magnet::new(&magnet).context("Parsing failed")?;
            let response = get_peers_from_magnet(&magnet)
                .await
                .context("Failed to get peers")?;
            let info_hash = magnet.info_hash_to_slice();
            let peer = &response.peers.0[0];
            let reserved: [u8; 8] = [0, 0, 0, 0, 0, 16, 0, 0];

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
            let info_hash_received = &res[28..48];
            let peer_reserved_bit = &res[20..28];
            assert_eq!(info_hash_received, info_hash, "Info Hash Mismatch");
            println!("Peer ID: {}", peer_id);
            // println!("Peer Info Hash: {:?}", info_hash_received);
            // println!("Peer Reserved Bit: {:?}", peer_reserved_bit);

            // send bitfield
            // no need to do for this challenge
            let codec = MessageFramer;
            let mut tcp_stream = Framed::new(tcp_stream, codec);
    
            //get bitfield
            let _response = tcp_stream
                .next()
                .await
                .expect("Expecting a bitfield message")
                .context("Failed to get bitfield")?;
            // assert!(!response.payload.is_empty());
            if peer_reserved_bit[2].eq(&reserved[2]) {
                let content = fs::read("magnet.file").context("Read file")?;
                let extension_handshake: ExtensionHandshake = serde_bencode::from_bytes(&content).context("Convert file to a struct")?;
                let extension_payload = ExtensionPayload { 
                    extension_id: 0, 
                    payload: ExtensionType::ExtensionHandshakeMessage(extension_handshake) 
                };

                let extension_handshake_message = Message {
                    message_tag: MessageTag::Extension,
                    payload: Payload::ExtendedPayload(extension_payload),
                };
                let _ = tcp_stream.send(extension_handshake_message).await;

                //receive extension handshake
                let extension_reply = tcp_stream
                    .next()
                    .await
                    .expect("Expecting extension handshake reply")
                    .context("Failed to get reply message")?;

                // println!("{:?}", extension_reply);
                if let Payload::ExtendedPayload(extension_payload) = extension_reply.payload{
                    if let ExtensionType::ExtensionHandshakeMessage(handshake_payload) = extension_payload.payload{
                        println!("Peer Metadata Extension ID: {:?}", handshake_payload.m.ut_metadata);
                    }
                }
            } else {
                println!("Extension not supported {:?}", peer_reserved_bit);
            }
        },
        Type::MagnetInfo { magnet } => {
            let magnet: Magnet = Magnet::new(&magnet).context("Parsing failed")?;
            let response = get_peers_from_magnet(&magnet)
                .await
                .context("Failed to get peers")?;
            let info_hash = magnet.info_hash_to_slice();
            let peer = &response.peers.0[0];
            let reserved: [u8; 8] = [0, 0, 0, 0, 0, 16, 0, 0];

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
            let info_hash_received = &res[28..48];
            let peer_reserved_bit = &res[20..28];
            assert_eq!(info_hash_received, info_hash, "Info Hash Mismatch");
            println!("Peer ID: {}", peer_id);
            // println!("Peer Info Hash: {:?}", info_hash_received);
            // println!("Peer Reserved Bit: {:?}", peer_reserved_bit);

            // send bitfield
            // no need to do for this challenge
            let codec = MessageFramer;
            let mut tcp_stream = Framed::new(tcp_stream, codec);
    
            //get bitfield
            let _response = tcp_stream
                .next()
                .await
                .expect("Expecting a bitfield message")
                .context("Failed to get bitfield")?;
            // assert!(!response.payload.is_empty());
            if peer_reserved_bit[2].eq(&reserved[2]) {
                let content = fs::read("magnet.file").context("Read file")?;
                let extension_handshake: ExtensionHandshake = serde_bencode::from_bytes(&content).context("Convert file to a struct")?;
                let extension_payload = ExtensionPayload { 
                    extension_id: 0, 
                    payload: ExtensionType::ExtensionHandshakeMessage(extension_handshake) 
                };
                
                let extension_handshake_message = Message {
                    message_tag: MessageTag::Extension,
                    payload: Payload::ExtendedPayload(extension_payload),
                };

                let _ = tcp_stream.send(extension_handshake_message).await;

                //receive extension handshake
                let extension_reply = tcp_stream
                    .next()
                    .await
                    .expect("Expecting extension handshake reply")
                    .context("Failed to get reply message")?;

                // println!("{:?}", extension_reply);
                if let Payload::ExtendedPayload(extension_payload) = extension_reply.payload{
                    if let ExtensionType::ExtensionHandshakeMessage(handshake_payload) = extension_payload.payload{
                        let peer_metadata = handshake_payload.m.ut_metadata;
                        println!("Peer Metadata Extension ID: {:?}", &peer_metadata);
                        let extension_metadata_request = ExtensionMetadata::Request(
                            MetaData{
                                msg_type: 0,
                                piece: 0
                            }
                        );

                        let extension_metadata_payload = ExtensionPayload{
                            extension_id: peer_metadata,
                            payload: ExtensionType::MetaDataMessage(extension_metadata_request)
                        };

                        let extension_metadata_message = Message {
                            message_tag: MessageTag::Extension,
                            payload: Payload::ExtendedPayload(extension_metadata_payload),
                        };

                        let _res = tcp_stream.send(extension_metadata_message).await.context("Sending failed");
                    }
                }
            }
        }
    }
    Ok(())
}
