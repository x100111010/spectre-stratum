mod pow;
mod spectred;
mod stratum;
mod uint;

use crate::spectred::SpectredHandle;
pub use crate::uint::U256;
use anyhow::Result;
use clap::Parser;
use log::{debug, info, LevelFilter};
use spectred::{Client, Message};

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    rpc_url: String,
    #[clap(short, long, default_value = "127.0.0.1:6969")]
    stratum_addr: String,
    #[clap(short, long, default_value = "spectred-stratum")]
    extra_data: String,
    #[clap(short, long)]
    mining_addr: String,
    #[clap(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let level = if args.debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .filter_module("spectred_stratum", level)
        .init();

    let (handle, recv_cmd) = SpectredHandle::new();
    let stratum = stratum::Stratum::new(&args.stratum_addr, handle.clone()).await?;

    let (client, mut msgs) = Client::new(
        &args.rpc_url,
        &args.mining_addr,
        &args.extra_data,
        handle,
        recv_cmd,
    );
    while let Some(msg) = msgs.recv().await {
        match msg {
            Message::Info { version, .. } => {
                info!("Connected to Spectred {version}");
            }
            Message::NewTemplate => {
                debug!("Requesting new template");
                if !client.request_template() {
                    debug!("Channel closed");
                    break;
                }
            }
            Message::Template(template) => {
                debug!("Received block template");
                stratum.broadcast(template).await;
            }
            Message::SubmitBlockResult(error) => {
                debug!("Resolve pending job");
                match &error {
                    Some(e) => debug!("Submitted invalid block: {e}"),
                    None => info!("Found a block!"),
                }
                stratum.resolve_pending_job(error).await;
            }
        }
    }

    Ok(())
}
