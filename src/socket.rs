use std::collections::HashMap;
use tokio::{net::TcpStream, io::AsyncWriteExt};
use base64::{Engine as _, engine::general_purpose};
use tokio::io::AsyncReadExt;
use std::io::Write;
use rand::prelude::*;

const MAGIC_HASH: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub async fn handle_socket(mut stream: &mut TcpStream, _inital: String, headers: HashMap<String, String>) {
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

    let mut data = vec![];
    loop {
        let frame = FrameUtils::read(&mut stream).await;
        data.extend(frame.data);
        if frame.finished {
            //println!("opcode: {:X} - data: {:X?}", frame.opcode, data);
            match frame.opcode {
                0 => {
                    continue;
                }
                1 => {
                    let message = String::from_utf8(data.clone()).unwrap();

                    if message == "hello" {
                        let frame_out = FrameUtils::create(1,true, b"hello".to_vec(), Some(FrameUtils::random_mask()));
                        stream.write(&frame_out).await.unwrap();
                    }

                    println!("message: {}", message);
                }
                2 => {
                    println!("binary: {:X?}", data.clone());
                }
                8 => {
                    println!("socket gone 🦀");
                    return;
                }
                _ => {}
            }
            /*
            //ping example
            thread::sleep(Duration::from_secs(5));

            let frame_out = FrameUtils::create(9, true, vec![0,1,2,3], Some(FrameUtils::random_mask()));
            stream.write(&frame_out).await.unwrap();
            */
            data.clear();
        }
    }
}

struct FrameUtils {
    opcode: u8,
    finished: bool,
    data: Vec<u8>
}

impl FrameUtils {
    fn create(opcode: u8, finished: bool, data: Vec<u8>, mask: Option<[u8; 4]>) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];

        let mut first_byte: u8 = opcode & 15;
        if finished {
            first_byte |= 128;
        }
        buffer.push(first_byte);

        if data.len() > 65535 {
            buffer.push(255);
            let length = data.len() as u64;
            buffer.extend(length.to_be_bytes())
        } else if data.len() > 125 {
            buffer.push(254);
            let length = data.len() as u16;
            buffer.extend(length.to_be_bytes())
        } else {
            buffer.push(data.len() as u8 | 128);
        }

        match mask {
            Some(mask) => {
                buffer.extend(mask);
                for (index, byte) in data.iter().enumerate() {
                    buffer.push(byte ^ mask[index % 4]);
                }
            }
            None => {
                buffer.extend([0, 0, 0, 0]);
                buffer.extend(data);
            }
        }

        return buffer;
    }
    async fn read(stream: &mut TcpStream) -> Self {
        let first_byte = stream.read_u8().await.unwrap();
        let secound_byte = stream.read_u8().await.unwrap();

        let mut msg_len = (secound_byte & 0b01111111) as u64;

        if msg_len == 126 {
            msg_len = stream.read_u16().await.unwrap() as u64;
        }

        if msg_len == 127 {
            msg_len = stream.read_u64().await.unwrap();
        }

        let masked = (secound_byte & 0b10000000) != 0;

        let mask = match masked {
            true => {
                [
                    stream.read_u8().await.unwrap(), 
                    stream.read_u8().await.unwrap(), 
                    stream.read_u8().await.unwrap(), 
                    stream.read_u8().await.unwrap()
                ]
            }
            false => {
                [ 0, 0, 0, 0 ]
            }
        };

        let mut data: Vec<u8> = vec![];

        for i in 0..msg_len as usize {
            data.push(stream.read_u8().await.unwrap() ^ mask[i % 4]);
        }

        Self {
            opcode: (first_byte & 0b00001111),
            finished: (first_byte & 0b10000000) != 0,
            data,
        }
    }
    fn random_mask() -> [u8; 4] {
        let mut rng = rand::thread_rng();

        return [rng.gen(), rng.gen(), rng.gen(), rng.gen()];
    }
}