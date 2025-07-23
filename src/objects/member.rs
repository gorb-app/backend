use diesel::{
    ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable, SelectableHelper,
    insert_into,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState, Conn,
    error::Error,
    objects::{Me, Permissions, Role, GuildBan},
    schema::guild_bans,
    schema::guild_members,
};


use super::{User, load_or_empty};

#[derive(Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = guild_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MemberBuilder {
    pub uuid: Uuid,
    pub nickname: Option<String>,
    pub user_uuid: Uuid,
    pub guild_uuid: Uuid,
    pub is_owner: bool,
}

impl MemberBuilder {
    pub async fn build(&self, app_state: &AppState, me: Option<&Me>) -> Result<Member, Error> {
        let user;

        if let Some(me) = me {
            user = User::fetch_one_with_friendship(app_state, me, self.user_uuid).await?;
        } else {
            user = User::fetch_one(app_state, self.user_uuid).await?;
        }

        Ok(Member {
            uuid: self.uuid,
            nickname: self.nickname.clone(),
            user_uuid: self.user_uuid,
            guild_uuid: self.guild_uuid,
            is_owner: self.is_owner,
            user,
        })
    }

    pub async fn check_permission(
        &self,
        app_state: &AppState,
        permission: Permissions,
    ) -> Result<(), Error> {
        if !self.is_owner {
            let roles = Role::fetch_from_member(app_state, self.uuid).await?;
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
    pub user_uuid: Uuid,
    pub guild_uuid: Uuid,
    pub is_owner: bool,
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
        app_state: &AppState,
        me: &Me,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, Error> {
        let mut conn = app_state.pool.get().await?;

        use guild_members::dsl;
        let member: MemberBuilder = dsl::guild_members
            .filter(dsl::user_uuid.eq(user_uuid))
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .select(MemberBuilder::as_select())
            .get_result(&mut conn)
            .await?;

        member.build(app_state, Some(me)).await
    }

    pub async fn fetch_one_with_member(
        app_state: &AppState,
        me: Option<&Me>,
        uuid: Uuid,
    ) -> Result<Self, Error> {
        let mut conn = app_state.pool.get().await?;

        use guild_members::dsl;
        let member: MemberBuilder = dsl::guild_members
            .filter(dsl::uuid.eq(uuid))
            .select(MemberBuilder::as_select())
            .get_result(&mut conn)
            .await?;

        member.build(app_state, me).await
    }

    pub async fn fetch_all(
        app_state: &AppState,
        me: &Me,
        guild_uuid: Uuid,
    ) -> Result<Vec<Self>, Error> {
        let mut conn = app_state.pool.get().await?;

        use guild_members::dsl;
        let member_builders: Vec<MemberBuilder> = load_or_empty(
            dsl::guild_members
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .select(MemberBuilder::as_select())
                .load(&mut conn)
                .await,
        )?;

        let mut members = vec![];

        for builder in member_builders {
            members.push(builder.build(app_state, Some(me)).await?);
        }

        Ok(members)
    }

    pub async fn new(
        app_state: &AppState,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, Error> {
        let mut conn = app_state.pool.get().await?;

        use guild_bans::dsl;
        let banned = dsl::guild_bans
            .filter(guild_bans::guild_uuid.eq(guild_uuid))
            .filter(guild_bans::user_uuid.eq(user_uuid))
            .execute(&mut conn)
            .await;
        match banned {
            Ok(_) => Err(Error::Forbidden("User banned".to_string())),
            Err(diesel::result::Error::NotFound) => Ok(()),
            Err(e) => Err(e.into()),
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
            .execute(&mut conn)
            .await?;

        member.build(app_state, None).await
    }

    pub async fn delete(self, conn: &mut Conn) -> Result<(), Error> {
        diesel::delete(guild_members::table)
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

}
