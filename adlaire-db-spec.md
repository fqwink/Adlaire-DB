# Adlaire DB 仕様書

**バージョン：** 1.0  
**ステータス：** 設計中  
**最終更新：** 2026-09-10  

---

## 1. 概要

### 1.1 プロジェクト概要

Adlaire DB は **Turso Cloud と同等の機能を提供するセルフホスト NoSQL DB サーバー**である。

REST/JSON API と SSE（Server-Sent Events）を採用し、SQL エンジンを持たない。libSQL フォーク（sqld）を基盤として Rust で実装する。

### 1.2 ポジション

| 比較対象 | Adlaire DB との関係 |
|----------|---------------------|
| Turso Cloud | 提供機能の参照実装。ワイヤプロトコルは非互換、機能を対応 |
| libSQL / sqld | フォーク元。ストレージ・WAL・レプリケーション基盤として活用 |
| SQLite | libSQL 経由で互換性を維持 |

### 1.3 Turso Cloud との機能対応

| Turso | Adlaire DB |
|---|---|
| テーブル | コレクション |
| 行 | ドキュメント（JSON）|
| SQL クエリ | NoSQL フィルタ操作 |
| hrana プロトコル | REST/JSON |
| WebSocket | SSE（レプリケーション用）|

### 1.4 固定制約

| 項目 | 内容 |
|------|------|
| 実装言語 | Rust + 標準ライブラリ |
| ストレージ・WAL 基盤 | libSQL フォーク（sqld 含む）|
| API プロトコル | REST/JSON（通常操作）/ SSE（ストリーミング）|
| gRPC | **禁止** |
| デプロイ形態 | シングルバイナリ起動 |
| 対象 OS | Linux |

### 1.5 設計不変条件

**I-1：データ整合性は DB 層で保証**  
トランザクション・制約・整合性チェックはサーバー層で完結する。アプリケーション層に委ねない。

**I-2：外部 DB 依存は libSQL フォーク一本**  
SQLite・libSQL フォーク以外の外部 DB ライブラリに依存しない。

**I-3：シングルバイナリ**  
サーバー起動は `./adlaire-db <flags>` 一コマンドで完結する。外部デーモン・サイドカーを必要としない（Phase 1）。

**I-4：データ永続化の先行保証**  
クライアントへ成功応答を返す前に、書き込みデータが永続化（fsync）されていることを保証する。

**I-5：切断時の自動 ROLLBACK**  
クライアント切断時、進行中のトランザクションを即座に ROLLBACK して fsync する。未コミット状態を残さない。

---

## 2. 機能スコープ

### 2.1 データ操作

| 機能 | Phase | 説明 |
|------|-------|------|
| ドキュメント Find | 1 | フィルタ・ソート・ページネーション |
| ドキュメント Insert | 1 | 単件・バルク挿入 |
| ドキュメント Update | 1 | フィルタ条件での更新 |
| ドキュメント Delete | 1 | フィルタ条件での削除 |
| Count | 1 | フィルタ条件でのカウント |
| トランザクション | 3 | ACID・複数操作の原子的実行 |
| インデックス | 3 | フィールドへのインデックス付与 |

### 2.2 データベース管理

| 機能 | Phase | 説明 |
|------|-------|------|
| JWT 認証 | 1 | HS256 Bearer トークン |
| マルチ DB | 2 | URL パスで接続先 DB を指定 |
| DB 作成・削除・一覧 | 2 | 管理 API 経由での DB ライフサイクル管理 |
| トークン発行・失効 | 2 | DB ごと・全体のトークン管理 |
| メトリクス API | 2 | 接続数・操作数・ストレージ使用量 |
| コレクション管理 | 3 | コレクション作成・削除・一覧 |
| WAL レプリケーション | 4 | プライマリ・レプリカ構成・SSE 配信 |
| オンラインバックアップ | 5 | バックアップ取得・リストア |
| PITR | 5 | 任意時点への復元 |
| ブランチ | 6 | DB のブランチ作成・管理 |
| HA | 7 | 高可用性構成 |

---

## 3. データモデル

### 3.1 ドキュメント

コレクション内の基本データ単位。JSON オブジェクトで表現する。

```json
{
  "_id": "uuid-or-user-defined-string",
  "name": "Alice",
  "age": 30,
  "tags": ["admin", "user"],
  "address": {
    "city": "Tokyo",
    "zip": "100-0001"
  },
  "score": 98.5,
  "active": true,
  "deleted_at": null
}
```

