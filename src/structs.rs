use actix_web::HttpResponse;
use diesel::{delete, insert_into, prelude::{Insertable, Queryable}, ExpressionMethods, QueryDsl, Selectable, SelectableHelper};
use log::error;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use diesel_async::{pooled_connection::AsyncDieselConnectionManager, RunQueryDsl};

use crate::{Conn, Data, schema::*};

#[derive(Queryable, Selectable, Insertable, Clone)]
#[diesel(table_name = channels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ChannelBuilder {
    uuid: Uuid,
    guild_uuid: Uuid,
    name: String,
    description: Option<String>,
}

impl ChannelBuilder {
    async fn build(self, conn: &mut Conn) -> Result<Channel, crate::Error> {
        use self::channel_permissions::dsl::*;
        let channel_permission: Vec<ChannelPermission> = channel_permissions
            .filter(channel_uuid.eq(self.uuid))
            .select((role_uuid, permissions))
            .load(conn)
            .await?;

        Ok(Channel {
            uuid: self.uuid,
            guild_uuid: self.guild_uuid,
            name: self.name,
            description: self.description,
            permissions: channel_permission,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Channel {
    pub uuid: Uuid,
    pub guild_uuid: Uuid,
    name: String,
    description: Option<String>,
    pub permissions: Vec<ChannelPermission>,
}

#[derive(Serialize, Deserialize, Clone, Queryable)]
#[diesel(table_name = channel_permissions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ChannelPermission {
    pub role_uuid: Uuid,
    pub permissions: i64,
}

impl Channel {
    pub async fn fetch_all(
        pool: &deadpool::managed::Pool<AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>, Conn>,
        guild_uuid: Uuid,
    ) -> Result<Vec<Self>, HttpResponse> {
        let mut conn = pool.get().await.unwrap();

        use channels::dsl;
        let channel_builders_result: Result<Vec<ChannelBuilder>, diesel::result::Error> = dsl::channels
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .select(ChannelBuilder::as_select())
            .load(&mut conn)
            .await;

        if let Err(error) = channel_builders_result {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        let channel_builders = channel_builders_result.unwrap();

        let channel_futures = channel_builders.iter().map(async move |c| {
            let mut conn = pool.get().await?;
            c.clone().build(&mut conn).await
        });

        
        let channels = futures::future::try_join_all(channel_futures).await;

        if let Err(error) = channels {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        Ok(channels.unwrap())
    }

    pub async fn fetch_one(
        conn: &mut Conn,
        channel_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
        use channels::dsl;
        let channel_builder_result: Result<ChannelBuilder, diesel::result::Error> = dsl::channels
            .filter(dsl::uuid.eq(channel_uuid))
            .select(ChannelBuilder::as_select())
            .get_result(conn)
            .await;

        if let Err(error) = channel_builder_result {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        let channel_builder = channel_builder_result.unwrap();

        let channel = channel_builder.build(conn).await;

        if let Err(error) = channel {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        Ok(channel.unwrap())
    }

    pub async fn new(
        data: actix_web::web::Data<Data>,
        guild_uuid: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<Self, HttpResponse> {
        let mut conn = data.pool.get().await.unwrap();

        let channel_uuid = Uuid::now_v7();

        let new_channel = ChannelBuilder {
            uuid: channel_uuid,
            guild_uuid: guild_uuid,
            name: name.clone(),
            description: description.clone(),
        };

        let insert_result = insert_into(channels::table)
            .values(new_channel)
            .execute(&mut conn)
            .await;

        if let Err(error) = insert_result {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        // returns different object because there's no reason to build the channelbuilder (wastes 1 database request)
        let channel = Self {
            uuid: channel_uuid,
            guild_uuid,
            name,
            description,
            permissions: vec![],
        };

        let cache_result = data
            .set_cache_key(channel_uuid.to_string(), channel.clone(), 1800)
            .await;

        if let Err(error) = cache_result {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        let cache_deletion_result = data.del_cache_key(format!("{}_channels", guild_uuid)).await;

        if let Err(error) = cache_deletion_result {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(channel)
    }

    pub async fn delete(self, conn: &mut Conn) -> Result<(), HttpResponse> {
        use channels::dsl;
        let result = delete(channels::table)
            .filter(dsl::uuid.eq(self.uuid))
            .execute(conn)
            .await;

        if let Err(error) = result {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(())
    }

    pub async fn fetch_messages(
        &self,
        conn: &mut Conn,
        amount: i64,
        offset: i64,
    ) -> Result<Vec<Message>, HttpResponse> {
        use messages::dsl;
        let messages: Result<Vec<Message>, diesel::result::Error> = dsl::messages
            .filter(dsl::channel_uuid.eq(self.uuid))
            .select(Message::as_select())
            .limit(amount)
            .offset(offset)
            .load(conn)
            .await;

        if let Err(error) = messages {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(messages.unwrap())
    }

    pub async fn new_message(
        &self,
        conn: &mut Conn,
        user_uuid: Uuid,
        message: String,
    ) -> Result<Message, HttpResponse> {
        let message_uuid = Uuid::now_v7();

        let message = Message {
            uuid: message_uuid,
            channel_uuid: self.uuid,
            user_uuid,
            message,
        };

        let insert_result = insert_into(messages::table)
            .values(message.clone())
            .execute(conn)
            .await;

        if let Err(error) = insert_result {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(message)
    }
}

#[derive(Clone, Copy)]
pub enum Permissions {
    SendMessage = 1,
    CreateChannel = 2,
    DeleteChannel = 4,
    ManageChannel = 8,
    CreateRole = 16,
    DeleteRole = 32,
    ManageRole = 64,
    CreateInvite = 128,
    ManageInvite = 256,
    ManageServer = 512,
    ManageMember = 1024,
}

impl Permissions {
    pub fn fetch_permissions(permissions: i64) -> Vec<Self> {
        let all_perms = vec![
            Self::SendMessage,
            Self::CreateChannel,
            Self::DeleteChannel,
            Self::ManageChannel,
            Self::CreateRole,
            Self::DeleteRole,
            Self::ManageRole,
            Self::CreateInvite,
            Self::ManageInvite,
            Self::ManageServer,
            Self::ManageMember,
        ];

        all_perms
            .into_iter()
            .filter(|p| permissions & (*p as i64) != 0)
            .collect()
    }
}

#[derive(Serialize, Queryable, Selectable, Insertable, Clone)]
#[diesel(table_name = guilds)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct GuildBuilder {
    uuid: Uuid,
    name: String,
    description: Option<String>,
    owner_uuid: Uuid,
}

impl GuildBuilder {
    async fn build(self, conn: &mut Conn) -> Result<Guild, HttpResponse> {
        let member_count = Member::count(conn, self.uuid).await?;

        let roles = Role::fetch_all(conn, self.uuid).await?;

        Ok(Guild {
            uuid: self.uuid,
            name: self.name,
            description: self.description,
            icon: String::from("bogus"),
            owner_uuid: self.owner_uuid,
            roles: roles,
            member_count: member_count,
        })
    }
}

#[derive(Serialize)]
pub struct Guild {
    pub uuid: Uuid,
    name: String,
    description: Option<String>,
    icon: String,
    owner_uuid: Uuid,
    pub roles: Vec<Role>,
    member_count: i64,
}

impl Guild {
    pub async fn fetch_one(conn: &mut Conn, guild_uuid: Uuid) -> Result<Self, HttpResponse> {
        use guilds::dsl;
        let guild_builder: Result<GuildBuilder, diesel::result::Error> = dsl::guilds
            .filter(dsl::uuid.eq(guild_uuid))
            .select(GuildBuilder::as_select())
            .get_result(conn)
            .await;

        if let Err(error) = guild_builder {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        let guild = guild_builder.unwrap().build(conn).await?;

        Ok(guild)
    }

    pub async fn fetch_amount(
        pool: &deadpool::managed::Pool<AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>, Conn>,
        offset: i64,
        amount: i64,
    ) -> Result<Vec<Self>, HttpResponse> {
        // Fetch guild data from database
        let mut conn = pool.get().await.unwrap();

        use guilds::dsl;
        let guild_builders: Vec<GuildBuilder> = dsl::guilds
            .select(GuildBuilder::as_select())
            .order_by(dsl::uuid)
            .offset(offset)
            .limit(amount)
            .load(&mut conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError().finish()
            })?;

        // Process each guild concurrently
        let guild_futures = guild_builders.iter().map(async move |g| {
            let mut conn = pool.get().await.unwrap();
            g.clone().build(&mut conn).await
        });

        // Execute all futures concurrently and collect results
        futures::future::try_join_all(guild_futures).await
    }

    pub async fn new(
        conn: &mut Conn,
        name: String,
        description: Option<String>,
        owner_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
        let guild_uuid = Uuid::now_v7();

        let guild_builder = GuildBuilder {
            uuid: guild_uuid,
            name: name.clone(),
            description: description.clone(),
            owner_uuid,
        };

        insert_into(guilds::table)
            .values(guild_builder)
            .execute(conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError().finish()
            })?;

        Ok(Guild {
            uuid: guild_uuid,
            name,
            description,
            icon: "bogus".to_string(),
            owner_uuid,
            roles: vec![],
            member_count: 1,
        })
    }

    pub async fn get_invites(&self, conn: &mut Conn) -> Result<Vec<Invite>, HttpResponse> {
        use invites::dsl;
        let invites = dsl::invites
            .filter(dsl::guild_uuid.eq(self.uuid))
            .select(Invite::as_select())
            .load(conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError().finish()
            })?;

        Ok(invites)
    }

    pub async fn create_invite(
        &self,
        conn: &mut Conn,
        member: &Member,
        custom_id: Option<String>,
    ) -> Result<Invite, HttpResponse> {
        let invite_id;

        if let Some(id) = custom_id {
            invite_id = id;
            if invite_id.len() > 32 {
                return Err(HttpResponse::BadRequest().finish());
            }
        } else {
            let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

            invite_id = random_string::generate(8, charset);
        }

        let invite = Invite {
            id: invite_id,
            user_uuid: member.user_uuid,
            guild_uuid: self.uuid,
        };

        insert_into(invites::table)
            .values(invite.clone())
            .execute(conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError().finish()
            })?;

        Ok(invite)
    }
}

#[derive(Serialize, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Role {
    uuid: Uuid,
    guild_uuid: Uuid,
    name: String,
    color: i32,
    position: i32,
    permissions: i64,
}

impl Role {
    pub async fn fetch_all(
        conn: &mut Conn,
        guild_uuid: Uuid,
    ) -> Result<Vec<Self>, HttpResponse> {
        use roles::dsl;
        let roles: Vec<Role> = dsl::roles
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .select(Role::as_select())
            .load(conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError().finish()
            })?;

        Ok(roles)
    }

    pub async fn fetch_one(
        conn: &mut Conn,
        role_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
        use roles::dsl;
        let role: Role = dsl::roles
            .filter(dsl::uuid.eq(role_uuid))
            .select(Role::as_select())
            .get_result(conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError().finish()
            })?;

        Ok(role)
    }

    pub async fn new(
        conn: &mut Conn,
        guild_uuid: Uuid,
        name: String,
    ) -> Result<Self, HttpResponse> {
        let role_uuid = Uuid::now_v7();

        let role = Role {
            uuid: role_uuid,
            guild_uuid,
            name,
            color: 16777215,
            position: 0,
            permissions: 0,
        };

        insert_into(roles::table)
            .values(role.clone())
            .execute(conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError().finish()
            })?;

        Ok(role)
    }
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = guild_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Member {
    pub uuid: Uuid,
    pub nickname: Option<String>,
    pub user_uuid: Uuid,
    pub guild_uuid: Uuid,
}

impl Member {
    async fn count(conn: &mut Conn, guild_uuid: Uuid) -> Result<i64, HttpResponse> {
        use guild_members::dsl;
        let count: i64 = dsl::guild_members
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .count()
            .get_result(conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError()
            })?;

        Ok(count)
    }

    pub async fn fetch_one(
        conn: &mut Conn,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
    use guild_members::dsl;
    let member: Member = dsl::guild_members
        .filter(dsl::user_uuid.eq(user_uuid))
        .filter(dsl::guild_uuid.eq(guild_uuid))
        .select(Member::as_select())
        .get_result(conn)
        .await
        .map_err(|error| {
            error!("{}", error);
            HttpResponse::InternalServerError().finish()
        })?;

        Ok(member)
    }

    pub async fn new(
        conn: &mut Conn,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
        let member_uuid = Uuid::now_v7();

        let member = Member {
            uuid: member_uuid,
            guild_uuid,
            user_uuid,
            nickname: None,
        };

        insert_into(guild_members::table)
            .values(member)
            .execute(conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError().finish()
            })?;

        Ok(Self {
            uuid: member_uuid,
            nickname: None,
            user_uuid,
            guild_uuid,
        })
    }
}

#[derive(Clone, Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Message {
    uuid: Uuid,
    channel_uuid: Uuid,
    user_uuid: Uuid,
    message: String,
}

/// Server invite struct
#[derive(Clone, Serialize, Queryable, Selectable, Insertable)]
pub struct Invite {
    /// case-sensitive alphanumeric string with a fixed length of 8 characters, can be up to 32 characters for custom invites
    id: String,
    /// User that created the invite
    user_uuid: Uuid,
    /// UUID of the guild that the invite belongs to
    pub guild_uuid: Uuid,
}

impl Invite {
    pub async fn fetch_one(conn: &mut Conn, invite_id: String) -> Result<Self, HttpResponse> {
        use invites::dsl;
        let invite: Invite = dsl::invites
            .filter(dsl::id.eq(invite_id))
            .select(Invite::as_select())
            .get_result(conn)
            .await
            .map_err(|error| {
                error!("{}", error);
                HttpResponse::InternalServerError().finish()
            })?;

        Ok(invite)
    }
}

#[derive(Deserialize)]
pub struct StartAmountQuery {
    pub start: Option<i32>,
    pub amount: Option<i32>,
}
