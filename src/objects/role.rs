use diesel::{
    ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable, SelectableHelper, insert_into,
    update,
};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use uuid::Uuid;

use crate::{Conn, error::Error, schema::roles, utils::order_by_is_above};

use super::{HasIsAbove, HasUuid, load_or_empty};

#[derive(Serialize, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Role {
    uuid: Uuid,
    guild_uuid: Uuid,
    name: String,
    color: i32,
    is_above: Option<Uuid>,
    permissions: i64,
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

    pub async fn fetch_one(conn: &mut Conn, role_uuid: Uuid) -> Result<Self, Error> {
        use roles::dsl;
        let role: Role = dsl::roles
            .filter(dsl::uuid.eq(role_uuid))
            .select(Role::as_select())
            .get_result(conn)
            .await?;

        Ok(role)
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
