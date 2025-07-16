use axum::body::Bytes;
use diesel::{
    ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper, delete, insert_into,
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
    objects::{Friend, FriendRequest, User},
    schema::{friend_requests, friends, guild_members, guilds, users},
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
        app_state: &AppState,
        cdn_url: Url,
        avatar: Bytes,
    ) -> Result<(), Error> {
        let avatar_clone = avatar.clone();
        let image_type = task::spawn_blocking(move || image_check(avatar_clone)).await??;

        let mut conn = app_state.pool.get().await?;

        if let Some(avatar) = &self.avatar {
            let avatar_url: Url = avatar.parse()?;

            let relative_url = avatar_url.path().trim_start_matches('/');

            app_state.bunny_storage.delete(relative_url).await?;
        }

        let path = format!("avatar/{}/{}.{}", self.uuid, Uuid::now_v7(), image_type);

        app_state.bunny_storage.upload(path.clone(), avatar).await?;

        let avatar_url = cdn_url.join(&path)?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::avatar.eq(avatar_url.as_str()))
            .execute(&mut conn)
            .await?;

        if app_state.get_cache_key(self.uuid.to_string()).await.is_ok() {
            app_state.del_cache_key(self.uuid.to_string()).await?
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

    pub async fn set_username(
        &mut self,
        app_state: &AppState,
        new_username: String,
    ) -> Result<(), Error> {
        if !USERNAME_REGEX.is_match(&new_username)
            || new_username.len() < 3
            || new_username.len() > 32
        {
            return Err(Error::BadRequest("Invalid username".to_string()));
        }

        let mut conn = app_state.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::username.eq(new_username.as_str()))
            .execute(&mut conn)
            .await?;

        if app_state.get_cache_key(self.uuid.to_string()).await.is_ok() {
            app_state.del_cache_key(self.uuid.to_string()).await?
        }

        self.username = new_username;

        Ok(())
    }

    pub async fn set_display_name(
        &mut self,
        app_state: &AppState,
        new_display_name: String,
    ) -> Result<(), Error> {
        let mut conn = app_state.pool.get().await?;

        let new_display_name_option = if new_display_name.is_empty() {
            None
        } else {
            Some(new_display_name)
        };

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set(dsl::display_name.eq(&new_display_name_option))
            .execute(&mut conn)
            .await?;

        if app_state.get_cache_key(self.uuid.to_string()).await.is_ok() {
            app_state.del_cache_key(self.uuid.to_string()).await?
        }

        self.display_name = new_display_name_option;

        Ok(())
    }

    pub async fn set_email(
        &mut self,
        app_state: &AppState,
        new_email: String,
    ) -> Result<(), Error> {
        if !EMAIL_REGEX.is_match(&new_email) {
            return Err(Error::BadRequest("Invalid username".to_string()));
        }

        let mut conn = app_state.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set((
                dsl::email.eq(new_email.as_str()),
                dsl::email_verified.eq(false),
            ))
            .execute(&mut conn)
            .await?;

        if app_state.get_cache_key(self.uuid.to_string()).await.is_ok() {
            app_state.del_cache_key(self.uuid.to_string()).await?
        }

        self.email = new_email;

        Ok(())
    }

    pub async fn set_pronouns(
        &mut self,
        app_state: &AppState,
        new_pronouns: String,
    ) -> Result<(), Error> {
        let mut conn = app_state.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set((dsl::pronouns.eq(new_pronouns.as_str()),))
            .execute(&mut conn)
            .await?;

        if app_state.get_cache_key(self.uuid.to_string()).await.is_ok() {
            app_state.del_cache_key(self.uuid.to_string()).await?
        }

        Ok(())
    }

    pub async fn set_about(
        &mut self,
        app_state: &AppState,
        new_about: String,
    ) -> Result<(), Error> {
        let mut conn = app_state.pool.get().await?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.uuid))
            .set((dsl::about.eq(new_about.as_str()),))
            .execute(&mut conn)
            .await?;

        if app_state.get_cache_key(self.uuid.to_string()).await.is_ok() {
            app_state.del_cache_key(self.uuid.to_string()).await?
        }

        Ok(())
    }

    pub async fn friends_with(
        &self,
        conn: &mut Conn,
        user_uuid: Uuid,
    ) -> Result<Option<Friend>, Error> {
        use friends::dsl;

        let friends: Vec<Friend> = if self.uuid < user_uuid {
            load_or_empty(
                dsl::friends
                    .filter(dsl::uuid1.eq(self.uuid))
                    .filter(dsl::uuid2.eq(user_uuid))
                    .load(conn)
                    .await,
            )?
        } else {
            load_or_empty(
                dsl::friends
                    .filter(dsl::uuid1.eq(user_uuid))
                    .filter(dsl::uuid2.eq(self.uuid))
                    .load(conn)
                    .await,
            )?
        };

        if friends.is_empty() {
            return Ok(None);
        }

        Ok(Some(friends[0].clone()))
    }

    pub async fn add_friend(&self, conn: &mut Conn, user_uuid: Uuid) -> Result<(), Error> {
        if self.friends_with(conn, user_uuid).await?.is_some() {
            // TODO: Check if another error should be used
            return Err(Error::BadRequest("Already friends with user".to_string()));
        }

        use friend_requests::dsl;

        let friend_request: Vec<FriendRequest> = load_or_empty(
            dsl::friend_requests
                .filter(dsl::sender.eq(user_uuid))
                .filter(dsl::receiver.eq(self.uuid))
                .load(conn)
                .await,
        )?;

        #[allow(clippy::get_first)]
        if let Some(friend_request) = friend_request.get(0) {
            use friends::dsl;

            if self.uuid < user_uuid {
                insert_into(friends::table)
                    .values((dsl::uuid1.eq(self.uuid), dsl::uuid2.eq(user_uuid)))
                    .execute(conn)
                    .await?;
            } else {
                insert_into(friends::table)
                    .values((dsl::uuid1.eq(user_uuid), dsl::uuid2.eq(self.uuid)))
                    .execute(conn)
                    .await?;
            }

            use friend_requests::dsl as frdsl;

            delete(friend_requests::table)
                .filter(frdsl::sender.eq(friend_request.sender))
                .filter(frdsl::receiver.eq(friend_request.receiver))
                .execute(conn)
                .await?;

            Ok(())
        } else {
            use friend_requests::dsl;

            insert_into(friend_requests::table)
                .values((dsl::sender.eq(self.uuid), dsl::receiver.eq(user_uuid)))
                .execute(conn)
                .await?;

            Ok(())
        }
    }

    pub async fn remove_friend(&self, conn: &mut Conn, user_uuid: Uuid) -> Result<(), Error> {
        if self.friends_with(conn, user_uuid).await?.is_none() {
            // TODO: Check if another error should be used
            return Err(Error::BadRequest("Not friends with user".to_string()));
        }

        use friends::dsl;

        if self.uuid < user_uuid {
            delete(friends::table)
                .filter(dsl::uuid1.eq(self.uuid))
                .filter(dsl::uuid2.eq(user_uuid))
                .execute(conn)
                .await?;
        } else {
            delete(friends::table)
                .filter(dsl::uuid1.eq(user_uuid))
                .filter(dsl::uuid2.eq(self.uuid))
                .execute(conn)
                .await?;
        }

        Ok(())
    }

    pub async fn get_friends(&self, app_state: &AppState) -> Result<Vec<User>, Error> {
        use friends::dsl;

        let mut conn = app_state.pool.get().await?;

        let friends1 = load_or_empty(
            dsl::friends
                .filter(dsl::uuid1.eq(self.uuid))
                .select(Friend::as_select())
                .load(&mut conn)
                .await,
        )?;

        let friends2 = load_or_empty(
            dsl::friends
                .filter(dsl::uuid2.eq(self.uuid))
                .select(Friend::as_select())
                .load(&mut conn)
                .await,
        )?;

        let friend_futures = friends1.iter().map(async move |friend| {
            User::fetch_one_with_friendship(app_state, self, friend.uuid2).await
        });

        let mut friends = futures::future::try_join_all(friend_futures).await?;

        let friend_futures = friends2.iter().map(async move |friend| {
            User::fetch_one_with_friendship(app_state, self, friend.uuid1).await
        });

        friends.append(&mut futures::future::try_join_all(friend_futures).await?);

        Ok(friends)
    }

    /* TODO
    pub async fn get_friend_requests(&self, conn: &mut Conn) -> Result<Vec<FriendRequest>, Error> {
        use friend_requests::dsl;

        let friend_request: Vec<FriendRequest> = load_or_empty(
            dsl::friend_requests
                .filter(dsl::receiver.eq(self.uuid))
                .load(conn)
                .await
        )?;

        Ok()
    }

    pub async fn delete_friend_request(&self, conn: &mut Conn, user_uuid: Uuid) -> Result<Vec<FriendRequest>, Error> {
        use friend_requests::dsl;

        let friend_request: Vec<FriendRequest> = load_or_empty(
            dsl::friend_requests
                .filter(dsl::sender.eq(user_uuid))
                .filter(dsl::receiver.eq(self.uuid))
                .load(conn)
                .await
        )?;

        Ok()
    }
    */
}
