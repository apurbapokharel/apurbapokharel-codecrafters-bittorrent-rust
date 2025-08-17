use codecrafters_bittorrent::{
    handshake::Handshake,
    magnet::Magnet,
    message::{ExtensionPayload, Message, MessageFramer, MessageTag, Payload},
    torrent::Torrent,
    utils::{
        decode_bencoded_value, establish_handshake, establish_handshake_and_download,
        get_peers_from_magnet, get_peers_from_tracker_url, read_and_deserialize_torrent,
    },
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::Framed;

use anyhow::Context;
use clap::{Parser, Subcommand};
use hex;
use std::net::SocketAddrV4;

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
            let extension = &res[28..48];
            println!("Peer ID: {}", peer_id);

            //send bitfield
            let bitfield_message = Message {
                message_tag: MessageTag::Bitfield,
                payload: Payload::SimplePayload(Vec::new()),
            };
            let codec = MessageFramer;
            let mut tcp_stream = Framed::new(tcp_stream, codec);
            let _ = tcp_stream.send(bitfield_message).await;
            //get bitfield
            let response = tcp_stream
                .next()
                .await
                .expect("Expecting a bitfield message")
                .context("Failed to get bitfield")?;
            // assert!(!response.payload.is_empty());
            if extension.eq(&reserved) {
                //send extension handshake
                let extension_handshake_dict = "d1:md11:ut_metadatai13eee";
                let extension_payload = ExtensionPayload {
                    extension_id: 1,
                    dict: extension_handshake_dict.into(),
                };
                let extension_handshake = Message {
                    message_tag: MessageTag::Extension,
                    payload: Payload::ExtendedPayload(extension_payload),
                };
                // let _ = tcp_stream.send(extension_handshake).await;
                //receive extension handshake
            } else {
                println!("Extension not supported {:?}", extension);
            }
        }
    }
    Ok(())
}
