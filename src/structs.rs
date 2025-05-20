use std::str::FromStr;

use actix_web::{web::BytesMut, HttpResponse};
use bindet::FileType;
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, prelude::FromRow};
use tokio::task;
use url::Url;
use uuid::Uuid;

use crate::Data;

#[derive(Serialize, Deserialize, Clone)]
pub struct Channel {
    pub uuid: Uuid,
    pub guild_uuid: Uuid,
    name: String,
    description: Option<String>,
    pub permissions: Vec<ChannelPermission>,
}

#[derive(Serialize, Clone, FromRow)]
struct ChannelPermissionBuilder {
    role_uuid: String,
    permissions: i32,
}

impl ChannelPermissionBuilder {
    fn build(&self) -> ChannelPermission {
        ChannelPermission {
            role_uuid: Uuid::from_str(&self.role_uuid).unwrap(),
            permissions: self.permissions,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, FromRow)]
pub struct ChannelPermission {
    pub role_uuid: Uuid,
    pub permissions: i32,
}

impl Channel {
    pub async fn fetch_all(
        pool: &Pool<Postgres>,
        guild_uuid: Uuid,
    ) -> Result<Vec<Self>, HttpResponse> {
        let row = sqlx::query_as(&format!(
            "SELECT CAST(uuid AS VARCHAR), name, description FROM channels WHERE guild_uuid = '{}'",
            guild_uuid
        ))
        .fetch_all(pool)
        .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        let channels: Vec<(String, String, Option<String>)> = row.unwrap();

        let futures = channels.iter().map(async |t| {
            let (uuid, name, description) = t.to_owned();

            let row = sqlx::query_as(&format!("SELECT CAST(role_uuid AS VARCHAR), permissions FROM channel_permissions WHERE channel_uuid = '{}'", uuid))
                .fetch_all(pool)
                .await;

            if let Err(error) = row {
                error!("{}", error);

                return Err(HttpResponse::InternalServerError().finish())
            }

            let channel_permission_builders: Vec<ChannelPermissionBuilder> = row.unwrap();

            Ok(Self {
                uuid: Uuid::from_str(&uuid).unwrap(),
                guild_uuid,
                name,
                description,
                permissions: channel_permission_builders.iter().map(|b| b.build()).collect(),
            })
        });

        let channels = futures::future::join_all(futures).await;

        let channels: Result<Vec<Channel>, HttpResponse> = channels.into_iter().collect();

        channels
    }