**`_id` フィールド：**
- 省略時はサーバーが UUID v4 を自動採番
- ユーザー指定時は文字列・数値を受け付ける
- コレクション内で一意。重複時は `409 DUPLICATE_ID`

### 3.2 型システム

| 型 | JSON 表現 | 例 |
|---|---|---|
| 文字列 | `string` | `"hello"` |
| 数値 | `number` | `42`, `3.14` |
| 真偽値 | `boolean` | `true`, `false` |
| null | `null` | `null` |
| 配列 | `array` | `[1, 2, 3]` |
| オブジェクト | `object` | `{"key": "value"}` |

### 3.3 コレクション

- SQL のテーブル相当
- スキーマレス（フィールド定義は任意）
- DB 内に複数作成可能
- 名前のバリデーション：`^[a-zA-Z0-9_-]{1,127}$`

---

## 4. CLI オプション

```sh
adlaire-db serve \
  --data /var/lib/adlaire \
  --port 8080 \
  --admin-port 8081 \
  --auth-jwt-secret <32バイト以上のシークレット> \
  --auth-jwt-secret-file <ファイルパス> \
  --admin-auth-token <管理トークン> \
  --busy-timeout <MS> \
  --shutdown-timeout <SECS>
```

| フラグ | 説明 | デフォルト |
|--------|------|-----------|
| `--data` | データディレクトリ | `./data` |
| `--port` | クライアント向けポート | `8080` |
| `--admin-port` | 管理 API ポート | `8081` |
| `--auth-jwt-secret` | JWT 署名シークレット | — |
| `--auth-jwt-secret-file` | JWT シークレットのファイルパス | — |
| `--admin-auth-token` | 管理 API 認証トークン | — |
| `--busy-timeout` | DB ビジー時の待機上限（ms）| `5000` |
| `--shutdown-timeout` | グレースフルシャットダウン上限（秒）| `30` |

---

## 5. 認証

### 5.1 JWT 認証

すべてのクライアント API リクエストに `Authorization: Bearer <JWT>` を要求する。

**JWT 仕様：**
- アルゴリズム：HS256
- 署名シークレット：`--auth-jwt-secret` または `--auth-jwt-secret-file`（32 バイト以上）
- 有効期限：`exp` クレームで指定（省略時は無期限）

### 5.2 JWT クレーム

```json
{
  "sub": "user-id",
  "exp": 1800000000,
  "dbs": ["db1", "db2"]
}
```

| クレーム | 必須 | 説明 |
|----------|------|------|
| `sub` | 任意 | ユーザー識別子 |
| `exp` | 任意 | 有効期限（Unix 秒）|
| `dbs` | 任意 | アクセス許可 DB 名リスト。省略時は全 DB 許可 |

### 5.3 DB スコープ

`dbs` クレームに DB 名が含まれない場合、そのDBへのアクセスは `403 FORBIDDEN` を返す。

### 5.4 管理者トークン失効

失効済みトークンリストを DB 内に保持する。失効済み `jti` を持つ JWT は `401 TOKEN_REVOKED` を返す。

### 5.5 JWT 優先順位

```
--auth-jwt-secret-file > --auth-jwt-secret > ADLAIRE_JWT_SECRET (env) > config.toml
```

---

## 6. API 定義

### 6.1 共通仕様

**リクエストヘッダー：**
```
Content-Type: application/json
Authorization: Bearer <JWT>
```

**レスポンス形式：**
```json
{"field": "value"}
```

**エラーレスポンス：**
```json
{"error": "ERROR_CODE", "message": "human readable message"}
```

### 6.2 エラーコード一覧

| コード | HTTP | 意味 |
|--------|------|------|
| `AUTH_REQUIRED` | 401 | JWT が必要 |
| `TOKEN_REVOKED` | 401 | 失効済みトークン |
| `FORBIDDEN` | 403 | DB アクセス権限なし |
| `NOT_FOUND` | 404 | DB・コレクション・ドキュメントが存在しない |
| `DUPLICATE_ID` | 409 | `_id` が重複 |
| `TX_EXPIRED` | 410 | トランザクションがタイムアウト |
| `TX_NOT_FOUND` | 404 | トランザクション ID が存在しない |
| `INVALID_FILTER` | 400 | フィルタ・演算子の構文エラー |
| `INVALID_DB_NAME` | 400 | DB 名バリデーションエラー |
| `INVALID_COL_NAME` | 400 | コレクション名バリデーションエラー |
| `INTERNAL_ERROR` | 500 | サーバー内部エラー |

### 6.3 データ操作 API（Phase 1）

#### Find（検索）

