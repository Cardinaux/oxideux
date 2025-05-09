use std::fs::File;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::{net::TcpStream, path::PathBuf};

use crate::parity::Entry;
use crate::request::{Request, RequestResult};
use anyhow::Result;

pub struct Connection(pub TcpStream);

impl Connection {
    #[inline]
    pub fn shutdown(&mut self, how: Shutdown) -> Result<()> {
        self.0.shutdown(how)?;
        Ok(())
    }

    #[inline]
    pub fn send_u32(&mut self, value: u32) -> Result<()> {
        self.0.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    #[inline]
    pub fn read_u32(&mut self) -> Result<u32> {
        let mut buffer = [0u8; 4];
        self.0.read_exact(&mut buffer)?;
        Ok(u32::from_le_bytes(buffer))
    }

    #[inline]
    pub fn send_string(&mut self, value: &String) -> Result<()> {
        let buffer = value.as_bytes();
        self.send_u32(buffer.len() as u32)?;
        self.0.write_all(buffer)?;
        Ok(())
    }

    #[inline]
    pub fn read_string(&mut self) -> Result<String> {
        let length = self.read_u32()? as usize;
        let mut buffer = vec![0u8; length];
        self.0.read_exact(&mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    #[inline]
    pub fn send_request(&mut self, request: &Request) -> Result<()> {
        let data = bincode::serialize(&request)?;
        let length = data.len() as u32;
        self.send_u32(length)?;
        self.0.write_all(&data)?;
        Ok(())
    }

    #[inline]
    pub fn read_request(&mut self) -> Result<Request> {
        let length = self.read_u32()? as usize;
        let mut buffer = vec![0u8; length];
        self.0.read_exact(&mut buffer)?;
        let request = bincode::deserialize::<Request>(&buffer)?;
        Ok(request)
    }

    #[inline]
    pub fn send_request_result(&mut self, result: RequestResult) -> Result<RequestResult> {
        let data = bincode::serialize(&result)?;
        let length = data.len();
        self.send_u32(length as u32)?;
        self.0.write_all(&data)?;
        Ok(result)
    }

    #[inline]
    pub fn read_request_result(&mut self) -> Result<RequestResult> {
        let length = self.read_u32()? as usize;
        let mut buffer = vec![0u8; length];
        self.0.read_exact(&mut buffer)?;
        let result = bincode::deserialize::<RequestResult>(&buffer)?;
        Ok(result)
    }

    #[inline]
    pub fn send_file(&mut self, entry: &Entry) -> Result<()> {
        dbg!(&entry);
        self.send_u32(entry.length as u32)?;
        let mut file = File::open(&entry.path)?;
        let mut file_buffer = [0u8; 4096];
        loop {
            let n = file.read(&mut file_buffer)?;
            if n == 0 {
                break;
            }
            self.0.write_all(&file_buffer[..n])?;
        }
        Ok(())
    }

    #[inline]
    pub fn read_file(&mut self, output: &PathBuf) -> Result<()> {
        let length = self.read_u32()? as usize;
        println!("Downloading file ({} MiB)", length / 1048576);
        let mut file = File::create(output)?;
        let mut buffer = [0u8; 4096];
        let mut bytes_read = 0;
        while bytes_read < length {
            let n = self.0.read(&mut buffer)?;
            bytes_read += n;
            file.write_all(&buffer[..n])?;
        }
        Ok(())
    }
}
