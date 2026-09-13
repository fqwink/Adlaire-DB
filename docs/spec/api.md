## 6. API契約仕様

URL、HTTP API、WebSocket API、管理 API、エラー形式、エラーコードを固定する責務である。外部 observable な response、status、wire format、error code はこの責務に従う。

### 6.1 API 仕様責務

### 6.1 URL 設計

#### Phase 1〜5（シングル DB）

```
POST http://localhost:8080/v2/pipeline
```

DB 名は URL に含まない。起動時に `--data` で指定した単一 DB を使用する。

#### Phase 6（マルチ DB・パスベースルーティング）

```
POST http://localhost:8080/{db-name}/v2/pipeline
```

DB 名を URL の第 1 セグメントに含める。Turso Cloud のサブドメインベース（`dbname.org.turso.io`）とは異なり、Adlaire DB ではパスベースを採用する（セルフホストでのサブドメイン設定コストを回避するため）。

### 6.2 HTTP API（hrana-http v2）

Turso Cloud・libSQL クライアント SDK が使用する hrana-http プロトコルを実装する。

#### POST /v2/pipeline

バッチ SQL 実行のメインエンドポイント。

**リクエスト：**

```
POST /v2/pipeline
Authorization: Bearer <JWT>
Content-Type: application/json

{
  "baton": null,
  "requests": [
    {
      "type": "execute",
      "stmt": {
        "sql": "SELECT id, name FROM users WHERE id = ?",
        "args": [{ "type": "integer", "value": "1" }],
        "named_args": [],
        "want_rows": true
      }
    },
    {
      "type": "close"
    }
  ]
}
```

`baton`：セッション継続識別子。`null` で新規セッション、前回レスポンスの `baton` で継続。
`type`：`"execute"`（SQL 実行）/ `"close"`（セッション終了）/ `"sequence"`（スクリプト実行）。

**Phase 3 の制約：**

- `baton` は互換性のために受理するが、サーバー側セッションは保持しない。レスポンスでは常に `baton: null` を返す
- `stmt.args` は positional 引数として処理する
- `stmt.named_args` は Phase 3 では未対応。空配列または省略のみ有効とし、非空の場合は当該 request を `results[].type="error"` として返す
- `want_rows=true` の場合は `query()`、`want_rows=false` の場合は `execute()` を使用する
- `sequence` は `execute_batch()` に SQL 文字列全体を渡し、セミコロンによる自前分割は行わない
- `close` は ok response を返し、その後の request は処理しない

**引数の型：**

| `type` | 説明 |
|--------|------|
| `"integer"` | 整数（value は文字列表現）|
| `"real"` | 浮動小数点 |
| `"text"` | 文字列 |
| `"blob"` | バイナリ（base64 エンコード）|
| `"null"` | NULL |

**レスポンス 200 OK：**

```json
{
  "baton": null,
  "base_url": null,
  "results": [
    {
      "type": "ok",
      "response": {
        "type": "execute",
        "result": {
          "cols": [
            { "name": "id",   "decltype": "INTEGER" },
            { "name": "name", "decltype": "TEXT" }
          ],
          "rows": [
            [{ "type": "integer", "value": "1" }, { "type": "text", "value": "Alice" }]
          ],
          "rows_affected": 0,
          "last_insert_rowid": null
        }
      }
    },
    {
      "type": "ok",
      "response": { "type": "close" }
    }
  ]
}
```

**エラー時（SQL エラー等）：**

```json
{
  "baton": null,
  "base_url": null,
  "results": [
    {
      "type": "error",
      "error": {
        "message": "no such table: users",
        "code": "SQLITE_ERROR"
      }
    }
  ]
}
```

**HTTP ステータスの扱い：**

| 条件 | HTTP | body |
|------|------|------|
| JSON が壊れている / 必須フィールド欠落 / request type 不明 | 400 | `{"error":"...","code":"INVALID_REQUEST"}` |
| DB が存在しない | 404 | `{"error":"database not found: ...","code":"DB_NOT_FOUND"}` |
| SQL エラー | 200 | `results[i].type="error"` |
| 引数変換エラー | 200 | `results[i].type="error"` |
| 権限不足による書き込み拒否 | 200 | `results[i].type="error","code":"PERMISSION_DENIED"` |

