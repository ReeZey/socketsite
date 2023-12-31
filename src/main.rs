use std::collections::HashMap;

use http::handle_http;
use socket::{handle_socket, Message};
use tokio::{net::{TcpListener, TcpStream}, io::{BufReader, AsyncBufReadExt}, sync::broadcast::{channel, Sender, Receiver}};

use uuid::Uuid;

mod http;
mod socket;

#[tokio::main]
async fn main() {
    let server = TcpListener::bind("0.0.0.0:51413").await.unwrap();

    let (sender, _) = channel(10);

    loop {
        let (stream, _socket_addr) = server.accept().await.unwrap();

        let local_sender = sender.clone();
        let local_receiver = sender.subscribe();

        tokio::spawn(async move {
            handle_connection(stream, local_sender, local_receiver).await;
        });
    }
}

async fn handle_connection(mut stream: TcpStream, sender: Sender<Message>, receiver: Receiver<Message>) {
    let buf_reader = BufReader::new(&mut stream);
    let mut lines = buf_reader.lines();
    let inital = lines.next_line().await.unwrap().unwrap();

    //println!("now");
    
    let mut headers: HashMap<String, String> = HashMap::new();
    while let Some(line) = lines.next_line().await.unwrap() {
        if line.len() == 0 { break; }

        //let lowercase = line.to_lowercase();
        let (key, value) = line.split_once(": ").unwrap();
        headers.insert(key.to_owned(), value.to_owned());
    }

    if let Some(upgrade_type) = headers.get("Upgrade") {
        if upgrade_type == "websocket" {
            //println!("socket");

            println!("> {:?}", stream.peer_addr().unwrap());
            handle_socket(stream, inital, headers, sender, receiver).await;
            return;
        }
    }
    //println!("http");
    handle_http(&mut stream, inital, headers).await;
}

#[derive(Debug, Clone)]
pub struct User {
    pub uuid: Uuid,
    pub name: String
}