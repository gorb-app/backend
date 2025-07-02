use std::{io, time::SystemTimeError};

use actix_web::{
    HttpResponse,
    error::{PayloadError, ResponseError},
    http::{
        StatusCode,
        header::{ContentType, ToStrError},
    },
};
use bunny_api_tokio::error::Error as BunnyError;
use deadpool::managed::{BuildError, PoolError};
use diesel::{ConnectionError, result::Error as DieselError};
use diesel_async::pooled_connection::PoolError as DieselPoolError;
use lettre::{
    address::AddressError, error::Error as EmailError, transport::smtp::Error as SmtpError,
};
use log::{debug, error};
use redis::RedisError;
use serde::Serialize;
use serde_json::Error as JsonError;
use thiserror::Error;
use tokio::task::JoinError;
use toml::de::Error as TomlError;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    SqlError(#[from] DieselError),
    #[error(transparent)]
    PoolError(#[from] PoolError<DieselPoolError>),
    #[error(transparent)]
    BuildError(#[from] BuildError),
    #[error(transparent)]
    RedisError(#[from] RedisError),
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error(transparent)]
    JoinError(#[from] JoinError),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    TomlError(#[from] TomlError),
    #[error(transparent)]
    JsonError(#[from] JsonError),
    #[error(transparent)]
    SystemTimeError(#[from] SystemTimeError),
    #[error(transparent)]
    ToStrError(#[from] ToStrError),
    #[error(transparent)]
    RandomError(#[from] getrandom::Error),
    #[error(transparent)]
    BunnyError(#[from] BunnyError),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    PayloadError(#[from] PayloadError),
    #[error(transparent)]
    WsClosed(#[from] actix_ws::Closed),
    #[error(transparent)]
    EmailError(#[from] EmailError),
    #[error(transparent)]
    SmtpError(#[from] SmtpError),
    #[error(transparent)]
    SmtpAddressError(#[from] AddressError),
    #[error("{0}")]
    PasswordHashError(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    TooManyRequests(String),
    #[error("{0}")]
    InternalServerError(String),
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        debug!("{self:?}");
        error!("{}: {}", self.status_code(), self);

        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(WebError::new(self.to_string()))
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            Error::SqlError(DieselError::NotFound) => StatusCode::NOT_FOUND,
            Error::BunnyError(BunnyError::NotFound(_)) => StatusCode::NOT_FOUND,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Error::Forbidden(_) => StatusCode::FORBIDDEN,
            Error::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Serialize)]
struct WebError {
    message: String,
}

impl WebError {
    fn new(message: String) -> Self {
        Self { message }
    }
}
