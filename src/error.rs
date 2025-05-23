use std::{io, time::SystemTimeError};

use actix_web::{error::ResponseError, http::{header::{ContentType, ToStrError}, StatusCode}, HttpResponse};
use deadpool::managed::{BuildError, PoolError};
use redis::RedisError;
use serde::Serialize;
use thiserror::Error;
use diesel::{result::Error as DieselError, ConnectionError};
use diesel_async::pooled_connection::PoolError as DieselPoolError;
use tokio::task::JoinError;
use serde_json::Error as JsonError;
use toml::de::Error as TomlError;
use log::error;

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
    #[error("{0}")]
    PasswordHashError(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        error!("{}: {}", self.status_code(), self.to_string());

        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(WebError::new(self.to_string()))
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            Error::SqlError(DieselError::NotFound) => StatusCode::NOT_FOUND,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::Unauthorized(_) => StatusCode::UNAUTHORIZED,
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
        Self {
            message,
        }
    }
}
