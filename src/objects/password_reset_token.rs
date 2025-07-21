use argon2::{
    PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::Utc;
use diesel::{ExpressionMethods, QueryDsl, update};
use diesel_async::RunQueryDsl;
use lettre::message::MultiPart;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState, Conn,
    error::Error,
    schema::users,
    utils::{CacheFns, PASSWORD_REGEX, generate_token, global_checks, user_uuid_from_identifier},
};

#[derive(Serialize, Deserialize)]
pub struct PasswordResetToken {
    user_uuid: Uuid,
    pub token: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl PasswordResetToken {
    pub async fn get(
        cache_pool: &redis::Client,
        token: String,
    ) -> Result<PasswordResetToken, Error> {
        let user_uuid: Uuid = cache_pool.get_cache_key(token.to_string()).await?;
        let password_reset_token = cache_pool
            .get_cache_key(format!("{user_uuid}_password_reset"))
            .await?;

        Ok(password_reset_token)
    }

    pub async fn get_with_identifier(
        conn: &mut Conn,
        cache_pool: &redis::Client,
        identifier: String,
    ) -> Result<PasswordResetToken, Error> {
        let user_uuid = user_uuid_from_identifier(conn, &identifier).await?;

        let password_reset_token = cache_pool
            .get_cache_key(format!("{user_uuid}_password_reset"))
            .await?;

        Ok(password_reset_token)
    }

    #[allow(clippy::new_ret_no_self)]
    pub async fn new(
        conn: &mut Conn,
        app_state: &AppState,
        identifier: String,
    ) -> Result<(), Error> {
        let token = generate_token::<32>()?;

        let user_uuid = user_uuid_from_identifier(conn, &identifier).await?;

        global_checks(conn, &app_state.config, user_uuid).await?;

        use users::dsl as udsl;
        let (username, email_address): (String, String) = udsl::users
            .filter(udsl::uuid.eq(user_uuid))
            .select((udsl::username, udsl::email))
            .get_result(conn)
            .await?;

        let password_reset_token = PasswordResetToken {
            user_uuid,
            token: token.clone(),
            created_at: Utc::now(),
        };

        app_state
            .cache_pool
            .set_cache_key(
                format!("{user_uuid}_password_reset"),
                password_reset_token,
                86400,
            )
            .await?;
        app_state
            .cache_pool
            .set_cache_key(token.clone(), user_uuid, 86400)
            .await?;

        let mut reset_endpoint = app_state.config.web.frontend_url.join("reset-password")?;

        reset_endpoint.set_query(Some(&format!("token={token}")));

        let email = app_state
            .mail_client
            .message_builder()
            .to(email_address.parse()?)
            .subject(format!("{} Password Reset", app_state.config.instance.name))
            .multipart(MultiPart::alternative_plain_html(
                format!("{} Password Reset\n\nHello, {}!\nSomeone requested a password reset for your Gorb account.\nClick the button below within 24 hours to reset your password.\n\n{}\n\nIf you didn't request a password reset, don't worry, your account is safe and you can safely ignore this email.\n\nThanks, The gorb team.", app_state.config.instance.name, username, reset_endpoint), 
                format!(r#"<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><style>:root {{--header-text-colour: #ffffff;--footer-text-colour: #7f7f7f;--button-text-colour: #170e08;--text-colour: #170e08;--background-colour: #fbf6f2;--primary-colour: #df5f0b;--secondary-colour: #e8ac84;--accent-colour: #e68b4e;}}@media (prefers-color-scheme: dark) {{:root {{--header-text-colour: #ffffff;--footer-text-colour: #585858;--button-text-colour: #ffffff;--text-colour: #f7eee8;--background-colour: #0c0704;--primary-colour: #f4741f;--secondary-colour: #7c4018;--accent-colour: #b35719;}}}}@media (max-width: 600px) {{.container {{width: 100%;}}}}body {{font-family: Arial, sans-serif;align-content: center;text-align: center;margin: 0;padding: 0;background-color: var(--background-colour);color: var(--text-colour);width: 100%;max-width: 600px;margin: 0 auto;border-radius: 5px;}}.header {{background-color: var(--primary-colour);color: var(--header-text-colour);padding: 20px;}}.verify-button {{background-color: var(--accent-colour);color: var(--button-text-colour);padding: 12px 30px;margin: 16px;font-size: 20px;transition: background-color 0.3s;cursor: pointer;border: none;border-radius: 14px;text-decoration: none;display: inline-block;}}.verify-button:hover {{background-color: var(--secondary-colour);}}.content {{padding: 20px 30px;}}.footer {{padding: 10px;font-size: 12px;color: var(--footer-text-colour);}}</style></head><body><div class="container"><div class="header"><h1>{} Password Reset</h1></div><div class="content"><h2>Hello, {}!</h2><p>Someone requested a password reset for your Gorb account.</p><p>Click the button below within 24 hours to reset your password.</p><a href="{}" class="verify-button">RESET PASSWORD</a><p>If you didn't request a password reset, don't worry, your account is safe and you can safely ignore this email.</p><div class="footer"><p>Thanks<br>The gorb team.</p></div></div></div></body></html>"#, app_state.config.instance.name, username, reset_endpoint)
            ))?;

        app_state.mail_client.send_mail(email).await?;

        Ok(())
    }

    pub async fn set_password(
        &self,
        conn: &mut Conn,
        app_state: &AppState,
        password: String,
    ) -> Result<(), Error> {
        if !PASSWORD_REGEX.is_match(&password) {
            return Err(Error::BadRequest(
                "Please provide a valid password".to_string(),
            ));
        }

        let salt = SaltString::generate(&mut OsRng);

        let hashed_password = app_state
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::PasswordHashError(e.to_string()))?;

        use users::dsl;
        update(users::table)
            .filter(dsl::uuid.eq(self.user_uuid))
            .set(dsl::password.eq(hashed_password.to_string()))
            .execute(conn)
            .await?;

        let (username, email_address): (String, String) = dsl::users
            .filter(dsl::uuid.eq(self.user_uuid))
            .select((dsl::username, dsl::email))
            .get_result(conn)
            .await?;

        let login_page = app_state.config.web.frontend_url.join("login")?;

        let email = app_state
            .mail_client
            .message_builder()
            .to(email_address.parse()?)
            .subject(format!("Your {} Password has been Reset", app_state.config.instance.name))
            .multipart(MultiPart::alternative_plain_html(
                format!("{} Password Reset Confirmation\n\nHello, {}!\nYour password has been successfully reset for your Gorb account.\nIf you did not initiate this change, please click the link below to reset your password <strong>immediately</strong>.\n\n{}\n\nThanks, The gorb team.", app_state.config.instance.name, username, login_page), 
                format!(r#"<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><style>:root {{--header-text-colour: #ffffff;--footer-text-colour: #7f7f7f;--button-text-colour: #170e08;--text-colour: #170e08;--background-colour: #fbf6f2;--primary-colour: #df5f0b;--secondary-colour: #e8ac84;--accent-colour: #e68b4e;}}@media (prefers-color-scheme: dark) {{:root {{--header-text-colour: #ffffff;--footer-text-colour: #585858;--button-text-colour: #ffffff;--text-colour: #f7eee8;--background-colour: #0c0704;--primary-colour: #f4741f;--secondary-colour: #7c4018;--accent-colour: #b35719;}}}}@media (max-width: 600px) {{.container {{width: 100%;}}}}body {{font-family: Arial, sans-serif;align-content: center;text-align: center;margin: 0;padding: 0;background-color: var(--background-colour);color: var(--text-colour);width: 100%;max-width: 600px;margin: 0 auto;border-radius: 5px;}}.header {{background-color: var(--primary-colour);color: var(--header-text-colour);padding: 20px;}}.verify-button {{background-color: var(--accent-colour);color: var(--button-text-colour);padding: 12px 30px;margin: 16px;font-size: 20px;transition: background-color 0.3s;cursor: pointer;border: none;border-radius: 14px;text-decoration: none;display: inline-block;}}.verify-button:hover {{background-color: var(--secondary-colour);}}.content {{padding: 20px 30px;}}.footer {{padding: 10px;font-size: 12px;color: var(--footer-text-colour);}}</style></head><body><div class="container"><div class="header"><h1>{} Password Reset Confirmation</h1></div><div class="content"><h2>Hello, {}!</h2><p>Your password has been successfully reset for your Gorb account.</p><p>If you did not initiate this change, please click the button below to reset your password <strong>immediately</strong>.</p><a href="{}" class="verify-button">RESET PASSWORD</a><div class="footer"><p>Thanks<br>The gorb team.</p></div></div></div></body></html>"#, app_state.config.instance.name, username, login_page)
            ))?;

        app_state.mail_client.send_mail(email).await?;

        self.delete(&app_state.cache_pool).await
    }

    pub async fn delete(&self, cache_pool: &redis::Client) -> Result<(), Error> {
        cache_pool
            .del_cache_key(format!("{}_password_reset", &self.user_uuid))
            .await?;
        cache_pool.del_cache_key(self.token.to_string()).await?;

        Ok(())
    }
}
