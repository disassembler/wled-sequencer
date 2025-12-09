// src/main.rs
use anyhow::Result;
use clap::Parser;
use wled_sequencer_lib::cli::Cli;
use wled_sequencer_lib::{play_sequence, PlaybackConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let log_env = env_logger::Env::default().filter_or("RUST_LOG", "wled_sequencer_lib=info");
    env_logger::init_from_env(log_env);
    
    let cli = Cli::parse();
    
    let config = PlaybackConfig {
        fseq_path: cli.file,
        wled_ip_address: cli.host,
        loop_enabled: cli.loop_enabled,
        ddp_port: cli.port,
    };

    play_sequence(config).await
}
