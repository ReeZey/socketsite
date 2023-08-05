use std::{collections::HashMap, io::Cursor};

use http::handle_http;
use socket::handle_socket;
use tokio::{net::{TcpListener, TcpStream}, io::{BufReader, AsyncBufReadExt, AsyncReadExt}};

mod http;
mod socket;

#[tokio::main]
async fn main() {
    let server = TcpListener::bind("0.0.0.0:80").await.unwrap();

    loop {
        let (stream, _socket_addr) = server.accept().await.unwrap();
        tokio::spawn(async move {
            handle_connection(stream).await;
        });
    }
}

async fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let mut lines = buf_reader.lines();
    let inital = lines.next_line().await.unwrap().unwrap();

    println!("now");
    
    let mut headers: HashMap<String, String> = HashMap::new();
    while let Some(line) = lines.next_line().await.unwrap() {
        if line.len() == 0 { break; }

        //let lowercase = line.to_lowercase();
        let (key, value) = line.split_once(": ").unwrap();
        headers.insert(key.to_owned(), value.to_owned());
    }

    if let Some(upgrade_type) = headers.get("Upgrade") {
        if upgrade_type == "websocket" {
            println!("socket");

            handle_socket(&mut stream, inital, headers).await;
            return;
        }
    }
    println!("http");
    handle_http(&mut stream, inital, headers).await;
}