// src/fseq_ffi.rs
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ptr;
use anyhow::{Result, bail, Context};
use std::io::Read;
use zstd::stream::Decoder as ZstdDecoder;
use std::os::raw::{c_char, c_int};
use std::ffi::CStr;
use std::convert::TryInto; 

include!(concat!(env!("OUT_DIR"), "/tinyfseq_bindings.rs")); 

// ----------------------------------------------------------------------
// FseqFile Struct
// ----------------------------------------------------------------------
/// Represents a parsed FSEQ file, holding the raw data and header information.
pub struct FseqFile {
    pub buffer: Vec<u8>,
    pub header: tf_header_t, 
}

impl FseqFile {
    /// Parses the raw buffer to initialize the file's header.
    pub fn parse(buffer: Vec<u8>) -> Result<Self> {
        
        const MIN_HEADER_SIZE: usize = 32;
        if buffer.len() < MIN_HEADER_SIZE {
            bail!("FSEQ buffer size is too small for a valid header (must be >= 32 bytes).");
        }

        let mut header: tf_header_t = unsafe { std::mem::zeroed() };
        
        let result = unsafe {
            TFHeader_read(
                buffer.as_ptr(), 
                buffer.len() as c_int, 
                &mut header, 
                ptr::null_mut()
            )
        };

        if result != tf_err_t_TF_OK {
            let error_string_ptr = unsafe { TFError_string(result) };
            let error_message = if error_string_ptr.is_null() {
                format!("Failed to parse FSEQ header with error code: {}", result)
            } else {
                let c_str = unsafe { CStr::from_ptr(error_string_ptr as *const c_char) };
                format!("FSEQ header read error: {}", c_str.to_string_lossy())
            };
            bail!(error_message);
        }

        let fseq_file = FseqFile {
            buffer,
            header,
        };

        Ok(fseq_file)
    }
    
    /// Prints a summary of the FSEQ header contents for debugging.
    pub fn dump_header_info(&self) {
        
        let raw_byte_20 = self.buffer[20];
        let raw_byte_21 = self.buffer[21];
        
        let ecbc_upper_4_bits = (raw_byte_20 & 0xF0) as u16;
        let ecbc_lower_8_bits = raw_byte_21 as u16;
        let extended_compression_block_count: u16 = (ecbc_upper_4_bits << 4) | ecbc_lower_8_bits;

        println!("\n--- FSEQ Header Dump ---");
        
        println!("Channel Data Offset (Byte 4-5): {}", self.header.channelDataOffset);
        println!("Variable Data Offset (Header Size, Byte 8-9): {}", self.header.variableDataOffset);
        
        println!("Version: {}.{}", self.header.majorVersion, self.header.minorVersion);
        println!("Frames: {}", self.header.frameCount);
        println!("Channels: {}", self.header.channelCount);
        println!("Step Time Ms: {}", self.header.frameStepTimeMillis);
        
        println!("--- Compression Data ---");
        println!("Raw Byte 20 (Type + ECBC): 0x{:X}", raw_byte_20);
        println!("Raw Byte 21 (Block Count): {}", raw_byte_21);
        println!("FFI-Parsed Compression Type: {} ({} & 0x0F)", self.header.compressionType, raw_byte_20);
        println!("FFI-Parsed Block Count (Byte 21): {}", self.header.compressionBlockCount);
        println!("Sparse Range Count (Byte 22): {}", self.header.channelRangeCount);
        println!("Calculated Block Count (ECBC 12-bit): {}", extended_compression_block_count);
        
        println!("------------------------\n");
    }

