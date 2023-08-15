use std::{
    io::{Error, ErrorKind, Read, Result, Write},
    net::TcpStream,
};

#[derive(Debug)]
pub struct Message {
    pub term: u8,
    pub body: MessageType,
}

#[derive(Debug)]
pub enum MessageType {
    RequestVote,
    Heartbeat,
    AppendEntry,
}

#[derive(Debug)]
pub enum Response {
    Ack,
    Nack,
    ImNotLeader,
}

impl Message {
    pub fn read_from(stream: &mut TcpStream) -> Result<Self> {
        let mut buf = [0; 2];
        stream.read_exact(&mut buf)?;

        let term = buf[0];
        let body = match buf[1] {
            0 => MessageType::RequestVote,
            1 => MessageType::Heartbeat,
            2 => MessageType::AppendEntry,
            _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid type")),
        };

        Ok(Self { term, body })
    }

    pub fn write_to(&self, stream: &mut TcpStream) -> Result<()> {
        match self.body {
            MessageType::RequestVote => {
                stream.write_all(&[self.term, 0])?;
            }
            MessageType::Heartbeat => {
                stream.write_all(&[self.term, 1])?;
            }
            MessageType::AppendEntry => {
                stream.write_all(&[self.term, 2])?;
            }
        }
        Ok(())
    }
}

impl Response {
    pub fn read_from(stream: &mut TcpStream) -> Result<Self> {
        let mut buf = [0; 1];
        stream.read_exact(&mut buf)?;

        match buf[0] {
            0 => Ok(Self::Ack),
            1 => Ok(Self::Nack),
            2 => Ok(Self::ImNotLeader),
            _ => Err(Error::new(ErrorKind::InvalidData, "Invalid Response")),
        }
    }

    pub fn write_to(&self, stream: &mut TcpStream) -> Result<()> {
        match self {
            Response::Ack => {
                stream.write_all(&[0])?;
            }
            Response::Nack => {
                stream.write_all(&[1])?;
            }
            Response::ImNotLeader => {
                stream.write_all(&[2])?;
            }
        }
        Ok(())
    }
}
