use std::{path::PathBuf, collections::HashMap, time::Duration};
use http_responder::HttpResponse;
use tokio::{net::{TcpListener, TcpStream}, io::{BufReader, AsyncBufReadExt, AsyncReadExt}, fs::File};
use tokio_tungstenite::{WebSocketStream, tungstenite::{Message, Error}};
use futures_util::{SinkExt, StreamExt};

mod http_responder;

#[tokio::main]
async fn main() {
    let socket_server = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tokio::spawn(async move {
        let web_server = TcpListener::bind("0.0.0.0:80").await.unwrap();
        loop {
            let (mut stream, _socket_addr) = web_server.accept().await.unwrap();
            tokio::spawn(async move {
                //handle_connection(&mut stream).await;
            });
        }
    });

    loop {
        let (stream, _socket_addr) = socket_server.accept().await.unwrap();

        tokio::spawn(async move {
            let mut ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
            handle_socket(&mut ws_stream).await.unwrap();
        });
    }
}

/*

- i lost myself somewhere when writing this
this is very clean simple http resolver, but that is not 
what this project was about so im scapping to to return to the former test

async fn handle_connection(mut stream: &mut TcpStream) {
    let bufreader = BufReader::new(&mut stream);
    let mut lines = bufreader.lines();
    
    let next_line = lines.next_line().await.unwrap();
    if next_line.is_none() {
        return;
    }

    let entire = next_line.unwrap();
    let (method, right) = entire.split_once(" ").unwrap();
    let (path, _http_type) = right.rsplit_once(" ").unwrap();

    let mut request: HashMap<String, String> = HashMap::new();
    while let Some(line) = lines.next_line().await.unwrap() {
        if line.len() == 0 { break; }

        let lowercase = line.to_lowercase();
        let (key, value) = lowercase.split_once(": ").unwrap();
        request.insert(key.to_owned(), value.to_owned());
    }
    //println!("{:#?}", request);

    if method != "GET" {
        HttpResponse::new(405, None, Some(b"I only speak GET".to_vec())).write(&mut stream).await;
        return;
    }

    let mut data = vec![];
    let mut path = PathBuf::from(format!("html{}", path));
    if path.is_dir() {
        let index_path = path.join("index.html");
        if !index_path.exists() {
            HttpResponse::new(418, None, Some(b"i'll fix this some other time, no index file exists".to_vec())).write(stream).await;
            return;
        }
        path = index_path;
    } else {
        if !path.exists() {
            HttpResponse::new(404, None, Some(b"requested file/folder does not exist".to_vec())).write(stream).await;
            return;
        }
    }
    File::open(&path).await.unwrap().read_to_end(&mut data).await.unwrap();

    let guess = mime_guess::from_path(path);

    let mut headers: HashMap<String, String> = HashMap::new();
    headers.insert("Content-Type".to_owned(), guess.first().unwrap().to_string());
    headers.insert("Content-Length".to_owned(), data.len().to_string());

    HttpResponse::new(200, Some(headers), Some(data)).write(stream).await;
}
*/

async fn handle_socket(ws_stream: &mut WebSocketStream<tokio::net::TcpStream>) -> Result<(), Error>{
    println!("new websocket");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut interval = tokio::time::interval(Duration::from_millis(5000));
    
    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        if msg.is_close() {
                            break;
                        }

                        println!("{}", msg);
                    }
                    None => break,
                }
            }
            _ = interval.tick() => {
                ws_sender.send(Message::Binary(b"tick".to_vec())).await.unwrap();
            }
        }
    }

    Ok(())
}