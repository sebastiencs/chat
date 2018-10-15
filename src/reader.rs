//! Parse received data and format writable data

use tokio_io::io::ReadHalf;
use futures::{Async, Poll};
use tokio_tcp::TcpStream;
use tokio_io::AsyncRead;
use futures::stream::Stream;
use bytes::{BufMut, BytesMut, Bytes};
use byteorder::{ByteOrder, NetworkEndian};

use peer::Msg;
use MESSAGE_MAX_LEN;

/// Kind of a message
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Kind {
    /// The data is a normal message
    Data,
    /// The data is a response to a message
    Response,
    /// Invalid data
    Wrong
}

impl Into<u8> for Kind {
    fn into(self) -> u8 {
        match self {
            Kind::Data => 0,
            Kind::Response => 1,
            Kind::Wrong => 2,
        }
    }
}

impl From<u8> for Kind {
    fn from(byte: u8) -> Kind {
        match byte {
            0 => Kind::Data,
            1 => Kind::Response,
            _ => Kind::Wrong,
        }
    }
}

/// Errors when parsing data
#[derive(Debug)]
pub enum ReaderError {
    /// Wrong flag for the message [`Kind`]
    WrongKindFlag,
    /// Wrong flag for the message length
    WrongLengthFlag,
    /// The length of the received data doesn't match header infomation
    IncorrectSize,
    /// std input/output error
    IO(::std::io::Error),
}

/// The Reader is responsible of parsing the received data
/// and return a [`Msg`]
pub struct Reader {
    /// An handle to a readable socket
    read: ReadHalf<TcpStream>,
    /// Buffer where we read the incoming data
    pending: BytesMut,
}

/// Information of the message from its header
#[derive(Clone, Copy)]
struct PayloadInfo {
    kind: Kind,
    received_len: usize,
    bytes_capacity: usize,
    payload_len: usize,
    header_len: usize
}

impl Reader {
    pub fn new(read: ReadHalf<TcpStream>) -> Reader {
        Reader { read, pending: BytesMut::new() }
    }

    /// Parse the header. It can takes differents size
    ///
    /// The header consists of:
    ///
    /// ## First byte:
    ///
    /// - HEADER\[0\] & 0x0F = [`Kind`] flag.
    /// - HEADER\[0\] & 0xF0 = Length flag (0x10, 0x20, 0x40 or 0x80).
    ///
    /// ## Following bytes:
    ///
    /// The following bytes correspond to the message length, depending of the length flag:
    /// - flag = 0x10 =>  HEADER\[1\] as u8
    /// - flag = 0x20 =>  HEADER[1, 2] as u16
    /// - flag = 0x40 =>  HEADER[1, 2, 3, 4] as u32
    /// - flag = 0x80 =>  HEADER[1, 2, 3, 4, 5, 6, 7, 8] as u64
    ///
    fn parse_header(&self) -> Result<Option<PayloadInfo>, ReaderError> {
        let bytes = self.pending.as_ref();
        let received_len = bytes.len();
        let bytes_capacity = self.pending.capacity();

        if received_len < 1 {
            return Ok(None);
        }

        let len_flag = bytes[0] & 0xF0;
        let kind = match Kind::from(bytes[0] & 0x0F) {
            Kind::Wrong => return Err(ReaderError::WrongKindFlag),
            kind => kind
        };

        let (uint_len, header_len) = match len_flag {
            0x10 => (1, 2),
            0x20 => (2, 3),
            0x40 => (4, 5),
            0x80 => (8, 9),
            _ => return Err(ReaderError::WrongLengthFlag)
        };

        if received_len < header_len {
            return Ok(None);
        }

        let payload_len = NetworkEndian::read_uint(&bytes[1..], uint_len) as usize;

        Ok(Some(PayloadInfo {
            kind, received_len, bytes_capacity, payload_len, header_len
        }))
    }

    /// Parse message and reallocate if necessary
    fn parse(&mut self) -> Poll<Option<Msg>, ReaderError> {
        let PayloadInfo {
            kind,
            received_len,
            bytes_capacity,
            payload_len,
            header_len
        } = match self.parse_header()? {
            Some(info) => info,
            None => return Ok(Async::NotReady),
        };

        let data_len = header_len + payload_len;

        if received_len < data_len {
            // We didn't received the full message
            if bytes_capacity < data_len {
                // The buffer is smaller than the message
                self.pending.reserve((data_len + 1) - bytes_capacity);
            }
            Ok(Async::NotReady)
        } else if received_len > data_len || payload_len > MESSAGE_MAX_LEN as usize {
            Err(ReaderError::IncorrectSize)
        } else {
            let msg = self.pending.take().into();
            self.pending.reserve(64);
            Ok(Async::Ready(Some(Msg::new(
                msg,
                kind,
                header_len
            ))))
        }
    }
}

impl Stream for Reader {
    type Item = Msg;
    type Error = ReaderError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            match self.read.read_buf(&mut self.pending)
                           .map_err(ReaderError::IO)? {
                Async::Ready(0) => return Ok(Async::Ready(None)),
                Async::Ready(_) => match self.parse() {
                    Ok(Async::NotReady) => (),
                    Err(e) => {
                        // Error while parsing, we drop the received data
                        let _ = self.pending.take();
                        self.pending.reserve(64);
                        return Err(e)
                    }
                    x => return x
                },
                Async::NotReady => return Ok(Async::NotReady),
            }
        }
    }
}

/// Make a ready-to-send buffer, with the header.
/// For the header format, see [`Reader::parse_header()`]
pub fn to_binary(data: &[u8], kind: Kind) -> Bytes {
    let kind_flag: u8 = kind.into();

    let mut buf = match data.len() {
        len if len <= 0xFF => {
            let mut buf = BytesMut::with_capacity(len + 2);
            buf.put_slice(&[kind_flag | 0x10, len as u8]);
            buf
        },
        len if len <= 0xFFFF => {
            let mut buf = BytesMut::with_capacity(len + 3);
            buf.put_u8(kind_flag | 0x20);
            buf.put_u16_be(len as u16);
            buf
        },
        len if len <= 0xFFFF_FFFF => {
            let mut buf = BytesMut::with_capacity(len + 5);
            buf.put_u8(kind_flag | 0x40);
            buf.put_u32_be(len as u32);
            buf
        },
        len => {
            let mut buf = BytesMut::with_capacity(len + 9);
            buf.put_u8(kind_flag | 0x80);
            buf.put_u64_be(len as u64);
            buf
        }
    };

    buf.put_slice(data);
    buf.into()
}
