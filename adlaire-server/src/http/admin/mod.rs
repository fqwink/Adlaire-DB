// Phase 6〜7 で各ハンドラを実装する。現時点はすべて 501 を返す stub。

use axum::http::StatusCode;

pub mod databases {
    use super::*;
    pub async fn list()   -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn create() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn get()    -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn delete() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}

pub mod tokens {
    use super::*;
    pub async fn list()   -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn create() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn get()    -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn revoke() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}

pub mod metrics {
    use super::*;
    pub async fn get() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}

pub mod backup {
    use super::*;
    pub async fn backup()  -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn restore() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn pitr()    -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}

pub mod branches {
    use super::*;
    pub async fn list()   -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn create() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn delete() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}
