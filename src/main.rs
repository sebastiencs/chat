//! Chat client/server
//!
//! # Summary
//!
//! This crate make use of [`actix`] Actor system and [`tokio`] asynchronous
//! run-time.
//!
//! The application can start in 2 modes:
//! - Server: waiting for client(s) to connect
//! - Client: Connecting to a server
//!
//! # Features:
//!
//! - The server can communicate with differents clients simultaneously.  
//! - Both client and server can send message to the other side.  
//! - When a message is received on one side, it automatically send back "message received".  
//! - The sending side show the roundtrip time.
//! - Any data can be send: binary, text (any encoding)
//! 

#[macro_use]
extern crate actix;
extern crate futures;
extern crate tokio_tcp;
extern crate tokio_reactor;
extern crate tokio_io;
extern crate tokio;
extern crate bytes;
extern crate byteorder;
extern crate atty;
extern crate clap;

use std::str::FromStr;
use actix::prelude::*;
use clap::{App, Arg};

mod server;
mod client;
mod reader;
mod peer;
mod user;

use client::Client;
use server::Server;

/// Maximum allowed message length
pub const MESSAGE_MAX_LEN: u64 = 0x0001_0000_0000_0000;

/// How to display received messages
#[derive(Debug, Clone)]
pub enum Display {
    /// Display data as binary
    Binary,
    /// Display data as utf8 if possible, otherwise as binary
    Utf8,
    /// Don't display data
    None
}

impl<'a> From<&'a str> for Display {
    fn from(s: &str) -> Display {
        match s {
            "binary" => Display::Binary,
            "utf8" => Display::Utf8,
            _ => Display:: None
        }
    }
}

/// Chat configuration
///
/// The structure is filled with the command line arguments
#[derive(Debug, Clone)]
pub struct Config {
    /// Run in client mode
    pub is_client: bool,
    /// Host address/hostname
    pub host: String,
    /// Port
    pub port: u16,
    /// Display mode
    pub display: Display
}

/// Read command line arguments and return a [`Config`]
fn get_config() -> Config {
    let args = App::new("chat")
        .version("1.0")
        .author("Sebastien Chapuis. <sebastien@chapu.is>")
        .about("A chat client/server")
        .arg(Arg::with_name("client")
             .short("c")
             .long("client")
             .help("Run as client"))
        .arg(Arg::with_name("host")
             .short("H")
             .long("host")
             .help("Address/hostname of the host to connect in client mode")
             .takes_value(true)
             .default_value("127.0.0.1"))
        .arg(Arg::with_name("port")
             .short("p")
             .long("port")
             .help("Port number to bind/listen")
             .takes_value(true)
             .validator(|s| u16::from_str(&s)
                        .map_err(|_| "Should be a number between 0 and 65535".to_owned())
                        .map(|_| ()))
             .default_value("12345"))
        .arg(Arg::with_name("display")
             .long("display")
             .help(
"How to display received messages
- binary: Display as binary.
- utf8: Try to display as utf8 text.
- none: Don't display received messages.\n")
             .possible_values(&["binary", "utf8", "none"])
             .takes_value(true)
             .default_value("binary"))
        .get_matches();

    Config {
        is_client: args.is_present("client"),
        host: args.value_of("host")
                  .map(|h| h.to_owned())
                  .unwrap(),
        port: args.value_of("port")
                  .and_then(|p| u16::from_str(&p).ok())
                  .unwrap(),
        display: args.value_of("display")
                     .map(Display::from)
                     .unwrap()
    }
}

fn main() {
    let config = get_config();

    System::run(|| {
        if config.is_client {
            Client::new(config).start();
        } else {
            Server::new(config).start();
        }
    });
}
