use diesel::{ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use uuid::Uuid;

use crate::{Conn, error::Error, schema::invites};

/// Server invite struct
#[derive(Clone, Serialize, Queryable, Selectable, Insertable)]
pub struct Invite {
    /// case-sensitive alphanumeric string with a fixed length of 8 characters, can be up to 32 characters for custom invites
    pub id: String,
    /// User that created the invite
    pub user_uuid: Uuid,
    /// UUID of the guild that the invite belongs to
    pub guild_uuid: Uuid,
}

impl Invite {
    pub async fn fetch_one(conn: &mut Conn, invite_id: String) -> Result<Self, Error> {
        use invites::dsl;
        let invite: Invite = dsl::invites
            .filter(dsl::id.eq(invite_id))
            .select(Invite::as_select())
            .get_result(conn)
            .await?;

        Ok(invite)
    }
}
