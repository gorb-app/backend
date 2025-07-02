use chrono::Utc;
use lettre::message::MultiPart;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Data, error::Error, utils::generate_token};

use super::Me;

#[derive(Serialize, Deserialize)]
pub struct EmailToken {
    user_uuid: Uuid,
    pub token: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl EmailToken {
    pub async fn get(data: &Data, user_uuid: Uuid) -> Result<EmailToken, Error> {
        let email_token = serde_json::from_str(
            &data
                .get_cache_key(format!("{user_uuid}_email_verify"))
                .await?,
        )?;

        Ok(email_token)
    }

    #[allow(clippy::new_ret_no_self)]
    pub async fn new(data: &Data, me: Me) -> Result<(), Error> {
        let token = generate_token::<32>()?;

        let email_token = EmailToken {
            user_uuid: me.uuid,
            token: token.clone(),
            // TODO: Check if this can be replaced with something built into valkey
            created_at: Utc::now(),
        };

        data.set_cache_key(format!("{}_email_verify", me.uuid), email_token, 86400)
            .await?;

        let mut verify_endpoint = data.config.web.frontend_url.join("verify-email")?;

        verify_endpoint.set_query(Some(&format!("token={token}")));

        let email = data
            .mail_client
            .message_builder()
            .to(me.email.parse()?)
            .subject(format!("{} E-mail Verification", data.config.instance.name))
            .multipart(MultiPart::alternative_plain_html(
                format!("Verify your {} account\n\nHello, {}!\nThanks for creating a new account on Gorb.\nThe final step to create your account is to verify your email address by visiting the page, within 24 hours.\n\n{}\n\nIf you didn't ask to verify this address, you can safely ignore this email\n\nThanks, The gorb team.", data.config.instance.name, me.username, verify_endpoint), 
                format!(r#"<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><style>:root{{--header-text-colour: #ffffff;--footer-text-colour: #7f7f7f;--button-text-colour: #170e08;--text-colour: #170e08;--background-colour: #fbf6f2;--primary-colour: #df5f0b;--secondary-colour: #e8ac84;--accent-colour: #e68b4e;}}@media (prefers-color-scheme: dark){{:root{{--header-text-colour: #ffffff;--footer-text-colour: #585858;--button-text-colour: #ffffff;--text-colour: #f7eee8;--background-colour: #0c0704;--primary-colour: #f4741f;--secondary-colour: #7c4018;--accent-colour: #b35719;}}}}@media (max-width: 600px){{.container{{width: 100%;}}}}body{{font-family: Arial, sans-serif;align-content: center;text-align: center;margin: 0;padding: 0;background-color: var(--background-colour);color: var(--text-colour);width: 100%;max-width: 600px;margin: 0 auto;border-radius: 5px;}}.header{{background-color: var(--primary-colour);color: var(--header-text-colour);padding: 20px;}}.verify-button{{background-color: var(--accent-colour);color: var(--button-text-colour);padding: 12px 30px;margin: 16px;font-size: 20px;transition: background-color 0.3s;cursor: pointer;border: none;border-radius: 14px;text-decoration: none;display: inline-block;}}.verify-button:hover{{background-color: var(--secondary-colour);}}.content{{padding: 20px 30px;}}.footer{{padding: 10px;font-size: 12px;color: var(--footer-text-colour);}}</style></head><body><div class="container"><div class="header"><h1>Verify your {} Account</h1></div><div class="content"><h2>Hello, {}!</h2><p>Thanks for creating a new account on Gorb.</p><p>The final step to create your account is to verify your email address by clicking the button below, within 24 hours.</p><a href="{}" class="verify-button">VERIFY ACCOUNT</a><p>If you didn't ask to verify this address, you can safely ignore this email.</p><div class="footer"><p>Thanks<br>The gorb team.</p></div></div></div></body></html>"#, data.config.instance.name, me.username, verify_endpoint)
            ))?;

        data.mail_client.send_mail(email).await?;

        Ok(())
    }

    pub async fn delete(&self, data: &Data) -> Result<(), Error> {
        data.del_cache_key(format!("{}_email_verify", self.user_uuid))
            .await?;

        Ok(())
    }
}
