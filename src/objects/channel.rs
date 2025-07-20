use diesel::{
    ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable, SelectableHelper, delete,
    insert_into, update,
};
use diesel_async::{RunQueryDsl, pooled_connection::AsyncDieselConnectionManager};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState, Conn,
    error::Error,
    schema::{channel_permissions, channels, messages},
    utils::{CHANNEL_REGEX, order_by_is_above},
};

use super::{HasIsAbove, HasUuid, Message, load_or_empty, message::MessageBuilder};

#[derive(Queryable, Selectable, Insertable, Clone, Debug)]
#[diesel(table_name = channels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ChannelBuilder {
    uuid: Uuid,
    guild_uuid: Uuid,
    name: String,
    description: Option<String>,
    is_above: Option<Uuid>,
}

impl ChannelBuilder {
    async fn build(self, conn: &mut Conn) -> Result<Channel, Error> {
        use self::channel_permissions::dsl::*;
        let channel_permission: Vec<ChannelPermission> = load_or_empty(
            channel_permissions
                .filter(channel_uuid.eq(self.uuid))
                .select(ChannelPermission::as_select())
                .load(conn)
                .await,
        )?;

        Ok(Channel {
            uuid: self.uuid,
            guild_uuid: self.guild_uuid,
            name: self.name,
            description: self.description,
            is_above: self.is_above,
            permissions: channel_permission,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Channel {
    pub uuid: Uuid,
    pub guild_uuid: Uuid,
    name: String,
    description: Option<String>,
    pub is_above: Option<Uuid>,
    pub permissions: Vec<ChannelPermission>,
}

#[derive(Serialize, Deserialize, Clone, Queryable, Selectable, Debug)]
#[diesel(table_name = channel_permissions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ChannelPermission {
    pub role_uuid: Uuid,
    pub permissions: i64,
}

impl HasUuid for Channel {
    fn uuid(&self) -> &Uuid {
        self.uuid.as_ref()
    }
}

impl HasIsAbove for Channel {
    fn is_above(&self) -> Option<&Uuid> {
        self.is_above.as_ref()
    }
}

impl Channel {
    pub async fn fetch_all(
        pool: &deadpool::managed::Pool<
            AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>,
            Conn,
        >,
        guild_uuid: Uuid,
    ) -> Result<Vec<Self>, Error> {
        let mut conn = pool.get().await?;

        use channels::dsl;
        let channel_builders: Vec<ChannelBuilder> = load_or_empty(
            dsl::channels
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .select(ChannelBuilder::as_select())
                .load(&mut conn)
                .await,
        )?;

        let channel_futures = channel_builders.iter().map(async move |c| {
            let mut conn = pool.get().await?;
            c.clone().build(&mut conn).await
        });

        futures_util::future::try_join_all(channel_futures).await
    }

    pub async fn fetch_one(app_state: &AppState, channel_uuid: Uuid) -> Result<Self, Error> {
        if let Ok(cache_hit) = app_state.get_cache_key(channel_uuid.to_string()).await {
            return Ok(serde_json::from_str(&cache_hit)?);
        }

        let mut conn = app_state.pool.get().await?;

        use channels::dsl;
        let channel_builder: ChannelBuilder = dsl::channels
            .filter(dsl::uuid.eq(channel_uuid))
            .select(ChannelBuilder::as_select())
            .get_result(&mut conn)
            .await?;

        let channel = channel_builder.build(&mut conn).await?;

        app_state
            .set_cache_key(channel_uuid.to_string(), channel.clone(), 60)
            .await?;

        Ok(channel)
    }

    pub async fn new(
        app_state: &AppState,
        guild_uuid: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<Self, Error> {
        if !CHANNEL_REGEX.is_match(&name) {
            return Err(Error::BadRequest("Channel name is invalid".to_string()));
        }

        let mut conn = app_state.pool.get().await?;

        let channel_uuid = Uuid::now_v7();

        let channels = Self::fetch_all(&app_state.pool, guild_uuid).await?;

        let channels_ordered = order_by_is_above(channels).await?;

        let last_channel = channels_ordered.last();

        let new_channel = ChannelBuilder {
            uuid: channel_uuid,
            guild_uuid,
            name: name.clone(),
            description: description.clone(),
            is_above: None,
        };

        insert_into(channels::table)
            .values(new_channel.clone())
            .execute(&mut conn)
            .await?;

        if let Some(old_last_channel) = last_channel {
            use channels::dsl;
            update(channels::table)
                .filter(dsl::uuid.eq(old_last_channel.uuid))
                .set(dsl::is_above.eq(new_channel.uuid))
                .execute(&mut conn)
                .await?;
        }

        // returns different object because there's no reason to build the channelbuilder (wastes 1 database request)
        let channel = Self {
            uuid: channel_uuid,
            guild_uuid,
            name,
            description,
            is_above: None,
            permissions: vec![],
        };

        app_state
            .set_cache_key(channel_uuid.to_string(), channel.clone(), 1800)
            .await?;

        if app_state
            .get_cache_key(format!("{guild_uuid}_channels"))
            .await
            .is_ok()
        {
            app_state
                .del_cache_key(format!("{guild_uuid}_channels"))
                .await?;
        }

        Ok(channel)
    }

    pub async fn delete(self, app_state: &AppState) -> Result<(), Error> {
        let mut conn = app_state.pool.get().await?;

        use channels::dsl;
        match update(channels::table)
            .filter(dsl::is_above.eq(self.uuid))
            .set(dsl::is_above.eq(None::<Uuid>))
            .execute(&mut conn)
            .await
        {
            Ok(r) => Ok(r),
            Err(diesel::result::Error::NotFound) => Ok(0),
            Err(e) => Err(e),
        }?;

        delete(channels::table)
            .filter(dsl::uuid.eq(self.uuid))
            .execute(&mut conn)
            .await?;

        match update(channels::table)
            .filter(dsl::is_above.eq(self.uuid))
            .set(dsl::is_above.eq(self.is_above))
            .execute(&mut conn)
            .await
        {
            Ok(r) => Ok(r),
            Err(diesel::result::Error::NotFound) => Ok(0),
            Err(e) => Err(e),
        }?;

        if app_state.get_cache_key(self.uuid.to_string()).await.is_ok() {
            app_state.del_cache_key(self.uuid.to_string()).await?;
        }

        if app_state
            .get_cache_key(format!("{}_channels", self.guild_uuid))
            .await
            .is_ok()
        {
            app_state
                .del_cache_key(format!("{}_channels", self.guild_uuid))
                .await?;
        }

        Ok(())
    }

    pub async fn fetch_messages(
        &self,
        app_state: &AppState,
        amount: i64,
        offset: i64,
    ) -> Result<Vec<Message>, Error> {
        let mut conn = app_state.pool.get().await?;

        use messages::dsl;
        let messages: Vec<MessageBuilder> = load_or_empty(
            dsl::messages
                .filter(dsl::channel_uuid.eq(self.uuid))
                .select(MessageBuilder::as_select())
                .order(dsl::uuid.desc())
                .limit(amount)
                .offset(offset)
                .load(&mut conn)
                .await,
        )?;

        let message_futures = messages.iter().map(async move |b| b.build(app_state).await);

        futures_util::future::try_join_all(message_futures).await
    }

    pub async fn new_message(
        &self,
        app_state: &AppState,
        user_uuid: Uuid,
        message: String,
        reply_to: Option<Uuid>,
    ) -> Result<Message, Error> {
        let message_uuid = Uuid::now_v7();

        let message = MessageBuilder {
            uuid: message_uuid,
            channel_uuid: self.uuid,
            user_uuid,
            message,
            reply_to,
            is_edited: false,
        };

        let mut conn = app_state.pool.get().await?;

        insert_into(messages::table)
            .values(message.clone())
            .execute(&mut conn)
            .await?;

        message.build(app_state).await
    }

    /*pub async fn edit_message(&self, data: &Data, user_uuid: Uuid, message_uuid: Uuid, message: String) -> Result<Message, Error> {
        use messages::dsl;

        let mut conn = data.pool.get().await?;

        update(messages::table)
            .filter(dsl::user_uuid.eq(user_uuid))
            .filter(dsl::uuid.eq(message_uuid))
            .set((dsl::is_edited.eq(true), dsl::message.eq(message)))
            .execute(&mut conn)
            .await?;

        Ok(())
    }*/

    pub async fn set_name(&mut self, app_state: &AppState, new_name: String) -> Result<(), Error> {
        if !CHANNEL_REGEX.is_match(&new_name) {
            return Err(Error::BadRequest("Channel name is invalid".to_string()));
        }

        let mut conn = app_state.pool.get().await?;

        use channels::dsl;
        update(channels::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::name.eq(&new_name))
            .execute(&mut conn)
            .await?;

        self.name = new_name;

        Ok(())
    }

    pub async fn set_description(
        &mut self,
        app_state: &AppState,
        new_description: String,
    ) -> Result<(), Error> {
        let mut conn = app_state.pool.get().await?;

        use channels::dsl;
        update(channels::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::description.eq(&new_description))
            .execute(&mut conn)
            .await?;

        self.description = Some(new_description);

        Ok(())
    }

    pub async fn move_channel(
        &mut self,
        app_state: &AppState,
        new_is_above: Uuid,
    ) -> Result<(), Error> {
        let mut conn = app_state.pool.get().await?;

        use channels::dsl;
        let old_above_uuid: Option<Uuid> = match dsl::channels
            .filter(dsl::is_above.eq(self.uuid))
            .select(dsl::uuid)
            .get_result(&mut conn)
            .await
        {
            Ok(r) => Ok(Some(r)),
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => Err(e),
        }?;

        if let Some(uuid) = old_above_uuid {
            update(channels::table)
                .filter(dsl::uuid.eq(uuid))
                .set(dsl::is_above.eq(None::<Uuid>))
                .execute(&mut conn)
                .await?;
        }

        match update(channels::table)
            .filter(dsl::is_above.eq(new_is_above))
            .set(dsl::is_above.eq(self.uuid))
            .execute(&mut conn)
            .await
        {
            Ok(r) => Ok(r),
            Err(diesel::result::Error::NotFound) => Ok(0),
            Err(e) => Err(e),
        }?;

        update(channels::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::is_above.eq(new_is_above))
            .execute(&mut conn)
            .await?;

        if let Some(uuid) = old_above_uuid {
            update(channels::table)
                .filter(dsl::uuid.eq(uuid))
                .set(dsl::is_above.eq(self.is_above))
                .execute(&mut conn)
                .await?;
        }

        self.is_above = Some(new_is_above);

        Ok(())
    }
}