```
POST /{db}/collections/{col}/find
Authorization: Bearer <JWT>
```

**リクエスト：**
```json
{
  "filter": {"age": {"$gt": 18}, "name": "Alice"},
  "sort": {"age": -1},
  "limit": 20,
  "skip": 0,
  "projection": ["name", "age"]
}
```

**レスポンス（200 OK）：**
```json
{
  "documents": [
    {"_id": "uuid1", "name": "Alice", "age": 30}
  ],
  "count": 1,
  "next_cursor": null
}
```

`limit` 上限を超える結果がある場合、`next_cursor` に次ページ取得用のカーソル文字列を返す。

#### Insert

```
POST /{db}/collections/{col}/insert
Authorization: Bearer <JWT>
```

**リクエスト：**
```json
{
  "documents": [
    {"name": "Alice", "age": 30},
    {"name": "Bob", "age": 25}
  ]
}
```

**レスポンス（201 Created）：**
```json
{
  "inserted_ids": ["uuid1", "uuid2"],
  "inserted_count": 2
}
```

#### Update

```
POST /{db}/collections/{col}/update
Authorization: Bearer <JWT>
```

**リクエスト：**
```json
{
  "filter": {"_id": "uuid1"},
  "update": {"$set": {"age": 31}},
  "upsert": false,
  "multi": false
}
```

**レスポンス（200 OK）：**
```json
{
  "matched_count": 1,
  "modified_count": 1,
  "upserted_id": null
}
```

#### Delete

```
POST /{db}/collections/{col}/delete
Authorization: Bearer <JWT>
```

**リクエスト：**
```json
{
  "filter": {"_id": "uuid1"},
  "multi": false
}
```

**レスポンス（200 OK）：**
```json
{
  "deleted_count": 1
}
```

#### Count

```
POST /{db}/collections/{col}/count
Authorization: Bearer <JWT>
```

**リクエスト：**
```json
{
  "filter": {"active": true}
}
```

**レスポンス（200 OK）：**
```json
{
  "count": 42
}
```

### 6.4 フィルタ演算子

フィールド名と演算子を組み合わせてフィルタを構成する。

**比較演算子：**

| 演算子 | 意味 | 例 |
|--------|------|----|
| `$eq` | 等しい | `{"age": {"$eq": 30}}` または `{"age": 30}` |
| `$ne` | 等しくない | `{"age": {"$ne": 0}}` |
| `$gt` | より大きい | `{"age": {"$gt": 18}}` |
| `$gte` | 以上 | `{"age": {"$gte": 18}}` |
| `$lt` | より小さい | `{"age": {"$lt": 65}}` |
| `$lte` | 以下 | `{"age": {"$lte": 65}}` |
| `$in` | 配列内のいずれか | `{"status": {"$in": ["active", "pending"]}}` |
| `$nin` | 配列内のいずれでもない | `{"status": {"$nin": ["deleted"]}}` |
| `$exists` | フィールドの存在 | `{"email": {"$exists": true}}` |

**論理演算子：**

| 演算子 | 意味 | 例 |
|--------|------|----|
| `$and` | AND 条件 | `{"$and": [{"age": {"$gt": 18}}, {"active": true}]}` |
| `$or` | OR 条件 | `{"$or": [{"role": "admin"}, {"role": "mod"}]}` |

### 6.5 更新演算子

| 演算子 | 意味 | 例 |
|--------|------|----|
| `$set` | フィールドを設定 | `{"$set": {"age": 31, "name": "Bob"}}` |
| `$unset` | フィールドを削除 | `{"$unset": {"temp_field": ""}}` |
| `$inc` | 数値をインクリメント | `{"$inc": {"score": 10}}` |
| `$push` | 配列に要素を追加 | `{"$push": {"tags": "vip"}}` |
| `$pull` | 配列から要素を削除 | `{"$pull": {"tags": "vip"}}` |

### 6.6 トランザクション API（Phase 3）

#### Begin

```
POST /{db}/tx/begin
Authorization: Bearer <JWT>
```

**レスポンス（200 OK）：**
```json
{
  "tx_id": "tx-uuid",
  "expires_at": "2026-09-10T12:00:30Z"
}
```

#### Execute（トランザクション内操作）

```
POST /{db}/tx/{tx_id}/execute
Authorization: Bearer <JWT>
```

**リクエスト：**
```json
{
  "collection": "users",
  "op": "insert",
  "documents": [{"name": "Alice"}]
}
```

`op` には `insert` / `update` / `delete` / `find` を指定する。

#### Commit