#### GET /v2/health

ヘルスチェックエンドポイント（認証不要）。

```
GET /v2/health

Response 200:
{ "status": "ok" }
```

### 6.3 WebSocket API（hrana-ws v3、Phase 9）

接続先：`ws://localhost:8080/v3/baton`（マルチ DB 時は `ws://localhost:8080/{db-name}/v3/baton`）

#### 接続・認証

```json
// Client → Server: hello
{"type": "hello", "jwt": "<JWT>"}

// Server → Client: hello_ok
{"type": "hello_ok"}

// Server → Client: hello_error（認証失敗）
{"type": "hello_error", "error": {"message": "...", "code": "AUTH_INVALID"}}
```

#### ストリームオープン・クローズ

```json
// Client → Server: open_stream
{"type": "request", "request_id": 1, "stream_id": 1,
 "body": {"type": "open_stream"}}

// Server → Client: response_ok
{"type": "response_ok", "request_id": 1,
 "response": {"type": "open_stream"}}

// Client → Server: close_stream
{"type": "request", "request_id": 99, "stream_id": 1,
 "body": {"type": "close_stream"}}
```

#### SQL 実行（execute）

```json
// Client → Server
{"type": "request", "request_id": 2, "stream_id": 1,
 "body": {
   "type": "execute",
   "stmt": {"sql": "INSERT INTO t VALUES (?)", "args": [{"type":"integer","value":"42"}], "want_rows": false}
 }}

// Server → Client: 成功
{"type": "response_ok", "request_id": 2,
 "response": {
   "type": "execute",
   "result": {"cols": [], "rows": [], "rows_affected": 1, "last_insert_rowid": "42"}
 }}

// Server → Client: SQL エラー
{"type": "response_error", "request_id": 2,
 "error": {"message": "no such table: t", "code": "SQLITE_ERROR"}}
```

#### インタラクティブトランザクション

```json
// BEGIN
{"type":"request","request_id":10,"stream_id":1,
 "body":{"type":"execute","stmt":{"sql":"BEGIN","args":[],"want_rows":false}}}

// INSERT
{"type":"request","request_id":11,"stream_id":1,
 "body":{"type":"execute","stmt":{"sql":"INSERT INTO t VALUES (1)","args":[],"want_rows":false}}}

// COMMIT
{"type":"request","request_id":12,"stream_id":1,
 "body":{"type":"execute","stmt":{"sql":"COMMIT","args":[],"want_rows":false}}}
```

ストリームが閉じられる前にプロセスが落ちた場合、SQLite のトランザクションは自動ロールバックされる。

### 6.4 管理 API

管理 API は独立したポート（デフォルト 8081）で提供する。外部に公開しないことを推奨する。

#### 管理 API 認証

管理ポートへのすべてのリクエストに `Authorization: Bearer <admin-token>` を要求する。

- `admin-token` は config.toml の `[admin] auth_token` または `--admin-auth-token` フラグで設定する
- 未設定時は認証を無効化する（開発・ローカル用。本番では必ず設定すること）
- 認証失敗時: `401 {"error":"unauthorized","code":"AUTH_REQUIRED"}`
- 管理トークンは JWT ではなく任意の文字列で良い（内部的には Bearer 文字列の完全一致で検証）

#### 管理 API 共通契約

Phase 7 以降の `/admin/v1/*` は、個別 endpoint で明記がない限り以下を必須とする。

