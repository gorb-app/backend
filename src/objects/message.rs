use diesel::{Insertable, Queryable, Selectable};
use serde::Serialize;
use uuid::Uuid;

use crate::{Data, error::Error, schema::messages};

use super::User;

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
    pub async fn build(&self, data: &Data) -> Result<Message, Error> {
        let user = User::fetch_one(data, self.user_uuid).await?;

        Ok(Message {
            uuid: self.uuid,
            channel_uuid: self.channel_uuid,
            user_uuid: self.user_uuid,
            message: self.message.clone(),
            reply_to: self.reply_to,
            user,
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
    user: User,
}