```
POST /{db}/tx/{tx_id}/commit
Authorization: Bearer <JWT>
```

**レスポンス（200 OK）：**
```json
{"committed": true}
```

#### Rollback

```
POST /{db}/tx/{tx_id}/rollback
Authorization: Bearer <JWT>
```

**レスポンス（200 OK）：**
```json
{"rolled_back": true}
```

**トランザクション保証：**
- ACID（DB 層で完結）
- タイムアウト：30 秒（`--tx-timeout` で変更可）
- クライアント切断時：即座に自動 ROLLBACK + fsync
- `tx_id` 不明時：`404 TX_NOT_FOUND`
- タイムアウト後：`410 TX_EXPIRED`

### 6.7 コレクション管理 API（Phase 3）

```
GET    /{db}/collections
POST   /{db}/collections
DELETE /{db}/collections/{col}
```

**GET /collections レスポンス：**
```json
{
  "collections": [
    {"name": "users", "doc_count": 1200, "size_bytes": 204800}
  ]
}
```

**POST /collections リクエスト：**
```json
{"name": "users"}
```

### 6.8 レプリケーション API（Phase 4）

#### SSE ストリーム

```
GET /{db}/replication/stream?from_seq={N}
Authorization: Bearer <JWT>
```

**レスポンス（200 OK、Server-Sent Events）：**
```
Content-Type: text/event-stream

data: {"seq":1,"op":"insert","col":"users","doc":{"_id":"uuid1","name":"Alice"}}

data: {"seq":2,"op":"update","col":"users","filter":{"_id":"uuid1"},"update":{"$set":{"age":31}}}

data: {"seq":3,"op":"delete","col":"users","filter":{"_id":"uuid2"}}
```

- `from_seq`：取得開始シーケンス番号（初回は `0`）
- COMMIT 境界のみ配信（未コミットフレームは配信しない）
- レプリカは `seq` の連続性を検証し、ギャップ検出時は接続を切断して再取得する

#### スナップショット

```
GET /{db}/replication/snapshot
Authorization: Bearer <JWT>
```

**レスポンス（200 OK）：**
```
Content-Type: application/json
X-Replication-Seq: 42

{"collections": {"users": [{"_id": "uuid1", ...}, ...]}, "seq": 42}
```

初回同期時にスナップショットを取得し、以後 `/stream` で差分を追う。

### 6.9 管理 API（Phase 2、ポート 8081）

管理 API は独立したポート（デフォルト 8081）で提供する。外部に公開しないことを推奨する。

**管理 API 認証：**
- `Authorization: Bearer <admin-token>` を要求
- `--admin-auth-token` または config.toml の `[admin] auth_token` で設定
- 未設定時は認証を無効化（開発・ローカル用）

#### DB 管理

```
GET    /admin/v1/databases
POST   /admin/v1/databases
GET    /admin/v1/databases/{name}
DELETE /admin/v1/databases/{name}
```

**POST /admin/v1/databases リクエスト：**
```json
{"name": "my-db"}
```

**GET /admin/v1/databases レスポンス：**
```json
{
  "databases": [
    {"name": "my-db", "size_bytes": 4096, "created_at": "2026-09-10T10:00:00Z"}
  ]
}
```

#### トークン管理

```
POST   /admin/v1/tokens
DELETE /admin/v1/tokens/{token_id}
GET    /admin/v1/tokens
```

**POST /admin/v1/tokens リクエスト：**
```json
{
  "dbs": ["my-db"],
  "expires_in": 86400
}
```

**POST /admin/v1/tokens レスポンス：**
```json
{
  "token": "<JWT>",
  "token_id": "jti-uuid",
  "expires_at": "2026-09-11T10:00:00Z"
}
```

#### メトリクス

```
GET /admin/v1/metrics
```

**レスポンス（200 OK）：**
```json
{
  "uptime_seconds": 3600,
  "databases": [
    {
      "name": "my-db",
      "size_bytes": 4096,
      "collections": 3,
      "doc_count": 1200,
      "ops_total": 1500,
      "ops_insert": 200,
      "ops_find": 1000,
      "ops_update": 200,
      "ops_delete": 100
    }
  ],
  "tokens_total": 5,
  "tokens_revoked": 1
}
```

### 6.10 バックアップ・PITR API（Phase 5）

```
POST /admin/v1/databases/{name}/backup
GET  /admin/v1/databases/{name}/backups
POST /admin/v1/databases/{name}/restore
```

**POST /backup レスポンス：**
```json
{
  "backup_id": "backup-uuid",
  "seq": 42,
  "created_at": "2026-09-10T10:00:00Z"
}
```