| 項目 | 固定仕様 |
|------|----------|
| Content-Type | request body を持つ JSON API は `application/json` のみ受理する。未指定または他 media type は `400 INVALID_REQUEST` |
| unknown field | `INVALID_REQUEST`。後方互換のため明記された field 以外を黙って無視しない |
| null | schema で `null 可` と明記された field 以外の `null` は `INVALID_REQUEST` |
| empty body | body なし endpoint に body がある場合は `INVALID_REQUEST` |
| path id | URL decode 後、対応する name/id validation を実行する。decode 不能は `INVALID_REQUEST` |
| list order | 作成日時昇順。同一 `created_at` は `id` 昇順 |
| pagination | `limit` 既定 100、最大 500、最小 1。`cursor` は opaque string。無効 cursor は `INVALID_REQUEST` |
| response time | `created_at`、`updated_at`、`loaded_at` は RFC3339 UTC 秒精度。ミリ秒を返さない |
| delete | 削除済みまたは存在しない resource は、個別 endpoint が冪等 204 と明記しない限り 404 |
| error body | 必ず `{"error": "...", "code": "..."}`。追加 field は返さない |
| secret | admin token、JWT、replication token、HA token、extension path の実 OS 絶対パスは response/log に出さない |

#### Turso Platform API 互換面（Phase 8）

Phase 8 では、Adlaire 独自の `/admin/v1/*` に加えて Turso Cloud Platform API 互換の `/v1/*` を提供する。既存 `/admin/v1/*` は Adlaire 内部管理 API として維持し、Turso 互換を名乗る SDK/CLI/API クライアント向けには `/v1/*` を正とする。

| 項目 | `/admin/v1/*` | `/v1/*` Turso 互換 |
|------|---------------|--------------------|
| 目的 | Adlaire 自己ホスト管理 | Turso Platform API 互換 |
| 認証 | Admin token | Platform token（実体は Admin token と同じ Bearer 完全一致） |
| DB 作成成功 | 201 `DbInfo` | 200 `{"database": TursoDatabaseInfo}` |
| Group 作成成功 | 201 `GroupInfo` | 200 `{"group": TursoGroupInfo}` |
| Location 一覧 | 200 `{"locations":[LocationInfo]}` | 200 `{"locations": {"<code>":"<display_name>"}}` |
| Organization 一覧 | 200 `{"organizations":[OrganizationInfo]}` | 200 `[TursoOrganizationInfo]` |
| Organization usage | 200 `{"usage":[UsageInfo]}` | 200 `{"organization": TursoOrganizationUsage}` |
| DB token 作成 | `/admin/v1/tokens` | `/v1/organizations/{organizationSlug}/databases/{databaseName}/auth/tokens` |
| unknown field | `INVALID_REQUEST` | Turso 互換 endpoint でも `INVALID_REQUEST`。互換のため黙って無視しない |
| error body | Adlaire error body | Adlaire error body。HTTP status は Turso 互換を優先し、`code` は §7.3 を使う |

`/v1/*` の Platform token は Phase 8 では独立した secret として新設しない。`[admin] auth_token`、`--admin-auth-token`、`ADLAIRE_ADMIN_TOKEN` の有効値と `Authorization: Bearer <token>` が完全一致した場合のみ認証成功とする。Authorization ヘッダなしは `401 AUTH_REQUIRED`、Bearer 形式不正または token 不一致は `401 AUTH_INVALID` とし、token 値の前後空白、大小文字差、prefix の欠落を補正してはならない。

`/v1/*` は Turso Cloud の URL と完全同一 host である必要はないが、path、method、query、主要 response wrapper、field casing は Turso Cloud 互換にする。自己ホストで再現できない field は null や省略にせず、下表の固定値を返す。

| Turso field | Adlaire value |
|-------------|---------------|
| database `Hostname` | `{databaseName}-{organizationSlug}.adlaire.local` |
| database `DbId` | Adlaire DB UUID |
| database `Name` | DB 名 |
| database `regions` | group の `locations` |
| database `primaryRegion` | group の `primary` |
| database `block_reads` | database configuration の `block_reads`。既定 `false` |
| database `block_writes` | database configuration の `block_writes`。既定 `false`。quota 超過状態は DTO ではなく `QUOTA_EXCEEDED` error と usage/quota API で表す |
| group `version` | `adlaire-{spec version}`。例: `adlaire-0.64` |
| group `uuid` | Adlaire group id |
| group `locations` | group location の配列。Phase 8 では 1 要素 |
| group `primary` | group の primary location |
| group `delete_protection` | group configuration の `delete_protection`。既定 `false` |
| organization `type` | `personal`。自己ホスト単一運営者を Turso personal organization として扱う |
| organization `overages` | `false` |
| organization `require_mfa` | `false` |
| organization `blocked_reads` | `false` |
| organization `blocked_writes` | quota 超過時のみ `true` |
| organization `plan_id` | `self-hosted` |
| organization `plan_timeline` | `none` |
| organization `platform` | `adlaire` |

