//! User Actor

use actix::prelude::*;
use actix::dev::ToEnvelope;
use std::io::Read;
use atty;

use MESSAGE_MAX_LEN;

/// Input datas
///
/// Wrap the input data to be sent between differents actors
/// (User -> Client/Server) as an Actix message
#[derive(Message, Clone)]
pub struct UserInput(pub Vec<u8>);

/// User Actor, reads data on stdin
///
/// When the data is read, it is sent as an Actix message to its parent
/// (Client/Server)
pub struct User<T>
where
    T: Actor,
    T: Handler<UserInput>,
    T::Context: ToEnvelope<T, UserInput>
{
    /// Address of the Actor that created `User`
    parent: Addr<T>
}

impl<T> User<T>
where
    T: Actor,
    T: Handler<UserInput>,
    T::Context: ToEnvelope<T, UserInput>
{
    pub fn new(parent: Addr<T>) -> Self {
        User { parent }
    }

    /// Loop reading stdin
    fn read_stdin(&self) {
        let isatty = atty::is(atty::Stream::Stdin);

        if isatty {
            println!("Reading stdin, CTRL+D to send\n");
        }

        loop {
            let mut input = Vec::new();
            if let Err(e) = ::std::io::stdin().read_to_end(&mut input) {
                println!("stdin error: {:?}", e);
                return;
            }
            if !isatty && input.is_empty() {
                println!("No more data on stdin.");
                println!("Still can receive messages from others..\n");
                return;
            }
            if input.len() > MESSAGE_MAX_LEN as usize {
                println!("Message is too big, cancelled");
                continue;
            }
            self.parent.do_send(UserInput(input));
        };
    }
}

impl<T> Actor for User<T>
where
    T: Actor,
    T: Handler<UserInput>,
    T::Context: ToEnvelope<T, UserInput>
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // The actor is created, start to read stdin
        self.read_stdin();
        ctx.stop();
    }
}