**POST /restore リクエスト：**
```json
{
  "backup_id": "backup-uuid",
  "target_seq": 35
}
```

### 6.11 ブランチ API（Phase 6）

```
GET    /admin/v1/databases/{name}/branches
POST   /admin/v1/databases/{name}/branches
DELETE /admin/v1/databases/{name}/branches/{branch}
```

**POST /branches リクエスト：**
```json
{"name": "feature-branch"}
```

ブランチは `{source}___{branch-name}` の命名規則で内部管理する。

---

## 7. WAL レプリケーション（Phase 4）

### 7.1 構成

```
プライマリ（書き込み・読み取り）
      ↓ SSE ストリーム（seq 付き操作ログ）
レプリカ（読み取り専用）
```

### 7.2 データ整合性保証

**① COMMIT 境界配信**  
未コミット操作のログは配信しない。レプリカは完全なトランザクションのみ適用する。

**② seq 連続性チェック**  
レプリカは受信した `seq` のギャップを検出した場合、適用を停止してスナップショットから再取得する。

**③ プライマリ fsync 後に配信**  
操作ログは fsync 完了後にのみ SSE ストリームへ送出する。

**④ 書き込みリダイレクト**  
レプリカへの書き込みリクエストは `409 WRITE_NOT_ALLOWED` と `primary_url` を返す。

```json
{"error": "WRITE_NOT_ALLOWED", "primary_url": "http://primary:8080"}
```

**⑤ フェンシングトークン**  
プライマリは応答に単調増加の `epoch` を付与する。レプリカは古い `epoch` のプライマリからの操作を拒否する。

### 7.3 レプリカ起動フロー

```
1. GET /{db}/replication/snapshot → 全データ取得・seq 記録
2. GET /{db}/replication/stream?from_seq={seq} → 差分追従開始
3. seq ギャップ検出時 → 1 に戻る
```

---

## 8. WAL アーカイブ・PITR（Phase 5）

### 8.1 概要

操作ログ（seq 付き）をオブジェクトストレージ（S3 互換）にアーカイブし、任意の `seq` 時点への復元を可能にする。

### 8.2 アーカイブ構造

```
archive/
  {db-name}/
    manifest.json
    snapshot-{seq}.json.gz
    log-{from_seq}-{to_seq}.jsonl.gz
```

**manifest.json：**
```json
{
  "db": "my-db",
  "latest_seq": 1000,
  "snapshots": [
    {"seq": 0, "file": "snapshot-0.json.gz"},
    {"seq": 500, "file": "snapshot-500.json.gz"}
  ],
  "log_segments": [
    {"from": 0, "to": 499, "file": "log-0-499.jsonl.gz"},
    {"from": 500, "to": 999, "file": "log-500-999.jsonl.gz"}
  ]
}
```

### 8.3 PITR 復元フロー

```
1. manifest.json を読み込み
2. target_seq 以前の最新スナップショットを選択
3. スナップショットを適用
4. target_seq までのログセグメントを順番に再生
5. 起動
```

---

## 9. フェーズサマリー

| フェーズ | 内容 | テストケース | 実装タスク |
|----------|------|-------------|-----------|
| **Phase 1** | REST/JSON・Find/Insert/Update/Delete/Count・JWT 認証・シングル DB | TC-1-1〜TC-1-6 (6件) | T1-1〜T1-8 (8件) |
| **Phase 2** | マルチ DB・トークン管理・管理 API・メトリクス | TC-2-1〜TC-2-5 (5件) | T2-1〜T2-5 (5件) |
| **Phase 3** | トランザクション・コレクション管理・インデックス | TC-3-1〜TC-3-5 (5件) | T3-1〜T3-6 (6件) |
| **Phase 4** | プライマリ・レプリカ構成・SSE レプリケーション | TC-4-1〜TC-4-4 (4件) | T4-1〜T4-6 (6件) |
| **Phase 5** | オンラインバックアップ・PITR・WAL アーカイブ | TC-5-1〜TC-5-6 (6件) | T5-1〜T5-7 (7件) |
| **Phase 6** | ブランチ | TC-6-1〜TC-6-3 (3件) | T6-1〜T6-4 (4件) |
| **Phase 7** | HA・拡張機能 | TC-7-1〜TC-7-4 (4件) | T7-1〜T7-6 (6件) |

---

## 10. フェーズ詳細

### Phase 1：REST/JSON API・JWT 認証

**目標**：単一 DB に対して Find/Insert/Update/Delete/Count が動作する

