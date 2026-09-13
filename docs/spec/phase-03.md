### Phase 3：HTTP サーバー・hrana パイプライン

**目標**：libSQL クライアント SDK が Adlaire DB に接続して SQL を実行できる最小構成

**スコープ：**
- hyper HTTP サーバー起動・SIGINT/SIGTERM ハンドラ
- GET `/v2/health`
- POST `/v2/pipeline`（hrana-http v2 完全実装）

**Phase 3 の実装境界：**

- Web フレームワーク（axum・actix-web・rocket 等）は使用しない。HTTP 受信、ルーティング、JSON パース、エラー応答は hyper ベースの自前実装とする
- `/v2/pipeline` の SQL 実行、transaction、result mapping、SQL error surface は §9.1.28 に従う
- `/v2/pipeline` は常に `default` DB を対象とする。`/{db-name}/v2/pipeline` は Phase 6 以降の経路であり、Phase 3 の完了条件には含めない
- `baton` は受け取るが Phase 3 ではセッションを保持しない。レスポンスの `baton` / `base_url` は常に `null` とする
- `execute` は positional `args` をサポートする。`named_args` が空でない場合は、その request を `results[].type="error"` として返す
- JSON 不正・未知の request type・必須フィールド欠落など、リクエスト全体を解釈できない場合は HTTP 400 `INVALID_REQUEST` を返す
- SQL 実行エラー、引数変換エラー、読み取り専用トークンによる書き込み拒否など、個別 request の失敗は HTTP 200 のまま `results[i].type="error"` として返す
- `close` を受け取ったら `close` の ok response を追加し、それ以降の request は処理しない
- JWT 認証は Phase 4 対象。Phase 3 では secret が未設定の場合のみ起動でき、全 request を `rw` の unauthenticated claims として処理する

**完了条件（テストケース）：**

```
TC-1: 認証なしモードで SQL 実行
  $ ./adlaire-db serve --data ./testdb --port 8080
  $ curl -s -X POST http://localhost:8080/v2/pipeline \
      -H "Content-Type: application/json" \
      -d '{"baton":null,"requests":[
            {"type":"execute","stmt":{"sql":"CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY, v TEXT)","args":[],"want_rows":false}},
            {"type":"execute","stmt":{"sql":"INSERT INTO t VALUES (1, \"hello\")","args":[],"want_rows":false}},
            {"type":"execute","stmt":{"sql":"SELECT * FROM t","args":[],"want_rows":true}},
            {"type":"close"}
          ]}'
  期待: results に rows=[[[integer,"1"],[text,"hello"]]] が含まれる

TC-2: ヘルスチェック
  $ curl -s http://localhost:8080/v2/health
  期待: {"status":"ok"}

TC-6: 起動・停止
  （a）./adlaire-db serve で起動 → "Adlaire DB listening on ..." ログ
  （b）Ctrl+C でクリーンシャットダウン → .lock ファイルが解放される
  （c）再起動できる（.lock がゾンビ残留しない）
```

**実装タスク：**

```
T-5: HTTP サーバー骨格（hyper）
  [ ] tokio ランタイム起動
  [ ] hyper service_fn + route(): POST /v2/pipeline, GET /v2/health
  [ ] --port でバインドアドレスを指定
  [ ] SIGINT / SIGTERM ハンドラ登録（graceful shutdown）
  [ ] "Adlaire DB listening" INFO ログ出力
  参照: §6.1, §8.1 Step 7〜8, §8.2
  検証: TC-2（ヘルスチェック）, TC-6（起動・停止）

T-6: hrana-http v2 パイプライン実装
  [ ] POST /v2/pipeline のリクエスト JSON デシリアライズ
      （baton, requests[].type, requests[].stmt.sql/args/named_args/want_rows）
  [ ] requests を libsql::Connection.query() / execute() / execute_batch() に渡す
  [ ] libsql の行・カラム型を hrana-http v2 results[] 形式に変換
      （cols, rows, rows_affected, last_insert_rowid）
  [ ] SQL エラーを results[i].type="error" として返す（HTTP 200 のまま）
  [ ] named_args が空でない execute は results[i].type="error" として返す
  [ ] "close" type リクエストを正しく処理する
  参照: §6.2, §3.3.3, §9.1.28
  検証: TC-1（SQL 実行）
```

**Phase 3 完了ゲート：**

Phase 3 は次をすべて満たした時点で完了と判定する。

