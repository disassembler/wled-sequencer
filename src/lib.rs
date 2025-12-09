// src/lib.rs
use anyhow::{Context, Result};
use std::fs::read;
use std::net::{Ipv4Addr, SocketAddr};
use ddp_rs::{connection::DDPConnection, protocol::PixelConfig};
use crate::fseq_ffi::FseqFile;
use crate::playback::{PlaybackContext, run_playback_loop}; 
use crate::monitor::run_monitor_thread;
use tokio::sync::watch;
use tokio::task;
use std::sync::Arc;

pub mod fseq_ffi;
pub mod playback;
pub mod monitor;
pub mod cli;

#[derive(Clone, Debug)]
pub struct PlaybackConfig {
    pub fseq_path: String,
    pub wled_ip_address: String, 
    pub loop_enabled: bool,
    pub ddp_port: u16,
}

// ----------------------------------------------------------------------
// Main Play Sequence Orchestration
// ----------------------------------------------------------------------
pub async fn play_sequence(config: PlaybackConfig) -> Result<()> {
    
    log::info!("Starting FSEQ Player...");
    log::info!("Configuration: {:?}", config);
    
    let buffer = read(&config.fseq_path)
        .context(format!("Failed to read FSEQ file at: {}", config.fseq_path))?;
    let fseq_file = FseqFile::parse(buffer)?;
    let fseq_arc = Arc::new(fseq_file);
    let (tx_stream_state, rx_stream_state) = watch::channel(false);
    
    run_monitor_thread(config.clone(), tx_stream_state);

    let mut rx_player_state = rx_stream_state.clone();
    let player_config = config;
    
    loop {
        // Wait for the START signal (state change to true)
        if !*rx_player_state.borrow() {
            log::info!("Player: Waiting for START signal from Monitor...");
            rx_player_state.changed().await.context("Monitor thread stopped unexpectedly.")?;
        }
        
        // Ensure state is true (START) and proceed
        if *rx_player_state.borrow() {
            log::info!("Player: Received START signal. Initializing DDP connection...");

            // Set up DDP connection
            let local_socket = std::net::UdpSocket::bind(
                SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)
            ).context("Failed to bind local UDP socket")?;
        
            let conn = DDPConnection::try_new(
                &format!("{}:{}", player_config.wled_ip_address, player_config.ddp_port),
                PixelConfig::default(),
                ddp_rs::protocol::ID::Default,
                local_socket 
            )?;

            let context = PlaybackContext {
                fseq_file: fseq_arc.clone(),
                loop_enabled: player_config.loop_enabled,
            };

            // Run the blocking playback loop on a separate *tokio* blocking thread
            let rx_stop = rx_player_state.clone();
            let mut join_handle = task::spawn_blocking(move || { 
                run_playback_loop(conn, context, rx_stop)
            });

            // Wait for termination condition (Monitor STOP or sequence finished)
            loop {
                tokio::select! {
                    res = &mut join_handle => { 
                        match res {
                            Ok(Ok(_)) => log::info!("Player: Playback loop finished naturally."),
                            Ok(Err(e)) => log::error!("Player: Playback loop crashed: {}", e),
                            Err(e) => log::error!("Player: Playback thread panicked: {}", e),
                        }
                        break;
                    }
                    _ = rx_player_state.changed() => {
                        if !*rx_player_state.borrow() {
                            log::info!("Player: Monitor requested STOP. Waiting for playback thread to terminate...");
                            
                            match (&mut join_handle).await { 
                                Ok(Ok(_)) => log::info!("Player: Playback thread terminated gracefully by Monitor signal."),
                                Ok(Err(e)) => log::error!("Player: Playback thread crashed during controlled shutdown: {}", e),
                                Err(e) => log::error!("Player: Playback thread panicked during controlled shutdown: {}", e),
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}