**スコープ：**
- `POST /{db}/collections/{col}/find`
- `POST /{db}/collections/{col}/insert`
- `POST /{db}/collections/{col}/update`
- `POST /{db}/collections/{col}/delete`
- `POST /{db}/collections/{col}/count`
- JWT HS256 認証（`exp`・`dbs` クレーム）
- フィルタ演算子：`$eq` / `$ne` / `$gt` / `$gte` / `$lt` / `$lte` / `$in` / `$nin` / `$exists` / `$and` / `$or`
- 更新演算子：`$set` / `$unset` / `$inc` / `$push` / `$pull`
- fsync 保証（I-4）

**対象外：**
- トランザクション（Phase 3）
- マルチ DB（Phase 2）
- インデックス（Phase 3）

**完了条件（テストケース）：**

```
TC-1-1: Insert / Find
  documents = [{"name": "Alice", "age": 30}] を insert
  filter = {"name": "Alice"} で find
  期待: documents[0].age = 30

TC-1-2: Update / Delete
  {"name": "Bob"} を insert
  {"$set": {"age": 25}} で update
  find → age = 25 を確認
  delete → find で documents = [] を確認

TC-1-3: フィルタ演算子
  age $gt 18 で find → 18 超のドキュメントのみ返る
  status $in ["active"] で find → active のみ返る
  $and / $or の組み合わせが正しく動作する

TC-1-4: JWT 認証
  有効 JWT → 200
  JWT なし → 401 AUTH_REQUIRED
  期限切れ JWT → 401 AUTH_REQUIRED
  dbs スコープ外 DB → 403 FORBIDDEN

TC-1-5: _id 重複
  同一 _id で insert を 2 回実行
  期待: 2 回目は 409 DUPLICATE_ID

TC-1-6: fsync 保証
  insert 成功後にプロセスを強制終了して再起動
  期待: データが残っている
```

**Phase 1 実装タスク：**

```
T1-1: axum HTTP サーバー起動・ルーティング設定
T1-2: JWT 検証ミドルウェア（HS256・exp・dbs クレーム）
T1-3: libSQL 接続管理・ドキュメントストレージ設計
T1-4: Find ハンドラ（フィルタ・ソート・ページネーション）
T1-5: Insert ハンドラ（バルク・_id 自動採番・重複チェック）
T1-6: Update ハンドラ（演算子処理・upsert・multi）
T1-7: Delete ハンドラ（multi 対応）
T1-8: 統合テスト TC-1-1〜TC-1-6
```

---

### Phase 2：マルチ DB・管理 API

**目標**：複数 DB の管理と JWT トークン管理が動作する

**スコープ：**
- URL パス `/{db}/` による DB 切り替え
- 管理 API（ポート 8081）：DB 作成・削除・一覧
- トークン発行・失効（`jti` 管理）
- メトリクス API（`GET /admin/v1/metrics`）
- DB 名バリデーション：`^[a-zA-Z0-9_-]{1,127}$`

**対象外：**
- トランザクション（Phase 3）
- レプリケーション（Phase 4）

**完了条件（テストケース）：**

```
TC-2-1: マルチ DB
  db_a・db_b を作成
  db_a に insert → db_b で find → 結果なし（DB 分離確認）

TC-2-2: DB 作成・削除
  POST /admin/v1/databases {"name": "test-db"} → 201
  GET /admin/v1/databases → test-db が含まれる
  DELETE /admin/v1/databases/test-db → 204
  GET /admin/v1/databases → test-db が含まれない

TC-2-3: トークン発行・失効
  POST /admin/v1/tokens → JWT 発行
  発行 JWT でデータ操作 → 200
  DELETE /admin/v1/tokens/{id} → 失効
  失効済み JWT でデータ操作 → 401 TOKEN_REVOKED

TC-2-4: dbs スコープ制限
  dbs=["db_a"] の JWT で db_b にアクセス → 403 FORBIDDEN

TC-2-5: メトリクス API
  操作実行後に GET /admin/v1/metrics
  期待: ops_total が増加している
```

**Phase 2 実装タスク：**

```
T2-1: パスベースマルチ DB ルーティング・DB 存在チェック
T2-2: DB 作成・削除・一覧（ファイルシステム管理）
T2-3: トークン発行（JWT 生成）・jti 管理
T2-4: トークン失効（失効リスト DB 保存・検証）
T2-5: メトリクス収集（インメモリカウンター）+ GET /admin/v1/metrics
```

---

### Phase 3：トランザクション・コレクション管理

**目標**：ACID トランザクションとコレクション管理が動作する