1. `cargo build` と `cargo test` が成功する
2. `TC-1` / `TC-2` / `TC-6` が手動または統合テストで成功する
3. `--auth-jwt-secret` または `ADLAIRE_JWT_SECRET` 指定時は、JWT 未実装として起動を拒否する
4. `meta/databases.json` / `meta/tokens.json` / `meta/branches.json` が初回起動時に初期化される
5. 起動時の DB オープン・WAL 設定・busy timeout・integrity_check のいずれかに失敗した場合、サーバーは起動成功扱いにしない
6. malformed JSON は HTTP 400、SQL エラーは HTTP 200 + hrana error という境界が守られている
7. ソースコードに Web フレームワーク依存がない

---


#### 実装詳細

#### 14.4 ルーティング設計

```rust
// http/mod.rs

use hyper::{Request, Response, body::Incoming};
use http_body_util::Full;
use bytes::Bytes;
use std::convert::Infallible;

/// API リクエストを (メソッド, パス) でハンドラに振り分ける
pub async fn route(
    req: Request<Incoming>,
    state: SharedState,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().as_str();
    let path   = req.uri().path();

    match (method, path) {
        // Phase 1: シングル DB
        ("GET",  "/v2/health")   => health::handle(req, state).await,
        ("POST", "/v2/pipeline") => pipeline::handle(req, state, "default").await,
        // Phase 6: パスベース DB ルーティング
        ("POST", p) if p.ends_with("/v2/pipeline") => {
            match extract_db_name(p) {
                Some(db) => pipeline::handle(req, state, db).await,
                None     => Ok(not_found()),
            }
        }
        // Phase 9: WebSocket upgrade
        ("GET", "/v3/baton")   => ws::handle(req, state).await,
        ("GET", p) if p.ends_with("/v3/baton") => {
            match extract_db_name(p) {
                Some(db) => ws::handle_db(req, state, db).await,
                None     => Ok(not_found()),
            }
        }
        _ => Ok(not_found()),
    }
}

/// 管理 API リクエストを振り分ける（認証チェック込み）
pub async fn admin_route(
    req: Request<Incoming>,
    state: SharedState,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // 認証チェック
    if let Some(expected) = &state.config.admin_auth_token {
        let provided = req.headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));
        match provided {
            Some(token) if token == expected.as_str() => {}
            _ => return Ok(json_error(
                http::StatusCode::UNAUTHORIZED,
                "AUTH_REQUIRED",
                "admin authentication required",
            )),
        }
    }

    let method = req.method().as_str();
    let path   = req.uri().path();

    match (method, path) {
        // Phase 6: DB CRUD
        ("GET",    "/admin/v1/databases")        => admin::databases::list(req, state).await,
        ("POST",   "/admin/v1/databases")        => admin::databases::create(req, state).await,
        ("GET",    p) if is_db_path(p)           => admin::databases::get(req, state).await,
        ("DELETE", p) if is_db_path(p)           => admin::databases::delete(req, state).await,
        // Phase 7: トークン CRUD
        ("GET",    "/admin/v1/tokens")           => admin::tokens::list(req, state).await,
        ("POST",   "/admin/v1/tokens")           => admin::tokens::create(req, state).await,
        ("GET",    p) if is_token_path(p)        => admin::tokens::get(req, state).await,
        ("DELETE", p) if is_token_path(p)        => admin::tokens::revoke(req, state).await,
        // Phase 10: メトリクス
        ("GET",    "/admin/v1/metrics")          => admin::metrics::get(req, state).await,
        // Phase 13〜14: バックアップ・PITR
        ("GET",    p) if p.ends_with("/backup")  => admin::backup::backup(req, state).await,
        ("POST",   p) if p.ends_with("/restore") => admin::backup::restore(req, state).await,
        ("POST",   p) if p.ends_with("/restore/point-in-time") => admin::backup::pitr(req, state).await,
        // Phase 15: ブランチ
        ("GET",    p) if p.ends_with("/branches")        => admin::branches::list(req, state).await,
        ("POST",   p) if p.ends_with("/branches")        => admin::branches::create(req, state).await,
        ("DELETE", p) if p.contains("/branches/")        => admin::branches::delete(req, state).await,
        _ => Ok(not_found()),
    }
}

/// パスから db_name を抽出する（"/{db_name}/v2/pipeline" 形式）
fn extract_db_name(path: &str) -> Option<&str> {
    let mut segs = path.trim_start_matches('/').split('/');
    match (segs.next(), segs.next(), segs.next(), segs.next()) {
        (Some(db), Some("v2"), Some("pipeline"), None) if !db.is_empty() => Some(db),
        _ => None,
    }
}

fn is_db_path(p: &str) -> bool {
    let segs: Vec<_> = p.trim_start_matches('/').split('/').collect();
    matches!(segs.as_slice(), ["admin", "v1", "databases", _])
}

fn is_token_path(p: &str) -> bool {
    let segs: Vec<_> = p.trim_start_matches('/').split('/').collect();
    matches!(segs.as_slice(), ["admin", "v1", "tokens", _])
}

fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(http::StatusCode::NOT_FOUND)
        .body(Full::default())
        .unwrap()
}

fn json_error(status: http::StatusCode, code: &str, msg: &str) -> Response<Full<Bytes>> {
    let body = serde_json::to_vec(&serde_json::json!({
        "error": msg, "code": code
    })).unwrap_or_default();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::from(body))
        .unwrap()
}
```


