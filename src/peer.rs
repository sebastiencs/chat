//! Peer Actor

use actix::dev::ToEnvelope;
use tokio_io::io::WriteHalf;
use actix::prelude::*;
use actix::io::{Writer, WriteHandler};
use tokio_tcp::TcpStream;
use bytes::Bytes;
use std::time::{Instant};
use std::collections::VecDeque;
use tokio_io::AsyncRead;

use reader::{Reader, ReaderError, Kind, to_binary};
use user::UserInput;
use {Config, Display};

/// Peer Actor
///
/// A Peer is responsible of writing and reading datas to/from an owning socket
pub struct Peer<T>
where
    T: Actor,
    T: Handler<PeerClose>,
    T::Context: ToEnvelope<T, PeerClose>
{
    /// Parent Actor
    parent: Addr<T>,
    /// An handle to a writable socket
    writer: Writer<WriteHalf<TcpStream>, ::std::io::Error>,
    /// List of [`Instant`] used to determine the roundtrip time
    /// of a message
    delays: VecDeque<Instant>,
    /// Configuration
    config: Config
}

/// A Actix message to notify that the Peer as been stopped
#[derive(Message)]
pub struct PeerClose;

impl<T> Peer<T>
where
    T: Actor,
    T: Handler<PeerClose>,
    T::Context: ToEnvelope<T, PeerClose>
{
    /// Create a Peer
    /// It takes ownership of the socket and add the stream of the
    /// socket to its Context Actor.
    pub fn new(config: Config, parent: Addr<T>, socket: TcpStream) -> Addr<Peer<T>> {
        let (read, write) = socket.split();

        Peer::create(move |ctx| {
            ctx.add_stream(Reader::new(read));
            let mut writer = actix::io::Writer::new(write, ctx);
            writer.set_buffer_capacity(0, 0);

            Peer { parent, writer, delays: VecDeque::new(), config }
        })
    }
}

impl<T> Actor for Peer<T>
where
    T: Actor,
    T: Handler<PeerClose>,
    T::Context: ToEnvelope<T, PeerClose>
{
    type Context = Context<Self>;

    fn stopped(&mut self, _: &mut Self::Context) {
        // Socket as been closed, notify the parent
        self.parent.do_send(PeerClose);
    }
}

impl<T> Handler<UserInput> for Peer<T>
where
    T: Actor,
    T: Handler<PeerClose>,
    T::Context: ToEnvelope<T, PeerClose>
{
    type Result = ();

    fn handle(&mut self, msg: UserInput, _: &mut Context<Self>) {
        // The user as submitted data, write it on the socket
        self.delays.push_back(Instant::now());
        self.writer.write(&to_binary(msg.0.as_ref(), Kind::Data));
    }
}

impl<T> WriteHandler<::std::io::Error> for Peer<T>
where
    T: Actor,
    T: Handler<PeerClose>,
    T::Context: ToEnvelope<T, PeerClose>
{}

/// Parsed message
///
/// A message after being read and parsed
#[derive(Debug)]
pub struct Msg {
    /// Bytes of the received datas, including header
    bytes: Bytes,
    /// [`Kind`] of the message
    kind: Kind,
    /// Header len
    header_len: usize
}

impl Msg {
    pub fn new(bytes: Bytes, kind: Kind, header_len: usize) -> Msg {
        Msg { bytes, kind, header_len }
    }

    /// Return the message without the header
    pub fn message(&self) -> Bytes {
        self.bytes.slice_from(self.header_len)
    }
}

impl<T> StreamHandler<Msg, ReaderError> for Peer<T>
where
    T: Actor,
    T: Handler<PeerClose>,
    T::Context: ToEnvelope<T, PeerClose>
{
    /// This function is called once the message has been fully read
    /// and parsed to a [`Msg`].
    fn handle(&mut self, msg: Msg, _ctx: &mut Self::Context) {
        match msg.kind {
            Kind::Data => {
                let bin = to_binary(b"message received", Kind::Response);
                self.writer.write(bin.as_ref());
                let message = msg.message();
                match self.config.display {
                    Display::Binary => println!("Message: {:?}", message),
                    Display::Utf8 => {
                        match String::from_utf8(message.to_vec()) {
                            Ok(utf8) => println!("Message[utf8]: {}", utf8),
                            _ => println!("Message: {:?}", message)
                        }
                    },
                    _ => println!("{} bytes received", message.len())
                }
            },
            Kind::Response => {
                let delay = self.delays.pop_front()
                                       .map(|s| s.elapsed())
                                       .unwrap_or_default();
                println!("Response: {:?} in {:?}", msg.message(), delay);
            },
            Kind::Wrong => {
            }
        }
    }
}
