use std::collections::{HashMap};
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use std::{io, thread};
use std::sync::{LazyLock, Mutex};
use std::thread::JoinHandle;
use base64::Engine;
use base64::engine::general_purpose;
use crate::http_response::*;
use crate::session::{SessionId, SESSIONS};
use crate::universe;
use crate::user::UserId;

static IS_ACTIVE: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));
static JOIN_HANDLE: LazyLock<Mutex<Option<JoinHandle<()>>>> = LazyLock::new(|| Mutex::new(None));
static TERMINATE_FLAG: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));
const MILLISECONDS_BETWEEN_NONBLOCKING_CALLS: u64 = 500;

// Public functions --------------------------------------------------------------------------------

pub fn is_active() {
    *IS_ACTIVE.lock().unwrap() = true;
}

pub fn start() {
    let mut join_lock = JOIN_HANDLE.lock().unwrap();
    assert!(join_lock.is_none());
    *join_lock = Some(thread::spawn(|| worker()));
}

pub fn stop() {
    let mut term_lock = TERMINATE_FLAG.lock().unwrap();
    *term_lock = true;
    drop(term_lock);

    let mut join_lock = JOIN_HANDLE.lock().unwrap();
    if join_lock.is_some() {
        _ = join_lock.take().unwrap().join();
    }
    drop(join_lock);
}

// Private functions -------------------------------------------------------------------------------

fn decode_headers(text_lines: &Vec<String>) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for x in 1..text_lines.len() {
        let text = &text_lines[x];
        let parts = text.split(":").collect::<Vec<&str>>();
        headers.insert(parts[0].to_string().trim().to_ascii_lowercase(), parts[1].to_string().trim().to_string());
    }
    headers
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

    let mut response = HttpResponse::new(HTTP_UNAUTHORIZED, "Authorization is required");
    let leading_parts = http_request.get(0).unwrap().split(" ").collect::<Vec<&str>>();
    if leading_parts.len() < 2 {
        response = HttpResponse::new(HTTP_BAD_REQUEST, "Badly-formatted method/URL\r\n");
    } else {
        let method = leading_parts.get(0).unwrap().to_string().to_uppercase();
        let url = leading_parts.get(1).unwrap().to_string().to_lowercase();

        let headers = decode_headers(&http_request);
        if headers.contains_key("X-SESSION-TAG") {
            // Look for a session tag (this will be the session-key for a session... hopefully)
            let session_id = headers.get("X-SESSION-TAG").unwrap();
            let mut session_lock = SESSIONS.lock().unwrap();
            if session_lock.get_mut(session_id).is_none() {
                let response = HttpResponse::new(HTTP_UNAUTHORIZED, "Invalid Session Key\r\n");
                stream.write_all(response.to_string().as_bytes()).unwrap();
                return;
            }
            drop(session_lock);
            response = quit_handler(session_id);//TODO this is a place holder
        } else if headers.contains_key("authorization") {
            // Look for basic authorization (i.e., a base64-encoded username and password)
            let auth_value = headers.get("authorization").unwrap();
            let auth_response = validate_authorization(auth_value);
            match auth_response {
                Ok(user_id) => {
                    // TODO look for authentication, and handle that... limited to creating a new session
                    //  println!("User ID is {}", user_id);
                    //  let response = format!("HTTP/1.1 200 OK\r\n\r\nuser_id is {}\r\n", user_id);
                    //  stream.write_all(response.as_bytes()).unwrap();
                    response = HttpResponse::new(HTTP_OK, "Auth looks good.\r\n");
                }
                Err(auth_http_response) => {
                    response = auth_http_response;
                }
            };
        }
    }

    stream.write_all(response.to_string().as_bytes()).unwrap();
}

fn quit_handler(session_id: &SessionId) -> HttpResponse {
    HttpResponse::new(HTTP_OK, "") // TODO placeholder
}

fn route_request() -> HttpResponse {
    // catch-all
    HttpResponse::new(HTTP_BAD_REQUEST, "Invalid Request")
}

fn validate_authorization(auth_value: &String) -> Result<UserId, HttpResponse> {
    let value_split = auth_value.split(" ").collect::<Vec<&str>>();
    if value_split.get(0).unwrap().to_ascii_lowercase() != "basic" {
        return Err(HttpResponse::new(HTTP_BAD_REQUEST, "Only basic authentication is supported"));
    }

    let encoded = value_split.get(1).unwrap();
    let decoded = String::from_utf8(general_purpose::STANDARD.decode(encoded).unwrap()).unwrap();
    let decoded = decoded.trim().split(":").collect::<Vec<&str>>();

    if decoded.len() != 2 {
        return Err(HttpResponse::new(HTTP_BAD_REQUEST, "Malformed Authentication"));
    }

    let user_name = decoded.get(0).unwrap().to_string();
    let password = decoded.get(1).unwrap().to_string();
    let user_id = universe::validate_credentials(user_name, password);
    if user_id.is_none() {
        return Err(HttpResponse::new(HTTP_UNAUTHORIZED, "User Not Found or Incorrect Password"));
    }

    Ok(user_id.unwrap())
}

// thread which listens for connections
fn worker() {
    let mut active_lock = IS_ACTIVE.lock().unwrap();
    *active_lock = true;
    drop(active_lock);

    let listener = TcpListener::bind("127.0.0.1:2000").unwrap();
    _ = listener.set_nonblocking(true);
    loop {
        let term_lock = TERMINATE_FLAG.lock().unwrap();
        if *term_lock {
            break;
        }
        drop(term_lock);

        match listener.incoming().next() {
            Some(Ok(stream)) => {
                handle_connection(stream);//TODO spawn this
            }
            Some(Err(ref e)) if e.kind() == io::ErrorKind::WouldBlock => {
                // No connection available, wait for a short time
                thread::sleep(Duration::from_millis(MILLISECONDS_BETWEEN_NONBLOCKING_CALLS));
                // TODO should we do session pruning here?
            }
            Some(Err(e)) => {
                // Handle other errors
                eprintln!("Error accepting connection: {}", e);
                break; // Or handle the error as appropriate
            }
            None => {
                // The iterator is exhausted. Should not happen with a non-blocking listener.
                eprintln!("Listener iterator exhausted. This should not happen with non-blocking I/O.");
                break;
            }
        }
    }

    // TODO terminate outstanding sessions... what does this mean?
}
