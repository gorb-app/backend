use std::str::FromStr;

use serde::Serialize;
use sqlx::{prelude::FromRow, Pool, Postgres};
use uuid::Uuid;
use actix_web::HttpResponse;
use log::error;

use crate::Data;

#[derive(Serialize, Clone)]
pub struct Channel {
    pub uuid: Uuid,
    pub guild_uuid: Uuid,
    name: String,
    description: Option<String>,
    pub permissions: Vec<ChannelPermission>
}

#[derive(Serialize, Clone, FromRow)]
struct ChannelPermissionBuilder {
    role_uuid: String,
    permissions: i32
}

impl ChannelPermissionBuilder {
    fn build(&self) -> ChannelPermission {
        ChannelPermission {
            role_uuid: Uuid::from_str(&self.role_uuid).unwrap(),
            permissions: self.permissions,
        }
    }
}

#[derive(Serialize, Clone, FromRow)]
pub struct ChannelPermission {
    pub role_uuid: Uuid,
    pub permissions: i32
}

impl Channel {
    pub async fn fetch_all(pool: &Pool<Postgres>, guild_uuid: Uuid) -> Result<Vec<Self>, HttpResponse> {
        let row = sqlx::query_as(&format!("SELECT CAST(uuid AS VARCHAR), name, description FROM channels WHERE guild_uuid = '{}'", guild_uuid))
            .fetch_all(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
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

        Ok(channels?)
    }

    pub async fn fetch_one(pool: &Pool<Postgres>, guild_uuid: Uuid, channel_uuid: Uuid) -> Result<Self, HttpResponse> {
        let row = sqlx::query_as(&format!("SELECT name, description FROM channels WHERE guild_uuid = '{}' AND uuid = '{}'", guild_uuid, channel_uuid))
            .fetch_one(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        let (name, description): (String, Option<String>) = row.unwrap();

        let row = sqlx::query_as(&format!("SELECT CAST(role_uuid AS VARCHAR), permissions FROM channel_permissions WHERE channel_uuid = '{}'", channel_uuid))
            .fetch_all(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        let channel_permission_builders: Vec<ChannelPermissionBuilder> = row.unwrap();

        Ok(Self {
            uuid: channel_uuid,
            guild_uuid,
            name,
            description,
            permissions: channel_permission_builders.iter().map(|b| b.build()).collect(),
        })
    }

    pub async fn new(data: actix_web::web::Data<Data>, guild_uuid: Uuid, name: String, description: Option<String>) -> Result<Self, HttpResponse> {
        let channel_uuid = Uuid::now_v7();

        let row = sqlx::query(&format!("INSERT INTO channels (uuid, guild_uuid, name, description) VALUES ('{}', '{}', $1, $2)", channel_uuid, guild_uuid))
            .bind(&name)
            .bind(&description)
            .execute(&data.pool)
            .await;
    
        if let Err(error) = row {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish())
        }

        let channel = Self {
            uuid: channel_uuid,
            guild_uuid,
            name,
            description,
            permissions: vec![],
        };

        let cache_result = data.set_cache_key(channel_uuid.to_string(), channel.clone(), 1800).await;

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

        all_perms.into_iter()
            .filter(|p| permissions & (*p as i64) != 0)
            .collect()
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
    pub async fn fetch_one(pool: &Pool<Postgres>, guild_uuid: Uuid) -> Result<Self, HttpResponse> {
        let row = sqlx::query_as(&format!("SELECT CAST(owner_uuid AS VARCHAR), name, description FROM guilds WHERE uuid = '{}'", guild_uuid))
            .fetch_one(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        let (owner_uuid_raw, name, description): (String, String, Option<String>) = row.unwrap();

        let owner_uuid = Uuid::from_str(&owner_uuid_raw).unwrap();

        let member_count = Member::count(pool, guild_uuid).await?;

        let roles = Role::fetch_all(pool, guild_uuid).await?;

        Ok(Self {
            uuid: guild_uuid,
            name,
            description,
            // FIXME: This isnt supposed to be bogus
            icon: String::from("bogus"),
            owner_uuid,
            roles,
            member_count,
        })
    }

    pub async fn new(pool: &Pool<Postgres>, name: String, description: Option<String>, owner_uuid: Uuid) -> Result<Self, HttpResponse> {
        let guild_uuid = Uuid::now_v7();

        let row = sqlx::query(&format!("INSERT INTO guilds (uuid, owner_uuid, name, description) VALUES ('{}', '{}', $1, $2)", guild_uuid, owner_uuid))
            .bind(&name)
            .bind(&description)
            .execute(pool)
            .await;
    
        if let Err(error) = row {
            error!("{}", error);
            return Err(HttpResponse::InternalServerError().finish())
        }
    
        let row = sqlx::query(&format!("INSERT INTO guild_members (uuid, guild_uuid, user_uuid) VALUES ('{}', '{}', '{}')", Uuid::now_v7(), guild_uuid, owner_uuid))
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

            return Err(HttpResponse::InternalServerError().finish())
        }

        Ok(Guild {
            uuid: guild_uuid,
            name,
            description,
            icon: "bogus".to_string(),
            owner_uuid,
            roles: vec![],
            member_count: 1
        })
    }
}

#[derive(Serialize, FromRow)]
pub struct Role {
    uuid: String,
    name: String,
    color: i64,
    position: i32,
    permissions: i64,
}

impl Role {
    pub async fn fetch_all(pool: &Pool<Postgres>, guild_uuid: Uuid) -> Result<Vec<Self>, HttpResponse> {
        let roles = sqlx::query_as(&format!("SELECT (uuid, name, color, position, permissions) FROM roles WHERE guild_uuid = '{}'", guild_uuid))
            .fetch_all(pool)
            .await;

        if let Err(error) = roles {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        Ok(roles.unwrap())
    }
}

pub struct Member {
    pub uuid: Uuid,
    pub nickname: String,
    pub user_uuid: Uuid,
    pub guild_uuid: Uuid,
}

impl Member {
    async fn count(pool: &Pool<Postgres>, guild_uuid: Uuid) -> Result<i64, HttpResponse> {
        let member_count = sqlx::query_scalar(&format!("SELECT COUNT(uuid) FROM guild_members WHERE guild_uuid = '{}'", guild_uuid))
            .fetch_one(pool)
            .await;

        if let Err(error) = member_count {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        Ok(member_count.unwrap())
    }

    pub async fn fetch_one(pool: &Pool<Postgres>, user_uuid: Uuid, guild_uuid: Uuid) -> Result<Self, HttpResponse> {
        let row = sqlx::query_as(&format!("SELECT CAST(uuid AS VARCHAR), nickname FROM guild_members WHERE guild_uuid = '{}' AND user_uuid = '{}'", guild_uuid, user_uuid))
            .fetch_one(pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        let (uuid, nickname): (String, String) = row.unwrap();

        Ok(Member {
            uuid: Uuid::from_str(&uuid).unwrap(),
            nickname,
            user_uuid,
            guild_uuid,
        })
    }
}
