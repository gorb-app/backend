use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message as Email, Tokio1Executor,
    message::{Mailbox, MessageBuilder as EmailBuilder},
    transport::smtp::authentication::Credentials,
};
use log::debug;
use serde::Deserialize;
use uuid::Uuid;

mod channel;
mod email_token;
mod guild;
mod invite;
mod me;
mod member;
mod message;
mod password_reset_token;
mod role;
mod user;
mod friends;

pub use channel::Channel;
pub use email_token::EmailToken;
pub use guild::Guild;
pub use invite::Invite;
pub use me::Me;
pub use member::Member;
pub use message::Message;
pub use password_reset_token::PasswordResetToken;
pub use role::Permissions;
pub use role::Role;
pub use user::User;
pub use friends::Friend;
pub use friends::FriendRequest;

use crate::error::Error;

pub trait HasUuid {
    fn uuid(&self) -> &Uuid;
}

pub trait HasIsAbove {
    fn is_above(&self) -> Option<&Uuid>;
}

fn load_or_empty<T>(
    query_result: Result<Vec<T>, diesel::result::Error>,
) -> Result<Vec<T>, diesel::result::Error> {
    match query_result {
        Ok(vec) => Ok(vec),
        Err(diesel::result::Error::NotFound) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum MailTls {
    StartTls,
    Tls,
}

impl From<String> for MailTls {
    fn from(value: String) -> Self {
        match &*value.to_lowercase() {
            "starttls" => Self::StartTls,
            _ => Self::Tls,
        }
    }
}

#[derive(Clone)]
pub struct MailClient {
    creds: Credentials,
    smtp_server: String,
    mbox: Mailbox,
    tls: MailTls,
}

impl MailClient {
    pub fn new<T: Into<MailTls>>(
        creds: Credentials,
        smtp_server: String,
        mbox: String,
        tls: T,
    ) -> Result<Self, Error> {
        Ok(Self {
            creds,
            smtp_server,
            mbox: mbox.parse()?,
            tls: tls.into(),
        })
    }

    pub fn message_builder(&self) -> EmailBuilder {
        Email::builder().from(self.mbox.clone())
    }

    pub async fn send_mail(&self, email: Email) -> Result<(), Error> {
        let mailer: AsyncSmtpTransport<Tokio1Executor> = match self.tls {
            MailTls::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_server)?
                    .credentials(self.creds.clone())
                    .build()
            }
            MailTls::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_server)?
                .credentials(self.creds.clone())
                .build(),
        };

        let response = mailer.send(email).await?;

        debug!("mail sending response: {response:?}");

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct StartAmountQuery {
    pub start: Option<i64>,
    pub amount: Option<i64>,
}
