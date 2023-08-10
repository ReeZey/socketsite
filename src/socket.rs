use std::{collections::HashMap, sync::Arc};
use tokio::{net::{TcpStream, tcp::OwnedReadHalf}, io::AsyncWriteExt, sync::{broadcast::{Sender, Receiver}, Mutex}};
use base64::{Engine as _, engine::general_purpose};
use tokio::io::AsyncReadExt;
use std::io::Write;
use rand::prelude::*;
use uuid::Uuid;

const MAGIC_HASH: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub async fn handle_socket(mut stream: TcpStream, _inital: String, headers: HashMap<String, String>, sender: Sender<Message>, mut receiver: Receiver<Message>) {
    let mut key: String = headers.get("Sec-WebSocket-Key").unwrap().to_owned();

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

    //let frame_out = FrameUtils::create(1,true, b"connected!".to_vec(), Some(FrameUtils::random_mask()));
    //stream.write(&frame_out).await.unwrap();

    let uuid = Uuid::new_v4();
    let (mut read_stream, mut write_stream) = stream.into_split();
    
    sender.send(Message::connect(&key, uuid)).unwrap();
    sender.send(Message::only_self(&format!("connected! you are {:?}", key), uuid)).unwrap();

    let alive = Arc::new(Mutex::new(true));
    let recv_alive = alive.clone();

    tokio::spawn(async move {
        while *recv_alive.lock().await {
            let message = receiver.recv().await.unwrap();

            match message.msg_type {
                MessageType::Connect => {
                    if message.uuid != uuid {
                        let frame_out = FrameUtils::create(1, true, format!("{} joined", message.text.unwrap()).as_bytes().to_vec(), Some(FrameUtils::random_mask()));
                        write_stream.write(&frame_out).await.unwrap();
                    }
                },
                MessageType::Disconnect => {
                    if message.uuid != uuid {
                        let frame_out = FrameUtils::create(1, true, format!("{} left", message.text.unwrap()).as_bytes().to_vec(), Some(FrameUtils::random_mask()));
                        write_stream.write(&frame_out).await.unwrap();
                    }
                },
                MessageType::Message => {
                    if message.uuid != uuid {
                        let frame_out = FrameUtils::create(1, true, message.text.unwrap().as_bytes().to_vec(), Some(FrameUtils::random_mask()));
                        write_stream.write(&frame_out).await.unwrap();
                    }
                },
                MessageType::OnlySelf => {
                    if message.uuid == uuid {
                        let frame_out = FrameUtils::create(1, true, message.text.unwrap().as_bytes().to_vec(), Some(FrameUtils::random_mask()));
                        write_stream.write(&frame_out).await.unwrap();
                    }
                },
            }
        }
    });

    let mut first = true;
    let mut data = vec![];
    loop {
        let frame = FrameUtils::read(&mut read_stream).await;

        data.extend(frame.data);
        if frame.finished {
            match frame.opcode {
                0 => {
                    continue;
                }
                1 => {
                    let mut message = String::from_utf8(data.clone()).unwrap();

                    if first {
                        if message != "0.0.13" {
                            sender.send(Message::only_self("reload", uuid)).unwrap();
                        }
                        first = false;
                        data.clear();
                        continue;
                    }

                    if message.starts_with("/") {
                        message.remove(0);

                        let (command, args) = match message.split_once(" ") {
                            Some((command, args)) => {
                                (command, Some(args))
                            },
                            None => {
                                (message.as_str(), None)
                            },
                        };

                        match command {
                            "rename" => 'yeet: {
                                if args.is_none() {
                                    sender.send(Message::only_self("SERVER: you need to send a username too noob", uuid)).unwrap();
                                    break 'yeet;
                                }

                                let old = key.clone();

                                key = args.unwrap().to_owned();

                                sender.send(Message::only_self(&format!("SERVER: you are now {:?}", key), uuid)).unwrap();
                                sender.send(Message::message(&format!("SERVER: {:?} became {:?}", old, key), uuid)).unwrap();
                            },
                            _ => {
                                sender.send(Message::only_self("SERVER: what what now?", uuid)).unwrap();
                            }
                        };
                        data.clear();
                        continue;
                    }

                    let message = message.trim();
                    println!("{} [{}]: {}", read_stream.peer_addr().unwrap().to_string(), key, message);
                    sender.send(Message::message(&format!("{}: {}", key, message), uuid)).unwrap();
                    sender.send(Message::only_self(&format!("you: {}", message), uuid)).unwrap();
                }
                2 => {
                    //println!("binary: {:X?}", data.clone());
                    //break;
                }
                8 => {
                    println!("socket gone 🦀");
                    sender.send(Message::disconnect(&key, uuid)).unwrap();
                    break;
                }
                _ => {}
            }
            data.clear();
        }
    }
    *alive.lock().await = false;
}

#[derive(Debug, Clone)]
pub struct Message {
    pub text: Option<String>,
    pub uuid: Uuid,
    pub msg_type: MessageType
}

impl Message {
    fn message(text: &str, uuid: Uuid) -> Self {
        Self {
            text: Some(text.to_owned()),
            uuid,
            msg_type: MessageType::Message
        }
    }
    fn only_self(text: &str, uuid: Uuid) -> Self {
        Self {
            text: Some(text.to_owned()),
            uuid,
            msg_type: MessageType::OnlySelf
        }
    }
    fn disconnect(text: &str, uuid: Uuid) -> Self {
        Self {
            text: Some(text.to_owned()),
            uuid,
            msg_type: MessageType::Disconnect
        }
    }
    fn connect(text: &str, uuid: Uuid) -> Self {
        Self {
            text: Some(text.to_owned()),
            uuid,
            msg_type: MessageType::Connect
        }
    }
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Connect,
    Disconnect,
    Message,
    OnlySelf
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

    async fn read(stream: &mut OwnedReadHalf) -> Self {
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
    /*
    async fn read_buf(stream: Vec<u8>) -> Self {
        let mut stream = Cursor::new(stream);

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
    */
    fn random_mask() -> [u8; 4] {
        let mut rng = rand::thread_rng();

        return [rng.gen(), rng.gen(), rng.gen(), rng.gen()];
    }
}