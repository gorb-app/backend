use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Conn, Data, error::Error, objects::Me, schema::users};

use super::load_or_empty;

#[derive(Deserialize, Serialize, Clone, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserBuilder {
    uuid: Uuid,
    username: String,
    display_name: Option<String>,
    avatar: Option<String>,
    pronouns: Option<String>,
    about: Option<String>,
}

impl UserBuilder {
    fn build(self) -> User {
        User {
            uuid: self.uuid,
            username: self.username,
            display_name: self.display_name,
            avatar: self.avatar,
            pronouns: self.pronouns,
            about: self.about,
            friends_since: None,
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct User {
    uuid: Uuid,
    username: String,
    display_name: Option<String>,
    avatar: Option<String>,
    pronouns: Option<String>,
    about: Option<String>,
    friends_since: Option<DateTime<Utc>>,
}

impl User {
    pub async fn fetch_one(data: &Data, user_uuid: Uuid) -> Result<Self, Error> {
        let mut conn = data.pool.get().await?;

        if let Ok(cache_hit) = data.get_cache_key(user_uuid.to_string()).await {
            return Ok(serde_json::from_str(&cache_hit)?);
        }

        use users::dsl;
        let user_builder: UserBuilder = dsl::users
            .filter(dsl::uuid.eq(user_uuid))
            .select(UserBuilder::as_select())
            .get_result(&mut conn)
            .await?;

        let user = user_builder.build();

        data.set_cache_key(user_uuid.to_string(), user.clone(), 1800)
            .await?;

        Ok(user)
    }

    pub async fn fetch_one_with_friendship(
        data: &Data,
        me: &Me,
        user_uuid: Uuid,
    ) -> Result<Self, Error> {
        let mut conn = data.pool.get().await?;

        let mut user = Self::fetch_one(data, user_uuid).await?;

        if let Some(friend) = me.friends_with(&mut conn, user_uuid).await? {
            user.friends_since = Some(friend.accepted_at);
        }

        Ok(user)
    }

    pub async fn fetch_amount(
        conn: &mut Conn,
        offset: i64,
        amount: i64,
    ) -> Result<Vec<Self>, Error> {
        use users::dsl;
        let user_builders: Vec<UserBuilder> = load_or_empty(
            dsl::users
                .limit(amount)
                .offset(offset)
                .select(UserBuilder::as_select())
                .load(conn)
                .await,
        )?;

        let users: Vec<User> = user_builders.iter().map(|u| u.clone().build()).collect();

        Ok(users)
    }
}
