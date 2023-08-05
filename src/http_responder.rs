use std::collections::HashMap;
use std::io::Write;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

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