use diesel::query_dsl::BelongingToDsl;
use diesel::{
    Associations, ExpressionMethods, Identifiable, Insertable, QueryDsl, Queryable, Selectable,
    SelectableHelper, insert_into, update,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Conn,
    error::Error,
    schema::{role_members, roles},
    utils::{CacheFns, order_by_is_above},
};

use super::{HasIsAbove, HasUuid, load_or_empty, member::MemberBuilder};

#[derive(Deserialize, Serialize, Clone, Identifiable, Queryable, Selectable, Insertable)]
#[diesel(table_name = roles)]
#[diesel(primary_key(uuid))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Role {
    uuid: Uuid,
    guild_uuid: Uuid,
    name: String,
    color: i32,
    is_above: Option<Uuid>,
    pub permissions: i64,
}

#[derive(Serialize, Clone, Identifiable, Queryable, Selectable, Insertable, Associations)]
#[diesel(table_name = role_members)]
#[diesel(belongs_to(MemberBuilder, foreign_key = member_uuid))]
#[diesel(belongs_to(Role, foreign_key = role_uuid))]
#[diesel(primary_key(role_uuid, member_uuid))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RoleMember {
    role_uuid: Uuid,
    member_uuid: Uuid,
}

impl HasUuid for Role {
    fn uuid(&self) -> &Uuid {
        self.uuid.as_ref()
    }
}

impl HasIsAbove for Role {
    fn is_above(&self) -> Option<&Uuid> {
        self.is_above.as_ref()
    }
}

impl Role {
    pub async fn fetch_all(conn: &mut Conn, guild_uuid: Uuid) -> Result<Vec<Self>, Error> {
        use roles::dsl;
        let roles: Vec<Role> = load_or_empty(
            dsl::roles
                .filter(dsl::guild_uuid.eq(guild_uuid))
                .select(Role::as_select())
                .load(conn)
                .await,
        )?;

        Ok(roles)
    }

    pub async fn fetch_from_member(
        conn: &mut Conn,
        cache_pool: &redis::Client,
        member: &MemberBuilder,
    ) -> Result<Vec<Self>, Error> {
        if let Ok(roles) = cache_pool
            .get_cache_key(format!("{}_roles", member.uuid))
            .await
        {
            return Ok(roles);
        }

        let roles: Vec<Role> = load_or_empty(
            RoleMember::belonging_to(member)
                .inner_join(roles::table)
                .select(Role::as_select())
                .load(conn)
                .await,
        )?;

        cache_pool
            .set_cache_key(format!("{}_roles", member.uuid), roles.clone(), 300)
            .await?;

        Ok(roles)
    }

    pub async fn fetch_one(conn: &mut Conn, role_uuid: Uuid) -> Result<Self, Error> {
        use roles::dsl;
        let role: Role = dsl::roles
            .filter(dsl::uuid.eq(role_uuid))
            .select(Role::as_select())
            .get_result(conn)
            .await?;

        Ok(role)
    }

    pub async fn fetch_permissions(&self) -> Vec<Permissions> {
        Permissions::fetch_permissions(self.permissions)
    }

    pub async fn new(conn: &mut Conn, guild_uuid: Uuid, name: String) -> Result<Self, Error> {
        let role_uuid = Uuid::now_v7();

        let roles = Self::fetch_all(conn, guild_uuid).await?;

        let roles_ordered = order_by_is_above(roles).await?;

        let last_role = roles_ordered.last();

        let new_role = Role {
            uuid: role_uuid,
            guild_uuid,
            name,
            color: 16777215,
            is_above: None,
            permissions: 0,
        };

        insert_into(roles::table)
            .values(new_role.clone())
            .execute(conn)
            .await?;

        if let Some(old_last_role) = last_role {
            use roles::dsl;
            update(roles::table)
                .filter(dsl::uuid.eq(old_last_role.uuid))
                .set(dsl::is_above.eq(new_role.uuid))
                .execute(conn)
                .await?;
        }

        Ok(new_role)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Permissions {
    /// Lets users send messages in the guild or channel
    SendMessage = 1,
    /// Lets users create, delete and edit channels and categories or a singular channel depending on permission context
    ManageChannel = 2,
    /// Lets users manage roles in the guild
    ManageRole = 4,
    /// Lets users create invites in the guild
    CreateInvite = 8,
    /// Lets users manage invites in the guild
    ManageInvite = 16,
    /// Lets users change guild settings
    ManageGuild = 32,
    /// Lets users change member settings (nickname, etc)
    ManageMember = 64,
    /// Lets users ban members
    BanMember = 128,
    /// Lets users kick members
    KickMember = 256,
}

impl Permissions {
    pub fn fetch_permissions(permissions: i64) -> Vec<Self> {
        let all_perms = vec![
            Self::SendMessage,
            Self::ManageChannel,
            Self::ManageRole,
            Self::CreateInvite,
            Self::ManageInvite,
            Self::ManageGuild,
            Self::ManageMember,
            Self::BanMember,
            Self::KickMember,
        ];

        all_perms
            .into_iter()
            .filter(|p| permissions & (*p as i64) != 0)
            .collect()
    }
}
