use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use tokio::fs::File;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::net::TcpStream;

pub async fn handle_http(mut stream: &mut TcpStream, inital: String, _headers: HashMap<String, String>) {
    let (method, right) = inital.split_once(" ").unwrap();
    let (path, _http_type) = right.rsplit_once(" ").unwrap();

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


pub struct HttpResponse {
    status_code: u16,
    headers: Option<HashMap<String, String>>,
    data: Option<Vec<u8>>,
}

impl HttpResponse {
    pub fn new(status_code: u16, headers: Option<HashMap<String, String>>, data: Option<Vec<u8>>) -> HttpResponse {
        return HttpResponse { status_code, headers, data };
    }

    pub async fn write(&mut self, stream: &mut TcpStream) {
        let mut buffer: Vec<u8> = vec![];

        let mut data = vec![];
        if self.data.is_some() {
            data.extend(self.data.clone().unwrap());
        }

        writeln!(&mut buffer, "HTTP/1.1 {}", self.status_code).unwrap();
        if self.headers.is_some() {
            for (key, value) in self.headers.clone().unwrap() {
                writeln!(&mut buffer, "{}: {}", key, value).unwrap();
            }
        } else {
            if data.len() > 0 {
                writeln!(&mut buffer, "{}: {}", "Content-Length", data.len()).unwrap();
            }
        }
        buffer.extend(b"\n");
        buffer.extend(data);
        
        stream.write(&mut buffer).await.unwrap();
    }
}