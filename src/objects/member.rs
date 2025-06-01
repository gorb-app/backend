use diesel::{
    ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable, SelectableHelper, insert_into,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Conn, Data, error::Error, schema::guild_members};

use super::{User, load_or_empty};

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
    pub async fn count(conn: &mut Conn, guild_uuid: Uuid) -> Result<i64, Error> {
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