#### 14.6 hrana 変換ロジック

```rust
// hrana/convert.rs

use crate::{db::sqld_adapter::{SqlResult, SqlValue}, error::AppError};
use super::types::{Col, StmtResult, Value};

/// hrana `Value` → SQL 実行用 `SqlValue`（整数パース失敗は InvalidRequest を返す）
pub fn hrana_to_sql(v: &Value) -> Result<SqlValue, AppError> {
    Ok(match v {
        Value::Null             => SqlValue::Null,
        Value::Integer { value } => {
            let n = value.parse::<i64>().map_err(|_| AppError::InvalidRequest)?;
            SqlValue::Integer(n)
        }
        Value::Real    { value } => SqlValue::Real(*value),
        Value::Text    { value } => SqlValue::Text(value.clone()),
        Value::Blob    { value } => {
            use base64::Engine as _;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(value)
                .map_err(|_| AppError::InvalidRequest)?;
            SqlValue::Blob(bytes)
        }
    })
}

/// hrana `Value` のスライスを `SqlValue` の `Vec` に変換する（ws/session.rs から呼び出す）
pub fn hrana_values_to_params(args: &[Value]) -> Result<Vec<SqlValue>, AppError> {
    args.iter().map(hrana_to_sql).collect()
}

/// `SqlResult` → hrana `StmtResult`
pub fn sql_to_stmt_result(r: SqlResult) -> StmtResult {
    let rows = r.rows.into_iter().map(|row| {
        row.into_iter().map(sql_val_to_hrana).collect()
    }).collect();
    StmtResult {
        cols: r.cols.into_iter().map(|(name, decltype)| Col { name, decltype }).collect(),
        rows,
        rows_affected:     r.rows_affected,
        last_insert_rowid: r.last_insert_rowid.map(|n| n.to_string()),
    }
}

fn sql_val_to_hrana(v: SqlValue) -> Value {
    match v {
        SqlValue::Null       => Value::Null,
        SqlValue::Integer(n) => Value::Integer { value: n.to_string() },
        SqlValue::Real(f)    => Value::Real    { value: f },
        SqlValue::Text(s)    => Value::Text    { value: s },
        SqlValue::Blob(b)    => {
            use base64::Engine as _;
            Value::Blob { value: base64::engine::general_purpose::STANDARD.encode(&b) }
        }
    }
}
```


#### 14.15 HTTP ハンドラ実装

