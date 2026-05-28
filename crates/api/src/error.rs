use axum::{Json, response::IntoResponse};
use fluxer_neptunium::http::endpoints::ExecuteEndpointRequestError;
use oauth2::{
    HttpClientError, RequestTokenError, StandardErrorResponse, basic::BasicErrorResponseType,
};
use reqwest::StatusCode;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    UrlParse(oauth2::url::ParseError),
    OAuthReqwest(oauth2::reqwest::Error),
    Database(sqlx::Error),
    RequestToken(
        RequestTokenError<
            HttpClientError<oauth2::reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ),
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    ExecuteEndpointRequest(Box<ExecuteEndpointRequestError>),
    GenericError(String),
    OAuthInvalidState,
}

impl std::error::Error for ApiError {}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UrlParse(e) => f.write_fmt(format_args!("Failed to parse URL: {e}")),
            Self::OAuthReqwest(e) => f.write_fmt(format_args!("OAuth2 reqwest error: {e}")),
            Self::Database(e) => f.write_fmt(format_args!("Database error: {e}")),
            Self::RequestToken(e) => f.write_fmt(format_args!("Token request failed: {e}")),
            Self::Reqwest(e) => f.write_fmt(format_args!("Reqwest error: {e}")),
            Self::Serde(e) => f.write_fmt(format_args!("Serialization error: {e}")),
            Self::ExecuteEndpointRequest(e) => {
                f.write_fmt(format_args!("ExecuteEndpointRequestError: {e}"))
            }
            Self::GenericError(e) => e.fmt(f),
            Self::OAuthInvalidState => f.write_str("Invalid OAuth state"),
        }
    }
}

impl From<oauth2::url::ParseError> for ApiError {
    fn from(value: oauth2::url::ParseError) -> Self {
        Self::UrlParse(value)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<oauth2::reqwest::Error> for ApiError {
    fn from(value: oauth2::reqwest::Error) -> Self {
        Self::OAuthReqwest(value)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<Box<ExecuteEndpointRequestError>> for ApiError {
    fn from(value: Box<ExecuteEndpointRequestError>) -> Self {
        Self::ExecuteEndpointRequest(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("{self}");

        let (status, message) = match self {
            Self::UrlParse(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Failed to parse url"),
            ),
            Self::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Database error"),
            ),
            Self::OAuthReqwest(_) => (StatusCode::BAD_GATEWAY, String::from("OAuth request error")),
            Self::RequestToken(_) => (
                StatusCode::BAD_GATEWAY,
                String::from("Token request failed"),
            ),
            Self::Reqwest(_) => (
                StatusCode::BAD_GATEWAY,
                String::from("Internal network error"),
            ),
            Self::Serde(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Serialization error"),
            ),
            Self::ExecuteEndpointRequest(_) | Self::GenericError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, String::new())
            }
            Self::OAuthInvalidState => {
                (StatusCode::BAD_REQUEST, String::from("Invalid OAuth state"))
            }
        };

        (status, Json(api_types::ApiErrorBody { error: message })).into_response()
    }
}
