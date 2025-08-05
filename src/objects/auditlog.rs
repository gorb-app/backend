use uuid::Uuid;
use diesel::{insert_into, Insertable, Queryable, Selectable, SelectableHelper};
use serde::{Deserialize, Serialize};
use crate::{error::Error, schema::audit_logs, Conn};
use diesel_async::RunQueryDsl;


#[derive(Insertable, Selectable, Queryable, Serialize, Deserialize, Clone)]
#[diesel(table_name = audit_logs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AuditLog {
    
	pub uuid: Uuid,
	pub guild_uuid: Uuid,
	pub action_id: i16,
	pub by_uuid: Uuid,
	pub channel_uuid: Option<Uuid>,
	pub user_uuid: Option<Uuid>,
	pub message_uuid: Option<Uuid>,
	pub role_uuid: Option<Uuid>,
	pub audit_message: Option<String>,
	pub changed_from: Option<String>,
	pub changed_to: Option<String>,
}


impl AuditLog {
    #[allow(clippy::new_ret_no_self)]
    pub async fn new(
        conn: &mut Conn,
        guild_uuid: Uuid,
        action_id: i16,
        by_uuid: Uuid,
        channel_uuid: Option<Uuid>,
        user_uuid: Option<Uuid>,
        message_uuid: Option<Uuid>,
        role_uuid: Option<Uuid>,
        audit_message: Option<String>,
        changed_from: Option<String>,
        changed_to: Option<String>,
    ) ->Result<(), Error> {
        let audit_log = AuditLog {
            uuid: Uuid::now_v7(),
            guild_uuid,
            action_id,
            by_uuid,
            channel_uuid,
            user_uuid,
            message_uuid,
            role_uuid,
            audit_message,
            changed_from,
            changed_to
        };

        insert_into(audit_logs::table)
            .values(audit_log.clone())
            .execute(conn)
            .await?;

        Ok(())
    }
}
