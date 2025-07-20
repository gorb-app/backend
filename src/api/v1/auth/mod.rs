use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{Request, State}, middleware::{from_fn_with_state, Next}, response::IntoResponse, routing::{delete, get, post}, Router
};
use axum_extra::{headers::{authorization::Bearer, Authorization}, TypedHeader};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use uuid::Uuid;

use crate::{AppState, Conn, error::Error, schema::access_tokens::dsl};

mod devices;
mod login;
mod logout;
mod refresh;
mod register;
mod reset_password;
mod revoke;
mod verify_email;


#[derive(Serialize)]
pub struct Response {
    access_token: String,
    device_name: String,
}


pub fn router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    let router_with_auth = Router::new()
        .route("/verify-email", get(verify_email::get))
        .route("/verify-email", post(verify_email::post))
        .route("/revoke", post(revoke::post))
        .route("/devices", get(devices::get))
        .layer(from_fn_with_state(app_state, CurrentUser::check_auth_layer));

    Router::new()
        .route("/register", post(register::post))
        .route("/login", post(login::response))
        .route("/logout", delete(logout::res))
        .route("/refresh", post(refresh::post))
        .route("/reset-password", get(reset_password::get))
        .route("/reset-password", post(reset_password::post))
        .merge(router_with_auth)
}

pub async fn check_access_token(access_token: &str, conn: &mut Conn) -> Result<Uuid, Error> {
    let (uuid, created_at): (Uuid, i64) = dsl::access_tokens
        .filter(dsl::token.eq(access_token))
        .select((dsl::uuid, dsl::created_at))
        .get_result(conn)
        .await
        .map_err(|error| {
            if error == diesel::result::Error::NotFound {
                Error::Unauthorized("Invalid access token".to_string())
            } else {
                Error::from(error)
            }
        })?;

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let lifetime = current_time - created_at;

    if lifetime > 3600 {
        return Err(Error::Unauthorized("Invalid access token".to_string()));
    }

    Ok(uuid)
}

#[derive(Clone)]
pub struct CurrentUser<Uuid>(pub Uuid);

impl CurrentUser<Uuid> {
    pub async fn check_auth_layer(
        State(app_state): State<Arc<AppState>>,
        TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
        mut req: Request,
        next: Next
    ) -> Result<impl IntoResponse, Error> {
        let current_user = CurrentUser(check_access_token(auth.token(), &mut app_state.pool.get().await?).await?);

        req.extensions_mut().insert(current_user);
        Ok(next.run(req).await)
    }
}