#### DB 管理（Phase 7）

```
GET    /admin/v1/databases               DB 一覧
POST   /admin/v1/databases               DB 作成
GET    /admin/v1/databases/{name}        DB 情報取得
DELETE /admin/v1/databases/{name}        DB 削除
```

**POST /admin/v1/databases リクエスト：**

```json
{ "name": "my-db" }
```

**POST /admin/v1/databases レスポンス（201 Created）：**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "my-db",
  "created_at": "2026-09-10T12:00:00Z"
}
```

**GET /admin/v1/databases レスポンス（200 OK）：**

```json
{
  "databases": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "my-db",
      "created_at": "2026-09-10T12:00:00Z",
      "size_bytes": 4096
    }
  ]
}
```

**GET /admin/v1/databases/{name} レスポンス（200 OK）：**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "my-db",
  "created_at": "2026-09-10T12:00:00Z",
  "size_bytes": 4096
}
```

**DELETE /admin/v1/databases/{name} レスポンス：** `204 No Content`（ボディなし）

DB 名バリデーション規則：

| Phase | 作成時 validation | 読み取り/接続時 validation |
|-------|-------------------|----------------------------|
| Phase 1〜7 | `^[a-zA-Z0-9_-]{1,127}$` | 同左 |
| Phase 8 以降 | Turso 互換名 `^[a-z0-9-]{1,64}$` | 既存 Phase 1〜7 DB は legacy 名として読み取り・接続・削除を許可する |

Phase 8 以降の新規 DB 作成では、大文字、underscore、64 文字超過を `INVALID_DB_NAME` とする。既存 metadata に含まれる legacy 名は migration 時に rename せず、`legacy_name:true` を付与して後方互換として維持する。パストラバーサル文字（`/` `.` `..`）と予約語 `meta`・`admin` は全 Phase で不可とする。

#### トークン管理（Phase 7）

```
POST   /admin/v1/tokens          トークン発行
GET    /admin/v1/tokens          発行済みトークン一覧
GET    /admin/v1/tokens/{id}     トークン詳細
DELETE /admin/v1/tokens/{id}     トークン失効（revoke）
```

**POST /admin/v1/tokens リクエスト：**

```json
{
  "access": "rw",
  "expiry": "30d"
}
```

`expiry` フォーマット: `<数値><単位>` 形式。単位は `s`（秒）・`m`（分）・`h`（時間）・`d`（日）。省略時は無期限（JWT に `exp` クレームを含めない）。

**POST /admin/v1/tokens レスポンス（201 Created）：**

```json
{
  "id": "tok_abc123",
  "token": "eyJ...",
  "access": "rw",
  "created_at": "2026-09-10T12:00:00Z",
  "expires_at": "2026-10-10T12:00:00Z"
}
```

`token` フィールドはこのレスポンスでのみ返す。以降の GET では含まない。

**GET /admin/v1/tokens レスポンス（200 OK）：**

```json
{
  "tokens": [
    {
      "id": "tok_abc123",
      "access": "rw",
      "created_at": "2026-09-10T12:00:00Z",
      "expires_at": "2026-10-10T12:00:00Z",
      "revoked": false,
      "revoked_at": null
    }
  ]
}
```

**GET /admin/v1/tokens/{id} レスポンス（200 OK）：** 上記リスト要素と同形式（単一オブジェクト）。

**DELETE /admin/v1/tokens/{id} レスポンス：** `204 No Content`（ボディなし）。`tokens.json` の `revoked` を `true` に更新し、`revoked_at` に失効日時を記録する。既に失効済みの場合も `204` を返す（冪等）。

#### バックアップ・エクスポート（Phase 13〜14）

