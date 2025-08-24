use codecrafters_bittorrent::{
    magnet::Magnet, 
    torrent::Torrent, 
    utils::{
        self, decode_bencoded_value, establish_handshake, establish_handshake_and_download, get_peers_from_tracker_url, read_and_deserialize_torrent
    }
};
use std::{net::SocketAddrV4};
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
    },
    MagnetDownloadPiece{
        #[arg(short)]
        output: String,
        magnet: String,
        index: usize,
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
            let (extension_payload,_)  = utils::magnet_handshake(magnet)
                .await
                .context("Failed to receive magnet handshake")?;
            println!("Peer Metadata Extension ID: {:?}", extension_payload.m.ut_metadata);
        },
        Type::MagnetInfo { magnet } => {
            let (torrent,_)  = utils::get_magnet_metadata(magnet)
                .await
                .context("Failed to receive magnet handshake")?;
            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", torrent.info.length);
            println!("Info Hash: {}", hex::encode(&torrent.info_hash()));
            println!("Piece Length: {}", &torrent.info.pieces_length);
            println!("Piece Hashes:");
            for piece in &torrent.info.pieces.0{
                println!("{}", hex::encode(piece));
            }    
        },
        Type::MagnetDownloadPiece { output, magnet, index } => {
            let (torrent, mut strem)  = utils::get_magnet_metadata(magnet)
                .await
                .context("Failed to receive magnet handshake")?;
             
            let mut res: Vec<u8> = Vec::new();
            res = utils::fetch_a_piece(&torrent, &mut strem, *index)
                .await
                .context("Fetch a piece failed")?;
           
            tokio::fs::write(&output, res)
            .await
            .context("write out downloaded piece")?;
    }
    // res = fetch_all_pieces(&tor, &mut tcp_stream)
    //     .await
    //     .context("Fetch all piece failed")?;

    }
    Ok(())
}
