use crate::{Template, IntoResponse, StatusCode, Response, Html};
use std::borrow::Cow;


#[derive(Template)]
#[template(path = "error.html")]
pub struct Rejection {
    code: StatusCode,
    message: Cow<'static, str>,
}

impl IntoResponse for Rejection {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error message template rendering failed: {err}"),
            )
                .into_response(),
        }
    }
}

impl From<(StatusCode, &'static str)> for Rejection {
    fn from((code, message): (StatusCode, &'static str)) -> Rejection {
        Rejection { code, message: Cow::Borrowed(message) }
    }
}

impl From<(StatusCode, String)> for Rejection {
    fn from((code, message): (StatusCode, String)) -> Rejection {
        Rejection { code, message: Cow::Owned(message) }
    }
}