```
GET  /admin/v1/databases/{name}/backup                   オンラインバックアップ（SQLite ファイル）
POST /admin/v1/databases/{name}/restore                  バックアップファイルからリストア
POST /admin/v1/databases/{name}/restore/point-in-time    WAL アーカイブから特定時点へリストア
```

**GET /admin/v1/databases/{name}/backup レスポンス（200 OK）：**

- `Content-Type: application/octet-stream`
- `Content-Disposition: attachment; filename="{name}.db"`
- SQLite Online Backup API（`sqlite3_backup_*`）を使用したライブバックアップ
- バックアップ完了まで WAL チェックポイントを一時抑制し、書き込み中でも整合性を保証する
- バックアップ中も読み書きリクエストを継続受理する（ロックしない）

**POST /admin/v1/databases/{name}/restore リクエスト：**

- `Content-Type: application/octet-stream`（SQLite DB ファイルのバイナリ）
- DB への新規クエリ受け付けを一時停止し、受信ファイルを `data.db` へ書き込む
- 既存の WAL ファイル（`data.db-wal`）を削除しチェックポイント済み状態にする
- `PRAGMA integrity_check` でリストア後の整合性を確認し、失敗時は元の DB を復元して `409` を返す
- 成功時はリストア済み DB をオープンして受け付け再開

**POST /admin/v1/databases/{name}/restore レスポンス：** `204 No Content`

**POST /admin/v1/databases/{name}/restore/point-in-time リクエスト：**

```json
{ "timestamp": "2026-09-10T12:00:00Z" }
```

または

```json
{ "frame_no": 42 }
```

`timestamp` と `frame_no` はいずれか一方。両方指定時は `400 INVALID_REQUEST`。

処理フロー：

1. `wal_retention_days = 0` の場合: `503 {"error":"PITR_NOT_ENABLED","code":"PITR_NOT_ENABLED"}`
2. `wal-archive/` ディレクトリから `timestamp` 以前または指定 `frame_no` 以下のフレームを収集
3. 対象フレームが存在しない場合: `404 {"error":"FRAME_NOT_FOUND","code":"FRAME_NOT_FOUND"}`
4. DB を一時停止し、ベーススナップショットへ WAL フレームをリプレイして復元
5. `PRAGMA integrity_check` で整合性確認
6. 成功時: `204 No Content`

**POST /admin/v1/databases/{name}/restore/point-in-time レスポンス：** `204 No Content`

#### ブランチ管理（Phase 15）

```
POST   /admin/v1/databases/{name}/branches                 ブランチ作成
GET    /admin/v1/databases/{name}/branches                 ブランチ一覧
DELETE /admin/v1/databases/{name}/branches/{branch-name}   ブランチ削除
```

ブランチ DB は `{data-dir}/databases/{name}___{branch-name}/` に作成される独立した DB である。
ブランチ DB は通常の DB と同様に `/{name}___{branch-name}/v2/pipeline` でアクセス可能（Phase 6 以降の DB ルーティングを使用）。
ブランチ名のバリデーション規則は DB 名と同じ（`^[a-zA-Z0-9_-]{1,127}$`）。

**POST /admin/v1/databases/{name}/branches リクエスト：**

```json
{
  "branch_name": "feature-x",
  "from": "current"
}
```

`from` フィールド：

| 値 | 説明 |
|----|------|
| `"current"` | 現在の DB 状態のスナップショットからブランチを作成 |
| `{"timestamp":"2026-09-10T12:00:00Z"}` | WAL アーカイブから指定時点のスナップショットからブランチを作成（`wal_retention_days > 0` 必須） |
| `{"frame_no":42}` | 指定 WAL フレーム時点のスナップショットからブランチを作成 |

処理フロー（`from: "current"` の場合）：

1. `GET /admin/v1/databases/{name}/backup` と同じ Online Backup API でスナップショットを取得
2. `{data-dir}/databases/{name}___{branch-name}/data.db` へ書き込む
3. 新 DB を通常の DB として登録し libsql::Builder::new_local() でオープンする

処理フロー（`from: {timestamp}` または `{frame_no}` の場合）：

