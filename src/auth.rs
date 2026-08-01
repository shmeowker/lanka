use axum::{
	RequestPartsExt,
  extract::Extension,
  http::{
		HeaderMap,
		header::AUTHORIZATION, 
		request::Parts,
	},
};
use crate::{
	Arc,
	AppState,
	Deref,
	FromRequestParts,
	FromRef,
	LState,
	StatusCode,
	User,
	Utc,
};

#[derive(Deref)]
pub struct CurrentUser {
  user: Option<User>,
}

impl CurrentUser {
	pub fn is_some(&self) -> bool {
		self.user.is_some()
	}
}

impl<S> FromRequestParts<S> for CurrentUser
where 
	S: Send + Sync,
	Arc<AppState>: FromRef<S>,
{
  type Rejection = (StatusCode, &'static str);

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		let headers = HeaderMap::from_request_parts(parts, state).await;
		let Some(token) = headers
			.as_ref()
    	.ok()
    	.and_then(|h| h.get(AUTHORIZATION))
    	.and_then(|v| v.to_str().ok())
			.and_then(|s| s.strip_prefix("Bearer "))
		else {
    	return Ok(Self { user: None });
		};
		
		let Extension(state) = parts.extract::<Extension<LState>>()
			.await
			.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "State extraction failed."))?;
	
		let Some(session) = state.session.get_by_token(&token).await else {
			return Ok(Self { user: None });
		};

		if session.expires.is_some() && Utc::now() > session.expires.unwrap() {
			let _ = state.session.delete_by_token(token).await;
			return Ok(Self { user: None });
		}

		let user = state.user.get_by_id(session.user).await;

		Ok(Self { user: user })
	}
}
