use diesel::{
    ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable, SelectableHelper, insert_into,
    update,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::Error, schema::{role_members, roles}, utils::order_by_is_above, Conn, Data};

use super::{HasIsAbove, HasUuid, load_or_empty};

#[derive(Deserialize, Serialize, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Role {
    uuid: Uuid,
    guild_uuid: Uuid,
    name: String,
    color: i32,
    is_above: Option<Uuid>,
    pub permissions: i64,
}

#[derive(Serialize, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = role_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RoleMember {
    role_uuid: Uuid,
    member_uuid: Uuid,
}

impl RoleMember {
    async fn fetch_role(&self, conn: &mut Conn) -> Result<Role, Error> {
        use roles::dsl;
        let role: Role = dsl::roles
            .filter(dsl::uuid.eq(self.role_uuid))
            .select(Role::as_select())
            .get_result(conn)
            .await?;

        Ok(role)
    }
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

    pub async fn fetch_from_member(data: &Data, member_uuid: Uuid) -> Result<Vec<Self>, Error> {
        if let Ok(roles) = data.get_cache_key(format!("{}_roles", member_uuid)).await {
            return Ok(serde_json::from_str(&roles)?)
        }

        let mut conn = data.pool.get().await?;

        use role_members::dsl;
        let role_memberships: Vec<RoleMember> = load_or_empty(
            dsl::role_members
                .filter(dsl::member_uuid.eq(member_uuid))
                .select(RoleMember::as_select())
                .load(&mut conn)
                .await,
        )?;

        let mut roles = vec![];

        for membership in role_memberships {
            roles.push(membership.fetch_role(&mut conn).await?);
        }

        data.set_cache_key(format!("{}_roles", member_uuid), roles.clone(), 300).await?;

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
        Permissions::fetch_permissions(self.permissions.clone())
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
    SendMessage = 1,
    CreateChannel = 2,
    DeleteChannel = 4,
    ManageChannel = 8,
    CreateRole = 16,
    DeleteRole = 32,
    ManageRole = 64,
    CreateInvite = 128,
    ManageInvite = 256,
    ManageServer = 512,
    ManageMember = 1024,
}

impl Permissions {
    pub fn fetch_permissions(permissions: i64) -> Vec<Self> {
        let all_perms = vec![
            Self::SendMessage,
            Self::CreateChannel,
            Self::DeleteChannel,
            Self::ManageChannel,
            Self::CreateRole,
            Self::DeleteRole,
            Self::ManageRole,
            Self::CreateInvite,
            Self::ManageInvite,
            Self::ManageServer,
            Self::ManageMember,
        ];

        all_perms
            .into_iter()
            .filter(|p| permissions & (*p as i64) != 0)
            .collect()
    }
}