**スコープ：**
- トランザクション（begin/execute/commit/rollback）
- タイムアウト・切断時自動 ROLLBACK
- コレクション作成・削除・一覧
- フィールドインデックス（単一フィールド）

**対象外：**
- レプリケーション（Phase 4）
- 複合インデックス（Phase 3+）

**完了条件（テストケース）：**

```
TC-3-1: トランザクション COMMIT
  begin → insert × 2 → commit
  find → 2 件が存在する

TC-3-2: トランザクション ROLLBACK
  begin → insert → rollback
  find → 0 件（ロールバック確認）

TC-3-3: タイムアウト自動 ROLLBACK
  begin → 30 秒待機（操作なし）
  commit → 410 TX_EXPIRED
  find → 0 件（タイムアウトで自動 ROLLBACK 確認）

TC-3-4: 切断時自動 ROLLBACK
  begin → insert → クライアント切断
  再接続して find → 0 件

TC-3-5: コレクション管理
  POST /collections {"name": "items"} → 201
  GET /collections → items が含まれる
  DELETE /collections/items → 204
```

**Phase 3 実装タスク：**

```
T3-1: トランザクション状態管理（tx_id・タイムアウト・HashMap）
T3-2: begin/execute/commit/rollback ハンドラ
T3-3: タイムアウト監視（tokio タスク）
T3-4: 切断検知・自動 ROLLBACK（axum 接続ライフサイクル）
T3-5: コレクション作成・削除・一覧
T3-6: 統合テスト TC-3-1〜TC-3-5
```

---

### Phase 4：レプリケーション

**目標**：プライマリ・レプリカ構成で読み取りスケールアウトが動作する

**スコープ：**
- SSE 操作ログストリーム（`GET /{db}/replication/stream`）
- 初回スナップショット（`GET /{db}/replication/snapshot`）
- 書き込みリダイレクト（`409 WRITE_NOT_ALLOWED` + `primary_url`）
- レプリカ 1 台構成（N 台対応は Phase 4+）
- static config（`--primary-url` フラグ）

**対象外：**
- 動的ノード発見
- 自動フェイルオーバー（Phase 7）

**完了条件（テストケース）：**

```
TC-4-1: レプリケーション基本動作
  プライマリに insert → レプリカで find → データが同期されている

TC-4-2: 書き込みリダイレクト
  レプリカに insert → 409 WRITE_NOT_ALLOWED + primary_url が返る

TC-4-3: 初回スナップショット同期
  プライマリに 100 件 insert 後にレプリカを起動
  レプリカで find → 100 件が存在する

TC-4-4: seq ギャップ検出
  seq を意図的に欠損させてレプリカに送信
  期待: レプリカが接続を切断してスナップショットから再取得
```

**Phase 4 実装タスク：**

```
T4-1: 操作ログ記録（seq 付き・COMMIT 境界）
T4-2: SSE ストリームエンドポイント（GET /{db}/replication/stream）
T4-3: スナップショットエンドポイント（GET /{db}/replication/snapshot）
T4-4: レプリカモード（--primary-url・起動フロー・差分追従）
T4-5: 書き込みリダイレクト（409 + primary_url）
T4-6: 統合テスト TC-4-1〜TC-4-4
```

---

### Phase 5：バックアップ・PITR

**目標**：オンラインバックアップと任意時点復元が動作する

**スコープ：**
- 操作ログのオブジェクトストレージ（S3 互換）へのアーカイブ
- manifest.json 管理
- スナップショット + ログ再生による PITR
- `POST /admin/v1/databases/{name}/backup`
- `POST /admin/v1/databases/{name}/restore`

**対象外：**
- ブランチ（Phase 6）

**完了条件（テストケース）：**

```
TC-5-1: バックアップ取得
  insert 後に POST /backup → backup_id が返る
  S3 に snapshot と manifest が保存される

TC-5-2: リストア（最新）
  backup_id で POST /restore → データが復元される

TC-5-3: PITR（任意 seq）
  seq=100 時点のデータを target_seq=50 で restore
  find → seq=50 時点のデータが返る

TC-5-4: manifest 整合性
  manifest.json の seq と実データの seq が一致する

TC-5-5: バックアップ中の書き込み
  バックアップ実行中に insert を実行しても整合性が保たれる

TC-5-6: 破損ログセグメントの検出
  ログセグメントを改ざんして restore
  期待: エラーが返り、DB は変更されない
```

**Phase 5 実装タスク：**

