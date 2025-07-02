use std::collections::HashMap;

pub struct HttpResponse {
    code: u16,
    headers: HashMap<String, String>,
    data: String,
}

pub const HTTP_OK: u16 = 200;
pub const HTTP_CREATED: u16 = 201;
pub const HTTP_ACCEPTED: u16 = 202;
pub const HTTP_NO_CONTENT: u16 = 404;
pub const HTTP_BAD_REQUEST: u16 = 400;
pub const HTTP_UNAUTHORIZED: u16 = 401;
pub const HTTP_FORBIDDEN: u16 = 403;
pub const HTTP_NOT_FOUND: u16 = 404;
pub const HTTP_METHOD_NOT_ALLOWED: u16 = 405;
pub const HTTP_INTERNAL_SERVER_ERROR: u16 = 500;
pub const HTTP_NOT_IMPLEMENTED: u16 = 501;

impl HttpResponse {

    pub fn new(code: u16, data: &str) -> HttpResponse {
        HttpResponse{code, headers: HashMap::new(), data: data.into()}
    }

    pub fn new_with_headers(code: u16, headers: HashMap<String, String>, data: &str) -> HttpResponse {
        HttpResponse{code, headers, data: data.into()}
    }
    
    pub fn append_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.into(), value.into());
    }

    pub fn is_successful(&self) -> bool {
        self.code >= 200 && self.code < 299
    }

    pub fn to_string(&self) -> String {
        let detail = match self.code {
            HTTP_OK => "OK",
            HTTP_CREATED => "Created",
            HTTP_ACCEPTED => "Accepted",
            HTTP_NO_CONTENT => "No Content",
            HTTP_BAD_REQUEST => "Bad Request",
            HTTP_UNAUTHORIZED => "Unauthorized",
            HTTP_FORBIDDEN => "Forbidden",
            HTTP_NOT_FOUND => "Not Found",
            HTTP_METHOD_NOT_ALLOWED => "Method Not Allowed",
            HTTP_INTERNAL_SERVER_ERROR => "Internal Server Error",
            HTTP_NOT_IMPLEMENTED => "Not Implemented",
            _ => "Internal Server Error",
        };
        
        let mut s = format!("HTTP/1.1 {} {}\r\n", self.code, detail);
        for (key, value) in self.headers.clone() {
            s.push_str(&format!("{}: {}\r\n", key, value));
        }
        s.push_str("\r\n");
        s.push_str(&format!("{}\r\n", &self.data));
        s
    }
}
