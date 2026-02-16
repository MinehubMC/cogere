use crate::{auth::auth::AuthSession, models::auth::UserCredentials};
use askama::Template;
use axum::{
    Form,
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use axum_messages::{Message, Messages};
use serde::Deserialize;

#[derive(Template)]
#[template(path = "login.jinja")]
struct LoginTemplate {
    messages: Vec<Message>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub async fn login_page(
    messages: Messages,
    Query(NextUrl { next }): Query<NextUrl>,
) -> impl IntoResponse {
    Html(
        LoginTemplate {
            messages: messages.into_iter().collect(),
            next,
        }
        .render()
        .unwrap(),
    )
}

#[axum::debug_handler]
pub async fn login_post(
    auth_session: AuthSession,
    messages: Messages,
    Form(credentials): Form<UserCredentials>,
) -> impl IntoResponse {
    let user = match auth_session.authenticate(credentials.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            messages.error("Invalid credentials");

            let mut login_url = "/login".to_string();
            if let Some(next) = credentials.next {
                login_url = format!("{login_url}?next={next}");
            };

            return Redirect::to(&login_url).into_response();
        }
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    messages.success(format!("Successfully logged in as {}", user.username));

    if let Some(ref next) = credentials.next {
        Redirect::to(next)
    } else {
        Redirect::to("/")
    }
    .into_response()
}