```rust
// http/pipeline.rs

use hyper::{Request, Response, body::Incoming};
use http_body_util::{Full, BodyExt};
use bytes::Bytes;
use std::convert::Infallible;

/// リクエストボディを読み切り JSON にデシリアライズする
/// デシリアライズ失敗は 400 INVALID_REQUEST
async fn parse_json_body<T: serde::de::DeserializeOwned>(
    req: Request<Incoming>,
) -> Result<T, AppError> {
    let bytes = req.into_body().collect().await
        .map_err(|_| AppError::InvalidRequest)?.to_bytes();
    serde_json::from_slice::<T>(&bytes).map_err(|_| AppError::InvalidRequest)
}

fn json_ok<T: serde::Serialize>(body: &T) -> Response<Full<Bytes>> {
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    Response::builder()
        .status(http::StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::from(bytes))
        .unwrap()
}

// 単一 DB ハンドラ（Phase 1）
pub async fn handle(
    req: Request<Incoming>,
    state: SharedState,
    db_name: &str,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let claims = match extract_claims(&req, &state).await {
        Ok(c)  => c,
        Err(e) => return Ok(e.into_response()),
    };
    let pipeline_req = match parse_json_body::<PipelineRequest>(req).await {
        Ok(r)  => r,
        Err(e) => return Ok(e.into_response()),
    };
    let db = match state.db_mgr.get(db_name).await {
        Some(d) => d,
        None    => return Ok(AppError::DbNotFound(db_name.to_string()).into_response()),
    };
    match execute_pipeline(&db, &claims, &pipeline_req.requests, db_name).await {
        Ok(results) => Ok(json_ok(&PipelineResponse { baton: None, base_url: None, results })),
        Err(e)      => Ok(e.into_response()),
    }
}

// マルチ DB ハンドラ（Phase 6）— db_name をパスから受け取る
pub async fn handle_db(
    req: Request<Incoming>,
    state: SharedState,
    db_name: &str,
) -> Result<Response<Full<Bytes>>, Infallible> {
    handle(req, state, db_name).await
}

async fn execute_pipeline(
    db: &Arc<dyn SqldAdapter>,
    claims: &Claims,
    requests: &[StreamRequest],
    db_name: &str,
) -> Result<Vec<StreamResult>, AppError> {
    let mut responses = Vec::with_capacity(requests.len());
    for req in requests {
        match req {
            StreamRequest::Execute { stmt } => {
                if !stmt.named_args.is_empty() {
                    responses.push(StreamResult::Error {
                        error: HranaError {
                            message: "named arguments are not supported yet".into(),
                            code: "SQLITE_ERROR".into(),
                        },
                    });
                    continue;
                }

                if is_write_stmt(&stmt.sql) && claims.resolve_access(db_name) != AccessLevel::Rw {
                    responses.push(StreamResult::Error {
                        error: HranaError {
                            message: "write not permitted".into(),
                            code: "PERMISSION_DENIED".into(),
                        },
                    });
                    continue;
                }
                let sql_args: Result<Vec<_>, _> = stmt.args.iter().map(hrana_to_sql).collect();
                let sql_args = match sql_args {
                    Ok(a) => a,
                    Err(_) => {
                        responses.push(StreamResult::Error {
                            error: HranaError { message: "invalid argument value".into(), code: "SQLITE_ERROR".into() },
                        });
                        continue;
                    }
                };
                match db.execute(&stmt.sql, sql_args, stmt.want_rows).await {
                    Ok(result) => responses.push(StreamResult::Ok {
                        response: StreamResponse::Execute { result: sql_to_stmt_result(result) },
                    }),
                    Err(AppError::Sqld(msg)) => responses.push(StreamResult::Error {
                        error: HranaError { message: msg.clone(), code: sqld_error_code(&msg) },
                    }),
                    Err(e) => return Err(e),
                }
            }
            StreamRequest::Sequence { sql } => {
                // execute_batch に丸ごと渡すことで文字列リテラル内のセミコロンを誤分割しない
                match db.execute_batch(sql).await {
                    Ok(()) => responses.push(StreamResult::Ok {
                        response: StreamResponse::Sequence,
                    }),
                    Err(AppError::Sqld(msg)) => responses.push(StreamResult::Error {
                        error: HranaError { message: msg.clone(), code: sqld_error_code(&msg) },
                    }),
                    Err(e) => return Err(e),
                }
            }
            StreamRequest::Close => {
                responses.push(StreamResult::Ok { response: StreamResponse::Close });
                break;
            }
        }
    }
    Ok(responses)
}

fn sqld_error_code(msg: &str) -> String {
    if msg.contains("UNIQUE constraint") {
        "SQLITE_CONSTRAINT".to_string()
    } else {
        "SQLITE_ERROR".to_string()
    }
}


// 書き込み文プレフィックス判定
// 注意: CTE を使った書き込み（WITH ... INSERT）は検出できない。
//       SQLite が SQLITE_READONLY を返すため実害はない。
fn is_write_stmt(sql: &str) -> bool {
    let upper = sql.trim_start().to_ascii_uppercase();
    matches!(upper.split_whitespace().next().unwrap_or(""),
        "INSERT" | "UPDATE" | "DELETE" | "CREATE" | "DROP" | "ALTER" | "REPLACE" | "PRAGMA"
    )
}

// ヘルスチェックハンドラ
// http/health.rs
pub async fn handle(
    _req: Request<Incoming>,
    _state: SharedState,
) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(json_ok(&serde_json::json!({ "status": "ok" })))
}
```

---
