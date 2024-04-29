use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::StatusCode;
use serde::Deserialize;

use crate::error::{ApiError, ApiErrorJson};

#[derive(Debug, Deserialize)]
pub struct User {
    pub preferred_username: String,
}

#[async_trait]
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

            return Ok(user);
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
