use diesel::{ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    Conn,
    error::Error,
    schema::{channels, guilds, messages},
};

use super::Member;

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MessageBuilder {
    pub uuid: Uuid,
    pub channel_uuid: Uuid,
    pub user_uuid: Uuid,
    pub message: String,
    pub reply_to: Option<Uuid>,
}

impl MessageBuilder {
    pub async fn build(
        &self,
        conn: &mut Conn,
        cache_pool: &redis::Client,
    ) -> Result<Message, Error> {
        use channels::dsl;

        let guild_uuid = dsl::channels
            .filter(dsl::uuid.eq(self.channel_uuid))
            .inner_join(guilds::table)
            .select(guilds::uuid)
            .get_result(conn)
            .await?;

        let member = Member::fetch_one(conn, cache_pool, None, self.user_uuid, guild_uuid).await?;

        Ok(Message {
            uuid: self.uuid,
            channel_uuid: self.channel_uuid,
            user_uuid: self.user_uuid,
            message: self.message.clone(),
            reply_to: self.reply_to,
            member,
        })
    }
}

#[derive(Clone, Serialize)]
pub struct Message {
    uuid: Uuid,
    channel_uuid: Uuid,
    user_uuid: Uuid,
    message: String,
    reply_to: Option<Uuid>,
    member: Member,
}
