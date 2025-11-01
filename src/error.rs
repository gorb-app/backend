use std::{io, time::SystemTimeError};

use axum::{
    Json,
    extract::{
        multipart::MultipartError,
        rejection::{JsonRejection, QueryRejection}, ws::Message,
    },
    http::{
        StatusCode,
        header::{InvalidHeaderValue, ToStrError},
    },
    response::IntoResponse,
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
use tokio::{sync::mpsc, task::JoinError};
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
    JsonRejection(#[from] JsonRejection),
    #[error(transparent)]
    QueryRejection(#[from] QueryRejection),
    #[error(transparent)]
    MultipartError(#[from] MultipartError),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
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
    #[error(transparent)]
    AxumError(#[from] axum::Error),
    #[error(transparent)]
    MpscSendErrorStr(#[from] mpsc::error::SendError<&'static str>),
    #[error(transparent)]
    MpscSendError(#[from] mpsc::error::SendError<Message>),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let error = match self {
            Error::SqlError(DieselError::NotFound) => {
                (StatusCode::NOT_FOUND, Json(WebError::new(self.to_string())))
            }
            Error::BunnyError(BunnyError::NotFound(_)) => {
                (StatusCode::NOT_FOUND, Json(WebError::new(self.to_string())))
            }
            Error::BadRequest(_) => (
                StatusCode::BAD_REQUEST,
                Json(WebError::new(self.to_string())),
            ),
            Error::Unauthorized(_) => (
                StatusCode::UNAUTHORIZED,
                Json(WebError::new(self.to_string())),
            ),
            Error::Forbidden(_) => (StatusCode::FORBIDDEN, Json(WebError::new(self.to_string()))),
            Error::TooManyRequests(_) => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(WebError::new(self.to_string())),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(WebError::new(self.to_string())),
            ),
        };

        let (code, _) = error;

        debug!("{self:?}");
        error!("{code}: {self}");

        error.into_response()
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
