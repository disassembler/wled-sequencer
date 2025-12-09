// src/ddp.rs

use std::net::UdpSocket;
use std::io::Result;
use tokio::task;
use std::sync::Arc;

pub const DDP_PORT: u16 = 4048;

pub fn create_ddp_packet(sequence_num: u32, length: u16, pixel_data: &[u8]) -> Vec<u8> {
    let mut header = vec![
        0x41, // Magic/Flags (Type 1, Version 1)
        0x01, // ID (Source ID)
        0x00, // Status
        0x01, // Sequence Number (simplified: we're using a full u32 for simplicity)
    ];

    // Append sequence number (offset) as u32 Big Endian
    header.extend_from_slice(&sequence_num.to_be_bytes());
    // Append length as u16 Big Endian
    header.extend_from_slice(&length.to_be_bytes());
    
    // Append pixel data
    header.extend_from_slice(pixel_data);
    header
}

// Actual network send operation (blocking call wrapped in async context)
pub async fn send_ddp_bytes(socket: Arc<UdpSocket>, ip_address: String, bytes: Vec<u8>) -> Result<usize> {
    let target = format!("{}:{}", ip_address, DDP_PORT);
    // Use spawn_blocking for the synchronous UdpSocket::send_to call
    task::spawn_blocking(move || {
        socket.send_to(&bytes, target)
    }).await.unwrap()
}
