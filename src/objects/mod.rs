use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message as Email, Tokio1Executor,
    message::{Mailbox, MessageBuilder as EmailBuilder},
    transport::smtp::authentication::Credentials,
};
use log::debug;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod bans;
mod channel;
mod email_token;
mod friends;
mod guild;
mod invite;
mod me;
mod member;
mod message;
mod password_reset_token;
mod role;
mod user;
mod auditlog;

pub use bans::GuildBan;
pub use channel::Channel;
pub use email_token::EmailToken;
pub use friends::Friend;
pub use friends::FriendRequest;
pub use guild::Guild;
pub use invite::Invite;
pub use me::Me;
pub use member::Member;
pub use message::Message;
pub use password_reset_token::PasswordResetToken;
pub use role::Permissions;
pub use role::Role;
pub use user::User;
pub use auditlog::AuditLog;

use crate::error::Error;

pub trait HasUuid {
    fn uuid(&self) -> &Uuid;
}

pub trait HasIsAbove {
    fn is_above(&self) -> Option<&Uuid>;
}
/*
pub trait Cookies {
    fn cookies(&self) -> CookieJar;
    fn cookie<T: AsRef<str>>(&self, cookie: T) -> Option<Cookie>;
}

impl Cookies for Request<Body> {
    fn cookies(&self) -> CookieJar {
        let cookies = self.headers()
            .get(axum::http::header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .map(|s| Cookie::split_parse(s.to_string()))
            .and_then(|c| c.collect::<Result<Vec<Cookie>, cookie::ParseError>>().ok())
            .unwrap_or(vec![]);

        let mut cookie_jar = CookieJar::new();

        for cookie in cookies {
            cookie_jar.add(cookie)
        }

        cookie_jar
    }

    fn cookie<T: AsRef<str>>(&self, cookie: T) -> Option<Cookie> {
        self.cookies()
            .get(cookie.as_ref())
            .and_then(|c| Some(c.to_owned()))
    }
}
*/

#[derive(Serialize)]
pub struct Pagination<T> {
    objects: Vec<T>,
    amount: i32,
    pages: i32,
    page: i32,
}

#[derive(Deserialize)]
pub struct PaginationRequest {
    pub page: i32,
    pub per_page: Option<i32>,
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
