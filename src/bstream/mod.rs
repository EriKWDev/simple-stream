// Copyright 2015 Nathan Sizemore <nathanrsizemore@gmail.com>
//
// This Source Code Form is subject to the
// terms of the Mozilla Public License, v.
// 2.0. If a copy of the MPL was not
// distributed with this file, You can
// obtain one at
// http://mozilla.org/MPL/2.0/.


//! Bstream module.
//! This is a blocking stream designed to block on read/write until


use std::result::Result;
use std::net::{TcpStream, Shutdown};
use std::io::{Read, Write, Error};

use super::readbuffer::ReadBuffer;


/// Represents the result of attempting a read on the underlying file descriptor
pub type ReadResult = Result<Vec<u8>, Error>;

/// Represents the result attempting a write on the underlying fild descriptor
pub type WriteResult = Result<(), Error>;


/// States the current stream can be in
#[derive(PartialEq, Clone)]
enum ReadState {
    /// Currently reading the payload length
    PayloadLen,
    /// Currently reading the payload
    Payload
}

pub struct Bstream {
    /// Current state
    state: ReadState,
    /// Underlying std::net::TcpStream
    stream: TcpStream,
    /// Message buffer
    buffer: ReadBuffer
}


impl Bstream {

    /// Returns a new Bstream
    pub fn new(stream: TcpStream) -> Bstream {
        Bstream {
            state: ReadState::PayloadLen,
            stream: stream,
            buffer: ReadBuffer::new()
        }
    }

    /// Performs a blocking read and returns when a complete message
    /// has been returned, or an error has occured
    pub fn read(&mut self) -> ReadResult {
        loop {
            // Create a buffer for this specific read iteration
            let count = self.buffer.remaining();
            let mut buffer = Vec::<u8>::with_capacity(count as usize);
            unsafe { buffer.set_len(count as usize); }

            let result = self.stream.read(&mut buffer[..]);
            if result.is_err() {
                return Err(result.unwrap_err());
            }

            let num_read = result.unwrap();
            for x in 0..num_read {
                self.buffer.push(buffer[x]);
            }

            if self.buffer.remaining() == 0 {
                if self.state == ReadState::PayloadLen {
                    let mut index = 0;
                    for byte in self.buffer.current_buffer().iter() {
                        println!("byte {}: {}", index, byte);
                        index += 1;
                    }

                    self.buffer.calc_payload_len();
                    let p_len = self.buffer.payload_len();
                    self.buffer.set_capacity(p_len);
                    self.state = ReadState::Payload;
                } else { // Payload completely read
                    let mut index = 0;
                    for byte in self.buffer.current_buffer().iter() {
                        println!("byte {}: {}", index, byte);
                        index += 1;
                    }

                    self.buffer.reset();
                    self.state = ReadState::PayloadLen;
                    break;
                }
            }
        }
        let mut buffer = self.buffer.drain_queue();

        // This should always be .len() of 1
        // if it isn't - we're doing some bad stuff in here
        if buffer.len() != 1 {
            panic!("Error - Bstream.read() - Internal buffer was not equal to one...?")
        }

        match buffer.pop() {
            Some(buf) => Ok(buf),
            None => unimplemented!()
        }
    }

    /// Performs a blocking write operation and returns the complete buffer has
    /// been written, or an error has occured
    pub fn write(&mut self, buffer: &Vec<u8>) -> WriteResult {
        let mut plen_buf = [0u8; 2];
        let plen = buffer.len() as u16;
        plen_buf[0] = (plen >> 8) as u8;
        plen_buf[1] = plen as u8;

        let mut n_buffer = Vec::<u8>::with_capacity(buffer.len() + 2);
        n_buffer.push(plen_buf[0]);
        n_buffer.push(plen_buf[1]);

        for x in 0..buffer.len() {
            n_buffer.push(buffer[x]);
        }

        match self.stream.write_all(&n_buffer[..]) {
            Ok(()) => {
                let _ = self.stream.flush();
                Ok(())
            }
            Err(e) => Err(e)
        }
    }

    /// Shuts down the connection
    pub fn shutdown(&self) {
        let result = self.stream.shutdown(Shutdown::Both);
        if result.is_err() {
            panic!("Error shutting down stream: {}", result.unwrap_err())
        }
    }
}

impl Clone for Bstream {
    fn clone(&self) -> Bstream {
        Bstream {
            state: self.state.clone(),
            stream: self.stream.try_clone().unwrap(),
            buffer: self.buffer.clone()
        }
    }
}
