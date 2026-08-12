use axum::{
	RequestPartsExt,
	extract::Extension,
	http::{
		HeaderMap,
		header::{AUTHORIZATION, COOKIE},
		request::Parts
	},
};
use axum_extra::extract::cookie::{CookieJar, Cookie};

use crate::{
	AppState,
	Arc,
	Deref,
	FromRef,
	FromRequestParts,
	LState,
	StatusCode,
	User,
	Utc
};


#[derive(Deref)]
pub struct CurrentUser(Option<User>);

impl CurrentUser {
	pub fn unwrap(&self) -> User {
		self.0.clone().unwrap()
	}
}

impl<S> FromRequestParts<S> for CurrentUser
where
	S: Send + Sync,
	Arc<AppState>: FromRef<S>,
{
	type Rejection = (StatusCode, &'static str);

	async fn from_request_parts(
		parts: &mut Parts, state: &S
	) -> Result<Self, Self::Rejection> {
		let cookies = CookieJar::from_headers(&parts.headers);
		let Some(token) = cookies
			.get("Authorization")
			.and_then(move |c| Some(c.value()))
		else {
			return Ok(Self(None));
		};
		let state = Arc::<AppState>::from_ref(state);
		let Some(session) = state.session
			.get_by_token(&token)
			.await
		else {
			return Ok(Self(None));
		};

		if Utc::now() > session.expires {
			let _ = state.session.delete_by_token(&token).await;
			return Ok(Self(None));
		}

		let user = state.user.get_by_id(session.user).await;

		Ok(Self(user))
	}
}
