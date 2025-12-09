// src/monitor.rs
use std::process::Command;
use std::thread;
use std::time::Duration;
use tokio::sync::watch::Sender;
use crate::PlaybackConfig;

// ----------------------------------------------------------------------
// ICMP Monitoring Function
// ----------------------------------------------------------------------
fn check_device_status(ip: &str) -> bool {
    // Uses the system's 'ping' command, configured to send 1 packet and timeout quickly (1 second).
    let output = if cfg!(target_os = "windows") {
        Command::new("ping")
            .args(&["-n", "1", "-w", "1000", ip])
            .output()
    } else {
        Command::new("ping")
            .args(&["-c", "1", "-W", "1", ip])
            .output()
    };

    match output {
        Ok(output) => {
            output.status.success()
        }
        Err(e) => {
            log::error!("Failed to execute system ping command (is 'ping' in PATH?): {:?}", e);
            false
        }
    }
}

// ----------------------------------------------------------------------
// Monitor/Poller Thread Logic
// ----------------------------------------------------------------------
pub fn run_monitor_thread(config: PlaybackConfig, tx_stream_state: Sender<bool>) {
    let monitor_ip = config.wled_ip_address.clone();
    let monitor_tx = tx_stream_state;
    
    // We spawn a blocking thread since the system ping check is blocking and runs forever.
    tokio::task::spawn_blocking(move || {
        let mut consecutive_failures = 0;
        const FAILURE_THRESHOLD: i32 = 3;
        
        log::info!("Monitor started. Polling {} every 30 seconds...", monitor_ip);

        loop {
            // PERFORM CHECK (runs immediately on first iteration)
            let is_up = check_device_status(&monitor_ip);

            match is_up {
                true => {
                    // Device is UP, reset counter and send START if needed
                    if consecutive_failures > 0 {
                        log::info!("Monitor: Device UP. Cleared {} failures.", consecutive_failures);
                        consecutive_failures = 0;
                    }
                    
                    if !*monitor_tx.borrow() {
                        log::info!("Monitor: Device is UP. Sending START signal to Player.");
                        monitor_tx.send(true).unwrap_or_else(|e| log::error!("Monitor failed to send START signal: {}", e));
                    }
                }
                false => {
                    // Device is DOWN. Only process failures if we are actively streaming.
                    if *monitor_tx.borrow() { 
                        consecutive_failures += 1;
                        
                        log::error!("Monitor: Device DOWN (Failure {}/{})", consecutive_failures, FAILURE_THRESHOLD);

                        // Check for STOP condition (hits 3 AND streaming).
                        if consecutive_failures >= FAILURE_THRESHOLD {
                            log::error!("Monitor: Hit failure threshold. Sending STOP signal to Player.");
                            monitor_tx.send(false).unwrap_or_else(|e| log::error!("Monitor failed to send STOP signal: {}", e));
                        }
                    }
                    // If we are not streaming, we quietly poll.
                }
            }
            thread::sleep(Duration::from_secs(30)); 
        }
    });
}
