use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use uuid::Uuid;
use crate::user::UserId;

pub type SessionId = String;

pub static SESSIONS: LazyLock<Mutex<HashMap<SessionId, Session>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
const SESSION_TIMEOUT_SECONDS: u64 = 60; // TODO should be 5 minutes, maybe?

pub struct Session {
    session_id: SessionId,
    user_id: UserId,
    last_request_at: Instant,
}

pub fn close_session(session_id: &SessionId) {
    //TODO anything else required? This is effectively logging out the user
    SESSIONS.lock().unwrap().remove(session_id);
}

pub fn create_session(user_id: UserId) -> SessionId {
    // Is there any other session open for the user? If so, close it.
    for session in SESSIONS.lock().unwrap().values_mut() {
        if session.user_id == user_id {
            close_session(&session.session_id)
        }
    }

    let session_id: SessionId = Uuid::new_v4().to_string();
    let result = session_id.clone();
    let session = Session{session_id, user_id, last_request_at: Instant::now()};
    SESSIONS.lock().unwrap().insert(session.session_id.clone(), session);
    result
}

pub fn prune_sessions() {
    let now = Instant::now();
    for session in SESSIONS.lock().unwrap().values_mut() {
        if session.last_request_at.elapsed().as_secs() > SESSION_TIMEOUT_SECONDS {
            // TODO post a timeout message to the user (not to the session)
            SESSIONS.lock().unwrap().remove(&session.session_id);
        }
    }
}