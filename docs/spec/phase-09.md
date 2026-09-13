### Phase 9：WebSocket（hrana-ws v3）

**目標**：Turso のインタラクティブトランザクション（hrana-ws v3）が動作する

#### hrana-ws v3 プロトコル概要

hrana-ws は libSQL / Turso の WebSocket ワイヤプロトコル。HTTP の hrana-http v2 と異なり、ステートフルな接続上でインタラクティブトランザクションを実現する。

参照仕様: [libSQL/sqld/docs/HRANA_3_SPEC.md](https://github.com/tursodatabase/libsql/blob/main/libsql-server/docs/HRANA_3_SPEC.md)

#### エンドポイント

```
ws://host:8080/v3/baton      ← hrana-ws v3
wss://host:8080/v3/baton     ← TLS 経由（リバースプロキシ）
```

#### 接続フロー

```
Client                          Server
  |                               |
  |── WebSocket Upgrade ─────────>|
  |<─ 101 Switching Protocols ───|
  |                               |
  |── ClientMsg: hello ──────────>|  jwt: "<JWT>"
  |<─ ServerMsg: hello ──────────|  OK or error
  |                               |
  |── ClientMsg: request ────────>|  request_id, stream_id, body
  |<─ ServerMsg: response_ok ────|  request_id, result
  |    or response_error          |
  |                               |
  |── ClientMsg: close_stream ──>|
  |── WebSocket Close ───────────>|
```

#### メッセージ型

**ClientMsg（クライアント→サーバー）：**

| type | 説明 |
|------|------|
| `hello` | 接続開始。`jwt` フィールドで認証 |
| `request` | stream_id と body を持つリクエスト |
| `close_stream` | ストリームのクローズ |

**request body の種類：**

| type | 説明 |
|------|------|
| `open_stream` | 新規ストリームを開く |
| `close_stream` | ストリームを閉じる |
| `execute` | 単一 SQL 文を実行（hrana-http の execute と同形式）|
| `batch` | 複数 SQL 文をバッチ実行 |
| `sequence` | テキスト形式の複数 SQL 文を順序実行 |
| `describe` | SQL 文のパラメータ・カラム情報を返す |
| `store_sql` / `close_sql` | SQL テキストを ID でキャッシュ（再利用） |

**ServerMsg（サーバー→クライアント）：**

| type | 説明 |
|------|------|
| `hello_ok` | 認証成功 |
| `hello_error` | 認証失敗 |
| `response_ok` | リクエスト成功。request_id + result |
| `response_error` | リクエスト失敗。request_id + error |

#### インタラクティブトランザクション

ストリーム上でトランザクション状態を保持する。

```
open_stream(stream_id=1)
execute(stream_id=1, sql="BEGIN")
execute(stream_id=1, sql="INSERT INTO t VALUES (1)")
execute(stream_id=1, sql="INSERT INTO t VALUES (2)")
execute(stream_id=1, sql="COMMIT")
close_stream(stream_id=1)
```

複数の stream を同一 WebSocket 接続上で多重化できる（stream_id で識別）。

#### libsql との統合（Phase 9）

Phase 1〜8 と同様、**Adlaire 独自の hrana-ws プロトコル変換レイヤーを実装する**（§3.3.3 の hrana-http 変換層と同じ設計方針）。libsql crate には WebSocket サーバー機能はないため、Adlaire が全て実装する。

採用理由：

- hrana-ws プロトコルのセッション管理・認証は Adlaire が既に制御している（hrana-http と同じ構造）
- Phase 1〜8 で構築した hrana-http 変換レイヤー（§3.3.3）の延長として実装でき、アーキテクチャの一貫性を保てる
- WebSocket コネクションのライフサイクル（hello / stream_id / baton 管理）を Adlaire が完全制御できる

WebSocket フレームの受受信・送信には `tokio-tungstenite` クレートを使用する。クエリ実行は Phase 1〜8 と同じ libsql crate 経路を使い、SQL execution、transaction、result mapping、permission precedence は §9.1.28 に従う。

**Phase 9 実装固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| Upgrade path | `/v3/baton` は `default` DB、`/{db-name}/v3/baton` は path DB。unknown DB は upgrade 前に `404 DB_NOT_FOUND` |
| Upgrade validation | `Connection: upgrade`、`Upgrade: websocket`、`Sec-WebSocket-Key`、`Sec-WebSocket-Version: 13` 必須。不正は HTTP 400 |
| Subprotocol | `Sec-WebSocket-Protocol` は client 提示順を保持して解釈し、server が対応する `hrana3`、`hrana2`、`hrana1` のうち最初に一致したものを選択する。`hrana3-protobuf` は Phase 9 では未対応のため選択しない |
| Auth | WebSocket upgrade 後、`hello` で認証する。client は hello 応答前に request を送ってよいが、server は hello 認証完了後に受信順で処理する。hello 失敗時は未処理 request へ response を返さず close |
| Message size | 1 frame 最大 1 MiB。超過は close code 1009 |
| request_id | connection 内で response と 1:1 対応。重複 request_id は許可するが response は受信順で返す |
| stream_id | `open_stream` 前の execute/batch/sequence/describe は response_error `INVALID_REQUEST` |
| unknown field | Hrana wire protocol の JSON object に含まれる unknown field は互換のため無視する。管理 API の unknown field 拒否方針を適用しない |
| cursor API | `open_cursor` / `fetch_cursor` / `close_cursor` は Phase 9 では response_error `NOT_IMPLEMENTED` とし、connection は維持する |
| transaction | BEGIN 後に connection close した場合は rollback。COMMIT 成功応答前に切断した場合は成功扱いにしない |
| store_sql | `sql_id` は connection 内だけ有効。未登録 id の参照と二重登録は `INVALID_REQUEST` |
| ro token | `a:"ro"` で write SQL、BEGIN IMMEDIATE/EXCLUSIVE、DDL、ATTACH write は `PERMISSION_DENIED` |
| close | 正常 close は 1000。protocol validation 失敗は 1002。server shutdown は 1012 |
| persistence | WebSocket session/stream/store_sql は永続化しない。SQL commit 済みデータだけ DB に残る |

**完了条件（テストケース）：**

```
TC-3-1: インタラクティブトランザクション
  TypeScript SDK:
  const tx = await db.transaction("write");
  await tx.execute("INSERT INTO t VALUES (1)");
  await tx.execute("INSERT INTO t VALUES (2)");
  await tx.commit();
  const r = await db.execute("SELECT COUNT(*) FROM t");
  期待: r.rows[0][0] = 2

TC-3-2: トランザクションロールバック
  const tx = await db.transaction("write");
  await tx.execute("INSERT INTO t VALUES (99)");
  await tx.rollback();
  const r = await db.execute("SELECT * FROM t WHERE id = 99");
  期待: r.rows.length = 0

TC-3-3: 複数ストリームの多重化
  stream_id=1 で BEGIN → INSERT (sleep)
  stream_id=2 で SELECT（別トランザクション）→ 正常応答
  stream_id=1 で COMMIT
  期待: 2 ストリームが干渉しない

TC-3-4: JWT 認証（WebSocket）
  hello メッセージに有効 JWT → hello_ok
  hello メッセージに不正 JWT → hello_error

TC-3-4 補足: WebSocket protocol boundary
  （a）hello 前 request → hello_error + close
  （b）open_stream 前 execute → response_error INVALID_REQUEST
  （c）store_sql 未登録 id 参照 → response_error INVALID_REQUEST
  （d）1 MiB 超過 frame → close code 1009
  （e）open transaction 中に切断 → rollback
```

---


#### 実装詳細

#### 14.20 ws モジュールスタブ（Phase 3〜8）

Phase 3〜8 の間、`route()` が参照する `ws::handle()` / `ws::handle_db()` はスタブとして実装する。Phase 9 で WebSocket upgrade に差し替える。

```rust
// ws/mod.rs  — Phase 3〜8 スタブ

use hyper::{Request, Response, body::Incoming};
use http_body_util::Full;
use bytes::Bytes;
use std::convert::Infallible;
use crate::state::SharedState;

/// hrana-ws v3 WebSocket ハンドラ（シングル DB）。Phase 9 で実装。
pub async fn handle(
    _req: Request<Incoming>,
    _state: SharedState,
) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::builder()
        .status(http::StatusCode::NOT_IMPLEMENTED)
        .body(Full::default())
        .unwrap())
}

/// hrana-ws v3 WebSocket ハンドラ（マルチ DB）。Phase 9 で実装。
pub async fn handle_db(
    _req: Request<Incoming>,
    _state: SharedState,
    _db_name: &str,
) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::builder()
        .status(http::StatusCode::NOT_IMPLEMENTED)
        .body(Full::default())
        .unwrap())
}
```

#### 14.9 WebSocket セッション管理（Phase 9）

```rust
// ws/types.rs  ─ hrana-ws v3 メッセージ型定義

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestBody {
    OpenStream,
    CloseStream,
    Execute   { stmt: Stmt },
    Batch     { batch: Vec<Stmt> },
    Sequence  { sql: String },
    Describe  { stmt: Stmt },
    StoreSql  { sql_id: u32, sql: String },
    CloseSql  { sql_id: u32 },
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseBody {
    OpenStream,
    CloseStream,
    Execute   { result: StmtResult },
    Batch     { step_results: Vec<Option<StmtResult>>, step_errors: Vec<Option<HranaError>> },
    Sequence,
    Describe  { cols: Vec<Col>, params: Vec<DescribeParam> },
    StoreSql,
    CloseSql,
}

#[derive(Debug, serde::Serialize)]
pub struct DescribeParam { pub name: Option<String> }
```

```rust
// ws/session.rs
use std::collections::HashMap;
use crate::hrana::convert::{hrana_values_to_params, sql_to_stmt_result};
use crate::db::sqld_adapter::{SqldAdapter, SqlResult, SqlValue, to_libsql_params, from_libsql_value};

// sql_to_stmt_result のローカルエイリアス（ws 内での可読性向上）
fn to_stmt_result(r: SqlResult) -> StmtResult { sql_to_stmt_result(r) }

/// WsStream の永続コネクション上で単一 Stmt を実行し SqlResult を返す。
/// stmt.want_rows が true なら query()、false なら execute() を使い分ける。
async fn exec_stmt_on_conn(
    conn: &libsql::Connection,
    stmt: &Stmt,
) -> Result<SqlResult, AppError> {
    let sql_args = hrana_values_to_params(&stmt.args)?;
    let params   = to_libsql_params(sql_args);
    if stmt.want_rows {
        let mut rows = conn.query(&stmt.sql, params).await
            .map_err(|e| AppError::Sqld(e.to_string()))?;
        let col_count = rows.column_count();
        let cols: Vec<(Option<String>, Option<String>)> = (0..col_count)
            .map(|i| (rows.column_name(i).map(|s| s.to_string()), None))
            .collect();
        let mut result_rows: Vec<Vec<SqlValue>> = vec![];
        while let Some(row) = rows.next().await.map_err(|e| AppError::Sqld(e.to_string()))? {
            let cells = (0..col_count)
                .map(|i| from_libsql_value(row.get_value(i).unwrap_or(libsql::Value::Null)))
                .collect();
            result_rows.push(cells);
        }
        Ok(SqlResult { cols, rows: result_rows, rows_affected: 0, last_insert_rowid: None })
    } else {
        let rows_affected     = conn.execute(&stmt.sql, params).await
            .map_err(|e| AppError::Sqld(e.to_string()))?;
        let last_insert_rowid = conn.last_insert_rowid();
        Ok(SqlResult { cols: vec![], rows: vec![], rows_affected, last_insert_rowid: Some(last_insert_rowid) })
    }
}

pub struct WsSession {
    db:      std::sync::Arc<dyn SqldAdapter>,  // Arc<libsql::Database> ではなくアダプタ経由
    streams: HashMap<u32, WsStream>,           // stream_id → WsStream
    auth:    Claims,
}

pub struct WsStream {
    conn:    libsql::Connection,
    tx_mode: TransactionMode,
}

#[derive(PartialEq)]
pub enum TransactionMode { None, ReadOnly, ReadWrite }

impl WsSession {
    pub fn new(db: std::sync::Arc<dyn SqldAdapter>, auth: Claims) -> Self {
        Self { db, streams: HashMap::new(), auth }
    }

    pub async fn handle_request(
        &mut self,
        stream_id: u32,
        body: RequestBody,
    ) -> Result<ResponseBody, AppError> {
        match body {
            RequestBody::OpenStream => {
                let conn = self.db.connect()?;
                self.streams.insert(stream_id, WsStream {
                    conn,
                    tx_mode: TransactionMode::None,
                });
                Ok(ResponseBody::OpenStream)
            }
            RequestBody::Execute { stmt } => {
                let stream = self.streams.get_mut(&stream_id)
                    .ok_or(AppError::InvalidRequest)?;
                // BEGIN/COMMIT/ROLLBACK でトランザクション状態を更新
                let sql_upper = stmt.sql.trim_start().to_ascii_uppercase();
                if sql_upper.starts_with("BEGIN") && sql_upper.contains("READ ONLY") {
                    stream.tx_mode = TransactionMode::ReadOnly;
                } else if sql_upper.starts_with("BEGIN") {
                    stream.tx_mode = TransactionMode::ReadWrite;
                } else if sql_upper.starts_with("COMMIT") || sql_upper.starts_with("ROLLBACK") {
                    stream.tx_mode = TransactionMode::None;
                }
                let result = exec_stmt_on_conn(&stream.conn, &stmt).await?;
                Ok(ResponseBody::Execute { result: to_stmt_result(result) })
            }
            RequestBody::CloseStream => {
                self.streams.remove(&stream_id);
                Ok(ResponseBody::CloseStream)
            }
            RequestBody::Sequence { sql } => {
                let stream = self.streams.get_mut(&stream_id).ok_or(AppError::InvalidRequest)?;
                // execute_batch に丸ごと渡すことで文字列リテラル内のセミコロンを誤分割しない
                stream.conn.execute_batch(&sql).await
                    .map_err(|e| AppError::Sqld(e.to_string()))?;
                Ok(ResponseBody::Sequence)
            }
            RequestBody::Describe { stmt } => {
                let stream = self.streams.get_mut(&stream_id).ok_or(AppError::InvalidRequest)?;
                let desc = stream.conn.prepare(&stmt.sql).await.map_err(|e| AppError::Sqld(e.to_string()))?;
                let cols = desc.columns().iter().map(|c| Col {
                    name:     c.name().map(str::to_string),
                    decltype: c.decl_type().map(str::to_string),
                }).collect();
                Ok(ResponseBody::Describe { cols, params: vec![] })
            }
            RequestBody::Batch { batch } => {
                let stream = self.streams.get_mut(&stream_id).ok_or(AppError::InvalidRequest)?;
                let mut step_results = Vec::with_capacity(batch.len());
                let mut step_errors  = Vec::with_capacity(batch.len());
                for stmt in &batch {
                    match exec_stmt_on_conn(&stream.conn, stmt).await {
                        Ok(r)  => { step_results.push(Some(to_stmt_result(r))); step_errors.push(None); }
                        Err(e) => { step_results.push(None); step_errors.push(Some(HranaError { message: e.to_string(), code: "SQLITE_ERROR".into() })); }
                    }
                }
                Ok(ResponseBody::Batch { step_results, step_errors })
            }
            // store_sql / close_sql（SQL テキストキャッシュ）は Phase 9 完了条件に含める
            RequestBody::StoreSql { .. } | RequestBody::CloseSql { .. } => {
                // 実装では statement id → SQL 文字列のキャッシュを WsSession 内に保持する
                // ここに到達する実装は Phase 9 未完了扱い
                Err(AppError::InvalidRequest)
            }
        }
    }
}
```