    /// Retrieves the frame data. Handles ZLIB/ZSTD decompression if needed.
    pub fn get_frame(&self, frame_num: u32) -> Result<Vec<u8>> {
        if frame_num >= self.header.frameCount {
            bail!("Frame number {} is out of bounds (total frames: {})", frame_num, self.header.frameCount);
        }

        let channel_count = self.header.channelCount as usize;
        let frame_size = channel_count; 
        
        match self.header.compressionType { 
            tf_compression_type_t_TF_COMPRESSION_NONE => {
                // UNCOMPRESSED LOGIC: Frames are stored contiguously starting at channelDataOffset.
                
                let data_offset = self.header.channelDataOffset as usize;
                let frame_offset = frame_num as usize * frame_size;
                
                let frame_start = data_offset + frame_offset;
                let frame_end = frame_start + frame_size;
                
                if frame_end > self.buffer.len() {
                    bail!("Uncompressed frame boundaries ({}-{}) are outside the file buffer (size: {}). File is likely truncated.", 
                          frame_start, frame_end, self.buffer.len());
                }

                // Slice the raw data from the buffer
                return Ok(self.buffer[frame_start..frame_end].to_vec());
            }
            tf_compression_type_t_TF_COMPRESSION_ZSTD => {
                // ZSTD DECOMPRESSION LOGIC
                
                // Calculate ECBC
                let raw_byte_20 = self.buffer[20];
                let raw_byte_21 = self.buffer[21];
                let ecbc_upper_4_bits = (raw_byte_20 & 0xF0) as u16;
                let ecbc_lower_8_bits = raw_byte_21 as u16;
                let extended_compression_block_count: u16 = (ecbc_upper_4_bits << 4) | ecbc_lower_8_bits;
                let block_count = extended_compression_block_count as usize;
                
                let frames_per_block: u32 = 256; 
                let compressed_data_slice: &[u8];

                if block_count == 0 {
                    bail!("Compressed FSEQ file with block count 0 is unsupported (Single-stream ZSTD).");
                }

                // Block Indexing Fix (Handling Block 0 10-frame exception)
                let block_index: usize;
                let frame_in_block: u32;

                let mut current_frame_num: u32 = 0;
                let mut current_block_index: usize = 0;
                const BLOCK0_SIZE_FRAMES: u32 = 10;

                loop {
                    let frames_in_current_block = if current_block_index == 0 { 
                        BLOCK0_SIZE_FRAMES
                    } else { 
                        frames_per_block 
                    };

                    if frame_num < current_frame_num + frames_in_current_block {
                        block_index = current_block_index;
                        frame_in_block = frame_num - current_frame_num;
                        eprintln!("DEBUG: Block indexing fix applied: Block index={}, Frame in block={}", block_index, frame_in_block);
                        break;
                    }

                    current_frame_num += frames_in_current_block;
                    current_block_index += 1;

                    if current_block_index >= block_count {
                        bail!("Frame number {} is outside the available blocks.", frame_num);
                    }
                }
                
                // --- Block Metadata Reading ---
                let block_metadata_size = block_count * 8;
                let compressed_data_section_start = self.header.channelDataOffset as usize;
                let block_metadata_table_start = self.header.variableDataOffset as usize - block_metadata_size;
                let block_metadata_read_start = block_metadata_table_start + (block_index * 8); 
                
                eprintln!("DEBUG: Calculated Block Count (ECBC): {}", block_count);
                eprintln!("DEBUG: Block Metadata Size: {} bytes", block_metadata_size);
                eprintln!("DEBUG: Compressed Data Section Start (Channel Data Offset): {}", compressed_data_section_start);
                eprintln!("DEBUG: Block {} metadata read start index: {}", block_index, block_metadata_read_start);

                if block_metadata_read_start + 8 > self.buffer.len() {
                     bail!("File buffer too small to read block metadata (Expected 8 bytes at offset {}).", block_metadata_read_start);
                }
                
                let block_data_slice = &self.buffer[block_metadata_read_start..block_metadata_read_start + 8];
                eprintln!("DEBUG: Raw 8 bytes for block metadata: {:?}", block_data_slice);

                let first_frame_id = u32::from_le_bytes(block_data_slice[0..4].try_into().unwrap());
                let size = u32::from_le_bytes(block_data_slice[4..8].try_into().unwrap());
                
                eprintln!("DEBUG: Converted values: firstFrameId={}, size={}", first_frame_id, size);

                
                let mut compressed_chunk_start = compressed_data_section_start + first_frame_id as usize; 
                let mut compressed_chunk_end = compressed_chunk_start + size as usize;
                
                // Corruption workaround remains for Block 0, using the size from metadata
                if compressed_chunk_start > self.buffer.len() && block_index == 0 {
                    eprintln!("DEBUG: CORRUPTION WORKAROUND TRIGGERED. Bad offset: {}. Forcing Block 0 start.", compressed_chunk_start);
                    
                    compressed_chunk_start = compressed_data_section_start; 
                    compressed_chunk_end = compressed_chunk_start + size as usize; 
                    
                    eprintln!("DEBUG: CORRUPTION FIX APPLIED: ChunkStart forced to {}, ChunkEnd set to {} (End of file: {}).", 
                              compressed_chunk_start, compressed_chunk_end, self.buffer.len());
                }


                eprintln!("DEBUG: Calculated boundaries (final): ChunkStart={}, ChunkEnd={}, FileSize={}",
                          compressed_chunk_start, compressed_chunk_end, self.buffer.len());


                if compressed_chunk_end > self.buffer.len() {
                    bail!("Compressed data chunk boundaries ({}-{}) are outside the file buffer (size: {}). This indicates incorrect offset/size values in the file metadata.", 
                          compressed_chunk_start, compressed_chunk_end, self.buffer.len());
                }

                compressed_data_slice = &self.buffer[compressed_chunk_start..compressed_chunk_end];
                eprintln!("DEBUG: ZSTD Header Skip Reverted: Passing chunk of size {} bytes to decoder.", compressed_data_slice.len());


                // --- ZSTD Decompression & Extraction Steps ---
                let mut decoder = ZstdDecoder::new(compressed_data_slice).context("Failed to create ZSTD decoder.")?;
                let mut decompressed_data = Vec::new();
                
                decoder.read_to_end(&mut decompressed_data).context("Failed to decompress ZSTD frame data.")?;

                let frame_start = frame_in_block as usize * frame_size;
                let frame_end = frame_start + frame_size;

                if frame_end > decompressed_data.len() {
                    bail!("Decompressed data ({} bytes) is too small to contain the requested frame (starts at {}).", decompressed_data.len(), frame_start);
                }

                return Ok(decompressed_data[frame_start..frame_end].to_vec());
            }
            tf_compression_type_t_TF_COMPRESSION_ZLIB => {
                // ZLIB DECOMPRESSION LOGIC (Still requires the same indexing and structural fixes. Please focus on ZSTD/None first.)
                bail!("ZLIB compression logic requires implementation.");
            }
            _ => {
                bail!("Unknown FSEQ compression type: {}", self.header.compressionType);
            }
        }
    }
    
    // Getters
    pub fn get_step_time(&self) -> i32 {
        self.header.frameStepTimeMillis as i32
    }

    pub fn get_channel_count(&self) -> u32 {
        self.header.channelCount
    }

    pub fn get_frame_count(&self) -> u32 {
        self.header.frameCount
    }
}
