use std::collections::HashMap;
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::thread;
use base64::Engine;
use base64::engine::general_purpose;
use rusqlite::{Connection, Result};
use crate::universe::UNIVERSE;
use crate::user::UserId;

pub struct Server {
    database: Connection,
}

struct HttpResponse {
    code: u16,
    detail: String,
    data: String,
}

impl Server {
    pub fn new(database: Connection) -> Server {
        Server { database: database }
    }

    pub fn process(&self) {
        let listener = TcpListener::bind("127.0.0.1:2000").unwrap();

        for stream in listener.incoming() {
            let stream = stream.unwrap();
            thread::spawn(|| {
                Self::handle_connection(stream)
            });
        }
    }

    fn handle_connection(mut stream: TcpStream) {
        let mut http_request: Vec<String> = vec![];
        {
            let buf_reader = BufReader::new(&stream);
            http_request = buf_reader
                .lines()
                .map(|result| result.unwrap())
                .take_while(|line| !line.is_empty())
                .collect();
        }

        let leading_parts = http_request.get(0).unwrap().split(" ").collect::<Vec<&str>>();
        let method = leading_parts.get(0);
        let url = leading_parts.get(1);

        let headers = Self::decode_headers(&http_request);
        let auth_response = Self::validate_authorization(headers);
        match auth_response {
            Ok(user_id) => {
                println!("User ID is {}", user_id);
                let response = format!("HTTP/1.1 200 OK\r\n\r\nuser_id is {}\r\n", user_id);
                stream.write_all(response.as_bytes()).unwrap();
            }
            Err(error) => {
                let response = format!("HTTP/1.1 {} {}\r\n\r\n{}\r\n", error.code, error.detail, error.data);
                stream.write_all(response.as_bytes()).unwrap();
                return;
            }
        };
    }

    fn decode_headers(text_lines: &Vec<String>) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        for x in 1..text_lines.len() {
            let text = &text_lines[x];
            let parts = text.split(":").collect::<Vec<&str>>();
            headers.insert(parts[0].to_string().trim().to_ascii_lowercase(), parts[1].to_string().trim().to_string());
        }
        headers
    }

    fn validate_authorization(headers: HashMap<String, String>) -> Result<UserId, HttpResponse> {
        if !headers.contains_key("authorization") {
            return Err(HttpResponse{code: 401, detail: "Unauthorized".to_string(), data: "Authorization header missing".to_string()});
        }

        let auth_value = headers.get("authorization").unwrap();
        let value_split = auth_value.split(" ").collect::<Vec<&str>>();
        if value_split.get(0).unwrap().to_ascii_lowercase() != "basic" {
            return Err(HttpResponse{code: 400, detail: "Bad Request".to_string(), data: "Only basic authentication is supported".to_string()});
        }

        let encoded = value_split.get(1).unwrap();
        let decoded = String::from_utf8(general_purpose::STANDARD.decode(encoded).unwrap()).unwrap();
        let decoded = decoded.trim().split(":").collect::<Vec<&str>>();

        if decoded.len() != 2 {
            return Err(HttpResponse{code: 400, detail: "Bad Request".to_string(), data: "Malformed Authentication".to_string()});
        }

        let user_name = decoded.get(0).unwrap().to_string();
        let password = decoded.get(1).unwrap().to_string();
        let user_id = UNIVERSE.lock().unwrap().validate_credentials(user_name, password);
        if user_id.is_none() {
            return Err(HttpResponse{code: 401, detail: "Unauthorized".to_string(), data: "User Not Found or Incorrect Password".to_string()});
        }

        Ok(user_id.unwrap())
    }
}
