use actix_web::web::BytesMut;
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper, update};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use tokio::task;
use url::Url;
use uuid::Uuid;

use crate::{
    Conn, Data,
    error::Error,
    schema::{guild_members, guilds, users},
    utils::{EMAIL_REGEX, USERNAME_REGEX, image_check},
};

use super::{Guild, guild::GuildBuilder, load_or_empty, member::MemberBuilder};

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Me {
    pub uuid: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    avatar: Option<String>,
    pronouns: Option<String>,
    about: Option<String>,
    pub email: String,
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
                .await,
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
        data: &Data,
        cdn_url: Url,
        avatar: BytesMut,
    ) -> Result<(), Error> {
        let avatar_clone = avatar.clone();
        let image_type = task::spawn_blocking(move || image_check(avatar_clone)).await??;

        let mut conn = data.pool.get().await?;

        if let Some(avatar) = &self.avatar {
            let avatar_url: Url = avatar.parse()?;

            let relative_url = avatar_url.path().trim_start_matches('/');

            data.bunny_storage.delete(relative_url).await?;
        }

        let path = format!("avatar/{}/avatar.{}", self.uuid, image_type);

        data.bunny_storage
            .upload(path.clone(), avatar.into())
            .await?;

        let avatar_url = cdn_url.join(&path)?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::avatar.eq(avatar_url.as_str()))
            .execute(&mut conn)
            .await?;

        if data.get_cache_key(self.uuid.to_string()).await.is_ok() {
            data.del_cache_key(self.uuid.to_string()).await?
        }

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

    pub async fn set_username(&mut self, data: &Data, new_username: String) -> Result<(), Error> {
        if !USERNAME_REGEX.is_match(&new_username) {
            return Err(Error::BadRequest("Invalid username".to_string()));
        }

        let mut conn = data.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::username.eq(new_username.as_str()))
            .execute(&mut conn)
            .await?;

        if data.get_cache_key(self.uuid.to_string()).await.is_ok() {
            data.del_cache_key(self.uuid.to_string()).await?
        }

        self.username = new_username;

        Ok(())
    }

    pub async fn set_display_name(
        &mut self,
        data: &Data,
        new_display_name: String,
    ) -> Result<(), Error> {
        let mut conn = data.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::display_name.eq(new_display_name.as_str()))
            .execute(&mut conn)
            .await?;

        if data.get_cache_key(self.uuid.to_string()).await.is_ok() {
            data.del_cache_key(self.uuid.to_string()).await?
        }

        self.display_name = Some(new_display_name);

        Ok(())
    }

    pub async fn set_email(&mut self, data: &Data, new_email: String) -> Result<(), Error> {
        if !EMAIL_REGEX.is_match(&new_email) {
            return Err(Error::BadRequest("Invalid username".to_string()));
        }

        let mut conn = data.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set((
                dsl::email.eq(new_email.as_str()),
                dsl::email_verified.eq(false),
            ))
            .execute(&mut conn)
            .await?;

        if data.get_cache_key(self.uuid.to_string()).await.is_ok() {
            data.del_cache_key(self.uuid.to_string()).await?
        }

        self.email = new_email;

        Ok(())
    }

    pub async fn set_pronouns(&mut self, data: &Data, new_pronouns: String) -> Result<(), Error> {
        let mut conn = data.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set((dsl::pronouns.eq(new_pronouns.as_str()),))
            .execute(&mut conn)
            .await?;

        if data.get_cache_key(self.uuid.to_string()).await.is_ok() {
            data.del_cache_key(self.uuid.to_string()).await?
        }

        Ok(())
    }

    pub async fn set_about(&mut self, data: &Data, new_about: String) -> Result<(), Error> {
        let mut conn = data.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set((dsl::about.eq(new_about.as_str()),))
            .execute(&mut conn)
            .await?;

        if data.get_cache_key(self.uuid.to_string()).await.is_ok() {
            data.del_cache_key(self.uuid.to_string()).await?
        }

        Ok(())
    }
}
