use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use diesel_async::RunQueryDsl;

use crate::{
    error::Error, objects::{load_or_empty, Guild}, schema::guild_bans, Conn
};


#[derive(Selectable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = guild_bans)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuildBan {
    pub guild_uuid: Uuid,
    pub user_uuid: Uuid,
    pub reason: Option<String>,
    pub ban_time: chrono::DateTime<chrono::Utc>,
}


impl GuildBan {
    pub async fn fetch_one(conn: &mut Conn, guild_uuid: Uuid, user_uuid: Uuid) -> Result<GuildBan, Error> {
        use guild_bans::dsl;
        let guild_ban = dsl::guild_bans
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .filter(dsl::user_uuid.eq(user_uuid))
            .select(GuildBan::as_select())
            .get_result(conn)
            .await?;

        Ok(guild_ban)
    }

    pub async fn fetch_all(conn: &mut Conn, guild_uuid: Uuid) -> Result<Vec<Self>, Error> {
        use guild_bans::dsl;
        let all_guild_bans = load_or_empty(dsl::guild_bans
            .filter(dsl::guild_uuid.eq(guild_uuid))
            .load(conn)
            .await
            )?;
        
        Ok(all_guild_bans)
    }

    pub async fn unban(self, conn: &mut Conn) -> Result<(), Error> {
        use guild_bans::dsl;
        diesel::delete(guild_bans::table)
            .filter(dsl::guild_uuid.eq(self.guild_uuid))
            .filter(dsl::user_uuid.eq(self.user_uuid))
            .execute(conn)
            .await?;
        Ok(())
    }
}
