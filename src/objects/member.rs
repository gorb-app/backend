use diesel::{
    Associations, BoolExpressionMethods, ExpressionMethods, Identifiable, Insertable, JoinOnDsl,
    QueryDsl, Queryable, Selectable, SelectableHelper, define_sql_function, delete, insert_into,
    sql_types::{Nullable, VarChar},
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Conn,
    error::Error,
    objects::PaginationRequest,
    schema::{friends, guild_bans, guild_members, users},
};

use super::{
    Friend, Guild, GuildBan, Me, Pagination, Permissions, Role, User, load_or_empty,
    user::UserBuilder,
};

define_sql_function! { fn coalesce(x: Nullable<VarChar>, y: Nullable<VarChar>, z: VarChar) -> Text; }

#[derive(Serialize, Queryable, Identifiable, Selectable, Insertable, Associations)]
#[diesel(table_name = guild_members)]
#[diesel(belongs_to(UserBuilder, foreign_key = user_uuid))]
#[diesel(belongs_to(Guild, foreign_key = guild_uuid))]
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

    async fn build_with_parts(
        &self,
        conn: &mut Conn,
        cache_pool: &redis::Client,
        user_builder: UserBuilder,
        friend: Option<Friend>,
    ) -> Result<Member, Error> {
        let mut user = user_builder.build();

        if let Some(friend) = friend {
            user.friends_since = Some(friend.accepted_at);
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

#[derive(Serialize, Deserialize, Clone)]
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
        me: Option<&Me>,
        user_uuid: Uuid,
        guild_uuid: Uuid,
    ) -> Result<Self, Error> {
        let member: MemberBuilder;
        let user: UserBuilder;
        let friend: Option<Friend>;
        use friends::dsl as fdsl;
        use guild_members::dsl;
        if let Some(me) = me {
            (member, user, friend) = dsl::guild_members
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .filter(dsl::user_uuid.eq(user_uuid))
                .inner_join(users::table)
                .left_join(
                    fdsl::friends.on(fdsl::uuid1
                        .eq(me.uuid)
                        .and(fdsl::uuid2.eq(users::uuid))
                        .or(fdsl::uuid2.eq(me.uuid).and(fdsl::uuid1.eq(users::uuid)))),
                )
                .select((
                    MemberBuilder::as_select(),
                    UserBuilder::as_select(),
                    Option::<Friend>::as_select(),
                ))
                .get_result(conn)
                .await?;
        } else {
            (member, user) = dsl::guild_members
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .filter(dsl::user_uuid.eq(user_uuid))
                .inner_join(users::table)
                .select((MemberBuilder::as_select(), UserBuilder::as_select()))
                .get_result(conn)
                .await?;

            friend = None;
        }

        member
            .build_with_parts(conn, cache_pool, user, friend)
            .await
    }

    pub async fn fetch_one_with_uuid(
        conn: &mut Conn,
        cache_pool: &redis::Client,
        me: Option<&Me>,
        uuid: Uuid,
    ) -> Result<Self, Error> {
        let member: MemberBuilder;
        let user: UserBuilder;
        let friend: Option<Friend>;
        use friends::dsl as fdsl;
        use guild_members::dsl;
        if let Some(me) = me {
            (member, user, friend) = dsl::guild_members
                .filter(dsl::uuid.eq(uuid))
                .inner_join(users::table)
                .left_join(
                    fdsl::friends.on(fdsl::uuid1
                        .eq(me.uuid)
                        .and(fdsl::uuid2.eq(users::uuid))
                        .or(fdsl::uuid2.eq(me.uuid).and(fdsl::uuid1.eq(users::uuid)))),
                )
                .select((
                    MemberBuilder::as_select(),
                    UserBuilder::as_select(),
                    Option::<Friend>::as_select(),
                ))
                .get_result(conn)
                .await?;
        } else {
            (member, user) = dsl::guild_members
                .filter(dsl::uuid.eq(uuid))
                .inner_join(users::table)
                .select((MemberBuilder::as_select(), UserBuilder::as_select()))
                .get_result(conn)
                .await?;

            friend = None;
        }

        member
            .build_with_parts(conn, cache_pool, user, friend)
            .await
    }

    pub async fn fetch_page(
        conn: &mut Conn,
        cache_pool: &redis::Client,
        me: &Me,
        guild_uuid: Uuid,
        pagination: PaginationRequest,
    ) -> Result<Pagination<Self>, Error> {
        let per_page = pagination.per_page.unwrap_or(50);
        let page_multiplier: i64 = ((pagination.page - 1) * per_page).into();

        if !(10..=100).contains(&per_page) {
            return Err(Error::BadRequest(
                "Invalid amount per page requested".to_string(),
            ));
        }

        use friends::dsl as fdsl;
        use guild_members::dsl;
        let member_builders: Vec<(MemberBuilder, UserBuilder, Option<Friend>)> = load_or_empty(
            dsl::guild_members
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .inner_join(users::table)
                .left_join(
                    fdsl::friends.on(fdsl::uuid1
                        .eq(me.uuid)
                        .and(fdsl::uuid2.eq(users::uuid))
                        .or(fdsl::uuid2.eq(me.uuid).and(fdsl::uuid1.eq(users::uuid)))),
                )
                .limit(per_page.into())
                .offset(page_multiplier)
                .order_by(coalesce(
                    dsl::nickname,
                    users::display_name,
                    users::username,
                ))
                .select((
                    MemberBuilder::as_select(),
                    UserBuilder::as_select(),
                    Option::<Friend>::as_select(),
                ))
                .load(conn)
                .await,
        )?;

        let pages = Member::count(conn, guild_uuid).await? as f32 / per_page as f32;

        let mut members = Pagination::<Member> {
            objects: Vec::with_capacity(member_builders.len()),
            amount: member_builders.len() as i32,
            pages: pages.ceil() as i32,
            page: pagination.page,
        };

        for (member, user, friend) in member_builders {
            members.objects.push(
                member
                    .build_with_parts(conn, cache_pool, user, friend)
                    .await?,
            );
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
