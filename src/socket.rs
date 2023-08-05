use std::collections::HashMap;
use tokio::{net::TcpStream, io::AsyncWriteExt};
use base64::{Engine as _, engine::general_purpose};
use tokio::io::AsyncReadExt;
use std::io::Write;

const MAGIC_HASH: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub async fn handle_socket(stream: &mut TcpStream, _inital: String, headers: HashMap<String, String>) {
    
    let mut buffer = vec![];

    //println!("{:#?}", headers);

    let key = headers.get("Sec-WebSocket-Key").unwrap();

    println!("key: {:?}", key);

    let to_be_hashed = format!("{}{}", key, MAGIC_HASH);

    println!("before sha1: {:?}", to_be_hashed);

    let mut hasher = sha1_smol::Sha1::new();
    hasher.update(to_be_hashed.as_bytes());

    let hash: String = hasher.digest().to_string();

    println!("sha1: {:?}", hash);

    let encoded: String = general_purpose::STANDARD.encode(hash);

    println!("encoded: {:?}", encoded);

    write!(&mut buffer, "HTTP/0.9 101 Switching Protocols\r\n").unwrap();
    write!(&mut buffer, "Connection: Upgrade\r\n").unwrap();
    write!(&mut buffer, "Upgrade: websocket\r\n").unwrap();
    write!(&mut buffer, "Sec-WebSocket-Accept: {}\r\n", encoded).unwrap(); 
    write!(&mut buffer, "\r\n").unwrap();

    stream.write(&mut buffer).await.unwrap();
    stream.flush().await.unwrap();

    let mut data = vec![];
    loop {
        stream.read_buf(&mut data).await.unwrap();
        println!("{:?}", stream);
    }

    /*
    
    HTTP/1.1 101 Switching Protocols
Upgrade: foo/2
Connection: Upgrade

     */
}