1. WAL アーカイブから指定時点のベーススナップショットを取得
2. 対象フレームまで WAL リプレイして復元
3. 復元した DB を `{name}___{branch-name}` として登録しオープン

**POST /admin/v1/databases/{name}/branches レスポンス（201 Created）：**

```json
{
  "branch_name": "feature-x",
  "source_db": "my-db",
  "db_name": "my-db___feature-x",
  "created_at": "2026-09-10T12:00:00Z",
  "from_frame": 42
}
```

`from_frame` は `from: "current"` の場合でも実際にコピーされた時点の WAL フレーム番号を返す。

**GET /admin/v1/databases/{name}/branches レスポンス（200 OK）：**

```json
{
  "branches": [
    {
      "branch_name": "feature-x",
      "source_db": "my-db",
      "db_name": "my-db___feature-x",
      "created_at": "2026-09-10T12:00:00Z",
      "from_frame": 42
    }
  ]
}
```

ブランチのメタデータは `{data-dir}/meta/branches.json` に保存する。

**DELETE /admin/v1/databases/{name}/branches/{branch-name} レスポンス：** `204 No Content`

ブランチ DB（`{data-dir}/databases/{name}___{branch-name}/`）を削除し、`branches.json` から除去する。ブランチが存在しない場合も `204` を返す（冪等）。

#### メトリクス（Phase 10）

```
GET /admin/v1/metrics     全 DB のメトリクス取得
```

**GET /admin/v1/metrics レスポンス（200 OK）：**

```json
{
  "uptime_seconds": 3600,
  "tokens_total": 5,
  "tokens_revoked": 1,
  "databases": [
    {
      "name": "my-db",
      "queries_total": 1234,
      "rows_read_total": 5678,
      "rows_written_total": 91,
      "connections_active": 2,
      "size_bytes": 4096,
      "wal_size_bytes": 1024
    }
  ]
}
```

---

### 6.2 エラー契約責務

### 7.1 HTTP ステータスコード

| ステータス | 使用ケース |
|-----------|-----------|
| 200 OK | 正常処理（SQL エラーも 200 で results に含める）|
| 400 Bad Request | リクエスト JSON の形式エラー |
| 401 Unauthorized | JWT なし・JWT 検証失敗 |
| 402 Payment Required | Turso Platform API 互換面で plan/quota 相当の制限により操作できない |
| 403 Forbidden | 権限不足（ro トークンで書き込み等）|
| 404 Not Found | 存在しない DB・エンドポイント |
| 405 Method Not Allowed | 既知 path に未定義 method が指定された |
| 406 Not Acceptable | `Accept` header が endpoint の response media type と一致しない |
| 409 Conflict | 既存 resource との競合・integrity/HA 状態不整合 |
| 413 Payload Too Large | request body が endpoint の上限を超えた |
| 501 Not Implemented | 未来 Phase の stub endpoint。成功応答として扱わない |
| 503 Service Unavailable | storage busy、replication timeout、usage unavailable、leader unavailable |
| 500 Internal Server Error | サーバー内部エラー |

### 7.2 エラーレスポンス形式

```json
{
  "error": "database not found: my-db",
  "code":  "DB_NOT_FOUND"
}
```

### 7.3 エラーコード一覧

