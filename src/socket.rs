use std::{collections::HashMap, time::Duration, thread};
use tokio::{net::TcpStream, io::AsyncWriteExt};
use base64::{Engine as _, engine::general_purpose};
use tokio::io::AsyncReadExt;
use std::io::Write;

const MAGIC_HASH: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub async fn handle_socket(stream: &mut TcpStream, _inital: String, headers: HashMap<String, String>) {
    let key = headers.get("Sec-WebSocket-Key").unwrap();

    let to_be_hashed = format!("{}{}", key, MAGIC_HASH);

    let mut hasher = sha1_smol::Sha1::new();
    hasher.update(to_be_hashed.as_bytes());

    let hash: Vec<u8> = hasher.digest().bytes().to_vec();

    let encoded: String = general_purpose::STANDARD.encode(hash);

    let mut buffer = vec![];
    write!(&mut buffer, "HTTP/1.1 101 Switching Protocols\r\n").unwrap();
    write!(&mut buffer, "Connection: Upgrade\r\n").unwrap();
    write!(&mut buffer, "Upgrade: websocket\r\n").unwrap();
    write!(&mut buffer, "Sec-WebSocket-Accept: {}\r\n", encoded).unwrap();
    write!(&mut buffer, "\r\n").unwrap();
    stream.write(&mut buffer).await.unwrap();

    let mut output = vec![];
    loop {
        let first_byte: u8 = stream.read_u8().await.unwrap();
        let secound_byte: u8 = stream.read_u8().await.unwrap();

        let opcode: usize = (first_byte & 0b00001111) as usize;
        let fin: bool = (first_byte & 0b10000000) != 0;

        let mut msglen: usize = (secound_byte & 0b01111111) as usize;
        let mask: bool = (secound_byte & 0b10000000) != 0;

        println!("{} {}", opcode, mask);

        if msglen == 126 {
            msglen = stream.read_u16().await.unwrap() as usize;
        }

        if msglen == 127 {
            msglen = stream.read_u64().await.unwrap() as usize;
        }

        if !mask {
            return;
        }

        let mask = [
            stream.read_u8().await.unwrap(), 
            stream.read_u8().await.unwrap(), 
            stream.read_u8().await.unwrap(), 
            stream.read_u8().await.unwrap()
        ];

        for i in 0..msglen {
            output.push(stream.read_u8().await.unwrap() ^ mask[i % 4]);
        }

        if fin {
            println!("{:X?}", output);

            if output == b"hello" {
                println!("hejsan");
                
                let mut buffer: Vec<u8> = vec![];
                let first_byte: u8 = 0b1000_0001;
                buffer.push(first_byte);
                
                let secound_byte: u8 = 0b1000_0101;
                buffer.push(secound_byte);
                buffer.extend([0,0,0,0]);
                buffer.extend([0x68, 0x65, 0x6C, 0x6C, 0x6F]);
                stream.write(&buffer).await.unwrap();
            }

            output.clear();
        }
    }
}