```
T5-1: S3 互換ストレージクライアント（aws-sdk-s3 or 互換）
T5-2: 操作ログのセグメント化・アーカイブ
T5-3: manifest.json 生成・更新
T5-4: スナップショット生成・アップロード
T5-5: PITR 復元ロジック（スナップショット適用 + ログ再生）
T5-6: backup / restore API ハンドラ
T5-7: 統合テスト TC-5-1〜TC-5-6
```

---

### Phase 6：ブランチ

**目標**：DB のブランチ作成・独立した読み書きが動作する

**スコープ：**
- ブランチ作成（スナップショットコピー）
- ブランチへの独立した読み書き
- ブランチ削除
- 命名規則：`{source}___{branch-name}`

**対象外：**
- ブランチのマージ

**完了条件（テストケース）：**

```
TC-6-1: ブランチ作成
  db_a にデータあり → ブランチ feature を作成
  feature で find → db_a のデータが存在する

TC-6-2: ブランチの独立性
  feature に insert → db_a で find → feature のデータは見えない

TC-6-3: ブランチ削除
  DELETE /branches/feature → 204
  feature へのアクセス → 404 NOT_FOUND
```

**Phase 6 実装タスク：**

```
T6-1: ブランチ作成（スナップショットコピー + メタデータ登録）
T6-2: ブランチ命名・パス解決（{source}___{branch}）
T6-3: ブランチ削除（データ削除 + メタデータ削除）
T6-4: 統合テスト TC-6-1〜TC-6-3
```

---

### Phase 7：HA・拡張機能

**目標**：高可用性構成と拡張機能ロードが動作する

**スコープ：**
- Raft ベースのリーダー選出（`openraft` クレート採用）
- 自動フェイルオーバー
- 拡張機能ロード（`.so`、別プロセスサンドボックス）

**対象外：**
- SQL エンジン内製化（Phase 8）
- Wasm 拡張（Phase 7+）

**完了条件（テストケース）：**

```
TC-7-1: リーダー選出
  3 ノード起動 → リーダーが 1 台選出される

TC-7-2: 自動フェイルオーバー
  リーダーを停止 → 残り 2 ノードで新リーダーが選出される
  クライアントは新リーダーに自動再接続される

TC-7-3: 拡張機能ロード
  .so 拡張を --extension フラグで指定
  拡張が提供する関数が操作で使用できる

TC-7-4: 拡張クラッシュ耐性
  拡張プロセスがクラッシュしてもサーバーは継続動作する
```

**Phase 7 実装タスク：**

```
T7-1: openraft 統合（ストレージアダプター実装）
T7-2: リーダー選出・ログ複製
T7-3: フェイルオーバー・クライアント再接続
T7-4: 拡張機能ロード（別プロセス起動・IPC 設計）
T7-5: 拡張クラッシュ検知・再起動
T7-6: 統合テスト TC-7-1〜TC-7-4
```

---

## 11. ディレクトリ構造

```
data/
  databases.json          # DB 一覧メタデータ
  tokens/
    revoked.json          # 失効済み jti リスト
  {db-name}/
    meta.json             # DB メタデータ（コレクション一覧等）
    data.db               # libSQL データファイル
    data.db-wal           # WAL ファイル
    oplog/                # 操作ログ（seq 付き）
      000000001.jsonl
      000000002.jsonl
    branches.json         # ブランチ一覧
```

---

## 12. 用語集

| 用語 | 定義 |
|------|------|
| Turso Cloud | libSQL のマネージドホスティングサービス。Adlaire DB の機能互換の参照実装 |
| libSQL | SQLite フォーク。HTTP API・WAL レプリケーション等を追加した OSS DB ライブラリ |
| sqld | libSQL のサーバーコンポーネント。Adlaire DB のストレージ・WAL 基盤として活用 |
| コレクション | ドキュメントの集合。SQL のテーブル相当 |
| ドキュメント | JSON オブジェクト形式のデータ単位。SQL の行相当 |
| `_id` | ドキュメントのプライマリキー |
| seq | 操作ログのシーケンス番号。レプリケーション・PITR の追跡に使用 |
| SSE | Server-Sent Events。HTTP でサーバーからクライアントへ一方向にイベントをプッシュする仕組み |
| PITR | Point-in-Time Recovery。任意時点への DB 復元 |
| ブランチ | DB のスナップショットコピー。独立した読み書きが可能 |
| epoch | フェンシングトークン。プライマリの世代番号 |
| WAL | Write-Ahead Log。書き込みを先にログに記録する耐障害性のための仕組み |
| hrana | Turso / libSQL のワイヤプロトコル名。Adlaire DB では採用しない |
