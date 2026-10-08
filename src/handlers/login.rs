use axum::{
	extract::Form,
};
use axum_extra::extract::cookie::{CookieJar, Cookie, SameSite};
use time::{OffsetDateTime, Duration};

use crate::{
    CurrentUser,
	Deserialize,
	IntoResponse,
	LState,
	Redirect,
	Response,
	Rejection,
	StatusCode,
};

#[derive(Deserialize)]
pub struct LoginData {
	pub login: String,
	pub password: String,
}

pub async fn login(
	state: LState,
	cookies: CookieJar,
	form: Form<LoginData>,
) -> Result<Response, Rejection> {
	let Some(user) = state.user.get_by_login(&form.login).await else {
		return Err((StatusCode::UNAUTHORIZED, "Invalid username or email.").into());
	};
	if user.match_password(&form.password) {
		let Ok(token) = state.session.create(user.id).await else {
			return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create session.").into());
		};
		let expiration = OffsetDateTime::now_utc() + Duration::weeks(1);
		let cookie = Cookie::build(("Authorization", token))
			.path("/")
			.secure(true)
			.expires(expiration)
			.same_site(SameSite::Lax)
			.build();
		Ok(
			(cookies.add(cookie), Redirect::to("/")).into_response()
		)
	} else {
		Err((StatusCode::UNAUTHORIZED, "Invalid password.").into())
	}
}

pub async fn logout(
    state: LState,
    cookies: CookieJar,
    CurrentUser(user): CurrentUser
) -> Result<Response, Rejection> {
    let Some(user) = user else {
        return Err((StatusCode::UNAUTHORIZED, "You are not logged in.").into())
    };

    let Some(cookie) = cookies.get("Authorization") else {
        return Err((StatusCode::UNAUTHORIZED, "No token provided.").into())
    };

    let token = cookie.value();

    let Some(session) = state.session.get_by_token(token).await else {
        return Err((StatusCode::BAD_REQUEST, "Invalid session token.").into());
    };

    if session.user != user.id {
        return Err((StatusCode::FORBIDDEN, "Malformed payload.").into())
    }

    match state.session.delete_by_token(token).await {
        Ok(_) => Ok(Redirect::to("/").into_response()),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete session.").into())
    }
}