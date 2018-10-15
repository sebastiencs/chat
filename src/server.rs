//! Server Actor

use actix::prelude::*;
use tokio_tcp::{TcpListener, TcpStream};
use futures::stream::Stream;
use std::net::{SocketAddrV4, Ipv4Addr};

use peer::{Peer, PeerClose};
use user::{User, UserInput};
use Config;

/// Address of a [`User`]
type AUser = Addr<User<Server>>;
/// Address of a [`Peer`]
type APeer = Addr<Peer<Server>>;

/// Server Actor
///
/// The Server is responsible of handling new tcp connection,
/// [`User`] input and managing connected [`Peer`]
///
pub struct Server {
    /// List of connected [`Peer`]s
    peers: Vec<APeer>,
    /// A [`User`] actor
    user: Option<AUser>,
    /// Configuration
    config: Config
}

impl Server {
    /// Return a new [`Server`] with some configuration
    pub fn new(config: Config) -> Server {
        Server {
            peers: vec![],
            user: None,
            config
        }
    }
}

/// Wrap a [`TcpStream`] to handle the stream as an Actix message
#[derive(Message)]
struct TcpConnect(pub TcpStream);

impl Actor for Server {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // We start to bind the socket
        let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), self.config.port);

        let listener = match TcpListener::bind(&addr.into()) {
            Ok(listener) => listener,
            Err(e) => {
                println!("Can not bind to the address: {}", e);
                System::current().stop();
                return;
            }
        };

        let addr = listener.local_addr();

        // Add the socket as a stream to our actor's context
        ctx.add_message_stream(listener.incoming().map_err(|_| ()).map(|st| {
            TcpConnect(st)
        }));

        // Start the User actor
        let server = ctx.address();
        let user = Arbiter::start(|_| User::new(server));
        self.user = Some(user);

        println!("Running as server");
        if let Ok(addr) = addr {
            println!("Listening on {}", addr);
        };
    }
}

impl Handler<TcpConnect> for Server {
    type Result = ();

    fn handle(&mut self, tcp: TcpConnect, ctx: &mut Context<Self>) {
        // A new connection is established.
        // Create a Peer from it and add it to self.peers
        let socket: TcpStream = tcp.0;
        socket.set_nodelay(true).ok();
        self.peers.push(Peer::new(self.config.clone(), ctx.address(), socket));
    }
}

impl Handler<UserInput> for Server {
    type Result = ();

    fn handle(&mut self, input: UserInput, _ctx: &mut Context<Self>) {
        // Send the user input to all connected peers
        for peer in &self.peers {
            peer.do_send(input.clone());
        };
    }
}

impl Handler<PeerClose> for Server {
    type Result = ();

    fn handle(&mut self, _: PeerClose, _ctx: &mut Context<Self>) {
        // A connection has been close, clean self.peers
        self.peers.retain(Addr::connected);
    }
}
