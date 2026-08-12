use axum::{
	extract::Form,
	http::{
		header,
		HeaderValue,
	},
};
use axum_extra::extract::cookie::{CookieJar, Cookie, SameSite};

use crate::{
	Deserialize,
	IntoResponse,
	LState,
	Redirect,
	Response,
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
) -> Response {
	let Some(user) = state.user.get_by_login(&form.login).await else {
		return (StatusCode::UNAUTHORIZED, "Invalid username or email.").into_response();
	};
	if user.match_password(&form.password) {
		let Ok(token) = state.session.create(user.id).await else {
			return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create session.").into_response();
		};
		let cookie = Cookie::build(("Authorization", token))
			.path("/")
			.secure(true)
			.same_site(SameSite::Lax)
			.build();
		(
			cookies.add(cookie),
			Redirect::to("/"),
		).into_response()
	} else {
		(StatusCode::UNAUTHORIZED, "Invalid password.").into_response()
	}
}