use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use uuid::Uuid;
use crate::user;
use crate::user::UserId;

pub type SessionId = String;

static SESSIONS: LazyLock<Mutex<HashMap<SessionId, Session>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
const SESSION_TIMEOUT_SECONDS: u64 = 60; // TODO should be 5 minutes, maybe?

pub struct Session {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub last_request_at: Instant,
    is_closed: bool,
}

pub fn close_session(session_id: &SessionId) {
    for session in SESSIONS.lock().unwrap().values_mut() {
        if session.session_id == *session_id {
            session.is_closed = true;
            break;
        }
    }
}

pub fn create_session(user_id: UserId) -> SessionId {
    // Is there any other session open for the user? If so, close it.
    for session in SESSIONS.lock().unwrap().values_mut() {
        if session.user_id == user_id && !session.is_closed {
            session.is_closed = true;
        }
    }

    let session_id: SessionId = Uuid::new_v4().to_string();
    let result = session_id.clone();
    let session = Session{session_id, user_id, last_request_at: Instant::now(), is_closed: false};
    SESSIONS.lock().unwrap().insert(session.session_id.clone(), session);
    result
}

pub fn get_session(session_id: &SessionId) -> Option<Session> {
    for session in SESSIONS.lock().unwrap().values() {
        if session.session_id == *session_id && !session.is_closed {
            return Some(session.clone())
        }
    }
    None
}

// Iterates over sessions, removing those which have been closed
pub fn prune_sessions() {
    SESSIONS.lock().unwrap().retain(|_, session| !session.is_closed);
}

/// Updates the timestamp for a session provided it exists, and is not closed
pub fn touch_session(session_id: &SessionId) {
    for session in SESSIONS.lock().unwrap().values_mut() {
        if session.session_id == *session_id && !session.is_closed {
            session.last_request_at = Instant::now();
            break;
        }
    }
}

impl Session {
    pub fn clone(&self) -> Session {
        Session{session_id: self.session_id.clone(), user_id: self.user_id.clone(), last_request_at: Instant::now(), is_closed: false}
    }

    pub fn is_admin(&self) -> bool {
        self.user_id == user::ADMIN_USER_ID
    }
}
