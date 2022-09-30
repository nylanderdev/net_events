use std::io::{self, ErrorKind, Read, Write};
use std::marker::PhantomData;
use std::net::TcpStream;

use crate::protocol::{ParseHint, Serial};

pub struct Conn<M: Serial> {
    socket: TcpStream,
    buffer: Vec<u8>,
    bytes_required: Option<usize>,
    _m: PhantomData<M>,
}

impl<M: Serial> Conn<M> {
    pub fn from_tcp_stream(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;
        Ok(Self {
            socket: stream,
            buffer: vec![],
            bytes_required: None,
            _m: PhantomData::default(),
        })
    }
    pub fn send(&mut self, m: &M) -> SendResult {
        let bytes = m.serialize();
        match bytes {
            Ok(bytes) => {
                let write_res = self.socket.write(&bytes);
                if write_res.is_err() {
                    SendResult::Disconnected
                } else {
                    SendResult::Ok
                }
            }
            Err(_) => SendResult::Invalid
        }
    }
    pub fn recv(&mut self) -> RecvResult<M> {
        let bytes_to_request = if let Some(bytes_required) = self.bytes_required {
            bytes_required
        } else if self.buffer.is_empty() {
            M::minimum_size()
        } else {
            let parse_hint = M::parse_hint(&self.buffer);
            match parse_hint {
                ParseHint::Complete(_) => 0,
                ParseHint::Incomplete(missing) => missing,
                ParseHint::Invalid => return RecvResult::Invalid
            }
        };
        let mut buf = vec![0; bytes_to_request];
        let byte_recv_count = self.socket.read(&mut buf);
        if let Err(e) = byte_recv_count {
            return match e.kind() {
                ErrorKind::WouldBlock => RecvResult::None,
                _ => RecvResult::Disconnected
            };
        }
        let byte_recv_count = byte_recv_count.unwrap();
        self.buffer.extend_from_slice(&buf[..byte_recv_count]);
        if byte_recv_count < bytes_to_request {
            self.bytes_required = Some(bytes_to_request - byte_recv_count);
            RecvResult::None
        } else {
            let parse_hint = M::parse_hint(&self.buffer);
            match parse_hint {
                ParseHint::Complete(to_consume) => {
                    let m = M::deserialize(&self.buffer[..to_consume]);
                    let msg = match m {
                        Ok((m, _)) => RecvResult::Message(m),
                        Err(_) => RecvResult::Invalid
                    };
                    self.buffer.drain(..to_consume);
                    self.bytes_required = None;
                    msg
                }
                ParseHint::Incomplete(missing) => {
                    self.bytes_required = Some(missing);
                    self.recv()
                }
                ParseHint::Invalid => RecvResult::Invalid
            }
        }
    }
}

pub enum SendResult {
    Ok,
    Disconnected,
    Invalid,
}

pub enum RecvResult<M> {
    Disconnected,
    Message(M),
    Invalid,
    None,
}
