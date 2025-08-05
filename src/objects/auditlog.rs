use crate::{
    Conn,
    error::Error,
    objects::{Pagination, PaginationRequest, load_or_empty},
    schema::audit_logs,
};
use diesel::{
    ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable, SelectableHelper, insert_into,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub async fn count(conn: &mut Conn, guild_uuid: Uuid) -> Result<i64, Error> {
        use audit_logs::dsl;
        let count: i64 = dsl::audit_logs
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .count()
            .get_result(conn)
            .await?;

        Ok(count)
    }
    pub async fn fetch_page(
        conn: &mut Conn,
        guild_uuid: Uuid,
        pagination: PaginationRequest,
    ) -> Result<Pagination<AuditLog>, Error> {
        // TODO: Maybe add cache, but I do not know how
        let per_page = pagination.per_page.unwrap_or(20);
        let offset = (pagination.page - 1) * per_page;

        if !(10..=100).contains(&per_page) {
            return Err(Error::BadRequest(
                "Invalid amount per page requested".to_string(),
            ));
        }

        use audit_logs::dsl;
        let logs: Vec<AuditLog> = load_or_empty(
            dsl::audit_logs
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .limit(per_page.into())
                .offset(offset as i64)
                .select(AuditLog::as_select())
                .load(conn)
                .await,
        )?;

        let pages = (AuditLog::count(conn, guild_uuid).await? as f32 / per_page as f32).ceil();

        let paginated_logs = Pagination::<AuditLog> {
            objects: logs.clone(),
            amount: logs.len() as i32,
            pages: pages as i32,
            page: pagination.page,
        };

        Ok(paginated_logs)
    }

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
    ) -> Result<(), Error> {
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
            changed_to,
        };

        insert_into(audit_logs::table)
            .values(audit_log.clone())
            .execute(conn)
            .await?;

        Ok(())
    }
}
