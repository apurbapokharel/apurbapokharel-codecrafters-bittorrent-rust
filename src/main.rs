use codecrafters_bittorrent::{
        magnet::Magnet, torrent::Torrent, utils::{
            decode_bencoded_value, establish_handshake, establish_handshake_and_download, get_peers_from_tracker_url, read_and_deserialize_torrent
        }
    };
use serde::{Serialize, Deserialize};
use serde_json::{self};
use std::{net::{SocketAddrV4}};
use anyhow::{Context};
use hex;
use clap::{Parser, Subcommand};

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
        peer: SocketAddrV4
    },
    DownloadPiece{
        #[arg(short)]
        output: String,
        info: String,
        index: usize
    },
    Download{
        #[arg(short)]
        output: String,
        info: String    
    },
    MagnetParse{
        magnet: String
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
            let tor: Torrent = read_and_deserialize_torrent(info)
                .context("Unable to read and deserialize")?;
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
            let (response, _) = get_peers_from_tracker_url(info)
                .await
                .context("Unable to get response")?;
            for socket_address in &response.peers.0{
                let a = format!(
                    "{}:{}",
                    socket_address.ip(),
                    socket_address.port()
                );
                println!("{}", a);
            }
        },
        Type::Handshake { info, peer } => {
            let (_, peer_id) = establish_handshake(info, peer)
                .await
                .context("Unable to establish handhshake")?;
            println!("Peer ID: {}", peer_id);
        },
        Type::DownloadPiece { output, info, index } => {
            let _res = establish_handshake_and_download(&output, &info, Some(*index))
                .await
                .context("Downloading a single piece");
        },
        Type::Download{ output, info} => {
            let _res = establish_handshake_and_download(&output, &info, None)
                .await
                .context("Downloading all pieces");
        },
        Type::MagnetParse { magnet } => {
            let magnet: Magnet = Magnet::new(&magnet).context("Parsing failed")?;
            println!("Tracker URL: {}", &magnet.url);
            println!("Info Hash: {}", &magnet.info_hash);
        }
    }
    Ok(())
}
 



