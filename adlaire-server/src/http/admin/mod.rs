use std::convert::Infallible;

use hyper::{body::Incoming, Request};

use crate::{http::HttpResponse, state::SharedState};

fn not_implemented() -> Result<HttpResponse, Infallible> {
    Ok(crate::http::json_error(
        ::http::StatusCode::NOT_IMPLEMENTED,
        "NOT_IMPLEMENTED",
        "not implemented",
    ))
}

pub mod databases {
    use super::*;

    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn delete(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }
}

pub mod tokens {
    use super::*;

    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn revoke(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }
}

pub mod metrics {
    use super::*;

    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }
}

pub mod backup {
    use super::*;

    pub async fn backup(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn restore(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn pitr(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }
}

pub mod branches {
    use super::*;

    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn delete(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }
}
