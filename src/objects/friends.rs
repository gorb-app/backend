use chrono::{DateTime, Utc};
use diesel::{Queryable, Selectable};
use serde::Serialize;
use uuid::Uuid;

use crate::schema::{friend_requests, friends};

#[derive(Serialize, Queryable, Selectable, Clone)]
#[diesel(table_name = friends)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Friend {
    pub uuid1: Uuid,
    pub uuid2: Uuid,
    pub accepted_at: DateTime<Utc>,
}

#[derive(Serialize, Queryable, Selectable, Clone)]
#[diesel(table_name = friend_requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FriendRequest {
    pub sender: Uuid,
    pub receiver: Uuid,
    pub requested_at: DateTime<Utc>,
}
