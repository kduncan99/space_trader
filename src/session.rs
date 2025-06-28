use std::time::Instant;
use crate::user::UserId;

struct Session {
    user_id: UserId,
    last_request_at: Instant,
}