| コード | HTTP | 説明 |
|--------|------|------|
| `AUTH_REQUIRED` | 401 | Authorization ヘッダがない |
| `AUTH_INVALID` | 401 | JWT 署名検証失敗・失効済みトークン |
| `AUTH_EXPIRED` | 401 | JWT exp 切れ |
| `AUTH_DISABLED` | 401 | 認証無効モードで実行禁止の管理・HA・extension 操作へのアクセス |
| `PERMISSION_DENIED` | 403 | ro トークンで書き込み操作 |
| `DB_NOT_FOUND` | 404 | 指定 DB が存在しない |
| `TOKEN_NOT_FOUND` | 404 | 指定トークン ID が存在しない |
| `ORG_NOT_FOUND` | 404 | 指定 organization が存在しない |
| `GROUP_NOT_FOUND` | 404 | 指定 group が存在しない |
| `LOCATION_NOT_FOUND` | 404 | 指定 location が存在しない |
| `ENDPOINT_NOT_FOUND` | 404 | 指定 endpoint path が存在しない |
| `METHOD_NOT_ALLOWED` | 405 | 既知 endpoint path に未定義 HTTP method が指定された |
| `NOT_ACCEPTABLE` | 406 | `Accept` header が endpoint の response media type と一致しない |
| `ORG_ALREADY_EXISTS` | 409 | 同名 organization または slug が既に存在する |
| `GROUP_ALREADY_EXISTS` | 409 | 同一 organization 内に同名 group または slug が既に存在する |
| `LOCATION_ALREADY_EXISTS` | 409 | 同名 location が既に存在する |
| `QUOTA_EXCEEDED` | 403 / 402※ | quota 上限を超える操作 |
| `USAGE_UNAVAILABLE` | 503 | usage 計測値を取得できない |
| `ORG_SCOPE_DENIED` | 403 | organization/group scope 外の操作 |
| `DB_ALREADY_EXISTS` | 409 | 同名 DB が既に存在する |
| `INVALID_DB_NAME` | 400 | DB 名がバリデーションを通過しない |
| `INVALID_REQUEST` | 400 | リクエスト JSON が不正 |
| `PAYLOAD_TOO_LARGE` | 413 | request body が endpoint の上限を超えた |
| `SQLITE_ERROR` | 200※ | SQL 構文・実行エラー |
| `SQLITE_CONSTRAINT` | 200※ | 制約違反（UNIQUE 等） |
| `STORAGE_BUSY` | 503 | WAL ロック待機タイムアウト |
| `REPLICATION_TIMEOUT` | 503 | sync モードでレプリカ ACK タイムアウト |
| `PITR_NOT_ENABLED` | 503 | PITR 試行時に `wal_retention_days = 0` |
| `FRAME_NOT_FOUND` | 404 | PITR/ブランチ作成で指定フレームが存在しない |
| `RESTORE_INTEGRITY_FAILED` | 409 | リストア後の `integrity_check` 失敗 |
| `RESTORE_FRAME_CORRUPT` | 409 | WAL フレームの CRC32 検証失敗 |
| `EXTENSION_NOT_ALLOWED` | 403 | allowlist にない SQLite 拡張ロード |
| `EXTENSION_NOT_FOUND` | 404 | 指定 extension が存在しない |
| `EXTENSION_ALREADY_EXISTS` | 409 | 同名 extension が既に登録済み |
| `EXTENSION_SIGNATURE_INVALID` | 403 | extension sha256 / 署名検証失敗 |
| `EXTENSION_LOAD_FAILED` | 500 | SQLite extension load 失敗 |
| `HA_NO_LEADER` | 503 | HA leader が存在しない |
| `HA_SPLIT_BRAIN` | 409 | 複数 leader または term 不整合を検出 |
| `HA_PROMOTION_FAILED` | 409 | node 昇格または降格に失敗 |
| `DB_RESERVED_NAME` | 400 | `___` を含む DB 名の直接作成試行 |
| `NOT_IMPLEMENTED` | 501 | 未来 Phase の stub endpoint |
| `INTERNAL_ERROR` | 500 | サーバー内部エラー |

※ `SQLITE_ERROR` / `SQLITE_CONSTRAINT` は `POST /v2/pipeline` の HTTP レスポンスが 200 OK でも、`results[].type = "error"` として返す（hrana プロトコルの仕様）。HTTP 400 を返すのは `INVALID_REQUEST`（JSON 不正等）のみ。

※ `QUOTA_EXCEEDED` は `/admin/v1/*` と hrana write では 403、Turso Platform API 互換の `/v1/*` では Turso error status 互換を優先して 402 を返す。`code` はどちらも `QUOTA_EXCEEDED` とする。

### 7.4 エラーレスポンステストケース

