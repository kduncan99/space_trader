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
use lazy_static::lazy_static;
use crate::http_response::*;
use crate::session::{Session, SessionId};
use crate::{session, user};

pub static IS_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static HANDLER_HANDLES: LazyLock<Mutex<Vec<JoinHandle<()>>>> = LazyLock::new(|| Mutex::new(vec![]));
pub static JOIN_HANDLE: LazyLock<Mutex<Option<JoinHandle<()>>>> = LazyLock::new(|| Mutex::new(None));
pub static TERMINATE_FLAG: AtomicBool = AtomicBool::new(false);
const MILLISECONDS_BETWEEN_NONBLOCKING_CALLS: u64 = 100;
const HANDLER_PRUNE_RATIO: i32 = 100;

struct HandlerEntry {
    method: &'static str,
    path: &'static str,
    is_restricted: bool,
    func: fn (&Session) -> HttpResponse,
}

lazy_static! {
    static ref HANDLER_LOOKUP_TABLE: Vec<HandlerEntry> = {
        let mut table = Vec::new();
        table.push(HandlerEntry {method: "POST", path: "/admin/quit", is_restricted: true, func: handle_admin_quit});
        table.push(HandlerEntry {method: "GET", path: "/poll", is_restricted: false, func: handle_poll});
        table.push(HandlerEntry {method: "GET", path: "/", is_restricted: false, func: handle_no_operation});
        table
    };
}

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
    TERMINATE_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);

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
    let http_request: Vec<String> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("From {}:{}", stream.peer_addr().unwrap(), http_request.get(0).unwrap());

    let headers = decode_headers(&http_request);
    let session_id: SessionId = {
        let auth_value = headers.get("authorization");
        if auth_value.is_some() {
            match validate_authorization(auth_value.unwrap()) {
                Ok(session_id) => session_id,
                Err(http_response) => {
                    return http_response;
                }
            }
        } else {
            let sid = headers.get("x-session-id");
            if sid.is_none() {
                return HttpResponse::new(HTTP_UNAUTHORIZED, "Unauthorized");
            }
            sid.unwrap().clone()
        }
    };
    let session = session::get_session(&session_id);
    if session.is_none() {
        return HttpResponse::new(HTTP_INTERNAL_SERVER_ERROR, "Session disappeared");
    }
    let session = session.unwrap();
    session::touch_session(&session_id);

    // Grab the method and the url - we don't care about the http version.
    let leading_parts = http_request.get(0).unwrap().split(" ").collect::<Vec<&str>>();
    if leading_parts.len() < 2 {
        return HttpResponse::new(HTTP_BAD_REQUEST, "Badly-formatted method/URL");
    }

    let method = leading_parts.get(0).unwrap().to_string().to_uppercase();
    let mut url = leading_parts.get(1).unwrap().to_string().to_lowercase();
    if url.is_empty() {
        url = "/".to_string();
    }

    let mut found_path = false;
    for entry in HANDLER_LOOKUP_TABLE.iter() {
        if url == entry.path {
            found_path = true;
            if method == entry.method {
                return if entry.is_restricted && !session.is_admin() {
                    HttpResponse::new(HTTP_FORBIDDEN, "You are neither cosmic, nor an overlord.")
                } else {
                    let mut response = (entry.func)(&session);
                    if response.is_successful() {
                        response.append_header("x-session-id", session_id.as_str());
                    }
                    response
                }
            }
        }
    }

    if found_path {
        // we found a match on the path, but not the method
        HttpResponse::new(HTTP_METHOD_NOT_ALLOWED, "{method} not allowed on path {url}")
    } else {
        // we did not find the path
        HttpResponse::new(HTTP_NOT_FOUND, "path {url} not found")
    }
}

fn handle_admin_quit(session: &Session) -> HttpResponse {
    // TODO send messages to all and sundry
    TERMINATE_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);
    HttpResponse::new(HTTP_OK, "Sent termination request to server")
}

fn handle_no_operation(session: &Session) -> HttpResponse {
    HttpResponse::new(HTTP_OK, "")
}

fn handle_poll(session: &Session) -> HttpResponse {
    // TODO go grab pending messages for this user
    let mut data: String = "".to_string();
    data.push_str("Place-holder message\r\n");
    HttpResponse::new(HTTP_OK, data.as_str())
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
    let mut prune_counter = HANDLER_PRUNE_RATIO;
    loop {
        if TERMINATE_FLAG.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        // Every n-th time through the loop, we go prune the join handles.
        // They are preserved solely for the purposes of waiting for them to complete
        // when the server shuts down... but we don't want them to hang around the whole
        // time we are server-ing.
        prune_counter -= 1;
        if prune_counter == 0 {
            HANDLER_HANDLES.lock().unwrap().retain(|handle| handle.is_finished());
            prune_counter = HANDLER_PRUNE_RATIO;
        }

        match listener.incoming().next() {
            Some(Ok(mut stream)) => {
                let handle = thread::spawn(move || -> () {
                    let response = handle_connection(&stream);
                    stream.write_all(response.to_string().as_bytes()).unwrap();
                });
                HANDLER_HANDLES.lock().unwrap().push(handle);
            }
            Some(Err(ref e)) if e.kind() == io::ErrorKind::WouldBlock => {
                // No connection available, wait for a short time
                thread::sleep(Duration::from_millis(MILLISECONDS_BETWEEN_NONBLOCKING_CALLS));
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

    // Wait for workers to terminate - this normally is very quick;
    // we just want to make sure none of them accidentally get stopped in the middle of something.
    while HANDLER_HANDLES.lock().unwrap().len() > 0 {
        let handler = HANDLER_HANDLES.lock().unwrap().pop();
        if handler.is_some() {
            _ = handler.unwrap().join();
        }
    }

    // TODO terminate outstanding sessions... what does this mean?
    println!("Server stopping...");
    IS_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
}