    pub async fn fetch_one(
        pool: &Pool<Postgres>,
        guild_uuid: Uuid,
        channel_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
        let row = sqlx::query_as(&format!(
            "SELECT name, description FROM channels WHERE guild_uuid = '{}' AND uuid = '{}'",
            guild_uuid, channel_uuid
        ))
        .fetch_one(pool)
        .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        let (name, description): (String, Option<String>) = row.unwrap();

        let row = sqlx::query_as(&format!("SELECT CAST(role_uuid AS VARCHAR), permissions FROM channel_permissions WHERE channel_uuid = '{}'", channel_uuid))
            .fetch_all(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        let channel_permission_builders: Vec<ChannelPermissionBuilder> = row.unwrap();

        Ok(Self {
            uuid: channel_uuid,
            guild_uuid,
            name,
            description,
            permissions: channel_permission_builders
                .iter()
                .map(|b| b.build())
                .collect(),
        })
    }

    pub async fn new(
        data: actix_web::web::Data<Data>,
        guild_uuid: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<Self, HttpResponse> {
        let channel_uuid = Uuid::now_v7();

        let row = sqlx::query(&format!("INSERT INTO channels (uuid, guild_uuid, name, description) VALUES ('{}', '{}', $1, $2)", channel_uuid, guild_uuid))
            .bind(&name)
            .bind(&description)
            .execute(&data.pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

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

    pub async fn delete(self, pool: &Pool<Postgres>) -> Result<(), HttpResponse> {
        let result = sqlx::query(&format!(
            "DELETE FROM channels WHERE channel_uuid = '{}'",
            self.uuid
        ))
        .execute(pool)
        .await;

        if let Err(error) = result {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(())
    }

    pub async fn fetch_messages(
        &self,
        pool: &Pool<Postgres>,
        amount: i64,
        offset: i64,
    ) -> Result<Vec<Message>, HttpResponse> {
        let row = sqlx::query_as(&format!("SELECT CAST(uuid AS VARCHAR), CAST(user_uuid AS VARCHAR), CAST(channel_uuid AS VARCHAR), message FROM messages WHERE channel_uuid = '{}' ORDER BY uuid DESC LIMIT $1 OFFSET $2", self.uuid))
            .bind(amount)
            .bind(offset)
            .fetch_all(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        let message_builders: Vec<MessageBuilder> = row.unwrap();

        Ok(message_builders.iter().map(|b| b.build()).collect())
    }

    pub async fn new_message(
        &self,
        pool: &Pool<Postgres>,
        user_uuid: Uuid,
        message: String,
    ) -> Result<Message, HttpResponse> {
        let message_uuid = Uuid::now_v7();

        let row = sqlx::query(&format!("INSERT INTO messages (uuid, channel_uuid, user_uuid, message) VALUES ('{}', '{}', '{}', $1)", message_uuid, self.uuid, user_uuid))
            .bind(&message)
            .execute(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(Message {
            uuid: message_uuid,
            channel_uuid: self.uuid,
            user_uuid,
            message,
        })
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

#[derive(Serialize)]
pub struct Guild {
    pub uuid: Uuid,
    name: String,
    description: Option<String>,
    icon: Option<String>,
    owner_uuid: Uuid,
    pub roles: Vec<Role>,
    member_count: i64,
}

impl Guild {
    pub async fn fetch_one(pool: &Pool<Postgres>, guild_uuid: Uuid) -> Result<Self, HttpResponse> {
        let row = sqlx::query_as(&format!(
            "SELECT CAST(owner_uuid AS VARCHAR), name, description, icon FROM guilds WHERE uuid = '{}'",
            guild_uuid
        ))
        .fetch_one(pool)
        .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        let (owner_uuid_raw, name, description, icon): (String, String, Option<String>, Option<String>) = row.unwrap();

        let owner_uuid = Uuid::from_str(&owner_uuid_raw).unwrap();

        let member_count = Member::count(pool, guild_uuid).await?;

        let roles = Role::fetch_all(pool, guild_uuid).await?;

        Ok(Self {
            uuid: guild_uuid,
            name,
            description,
            icon,
            owner_uuid,
            roles,
            member_count,
        })
    }

    pub async fn new(
        pool: &Pool<Postgres>,
        name: String,
        description: Option<String>,
        owner_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
        let guild_uuid = Uuid::now_v7();

        let row = sqlx::query(&format!(
            "INSERT INTO guilds (uuid, owner_uuid, name, description) VALUES ('{}', '{}', $1, $2)",
            guild_uuid, owner_uuid
        ))
        .bind(&name)
        .bind(&description)
        .execute(pool)
        .await;

        if let Err(error) = row {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        let row = sqlx::query(&format!(
            "INSERT INTO guild_members (uuid, guild_uuid, user_uuid) VALUES ('{}', '{}', '{}')",
            Uuid::now_v7(),
            guild_uuid,
            owner_uuid
        ))
        .execute(pool)
        .await;

        if let Err(error) = row {
            error!("{}", error);

            let row = sqlx::query(&format!("DELETE FROM guilds WHERE uuid = '{}'", guild_uuid))
                .execute(pool)
                .await;

            if let Err(error) = row {
                error!("{}", error);
            }

            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(Guild {
            uuid: guild_uuid,
            name,
            description,
            icon: None,
            owner_uuid,
            roles: vec![],
            member_count: 1,
        })
    }

    pub async fn get_invites(&self, pool: &Pool<Postgres>) -> Result<Vec<Invite>, HttpResponse> {
        let invites = sqlx::query_as(&format!(
            "SELECT (id, guild_uuid, user_uuid) FROM invites WHERE guild_uuid = '{}'",
            self.uuid
        ))
        .fetch_all(pool)
        .await;

        if let Err(error) = invites {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(invites
            .unwrap()
            .iter()
            .map(|b: &InviteBuilder| b.build())
            .collect())
    }

    pub async fn create_invite(
        &self,
        pool: &Pool<Postgres>,
        member: &Member,
        custom_id: Option<String>,
    ) -> Result<Invite, HttpResponse> {
        let invite_id;

        if custom_id.is_none() {
            let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

            invite_id = random_string::generate(8, charset);
        } else {
            invite_id = custom_id.unwrap();
            if invite_id.len() > 32 {
                return Err(HttpResponse::BadRequest().finish());
            }
        }

        let result = sqlx::query(&format!(
            "INSERT INTO invites (id, guild_uuid, user_uuid) VALUES ($1, '{}', '{}'",
            self.uuid, member.user_uuid
        ))
        .bind(&invite_id)
        .execute(pool)
        .await;

        if let Err(error) = result {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(Invite {
            id: invite_id,
            user_uuid: member.user_uuid,
            guild_uuid: self.uuid,
        })
    }

    // FIXME: Horrible security
    pub async fn set_icon(&mut self, bunny_cdn: &bunny_api_tokio::Client, pool: &Pool<Postgres>, cdn_url: Url, icon: BytesMut) -> Result<(), HttpResponse> {
        let ico = icon.clone();

        let result = task::spawn_blocking(move || {
            let buf = std::io::Cursor::new(ico.to_vec());

            let detect = bindet::detect(buf).map_err(|e| e.kind());

            if let Ok(Some(file_type)) = detect {
                if file_type.likely_to_be == vec![FileType::Jpg] {
                    return String::from("jpg")
                } else if file_type.likely_to_be == vec![FileType::Png] {
                    return String::from("png")
                }
            }
            String::from("unknown")
        }).await;

        if let Err(error) = result {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        let image_type = result.unwrap();

        if image_type == "unknown" {
            return Err(HttpResponse::BadRequest().finish())
        }

        if let Some(icon) = &self.icon {
            let relative_url = icon.trim_start_matches("https://cdn.gorb.app/");

            let delete_result = bunny_cdn.storage.delete(relative_url).await;

            if let Err(error) = delete_result {
                error!("{}", error);

                return Err(HttpResponse::InternalServerError().finish())
            }
        }

        let path = format!("icons/{}/icon.{}", self.uuid, image_type);

        let upload_result = bunny_cdn.storage.upload(path.clone(), icon.into()).await;

        if let Err(error) = upload_result {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        
        let icon_url = cdn_url.join(&path).unwrap();

        let row = sqlx::query(&format!("UPDATE guilds SET icon = $1 WHERE uuid = '{}'", self.uuid))
            .bind(icon_url.as_str())
            .execute(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        self.icon = Some(icon_url.to_string());

        Ok(())
    }
}

#[derive(FromRow)]
struct RoleBuilder {
    uuid: String,
    guild_uuid: String,
    name: String,
    color: i64,
    position: i32,
    permissions: i64,
}

impl RoleBuilder {
    fn build(&self) -> Role {
        Role {
            uuid: Uuid::from_str(&self.uuid).unwrap(),
            guild_uuid: Uuid::from_str(&self.guild_uuid).unwrap(),
            name: self.name.clone(),
            color: self.color,
            position: self.position,
            permissions: self.permissions,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Role {
    uuid: Uuid,
    guild_uuid: Uuid,
    name: String,
    color: i64,
    position: i32,
    permissions: i64,
}

impl Role {
    pub async fn fetch_all(
        pool: &Pool<Postgres>,
        guild_uuid: Uuid,
    ) -> Result<Vec<Self>, HttpResponse> {
        let role_builders_result = sqlx::query_as(&format!("SELECT (uuid, guild_uuid, name, color, position, permissions) FROM roles WHERE guild_uuid = '{}'", guild_uuid))
            .fetch_all(pool)
            .await;

        if let Err(error) = role_builders_result {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        let role_builders: Vec<RoleBuilder> = role_builders_result.unwrap();

        Ok(role_builders.iter().map(|b| b.build()).collect())
    }

    pub async fn fetch_one(
        pool: &Pool<Postgres>,
        role_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
        let row = sqlx::query_as(&format!("SELECT (name, color, position, permissions) FROM roles WHERE guild_uuid = '{}' AND uuid = '{}'", guild_uuid, role_uuid))
            .fetch_one(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        let (name, color, position, permissions) = row.unwrap();

        Ok(Role {
            uuid: role_uuid,
            guild_uuid,
            name,
            color,
            position,
            permissions,
        })
    }

    pub async fn new(
        pool: &Pool<Postgres>,
        guild_uuid: Uuid,
        name: String,
    ) -> Result<Self, HttpResponse> {
        let role_uuid = Uuid::now_v7();

        let row = sqlx::query(&format!(
            "INSERT INTO channels (uuid, guild_uuid, name, position) VALUES ('{}', '{}', $1, $2)",
            role_uuid, guild_uuid
        ))
        .bind(&name)
        .bind(0)
        .execute(pool)
        .await;

        if let Err(error) = row {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish());
        }

        let role = Self {
            uuid: role_uuid,
            guild_uuid,
            name,
            color: 16777215,
            position: 0,
            permissions: 0,
        };

        Ok(role)
    }
}

pub struct Member {
    pub uuid: Uuid,
    pub nickname: Option<String>,
    pub user_uuid: Uuid,
    pub guild_uuid: Uuid,
}

impl Member {
    async fn count(pool: &Pool<Postgres>, guild_uuid: Uuid) -> Result<i64, HttpResponse> {
        let member_count = sqlx::query_scalar(&format!(
            "SELECT COUNT(uuid) FROM guild_members WHERE guild_uuid = '{}'",
            guild_uuid
        ))
        .fetch_one(pool)
        .await;

        if let Err(error) = member_count {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(member_count.unwrap())
    }

    pub async fn fetch_one(
        pool: &Pool<Postgres>,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
        let row = sqlx::query_as(&format!("SELECT CAST(uuid AS VARCHAR), nickname FROM guild_members WHERE guild_uuid = '{}' AND user_uuid = '{}'", guild_uuid, user_uuid))
            .fetch_one(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        let (uuid, nickname): (String, Option<String>) = row.unwrap();

        Ok(Self {
            uuid: Uuid::from_str(&uuid).unwrap(),
            nickname,
            user_uuid,
            guild_uuid,
        })
    }

    pub async fn new(
        pool: &Pool<Postgres>,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, HttpResponse> {
        let member_uuid = Uuid::now_v7();

        let row = sqlx::query(&format!(
            "INSERT INTO guild_members uuid, guild_uuid, user_uuid VALUES ('{}', '{}', '{}')",
            member_uuid, guild_uuid, user_uuid
        ))
        .execute(pool)
        .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(Self {
            uuid: member_uuid,
            nickname: None,
            user_uuid,
            guild_uuid,
        })
    }
}

#[derive(FromRow)]
struct MessageBuilder {
    uuid: String,
    channel_uuid: String,
    user_uuid: String,
    message: String,
}

impl MessageBuilder {
    fn build(&self) -> Message {
        Message {
            uuid: Uuid::from_str(&self.uuid).unwrap(),
            channel_uuid: Uuid::from_str(&self.channel_uuid).unwrap(),
            user_uuid: Uuid::from_str(&self.user_uuid).unwrap(),
            message: self.message.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct Message {
    uuid: Uuid,
    channel_uuid: Uuid,
    user_uuid: Uuid,
    message: String,
}

#[derive(FromRow)]
pub struct InviteBuilder {
    id: String,
    user_uuid: String,
    guild_uuid: String,
}

impl InviteBuilder {
    fn build(&self) -> Invite {
        Invite {
            id: self.id.clone(),
            user_uuid: Uuid::from_str(&self.user_uuid).unwrap(),
            guild_uuid: Uuid::from_str(&self.guild_uuid).unwrap(),
        }
    }
}

/// Server invite struct
#[derive(Serialize)]
pub struct Invite {
    /// case-sensitive alphanumeric string with a fixed length of 8 characters, can be up to 32 characters for custom invites
    id: String,
    /// User that created the invite
    user_uuid: Uuid,
    /// UUID of the guild that the invite belongs to
    pub guild_uuid: Uuid,
}

impl Invite {
    pub async fn fetch_one(pool: &Pool<Postgres>, invite_id: String) -> Result<Self, HttpResponse> {
        let invite: Result<InviteBuilder, sqlx::Error> =
            sqlx::query_as("SELECT id, user_uuid, guild_uuid FROM invites WHERE id = $1")
                .bind(invite_id)
                .fetch_one(pool)
                .await;

        if let Err(error) = invite {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish());
        }

        Ok(invite.unwrap().build())
    }
}
