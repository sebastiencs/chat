[![Build Status](https://travis-ci.org/sebastiencs/chat.svg?branch=master)](https://travis-ci.org/sebastiencs/chat)

# chat
Simple one on one chat

## Summary
This crate make use of [actix](https://actix.rs/) Actor system and [tokio](https://tokio.rs/) asynchronous run-time.

The application can start in 2 modes:
- Server: waiting for client(s) to connect
- Client: Connecting to a server

## Features:
- The server can communicate with differents clients simultaneously.  
- Both client and server can send message to the other side.  
- When a message is received on one side, it automatically send back "message received".  
- The sending side show the roundtrip time.  
- Any data can be send: binary, text (any encoding)  

## Build
```shell
cargo build --release
```

## Run
```shell
chat # Run as server
chat --client # Run as client
chat --help # For more options and informations
```

## Documentation
```shell
cargo doc --document-private-items --open
```

## Tests
```shell
cargo test
```
