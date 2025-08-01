use axum::body::Bytes;
use diesel::{
    ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable, SelectableHelper, insert_into,
    update,
};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use tokio::task;
use url::Url;
use uuid::Uuid;

use crate::{
    AppState, Conn,
    error::Error,
    schema::{guild_members, guilds, invites},
    utils::image_check,
};

use super::{Invite, Member, Role, load_or_empty, member::MemberBuilder};

#[derive(Serialize, Queryable, Selectable, Insertable, Clone)]
#[diesel(table_name = guilds)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuildBuilder {
    uuid: Uuid,
    name: String,
    description: Option<String>,
    icon: Option<String>,
}

impl GuildBuilder {
    pub async fn build(self, conn: &mut Conn) -> Result<Guild, Error> {
        let member_count = Member::count(conn, self.uuid).await?;

        let roles = Role::fetch_all(conn, self.uuid).await?;

        Ok(Guild {
            uuid: self.uuid,
            name: self.name,
            description: self.description,
            icon: self.icon.and_then(|i| i.parse().ok()),
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
        conn: &mut Conn,
        offset: i64,
        amount: i64,
    ) -> Result<Vec<Self>, Error> {
        // Fetch guild data from database
        use guilds::dsl;
        let guild_builders: Vec<GuildBuilder> = load_or_empty(
            dsl::guilds
                .select(GuildBuilder::as_select())
                .order_by(dsl::uuid)
                .offset(offset)
                .limit(amount)
                .load(conn)
                .await,
        )?;

        let mut guilds = vec![];

        for builder in guild_builders {
            guilds.push(builder.build(conn).await?);
        }

        Ok(guilds)
    }

    pub async fn new(conn: &mut Conn, name: String, owner_uuid: Uuid) -> Result<Self, Error> {
        let guild_uuid = Uuid::now_v7();

        let guild_builder = GuildBuilder {
            uuid: guild_uuid,
            name: name.clone(),
            description: None,
            icon: None,
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
            is_owner: true,
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
        conn: &mut Conn,
        app_state: &AppState,
        icon: Bytes,
    ) -> Result<(), Error> {
        let icon_clone = icon.clone();
        let image_type = task::spawn_blocking(move || image_check(icon_clone)).await??;

        if let Some(icon) = &self.icon {
            let relative_url = icon.path().trim_start_matches('/');

            app_state.bunny_storage.delete(relative_url).await?;
        }

        let path = format!("icons/{}/{}.{}", self.uuid, Uuid::now_v7(), image_type);

        app_state.bunny_storage.upload(path.clone(), icon).await?;

        let icon_url = app_state.config.bunny.cdn_url.join(&path)?;

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