```
ETC-1: 認証エラー
  （a）Authorization ヘッダなし
      POST /v2/pipeline （ヘッダなし）
      → 401 {"error":"authentication required","code":"AUTH_REQUIRED"}

  （b）Bearer プレフィックスなし
      Authorization: <rawtoken>
      → 401 {"error":"...","code":"AUTH_INVALID"}

  （c）署名が異なる JWT
      Authorization: Bearer <valid_header.valid_payload.wrong_signature>
      → 401 {"error":"...","code":"AUTH_INVALID"}

  （d）exp が過去の JWT
      Authorization: Bearer <JWT with exp=past>
      → 401 {"error":"...","code":"AUTH_EXPIRED"}

  （e）失効済みトークン（revoked:true）
      Authorization: Bearer <revoked_JWT>
      → 401 {"error":"...","code":"AUTH_INVALID"}

ETC-2: 権限エラー
  （a）ro トークンで INSERT 実行
      Authorization: Bearer <ro_token>
      POST /v2/pipeline {"requests":[{"type":"execute","stmt":{"sql":"INSERT INTO t VALUES(1)"}},...]}
      → 403 {"error":"permission denied","code":"PERMISSION_DENIED"}

  （b）ro トークンで SELECT 実行
      → 200 （SELECT は ro トークンで許可）

ETC-3: DB 未存在エラー（Phase 6〜）
  （a）存在しない DB 名でパイプライン
      POST /nonexistent-db/v2/pipeline
      → 404 {"error":"database not found: nonexistent-db","code":"DB_NOT_FOUND"}

  （b）削除済み DB 名でパイプライン
      （DB 作成→削除→同名でアクセス）
      → 404 {"error":"database not found: ...","code":"DB_NOT_FOUND"}

  （c）GET /admin/v1/databases/nonexistent
      → 404 {"error":"database not found: nonexistent","code":"DB_NOT_FOUND"}

ETC-4: SQL エラー
  （a）構文エラー
      {"stmt":{"sql":"SELEKT * FROM t"}}
      → 200 （pipeline 自体は成功）、results[0].type="error"
        {"type":"error","error":{"message":"near \"SELEKT\"...","code":"SQLITE_ERROR"}}

  （b）存在しないテーブル
      {"stmt":{"sql":"SELECT * FROM no_such_table"}}
      → results[0].type="error", code="SQLITE_ERROR"

  （c）UNIQUE 制約違反
      （同一 PRIMARY KEY で 2 回 INSERT）
      → results[0].type="error", code="SQLITE_CONSTRAINT"

ETC-5: リクエスト不正
  （a）JSON が壊れている
      POST /v2/pipeline body: "not json{"
      → 400 {"error":"invalid request body","code":"INVALID_REQUEST"}

  （b）requests フィールドがない
      POST /v2/pipeline {"baton":null}
      → 400 {"error":"...","code":"INVALID_REQUEST"}

  （c）type が不明
      {"type":"unknown_type"}
      → 400 または results[0].type="error"（hrana 仕様に従う）

ETC-6: DB 名バリデーション（Phase 6〜）
  （a）空文字列 POST /admin/v1/databases {"name":""}
      → 400 {"error":"...","code":"INVALID_DB_NAME"}

  （b）スペース含む POST /admin/v1/databases {"name":"my db"}
      → 400 {"code":"INVALID_DB_NAME"}

  （c）パストラバーサル POST /admin/v1/databases {"name":"../etc"}
      → 400 {"code":"INVALID_DB_NAME"}

  （d）127 文字以内・英数字・ハイフン・アンダースコアのみ有効
      "valid-name_123" → 201
      長さ 128 文字の文字列 → 400 {"code":"INVALID_DB_NAME"}

ETC-7: 重複エラー（Phase 6〜）
  POST /admin/v1/databases {"name":"dup"}
  POST /admin/v1/databases {"name":"dup"}（同名再作成）
  → 409 {"error":"database already exists: dup","code":"DB_ALREADY_EXISTS"}

ETC-8: ストレージビジーエラー
  （意図的な長時間トランザクション保持中に別接続で書き込み試行、
    --busy-timeout を短く設定してテスト）
  → 503 {"error":"database is busy","code":"STORAGE_BUSY"}
```

---
