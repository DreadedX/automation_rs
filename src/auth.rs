use axum::{
    async_trait,
    extract::{FromRequestParts, FromRef},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::config::OpenIDConfig;

#[derive(Debug, Deserialize)]
pub struct User {
    pub preferred_username: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for User
where
    OpenIDConfig: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Get the state
        let openid = OpenIDConfig::from_ref(state);

        // Create a request to the auth server
        // @TODO Do some discovery to find the correct url for this instead of assuming
        let mut req = reqwest::Client::new()
            .get(format!("{}/userinfo", openid.base_url));

        // Add auth header to the request if it exists
        if let Some(auth) = parts.headers.get(axum::http::header::AUTHORIZATION) {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }

        // Send the request
        let res = req.send()
            .await
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

        // If the request is success full the auth token is valid and we are given userinfo
        let status = res.status();
        if status.is_success() {
            let user = res.json()
                .await
                .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

            return Ok(user);
        } else {
            let err = res
                .text()
                .await
                .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

            return Err((status, err).into_response());
        }
    }
}
