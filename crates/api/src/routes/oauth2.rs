use axum::{
    Router,
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::get,
};
use axum_extra::extract::{
    PrivateCookieJar,
    cookie::{Cookie, Expiration, SameSite},
};
use chrono::{DateTime, TimeDelta, Utc};
use oauth2::{AuthorizationCode, CsrfToken, TokenResponse};
use rand::RngExt;
use reqwest::StatusCode;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    SESSION_COOKIE_NAME,
    db::schema::SessionJsonData,
    error::{ApiError, ApiResult},
    state::AppState,
};

const CSRF_COOKIE: &str = "oauth_csrf";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/oauth/initiate", get(initiate))
        .route("/oauth/callback", get(callback))
}

#[utoipa::path(get, path = "/oauth/initiate")]
async fn initiate(State(state): State<AppState>, jar: PrivateCookieJar) -> impl IntoResponse {
    let (auth_url, csrf_token) = state
        .oauth
        .authorize_url(CsrfToken::new_random)
        .add_scopes(state.oauth_scopes.clone())
        .url();

    let csrf_cookie = Cookie::build((CSRF_COOKIE, csrf_token.secret().clone()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::minutes(10))
        .build();

    (jar.add(csrf_cookie), Redirect::to(auth_url.as_str()))
}

#[derive(Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

async fn callback(
    Query(params): Query<CallbackParams>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> ApiResult<(PrivateCookieJar, impl IntoResponse)> {
    let stored_csrf = jar
        .get(CSRF_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or(ApiError::OAuthInvalidState)?;

    if stored_csrf != params.state {
        return Err(ApiError::OAuthInvalidState.into());
    }

    let jar = jar.remove(CSRF_COOKIE);

    let bearer_token = match state
        .oauth
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(&state.oauth_http_client)
        .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Error exchanging code: {e}");
            return Err(ApiError::RequestToken(e).into());
        }
    };
    let bearer_token = bearer_token.access_token().secret();

    let user = state.fluxer_api.get_user(bearer_token).await?;

    let session_token: String = state
        .rng
        .lock()
        .await
        .sample_iter(&rand::distr::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let expires_at = Utc::now() + TimeDelta::hours(24);

    state
        .db
        .create_dash_session(
            &session_token,
            user.id,
            expires_at,
            SessionJsonData {
                bearer_token: bearer_token.clone(),
                user_id: user.id,
            },
        )
        .await?;

    let session_cookie = Cookie::build((SESSION_COOKIE_NAME, session_token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .expires(Expiration::DateTime(
            match OffsetDateTime::from_unix_timestamp(expires_at.timestamp()) {
                Ok(timestamp) => timestamp,
                Err(e) => {
                    tracing::error!(
                        "Error converting {:?} to chrono DateTime: {}",
                        expires_at,
                        e
                    );
                    return Err(crate::error::ApiErrorResponse::StatusCode(
                        StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }
            },
        ))
        .build();

    Ok((jar.add(session_cookie), Redirect::to(&state.dashboard_uri)))
}
