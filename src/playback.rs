// src/playback.rs
use anyhow::{Result, bail};
use std::thread::sleep;
use std::time::Duration;
use std::sync::Arc; 
use tokio::sync::watch::Receiver;
use ddp_rs::connection::DDPConnection; 
use crate::fseq_ffi::FseqFile;

pub struct PlaybackContext {
    pub fseq_file: Arc<FseqFile>, 
    pub loop_enabled: bool,
}

pub fn run_playback_loop(
    mut conn: DDPConnection, 
    context: PlaybackContext,
    rx_stream_state: Receiver<bool>, 
) -> Result<()> {
    
    let frame_count = context.fseq_file.get_frame_count();
    let step_time_ms = context.fseq_file.get_step_time();
    let step_duration = Duration::from_millis(step_time_ms as u64);
    let mut frame_num = 0u32;
    let mut sequence_run_count = 0;

    log::info!("Player: Playback started ({} frames @ {}ms).", frame_count, step_time_ms);

    loop {
        // --- Frame Retrieval and Sending Logic ---
        match context.fseq_file.get_frame(frame_num) {
            Ok(frame_data) => {
                if let Err(e) = conn.write(&frame_data) {
                    log::error!("Error sending DDP packet for frame {}: {}", frame_num, e);
                }
            }
            Err(e) => {
                bail!("Error retrieving frame {}: {}", frame_num, e);
            }
        }

        // --- Frame Management and Stop Check ---
        frame_num += 1;
        
        if frame_num >= frame_count {
            sequence_run_count += 1;
            
            // ⭐️ Logging sequence completion 
            log::info!("✅ Sequence COMPLETED. Total runs: {}\n", sequence_run_count);
            
            if context.loop_enabled {
                frame_num = 0;
            } else {
                break;
            }
        }
        
        if rx_stream_state.has_changed().is_ok() {
            if !*rx_stream_state.borrow() {
                log::info!("Player: Stop signal received from Monitor. Halting DDP stream.");
                return Ok(());
            }
        }

        sleep(step_duration);
    }

    Ok(())
}
