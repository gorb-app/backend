use actix_web::web::BytesMut;
use argon2::{
    PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::Utc;
use diesel::{
    ExpressionMethods, QueryDsl, Selectable, SelectableHelper, delete,
    dsl::now,
    insert_into,
    prelude::{Insertable, Queryable},
    update,
};
use diesel_async::{RunQueryDsl, pooled_connection::AsyncDieselConnectionManager};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message as Email, Tokio1Executor,
    message::{Mailbox, MessageBuilder as EmailBuilder, MultiPart},
    transport::smtp::authentication::Credentials,
};
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::task;
use url::Url;
use uuid::Uuid;

use crate::{
    Conn, Data,
    error::Error,
    schema::*,
    utils::{
        EMAIL_REGEX, PASSWORD_REGEX, USERNAME_REGEX, generate_refresh_token, global_checks,
        image_check, order_by_is_above, user_uuid_from_identifier,
    },
};

pub trait HasUuid {
    fn uuid(&self) -> &Uuid;
}

pub trait HasIsAbove {
    fn is_above(&self) -> Option<&Uuid>;
}

fn load_or_empty<T>(
    query_result: Result<Vec<T>, diesel::result::Error>,
) -> Result<Vec<T>, diesel::result::Error> {
    match query_result {
        Ok(vec) => Ok(vec),
        Err(diesel::result::Error::NotFound) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum MailTls {
    StartTls,
    Tls,
}

impl From<String> for MailTls {
    fn from(value: String) -> Self {
        match &*value.to_lowercase() {
            "starttls" => Self::StartTls,
            _ => Self::Tls,
        }
    }
}

#[derive(Clone)]
pub struct MailClient {
    creds: Credentials,
    smtp_server: String,
    mbox: Mailbox,
    tls: MailTls,
}

impl MailClient {
    pub fn new<T: Into<MailTls>>(
        creds: Credentials,
        smtp_server: String,
        mbox: String,
        tls: T,
    ) -> Result<Self, Error> {
        Ok(Self {
            creds,
            smtp_server,
            mbox: mbox.parse()?,
            tls: tls.into(),
        })
    }

    pub fn message_builder(&self) -> EmailBuilder {
        Email::builder().from(self.mbox.clone())
    }

    pub async fn send_mail(&self, email: Email) -> Result<(), Error> {
        let mailer: AsyncSmtpTransport<Tokio1Executor> = match self.tls {
            MailTls::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_server)?
                    .credentials(self.creds.clone())
                    .build()
            }
            MailTls::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_server)?
                .credentials(self.creds.clone())
                .build(),
        };

        let response = mailer.send(email).await?;

        debug!("mail sending response: {:?}", response);

        Ok(())
    }
}

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

        futures::future::try_join_all(channel_futures).await
    }

    pub async fn fetch_one(data: &Data, channel_uuid: Uuid) -> Result<Self, Error> {
        if let Ok(cache_hit) = data.get_cache_key(channel_uuid.to_string()).await {
            return Ok(serde_json::from_str(&cache_hit)?);
        }

        let mut conn = data.pool.get().await?;

        use channels::dsl;
        let channel_builder: ChannelBuilder = dsl::channels
            .filter(dsl::uuid.eq(channel_uuid))
            .select(ChannelBuilder::as_select())
            .get_result(&mut conn)
            .await?;

        let channel = channel_builder.build(&mut conn).await?;

        data.set_cache_key(channel_uuid.to_string(), channel.clone(), 60)
            .await?;

        Ok(channel)
    }

    pub async fn new(
        data: actix_web::web::Data<Data>,
        guild_uuid: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<Self, Error> {
        let mut conn = data.pool.get().await?;

        let channel_uuid = Uuid::now_v7();

        let channels = Self::fetch_all(&data.pool, guild_uuid).await?;

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

        data.set_cache_key(channel_uuid.to_string(), channel.clone(), 1800)
            .await?;

        if data
            .get_cache_key(format!("{}_channels", guild_uuid))
            .await
            .is_ok()
        {
            data.del_cache_key(format!("{}_channels", guild_uuid))
                .await?;
        }

        Ok(channel)
    }

    pub async fn delete(self, data: &Data) -> Result<(), Error> {
        let mut conn = data.pool.get().await?;

        use channels::dsl;
        delete(channels::table)
            .filter(dsl::uuid.eq(self.uuid))
            .execute(&mut conn)
            .await?;

        if data.get_cache_key(self.uuid.to_string()).await.is_ok() {
            data.del_cache_key(self.uuid.to_string()).await?;
        }

        Ok(())
    }

    pub async fn fetch_messages(
        &self,
        data: &Data,
        amount: i64,
        offset: i64,
    ) -> Result<Vec<Message>, Error> {
        let mut conn = data.pool.get().await?;

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

        let message_futures = messages.iter().map(async move |b| b.build(data).await);

        futures::future::try_join_all(message_futures).await
    }

    pub async fn new_message(
        &self,
        data: &Data,
        user_uuid: Uuid,
        message: String,
    ) -> Result<Message, Error> {
        let message_uuid = Uuid::now_v7();

        let message = MessageBuilder {
            uuid: message_uuid,
            channel_uuid: self.uuid,
            user_uuid,
            message,
        };

        let mut conn = data.pool.get().await?;

        insert_into(messages::table)
            .values(message.clone())
            .execute(&mut conn)
            .await?;

        message.build(data).await
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
    icon: Option<String>,
    owner_uuid: Uuid,
}

impl GuildBuilder {
    async fn build(self, conn: &mut Conn) -> Result<Guild, Error> {
        let member_count = Member::count(conn, self.uuid).await?;

        let roles = Role::fetch_all(conn, self.uuid).await?;

        Ok(Guild {
            uuid: self.uuid,
            name: self.name,
            description: self.description,
            icon: self.icon.and_then(|i| i.parse().ok()),
            owner_uuid: self.owner_uuid,
            roles,
            member_count,
        })
    }
}

#[derive(Serialize)]
pub struct Guild {
    pub uuid: Uuid,
    name: String,
    description: Option<String>,
    icon: Option<Url>,
    owner_uuid: Uuid,
    pub roles: Vec<Role>,
    member_count: i64,
}

impl Guild {
    pub async fn fetch_one(conn: &mut Conn, guild_uuid: Uuid) -> Result<Self, Error> {
        use guilds::dsl;
        let guild_builder: GuildBuilder = dsl::guilds
            .filter(dsl::uuid.eq(guild_uuid))
            .select(GuildBuilder::as_select())
            .get_result(conn)
            .await?;

        guild_builder.build(conn).await
    }

    pub async fn fetch_amount(
        pool: &deadpool::managed::Pool<
            AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>,
            Conn,
        >,
        offset: i64,
        amount: i64,
    ) -> Result<Vec<Self>, Error> {
        // Fetch guild data from database
        let mut conn = pool.get().await?;

        use guilds::dsl;
        let guild_builders: Vec<GuildBuilder> = load_or_empty(
            dsl::guilds
                .select(GuildBuilder::as_select())
                .order_by(dsl::uuid)
                .offset(offset)
                .limit(amount)
                .load(&mut conn)
                .await,
        )?;

        // Process each guild concurrently
        let guild_futures = guild_builders.iter().map(async move |g| {
            let mut conn = pool.get().await?;
            g.clone().build(&mut conn).await
        });

        // Execute all futures concurrently and collect results
        futures::future::try_join_all(guild_futures).await
    }

    pub async fn new(conn: &mut Conn, name: String, owner_uuid: Uuid) -> Result<Self, Error> {
        let guild_uuid = Uuid::now_v7();

        let guild_builder = GuildBuilder {
            uuid: guild_uuid,
            name: name.clone(),
            description: None,
            icon: None,
            owner_uuid,
        };

        insert_into(guilds::table)
            .values(guild_builder)
            .execute(conn)
            .await?;

        let member_uuid = Uuid::now_v7();

        let member = MemberBuilder {
            uuid: member_uuid,
            nickname: None,
            user_uuid: owner_uuid,
            guild_uuid,
        };

        insert_into(guild_members::table)
            .values(member)
            .execute(conn)
            .await?;

        Ok(Guild {
            uuid: guild_uuid,
            name,
            description: None,
            icon: None,
            owner_uuid,
            roles: vec![],
            member_count: 1,
        })
    }

    pub async fn get_invites(&self, conn: &mut Conn) -> Result<Vec<Invite>, Error> {
        use invites::dsl;
        let invites = load_or_empty(
            dsl::invites
                .filter(dsl::guild_uuid.eq(self.uuid))
                .select(Invite::as_select())
                .load(conn)
                .await,
        )?;

        Ok(invites)
    }

    pub async fn create_invite(
        &self,
        conn: &mut Conn,
        user_uuid: Uuid,
        custom_id: Option<String>,
    ) -> Result<Invite, Error> {
        let invite_id;

        if let Some(id) = custom_id {
            invite_id = id;
            if invite_id.len() > 32 {
                return Err(Error::BadRequest("MAX LENGTH".to_string()));
            }
        } else {
            let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

            invite_id = random_string::generate(8, charset);
        }

        let invite = Invite {
            id: invite_id,
            user_uuid,
            guild_uuid: self.uuid,
        };

        insert_into(invites::table)
            .values(invite.clone())
            .execute(conn)
            .await?;

        Ok(invite)
    }

    // FIXME: Horrible security
    pub async fn set_icon(
        &mut self,
        bunny_cdn: &bunny_api_tokio::Client,
        conn: &mut Conn,
        cdn_url: Url,
        icon: BytesMut,
    ) -> Result<(), Error> {
        let icon_clone = icon.clone();
        let image_type = task::spawn_blocking(move || image_check(icon_clone)).await??;

        if let Some(icon) = &self.icon {
            let relative_url = icon.path().trim_start_matches('/');

            bunny_cdn.storage.delete(relative_url).await?;
        }

        let path = format!("icons/{}/icon.{}", self.uuid, image_type);

        bunny_cdn.storage.upload(path.clone(), icon.into()).await?;

        let icon_url = cdn_url.join(&path)?;

        use guilds::dsl;
        update(guilds::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::icon.eq(icon_url.as_str()))
            .execute(conn)
            .await?;

        self.icon = Some(icon_url);

        Ok(())
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
    is_above: Option<Uuid>,
    permissions: i64,
}

impl HasUuid for Role {
    fn uuid(&self) -> &Uuid {
        self.uuid.as_ref()
    }
}

impl HasIsAbove for Role {
    fn is_above(&self) -> Option<&Uuid> {
        self.is_above.as_ref()
    }
}

impl Role {
    pub async fn fetch_all(conn: &mut Conn, guild_uuid: Uuid) -> Result<Vec<Self>, Error> {
        use roles::dsl;
        let roles: Vec<Role> = load_or_empty(
            dsl::roles
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .select(Role::as_select())
                .load(conn)
                .await,
        )?;

        Ok(roles)
    }

    pub async fn fetch_one(conn: &mut Conn, role_uuid: Uuid) -> Result<Self, Error> {
        use roles::dsl;
        let role: Role = dsl::roles
            .filter(dsl::uuid.eq(role_uuid))
            .select(Role::as_select())
            .get_result(conn)
            .await?;

        Ok(role)
    }

    pub async fn new(conn: &mut Conn, guild_uuid: Uuid, name: String) -> Result<Self, Error> {
        let role_uuid = Uuid::now_v7();

        let roles = Self::fetch_all(conn, guild_uuid).await?;

        let roles_ordered = order_by_is_above(roles).await?;

        let last_role = roles_ordered.last();

        let new_role = Role {
            uuid: role_uuid,
            guild_uuid,
            name,
            color: 16777215,
            is_above: None,
            permissions: 0,
        };

        insert_into(roles::table)
            .values(new_role.clone())
            .execute(conn)
            .await?;

        if let Some(old_last_role) = last_role {
            use roles::dsl;
            update(roles::table)
                .filter(dsl::uuid.eq(old_last_role.uuid))
                .set(dsl::is_above.eq(new_role.uuid))
                .execute(conn)
                .await?;
        }

        Ok(new_role)
    }
}

#[derive(Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = guild_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MemberBuilder {
    pub uuid: Uuid,
    pub nickname: Option<String>,
    pub user_uuid: Uuid,
    pub guild_uuid: Uuid,
}

impl MemberBuilder {
    async fn build(&self, data: &Data) -> Result<Member, Error> {
        let user = User::fetch_one(data, self.user_uuid).await?;

        Ok(Member {
            uuid: self.uuid,
            nickname: self.nickname.clone(),
            user_uuid: self.user_uuid,
            guild_uuid: self.guild_uuid,
            user,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct Member {
    pub uuid: Uuid,
    pub nickname: Option<String>,
    pub user_uuid: Uuid,
    pub guild_uuid: Uuid,
    user: User,
}

impl Member {
    async fn count(conn: &mut Conn, guild_uuid: Uuid) -> Result<i64, Error> {
        use guild_members::dsl;
        let count: i64 = dsl::guild_members
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .count()
            .get_result(conn)
            .await?;

        Ok(count)
    }

    pub async fn check_membership(
        conn: &mut Conn,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<(), Error> {
        use guild_members::dsl;
        dsl::guild_members
            .filter(dsl::user_uuid.eq(user_uuid))
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .select(MemberBuilder::as_select())
            .get_result(conn)
            .await?;

        Ok(())
    }

    pub async fn fetch_one(data: &Data, user_uuid: Uuid, guild_uuid: Uuid) -> Result<Self, Error> {
        let mut conn = data.pool.get().await?;

        use guild_members::dsl;
        let member: MemberBuilder = dsl::guild_members
            .filter(dsl::user_uuid.eq(user_uuid))
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .select(MemberBuilder::as_select())
            .get_result(&mut conn)
            .await?;

        member.build(data).await
    }

    pub async fn fetch_all(data: &Data, guild_uuid: Uuid) -> Result<Vec<Self>, Error> {
        let mut conn = data.pool.get().await?;

        use guild_members::dsl;
        let member_builders: Vec<MemberBuilder> = load_or_empty(
            dsl::guild_members
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .select(MemberBuilder::as_select())
                .load(&mut conn)
                .await,
        )?;

        let member_futures = member_builders
            .iter()
            .map(async move |m| m.build(data).await);

        futures::future::try_join_all(member_futures).await
    }

    pub async fn new(data: &Data, user_uuid: Uuid, guild_uuid: Uuid) -> Result<Self, Error> {
        let mut conn = data.pool.get().await?;

        let member_uuid = Uuid::now_v7();

        let member = MemberBuilder {
            uuid: member_uuid,
            guild_uuid,
            user_uuid,
            nickname: None,
        };

        insert_into(guild_members::table)
            .values(&member)
            .execute(&mut conn)
            .await?;

        member.build(data).await
    }
}

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MessageBuilder {
    uuid: Uuid,
    channel_uuid: Uuid,
    user_uuid: Uuid,
    message: String,
}

impl MessageBuilder {
    pub async fn build(&self, data: &Data) -> Result<Message, Error> {
        let user = User::fetch_one(data, self.user_uuid).await?;

        Ok(Message {
            uuid: self.uuid,
            channel_uuid: self.channel_uuid,
            user_uuid: self.user_uuid,
            message: self.message.clone(),
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
    user: User,
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

#[derive(Deserialize, Serialize, Clone, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    uuid: Uuid,
    username: String,
    display_name: Option<String>,
    avatar: Option<String>,
}

impl User {
    pub async fn fetch_one(data: &Data, user_uuid: Uuid) -> Result<Self, Error> {
        let mut conn = data.pool.get().await?;

        if let Ok(cache_hit) = data.get_cache_key(user_uuid.to_string()).await {
            return Ok(serde_json::from_str(&cache_hit)?);
        }

        use users::dsl;
        let user: User = dsl::users
            .filter(dsl::uuid.eq(user_uuid))
            .select(User::as_select())
            .get_result(&mut conn)
            .await?;

        data.set_cache_key(user_uuid.to_string(), user.clone(), 1800)
            .await?;

        Ok(user)
    }

    pub async fn fetch_amount(
        conn: &mut Conn,
        offset: i64,
        amount: i64,
    ) -> Result<Vec<Self>, Error> {
        use users::dsl;
        let users: Vec<User> = load_or_empty(
            dsl::users
                .limit(amount)
                .offset(offset)
                .select(User::as_select())
                .load(conn)
                .await,
        )?;

        Ok(users)
    }
}

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Me {
    pub uuid: Uuid,
    username: String,
    display_name: Option<String>,
    avatar: Option<String>,
    email: String,
    pub email_verified: bool,
}

impl Me {
    pub async fn get(conn: &mut Conn, user_uuid: Uuid) -> Result<Self, Error> {
        use users::dsl;
        let me: Me = dsl::users
            .filter(dsl::uuid.eq(user_uuid))
            .select(Me::as_select())
            .get_result(conn)
            .await?;

        Ok(me)
    }

    pub async fn fetch_memberships(&self, conn: &mut Conn) -> Result<Vec<Guild>, Error> {
        use guild_members::dsl;
        let memberships: Vec<MemberBuilder> = load_or_empty(
            dsl::guild_members
                .filter(dsl::user_uuid.eq(self.uuid))
                .select(MemberBuilder::as_select())
                .load(conn)
                .await
        )?;

        let mut guilds: Vec<Guild> = vec![];

        for membership in memberships {
            use guilds::dsl;
            guilds.push(
                dsl::guilds
                    .filter(dsl::uuid.eq(membership.guild_uuid))
                    .select(GuildBuilder::as_select())
                    .get_result(conn)
                    .await?
                    .build(conn)
                    .await?,
            )
        }

        Ok(guilds)
    }

    pub async fn set_avatar(
        &mut self,
        bunny_cdn: &bunny_api_tokio::Client,
        conn: &mut Conn,
        cdn_url: Url,
        avatar: BytesMut,
    ) -> Result<(), Error> {
        let avatar_clone = avatar.clone();
        let image_type = task::spawn_blocking(move || image_check(avatar_clone)).await??;

        if let Some(avatar) = &self.avatar {
            let avatar_url: Url = avatar.parse()?;

            let relative_url = avatar_url.path().trim_start_matches('/');

            bunny_cdn.storage.delete(relative_url).await?;
        }

        let path = format!("avatar/{}/avatar.{}", self.uuid, image_type);

        bunny_cdn
            .storage
            .upload(path.clone(), avatar.into())
            .await?;

        let avatar_url = cdn_url.join(&path)?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::avatar.eq(avatar_url.as_str()))
            .execute(conn)
            .await?;

        self.avatar = Some(avatar_url.to_string());

        Ok(())
    }

    pub async fn verify_email(&self, conn: &mut Conn) -> Result<(), Error> {
        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::email_verified.eq(true))
            .execute(conn)
            .await?;

        Ok(())
    }

    pub async fn set_username(
        &mut self,
        conn: &mut Conn,
        new_username: String,
    ) -> Result<(), Error> {
        if !USERNAME_REGEX.is_match(&new_username) {
            return Err(Error::BadRequest("Invalid username".to_string()));
        }

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::username.eq(new_username.as_str()))
            .execute(conn)
            .await?;

        self.username = new_username;

        Ok(())
    }

    pub async fn set_display_name(
        &mut self,
        conn: &mut Conn,
        new_display_name: String,
    ) -> Result<(), Error> {
        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::display_name.eq(new_display_name.as_str()))
            .execute(conn)
            .await?;

        self.display_name = Some(new_display_name);

        Ok(())
    }

    pub async fn set_email(&mut self, conn: &mut Conn, new_email: String) -> Result<(), Error> {
        if !EMAIL_REGEX.is_match(&new_email) {
            return Err(Error::BadRequest("Invalid username".to_string()));
        }

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set((
                dsl::email.eq(new_email.as_str()),
                dsl::email_verified.eq(false),
            ))
            .execute(conn)
            .await?;

        self.email = new_email;

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct StartAmountQuery {
    pub start: Option<i64>,
    pub amount: Option<i64>,
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = email_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EmailToken {
    user_uuid: Uuid,
    pub token: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl EmailToken {
    pub async fn get(conn: &mut Conn, user_uuid: Uuid) -> Result<EmailToken, Error> {
        use email_tokens::dsl;
        let email_token = dsl::email_tokens
            .filter(dsl::user_uuid.eq(user_uuid))
            .select(EmailToken::as_select())
            .get_result(conn)
            .await?;

        Ok(email_token)
    }

    #[allow(clippy::new_ret_no_self)]
    pub async fn new(data: &Data, me: Me) -> Result<(), Error> {
        let token = generate_refresh_token()?;

        let mut conn = data.pool.get().await?;

        use email_tokens::dsl;
        insert_into(email_tokens::table)
            .values((
                dsl::user_uuid.eq(me.uuid),
                dsl::token.eq(&token),
                dsl::created_at.eq(now),
            ))
            .execute(&mut conn)
            .await?;

        let mut verify_endpoint = data.config.web.frontend_url.join("verify-email")?;

        verify_endpoint.set_query(Some(&format!("token={}", token)));

        let email = data
            .mail_client
            .message_builder()
            .to(me.email.parse()?)
            .subject(format!("{} E-mail Verification", data.config.instance.name))
            .multipart(MultiPart::alternative_plain_html(
                format!("Verify your {} account\n\nHello, {}!\nThanks for creating a new account on Gorb.\nThe final step to create your account is to verify your email address by visiting the page, within 24 hours.\n\n{}\n\nIf you didn't ask to verify this address, you can safely ignore this email\n\nThanks, The gorb team.", data.config.instance.name, me.username, verify_endpoint), 
                format!(r#"<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><style>:root{{--header-text-colour: #ffffff;--footer-text-colour: #7f7f7f;--button-text-colour: #170e08;--text-colour: #170e08;--background-colour: #fbf6f2;--primary-colour: #df5f0b;--secondary-colour: #e8ac84;--accent-colour: #e68b4e;}}@media (prefers-color-scheme: dark){{:root{{--header-text-colour: #ffffff;--footer-text-colour: #585858;--button-text-colour: #ffffff;--text-colour: #f7eee8;--background-colour: #0c0704;--primary-colour: #f4741f;--secondary-colour: #7c4018;--accent-colour: #b35719;}}}}@media (max-width: 600px){{.container{{width: 100%;}}}}body{{font-family: Arial, sans-serif;align-content: center;text-align: center;margin: 0;padding: 0;background-color: var(--background-colour);color: var(--text-colour);width: 100%;max-width: 600px;margin: 0 auto;border-radius: 5px;}}.header{{background-color: var(--primary-colour);color: var(--header-text-colour);padding: 20px;}}.verify-button{{background-color: var(--accent-colour);color: var(--button-text-colour);padding: 12px 30px;margin: 16px;font-size: 20px;transition: background-color 0.3s;cursor: pointer;border: none;border-radius: 14px;text-decoration: none;display: inline-block;}}.verify-button:hover{{background-color: var(--secondary-colour);}}.content{{padding: 20px 30px;}}.footer{{padding: 10px;font-size: 12px;color: var(--footer-text-colour);}}</style></head><body><div class="container"><div class="header"><h1>Verify your {} Account</h1></div><div class="content"><h2>Hello, {}!</h2><p>Thanks for creating a new account on Gorb.</p><p>The final step to create your account is to verify your email address by clicking the button below, within 24 hours.</p><a href="{}" class="verify-button">VERIFY ACCOUNT</a><p>If you didn't ask to verify this address, you can safely ignore this email.</p><div class="footer"><p>Thanks<br>The gorb team.</p></div></div></div></body></html>"#, data.config.instance.name, me.username, verify_endpoint)
            ))?;

        data.mail_client.send_mail(email).await?;

        Ok(())
    }

    pub async fn delete(&self, conn: &mut Conn) -> Result<(), Error> {
        use email_tokens::dsl;
        delete(email_tokens::table)
            .filter(dsl::user_uuid.eq(self.user_uuid))
            .filter(dsl::token.eq(&self.token))
            .execute(conn)
            .await?;

        Ok(())
    }
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = password_reset_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PasswordResetToken {
    user_uuid: Uuid,
    pub token: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl PasswordResetToken {
    pub async fn get(conn: &mut Conn, token: String) -> Result<PasswordResetToken, Error> {
        use password_reset_tokens::dsl;
        let password_reset_token = dsl::password_reset_tokens
            .filter(dsl::token.eq(token))
            .select(PasswordResetToken::as_select())
            .get_result(conn)
            .await?;

        Ok(password_reset_token)
    }

    pub async fn get_with_identifier(
        conn: &mut Conn,
        identifier: String,
    ) -> Result<PasswordResetToken, Error> {
        let user_uuid = user_uuid_from_identifier(conn, &identifier).await?;

        use password_reset_tokens::dsl;
        let password_reset_token = dsl::password_reset_tokens
            .filter(dsl::user_uuid.eq(user_uuid))
            .select(PasswordResetToken::as_select())
            .get_result(conn)
            .await?;

        Ok(password_reset_token)
    }

    #[allow(clippy::new_ret_no_self)]
    pub async fn new(data: &Data, identifier: String) -> Result<(), Error> {
        let token = generate_refresh_token()?;

        let mut conn = data.pool.get().await?;

        let user_uuid = user_uuid_from_identifier(&mut conn, &identifier).await?;

        global_checks(data, user_uuid).await?;

        use users::dsl as udsl;
        let (username, email_address): (String, String) = udsl::users
            .filter(udsl::uuid.eq(user_uuid))
            .select((udsl::username, udsl::email))
            .get_result(&mut conn)
            .await?;

        use password_reset_tokens::dsl;
        insert_into(password_reset_tokens::table)
            .values((
                dsl::user_uuid.eq(user_uuid),
                dsl::token.eq(&token),
                dsl::created_at.eq(now),
            ))
            .execute(&mut conn)
            .await?;

        let mut reset_endpoint = data.config.web.frontend_url.join("reset-password")?;

        reset_endpoint.set_query(Some(&format!("token={}", token)));

        let email = data
            .mail_client
            .message_builder()
            .to(email_address.parse()?)
            .subject(format!("{} Password Reset", data.config.instance.name))
            .multipart(MultiPart::alternative_plain_html(
                format!("{} Password Reset\n\nHello, {}!\nSomeone requested a password reset for your Gorb account.\nClick the button below within 24 hours to reset your password.\n\n{}\n\nIf you didn't request a password reset, don't worry, your account is safe and you can safely ignore this email.\n\nThanks, The gorb team.", data.config.instance.name, username, reset_endpoint), 
                format!(r#"<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><style>:root {{--header-text-colour: #ffffff;--footer-text-colour: #7f7f7f;--button-text-colour: #170e08;--text-colour: #170e08;--background-colour: #fbf6f2;--primary-colour: #df5f0b;--secondary-colour: #e8ac84;--accent-colour: #e68b4e;}}@media (prefers-color-scheme: dark) {{:root {{--header-text-colour: #ffffff;--footer-text-colour: #585858;--button-text-colour: #ffffff;--text-colour: #f7eee8;--background-colour: #0c0704;--primary-colour: #f4741f;--secondary-colour: #7c4018;--accent-colour: #b35719;}}}}@media (max-width: 600px) {{.container {{width: 100%;}}}}body {{font-family: Arial, sans-serif;align-content: center;text-align: center;margin: 0;padding: 0;background-color: var(--background-colour);color: var(--text-colour);width: 100%;max-width: 600px;margin: 0 auto;border-radius: 5px;}}.header {{background-color: var(--primary-colour);color: var(--header-text-colour);padding: 20px;}}.verify-button {{background-color: var(--accent-colour);color: var(--button-text-colour);padding: 12px 30px;margin: 16px;font-size: 20px;transition: background-color 0.3s;cursor: pointer;border: none;border-radius: 14px;text-decoration: none;display: inline-block;}}.verify-button:hover {{background-color: var(--secondary-colour);}}.content {{padding: 20px 30px;}}.footer {{padding: 10px;font-size: 12px;color: var(--footer-text-colour);}}</style></head><body><div class="container"><div class="header"><h1>{} Password Reset</h1></div><div class="content"><h2>Hello, {}!</h2><p>Someone requested a password reset for your Gorb account.</p><p>Click the button below within 24 hours to reset your password.</p><a href="{}" class="verify-button">RESET PASSWORD</a><p>If you didn't request a password reset, don't worry, your account is safe and you can safely ignore this email.</p><div class="footer"><p>Thanks<br>The gorb team.</p></div></div></div></body></html>"#, data.config.instance.name, username, reset_endpoint)
            ))?;

        data.mail_client.send_mail(email).await?;

        Ok(())
    }

    pub async fn set_password(&self, data: &Data, password: String) -> Result<(), Error> {
        if !PASSWORD_REGEX.is_match(&password) {
            return Err(Error::BadRequest(
                "Please provide a valid password".to_string(),
            ));
        }

        let salt = SaltString::generate(&mut OsRng);

        let hashed_password = data
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::PasswordHashError(e.to_string()))?;

        let mut conn = data.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.user_uuid))
            .set(dsl::password.eq(hashed_password.to_string()))
            .execute(&mut conn)
            .await?;

        let (username, email_address): (String, String) = dsl::users
            .filter(dsl::uuid.eq(self.user_uuid))
            .select((dsl::username, dsl::email))
            .get_result(&mut conn)
            .await?;

        let login_page = data.config.web.frontend_url.join("login")?;

        let email = data
            .mail_client
            .message_builder()
            .to(email_address.parse()?)
            .subject(format!("Your {} Password has been Reset", data.config.instance.name))
            .multipart(MultiPart::alternative_plain_html(
                format!("{} Password Reset Confirmation\n\nHello, {}!\nYour password has been successfully reset for your Gorb account.\nIf you did not initiate this change, please click the link below to reset your password <strong>immediately</strong>.\n\n{}\n\nThanks, The gorb team.", data.config.instance.name, username, login_page), 
                format!(r#"<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><style>:root {{--header-text-colour: #ffffff;--footer-text-colour: #7f7f7f;--button-text-colour: #170e08;--text-colour: #170e08;--background-colour: #fbf6f2;--primary-colour: #df5f0b;--secondary-colour: #e8ac84;--accent-colour: #e68b4e;}}@media (prefers-color-scheme: dark) {{:root {{--header-text-colour: #ffffff;--footer-text-colour: #585858;--button-text-colour: #ffffff;--text-colour: #f7eee8;--background-colour: #0c0704;--primary-colour: #f4741f;--secondary-colour: #7c4018;--accent-colour: #b35719;}}}}@media (max-width: 600px) {{.container {{width: 100%;}}}}body {{font-family: Arial, sans-serif;align-content: center;text-align: center;margin: 0;padding: 0;background-color: var(--background-colour);color: var(--text-colour);width: 100%;max-width: 600px;margin: 0 auto;border-radius: 5px;}}.header {{background-color: var(--primary-colour);color: var(--header-text-colour);padding: 20px;}}.verify-button {{background-color: var(--accent-colour);color: var(--button-text-colour);padding: 12px 30px;margin: 16px;font-size: 20px;transition: background-color 0.3s;cursor: pointer;border: none;border-radius: 14px;text-decoration: none;display: inline-block;}}.verify-button:hover {{background-color: var(--secondary-colour);}}.content {{padding: 20px 30px;}}.footer {{padding: 10px;font-size: 12px;color: var(--footer-text-colour);}}</style></head><body><div class="container"><div class="header"><h1>{} Password Reset Confirmation</h1></div><div class="content"><h2>Hello, {}!</h2><p>Your password has been successfully reset for your Gorb account.</p><p>If you did not initiate this change, please click the button below to reset your password <strong>immediately</strong>.</p><a href="{}" class="verify-button">RESET PASSWORD</a><div class="footer"><p>Thanks<br>The gorb team.</p></div></div></div></body></html>"#, data.config.instance.name, username, login_page)
            ))?;

        data.mail_client.send_mail(email).await?;

        self.delete(&mut conn).await
    }

    pub async fn delete(&self, conn: &mut Conn) -> Result<(), Error> {
        use password_reset_tokens::dsl;
        delete(password_reset_tokens::table)
            .filter(dsl::user_uuid.eq(self.user_uuid))
            .filter(dsl::token.eq(&self.token))
            .execute(conn)
            .await?;

        Ok(())
    }
}
