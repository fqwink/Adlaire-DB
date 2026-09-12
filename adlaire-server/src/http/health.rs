use std::convert::Infallible;

use hyper::{body::Incoming, Request};

use crate::{http::HttpResponse, state::SharedState};

pub async fn handle(
    _req: Request<Incoming>,
    _state: SharedState,
) -> Result<HttpResponse, Infallible> {
    Ok(crate::http::json_ok(&serde_json::json!({ "status": "ok" })))
}
