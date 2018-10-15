//! Client Actor

use actix::prelude::*;
use tokio_tcp::TcpStream;
use tokio_reactor::Handle;

use peer::{Peer, PeerClose};
use user::{UserInput, User};
use Config;

/// Address of a [`Peer`]
type APeer = Addr<Peer<Client>>;
/// Address of a [`User`]
type AUser = Addr<User<Client>>;

/// The Client Actor connect to a server, handle user input and
/// send/read data to/from the server
pub struct Client {
    /// The [`Peer`] Actor
    peer: Option<APeer>,
    /// The [`User`] Actor
    user: Option<AUser>,
    /// Configuration
    config: Config
}

impl Client {
    /// Create a Client
    pub fn new(config: Config) -> Client {
        Client {
            peer: None,
            user: None,
            config
        }
    }
}

impl Handler<UserInput> for Client {
    type Result = ();

    fn handle(&mut self, input: UserInput, _ctx: &mut Context<Self>) {
        if let Some(ref peer) = self.peer {
            peer.do_send(input);
        };
    }
}

impl Actor for Client {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Connect to the server
        let host = self.config.host.as_str();
        let port = self.config.port;

        let socket = match ::std::net::TcpStream::connect((host, port))
            .and_then(|socket| TcpStream::from_std(socket, &Handle::default()))
        {
            Ok(socket) => socket,
            Err(e) => {
                println!("Can not connect to server: {}", e);
                System::current().stop();
                return;
            }
        };

        socket.set_nodelay(true).ok();

        // Connected, we create a Peer
        let peer = Peer::new(self.config.clone(), ctx.address(), socket);

        // Start a User to handle input
        let client = ctx.address();
        let user = Arbiter::start(|_| {
            User::new(client)
        });

        self.peer = Some(peer);
        self.user = Some(user);

        println!("Running as client");
    }
}

impl Handler<PeerClose> for Client {
    type Result = ();

    fn handle(&mut self, _: PeerClose, _ctx: &mut Context<Self>) {
        println!("Connection closed");
        ::std::process::exit(1);
    }
}
