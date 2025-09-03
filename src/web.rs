use std::result;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::http::status::InvalidStatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{source}")]
pub struct ApiError {
    status_code: axum::http::StatusCode,
    source: Box<dyn std::error::Error>,
}

impl ApiError {
    pub fn new(status_code: axum::http::StatusCode, source: Box<dyn std::error::Error>) -> Self {
        Self {
            status_code,
            source,
        }
    }
}

impl From<ApiError> for ApiErrorJson {
    fn from(value: ApiError) -> Self {
        let error = ApiErrorJsonError {
            code: value.status_code.as_u16(),
            status: value.status_code.to_string(),
            reason: value.source.to_string(),
        };

        Self { error }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status_code,
            serde_json::to_string::<ApiErrorJson>(&self.into())
                .expect("Serialization should not fail"),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiErrorJsonError {
    code: u16,
    status: String,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorJson {
    error: ApiErrorJsonError,
}

impl TryFrom<ApiErrorJson> for ApiError {
    type Error = InvalidStatusCode;

    fn try_from(value: ApiErrorJson) -> result::Result<Self, Self::Error> {
        let status_code = axum::http::StatusCode::from_u16(value.error.code)?;
        let source = value.error.reason.into();

        Ok(Self {
            status_code,
            source,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub preferred_username: String,
}

impl<S> FromRequestParts<S> for User
where
    String: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Get the state
        let openid_url = String::from_ref(state);

        // Create a request to the auth server
        // TODO: Do some discovery to find the correct url for this instead of assuming
        // TODO: I think we can also just run Authlia in front of the endpoint instead
        // This would then give us a header containing the logged in user info?
        let mut req = reqwest::Client::new().get(format!("{}/userinfo", openid_url));

        // Add auth header to the request if it exists
        if let Some(auth) = parts.headers.get(axum::http::header::AUTHORIZATION) {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }

        // Send the request
        let res = req
            .send()
            .await
            .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.into()))?;

        // If the request is success full the auth token is valid and we are given userinfo
        let status = res.status();
        if status.is_success() {
            let user = res
                .json()
                .await
                .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.into()))?;

            Ok(user)
        } else {
            let err: ApiErrorJson = res
                .json()
                .await
                .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.into()))?;

            let err = ApiError::try_from(err)
                .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.into()))?;

            Err(err)
        }
    }
}
