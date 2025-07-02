use std::collections::{HashMap};
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use std::{io, thread};
use std::sync::{LazyLock, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;
use base64::Engine;
use base64::engine::general_purpose;
use crate::http_response::*;
use crate::session::SessionId;
use crate::{session, user};

pub static IS_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static JOIN_HANDLE: LazyLock<Mutex<Option<JoinHandle<()>>>> = LazyLock::new(|| Mutex::new(None));
pub static TERMINATE_FLAG: AtomicBool = AtomicBool::new(false);
const MILLISECONDS_BETWEEN_NONBLOCKING_CALLS: u64 = 500;

// Public functions --------------------------------------------------------------------------------

pub fn is_active() {
    IS_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn start() {
    let mut join_lock = JOIN_HANDLE.lock().unwrap();
    assert!(join_lock.is_none());
    *join_lock = Some(thread::spawn(|| worker()));
}

pub fn stop() {
    TERMINATE_FLAG.store(false, std::sync::atomic::Ordering::SeqCst);

    let mut join_lock = JOIN_HANDLE.lock().unwrap();
    if join_lock.is_some() {
        _ = join_lock.take().unwrap().join();
    }
    drop(join_lock);
}

// Private functions -------------------------------------------------------------------------------

// Decodes the lines representing the headers into a map
fn decode_headers(text_lines: &Vec<String>) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for x in 1..text_lines.len() {
        let text = &text_lines[x];
        let parts = text.split(":").collect::<Vec<&str>>();
        headers.insert(parts[0].to_string().trim().to_ascii_lowercase(), parts[1].to_string().trim().to_string());
    }
    headers
}

// Should be spun off as a separate thread.
// Handles a new connection represented by the given stream value.
// We do authentication here...
// Any connection can be authenticated in either of the following ways:
//  basic authorization in the header:
//      username and password are validated, and if validation passes, a new session is created which
//      contains a session id and a user id (along with some housekeeping stuff).
//      If a session already exists for the user, that session is marked closed in favor of the new one.
//      The session-id will be placed in the response header for this connection.
//  session-id in the header:
//      The session-id is checked and, if it exists, it counts as validation.
//      The request will be handled, and the session-id will be returned in the response header.
fn handle_connection(stream: &TcpStream) -> HttpResponse {
    let buf_reader = BufReader::new(stream);
    let mut http_request: Vec<String> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("From {}:{}", stream.peer_addr().unwrap(), http_request.get(0).unwrap());

    let headers = decode_headers(&http_request);
    let session_id;

    let auth_value = headers.get("authorization");
    if auth_value.is_some() {
        session_id =
            match validate_authorization(auth_value.unwrap()) {
                Ok(session_id) => session_id,
                Err(http_response) => {
                    return http_response;
                }
            };
    } else {
        let sid = headers.get("x-session-id");
        if sid.is_none() {
            return HttpResponse::new(HTTP_UNAUTHORIZED, "Unauthorized");
        }
        session_id = sid.unwrap().clone();
    }

    // Grab the method and the url - we don't care about the http version.
    // The URL is deconstructed into a vector of strings according to the path components.
    // We do some shenanigans on the URL, primarily caused by Rust not giving us a nice simple
    // substring function.
    let leading_parts = http_request.get(0).unwrap().split(" ").collect::<Vec<&str>>();
    if leading_parts.len() < 2 {
        return HttpResponse::new(HTTP_BAD_REQUEST, "Badly-formatted method/URL");
    }

    let method = leading_parts.get(0).unwrap().to_string().to_uppercase();
    let url = leading_parts.get(1).unwrap().to_string().to_lowercase();
    let clean_url = if url.starts_with("/") { url.strip_prefix("/").unwrap() } else { url.as_str() };

    let mut response = match clean_url {
        "admin/quit" => handle_admin_quit(&session_id, &method),
        "poll" => handle_poll(&session_id, &method),
        "" => handle_no_operation(&session_id, &method),
        _ => HttpResponse::new(HTTP_NOT_FOUND, "Path Not Recognized"),
    };
    
    if response.is_successful() {
        response.append_header("x-session-id", session_id.as_str());
    }
    response
}

fn handle_admin_quit(session_id: &SessionId, method: &String) -> HttpResponse {
    if method != "POST" {
        return HttpResponse::new(HTTP_BAD_REQUEST, "Method Not Allowed");
    }
    
    let session = session::get_session(session_id);
    if session.is_none() {
        return HttpResponse::new(HTTP_INTERNAL_SERVER_ERROR, "");
    }

    if session.unwrap().user_id != user::ADMIN_USER_ID {
        return HttpResponse::new(HTTP_FORBIDDEN, "You are not a real overlord");
    }

    // TODO send messages to all and sundry
    TERMINATE_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);
    session::touch_session(session_id);
    HttpResponse::new(HTTP_OK, "Sent termination request to server")
}

fn handle_no_operation(session_id: &SessionId, method: &String) -> HttpResponse {
    if method != "GET" {
        return HttpResponse::new(HTTP_BAD_REQUEST, "Method Not Allowed");
    }

    session::touch_session(session_id);
    HttpResponse::new(HTTP_OK, "")
}

fn handle_poll(session_id: &SessionId, method: &String) -> HttpResponse {
    if method != "GET" {
        return HttpResponse::new(HTTP_BAD_REQUEST, "Method Not Allowed");
    }

    // TODO go grab pending messages for this user

    session::touch_session(session_id);
    HttpResponse::new(HTTP_OK, "")
}

// Checks the authorization value against our users.
// If successful, we create a new session for this client and return the SessionId.
fn validate_authorization(auth_value: &String) -> Result<SessionId, HttpResponse> {
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
    let user_id = user::validate_credentials(user_name, password);
    if user_id.is_none() {
        return Err(HttpResponse::new(HTTP_UNAUTHORIZED, "User Not Found or Incorrect Password"));
    }

    let session_id = session::create_session(user_id.unwrap());
    Ok(session_id)
}

// thread which listens for connections
fn worker() {
    IS_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);

    let listener = TcpListener::bind("127.0.0.1:2000").unwrap();
    _ = listener.set_nonblocking(true);
    loop {
        if TERMINATE_FLAG.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        match listener.incoming().next() {
            Some(Ok(mut stream)) => {
                let response = handle_connection(&stream);//TODO spawn this
                stream.write_all(response.to_string().as_bytes()).unwrap();
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

    IS_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
    // TODO terminate outstanding sessions... what does this mean?
}
