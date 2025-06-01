use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Conn, Data, error::Error, schema::users};

use super::load_or_empty;

#[derive(Deserialize, Serialize, Clone, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    uuid: Uuid,
    username: String,
    display_name: Option<String>,
    avatar: Option<String>,
    pronouns: Option<String>,
    about: Option<String>,
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
