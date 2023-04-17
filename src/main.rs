use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
};

struct RESPString {
    value: String,
}

impl std::fmt::Display for RESPString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{}\r\n", self.value)
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                for line in BufReader::new(stream.try_clone().unwrap()).lines() {
                    stream.write(b"+PONG\r\n").unwrap();
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
