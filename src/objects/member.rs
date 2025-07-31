use diesel::{
    ExpressionMethods, Identifiable, Insertable, QueryDsl, Queryable, Selectable, SelectableHelper,
    delete, insert_into,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Conn,
    error::Error,
    objects::{GuildBan, Me, Permissions, Role},
    schema::{guild_bans, guild_members},
};

use super::{User, load_or_empty};

#[derive(Serialize, Queryable, Identifiable, Selectable, Insertable)]
#[diesel(table_name = guild_members)]
#[diesel(primary_key(uuid))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MemberBuilder {
    pub uuid: Uuid,
    pub nickname: Option<String>,
    pub user_uuid: Uuid,
    pub guild_uuid: Uuid,
    pub is_owner: bool,
}

impl MemberBuilder {
    pub async fn build(
        &self,
        conn: &mut Conn,
        cache_pool: &redis::Client,
        me: Option<&Me>,
    ) -> Result<Member, Error> {
        let user;

        if let Some(me) = me {
            user = User::fetch_one_with_friendship(conn, cache_pool, me, self.user_uuid).await?;
        } else {
            user = User::fetch_one(conn, cache_pool, self.user_uuid).await?;
        }

        let roles = Role::fetch_from_member(conn, cache_pool, self).await?;

        Ok(Member {
            uuid: self.uuid,
            nickname: self.nickname.clone(),
            user_uuid: self.user_uuid,
            guild_uuid: self.guild_uuid,
            is_owner: self.is_owner,
            user,
            roles,
        })
    }

    pub async fn check_permission(
        &self,
        conn: &mut Conn,
        cache_pool: &redis::Client,
        permission: Permissions,
    ) -> Result<(), Error> {
        if !self.is_owner {
            let roles = Role::fetch_from_member(conn, cache_pool, self).await?;
            let allowed = roles.iter().any(|r| r.permissions & permission as i64 != 0);
            if !allowed {
                return Err(Error::Forbidden("Not allowed".to_string()));
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Member {
    pub uuid: Uuid,
    pub nickname: Option<String>,
    #[serde(skip)]
    pub user_uuid: Uuid,
    pub guild_uuid: Uuid,
    pub is_owner: bool,
    user: User,
    roles: Vec<Role>,
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
    ) -> Result<MemberBuilder, Error> {
        use guild_members::dsl;
        let member_builder = dsl::guild_members
            .filter(dsl::user_uuid.eq(user_uuid))
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .select(MemberBuilder::as_select())
            .get_result(conn)
            .await?;

        Ok(member_builder)
    }

    pub async fn fetch_one(
        conn: &mut Conn,
        cache_pool: &redis::Client,
        me: &Me,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, Error> {
        use guild_members::dsl;
        let member: MemberBuilder = dsl::guild_members
            .filter(dsl::user_uuid.eq(user_uuid))
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .select(MemberBuilder::as_select())
            .get_result(conn)
            .await?;

        member.build(conn, cache_pool, Some(me)).await
    }

    pub async fn fetch_one_with_member(
        conn: &mut Conn,
        cache_pool: &redis::Client,
        me: Option<&Me>,
        uuid: Uuid,
    ) -> Result<Self, Error> {
        use guild_members::dsl;
        let member: MemberBuilder = dsl::guild_members
            .filter(dsl::uuid.eq(uuid))
            .select(MemberBuilder::as_select())
            .get_result(conn)
            .await?;

        member.build(conn, cache_pool, me).await
    }

    pub async fn fetch_all(
        conn: &mut Conn,
        cache_pool: &redis::Client,
        me: &Me,
        guild_uuid: Uuid,
    ) -> Result<Vec<Self>, Error> {
        use guild_members::dsl;
        let member_builders: Vec<MemberBuilder> = load_or_empty(
            dsl::guild_members
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .select(MemberBuilder::as_select())
                .load(conn)
                .await,
        )?;

        let mut members = vec![];

        for builder in member_builders {
            members.push(builder.build(conn, cache_pool, Some(me)).await?);
        }

        Ok(members)
    }

    pub async fn new(
        conn: &mut Conn,
        cache_pool: &redis::Client,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, Error> {
        let banned = GuildBan::fetch_one(conn, guild_uuid, user_uuid).await;

        match banned {
            Ok(_) => Err(Error::Forbidden("User banned".to_string())),
            Err(Error::SqlError(diesel::result::Error::NotFound)) => Ok(()),
            Err(e) => Err(e),
        }?;

        let member_uuid = Uuid::now_v7();

        let member = MemberBuilder {
            uuid: member_uuid,
            guild_uuid,
            user_uuid,
            nickname: None,
            is_owner: false,
        };

        insert_into(guild_members::table)
            .values(&member)
            .execute(conn)
            .await?;

        member.build(conn, cache_pool, None).await
    }

    pub async fn delete(self, conn: &mut Conn) -> Result<(), Error> {
        if self.is_owner {
            return Err(Error::Forbidden("Can not kick owner".to_string()));
        }
        delete(guild_members::table)
            .filter(guild_members::uuid.eq(self.uuid))
            .execute(conn)
            .await?;

        Ok(())
    }

    pub async fn ban(self, conn: &mut Conn, reason: &String) -> Result<(), Error> {
        if self.is_owner {
            return Err(Error::Forbidden("Can not ban owner".to_string()));
        }

        use guild_bans::dsl;
        insert_into(guild_bans::table)
            .values((
                dsl::guild_uuid.eq(self.guild_uuid),
                dsl::user_uuid.eq(self.user_uuid),
                dsl::reason.eq(reason),
            ))
            .execute(conn)
            .await?;

        self.delete(conn).await?;

        Ok(())
    }

    pub fn to_builder(&self) -> MemberBuilder {
        MemberBuilder {
            uuid: self.uuid,
            nickname: self.nickname.clone(),
            user_uuid: self.user_uuid,
            guild_uuid: self.guild_uuid,
            is_owner: self.is_owner,
        }
    }
}
