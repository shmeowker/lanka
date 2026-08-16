use axum::{
	extract::Form,
	http::{
		header,
		HeaderValue,
	},
};
use axum_extra::extract::cookie::{CookieJar, Cookie, SameSite};
use time::{OffsetDateTime, Duration};

use crate::{
	Deserialize,
	IntoResponse,
	LState,
	Redirect,
	Response,
	Rejection,
	StatusCode,
};


pub type LoginForm = Form<LoginData>;

#[derive(Deserialize)]
pub struct LoginData {
	pub login: String,
	pub password: String,
}

pub async fn login(
	state: LState,
	cookies: CookieJar,
	form: LoginForm,
) -> Result<Response, Rejection> {
	let Some(user) = state.user.get_by_login(&form.login).await else {
		return Err((StatusCode::UNAUTHORIZED, "Invalid username or email."));
	};
	if user.match_password(&form.password) {
		let Ok(token) = state.session.create(user.id).await else {
			return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create session."));
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
		Err((StatusCode::UNAUTHORIZED, "Invalid password."))
	}
}