// src/cli.rs
use clap::Parser;

/// FSEQ DDP Player
#[derive(Parser, Debug)]
#[command(author, version, about = "A DDP player that streams FSEQ data to a WLED controller, pausing automatically when the controller is offline.", long_about = None)]
pub struct Cli {
    /// IP address of the WLED controller (e.g., 192.168.1.50)
    #[arg(short, long)]
    pub host: String,

    /// UDP port for the Distributed Display Protocol (DDP)
    #[arg(short, long, default_value_t = 4048)]
    pub port: u16,

    /// Path to the FSEQ sequence file
    #[arg(short, long)]
    pub file: String,
    
    /// Enable continuous looping of the FSEQ sequence
    #[arg(long, default_value_t = true)]
    pub loop_enabled: bool,
}
