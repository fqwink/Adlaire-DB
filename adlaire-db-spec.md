# Adlaire DB 仕様書

**バージョン：** 0.4  
**ステータス：** 設計中  
**最終更新：** 2026-09-10  

---

## 1. 概要

### 1.1 プロジェクト概要

Adlaire DB は **Turso Cloud が提供する機能と同等の機能をセルフホストで実現する** DB サーバーである。

Turso Cloud は libSQL のマネージドホスティングサービスとして HTTP API・WebSocket API・レプリケーション・マルチDB管理などを提供している。Adlaire DB はこれと同等の機能を、libSQL フォークを基盤として Rust で実装し、自前インフラ上で運用できるようにする。

将来的には libSQL フォークの内部コンポーネント（WAL・ページストレージ・SQL エンジン等）を段階的に内製実装へ置き換えることを計画しているが、具体的な詳細・スケジュールはフェーズの進行とともに検討する。

### 1.2 ポジション

| 比較対象 | Adlaire DB との関係 |
|----------|---------------------|
| Turso Cloud | 提供機能の参照実装。クライアント API 互換を目指す |
| libSQL / sqld | フォーク元。Adlaire DB の全体基盤 |
| SQLite | libSQL 経由で互換性を維持 |

### 1.3 固定制約

| 項目 | 内容 |
|------|------|
| 実装言語 | Rust + 標準ライブラリ |
| ストレージ・SQL 基盤 | libSQL フォーク（sqld 含む）|
| 目標機能 | Turso Cloud 機能パリティ |
| 将来方針 | libSQL 内部の段階的内製化（詳細は各フェーズで検討）|
| デプロイ形態 | シングルバイナリ起動 |
| 対象 OS | Linux |

### 1.4 設計不変条件

実装のあらゆる判断においてこれらを最優先する。

**I-1：libSQL クライアント SDK 互換**  
既存の libSQL クライアント SDK（TypeScript・Rust・Go 等）が、Turso Cloud の URL を Adlaire DB の URL に差し替えるだけで動作しなければならない。クライアント側コードの変更は要求しない。

**I-2：外部 DB 依存は libSQL フォーク一本**  
SQLite・libSQL フォーク以外の外部 DB ライブラリ（PostgreSQL・MySQL ドライバ等）に依存しない。

**I-3：シングルバイナリ**  
サーバー起動は `./adlaire-db <flags>` 一コマンドで完結する。外部デーモン・サイドカーを必要としない（Phase 1）。

**I-4：データ永続化の先行保証**  
クライアントへ成功応答を返す前に、書き込みデータが永続化（fsync）されていることを保証する。

**I-5：内製化は段階的・計画的に**  
libSQL 内部コンポーネントの内製化はフェーズ完了後に計画・判断する。「実装が大変だから」という理由で無計画に外部依存を追加することは認めない。

---

## 2. Turso Cloud 機能パリティ

### 2.1 クライアント接続

| 機能 | Phase | 説明 |
|------|-------|------|
| HTTP API（hrana-http） | 1 | libSQL クライアント SDK が利用する HTTP/JSON API |
| JWT 認証 | 1 | Bearer トークンによる認証 |
| マルチDB（パスベース） | 2 | URL パスで接続先 DB を指定 |
| WebSocket API（hrana-ws） | 3 | インタラクティブトランザクション用 |
| 埋め込みレプリカ同期 | 3 | クライアント側ローカルレプリカとの同期プロトコル |

### 2.2 データベース管理

| 機能 | Phase | 説明 |
|------|-------|------|
| DB 作成・削除・一覧 | 2 | 管理 API 経由での DB ライフサイクル管理 |
| トークン発行・失効 | 2 | DB ごと・全体のトークン管理 |
| ブランチ | 5+ | DB のブランチ作成 |
| ポイントインタイムリストア | 5+ | 任意の時点への DB 復元 |

### 2.3 レプリケーション

| 機能 | Phase | 説明 |
|------|-------|------|
| プライマリ・レプリカ構成 | 4 | 書き込みはプライマリ、読み取りはレプリカへ |
| WAL ベース同期 | 4 | libSQL の WAL レプリケーションを使用 |

---

## 3. アーキテクチャ

### 3.1 全体構成

```
libSQL クライアント SDK / curl / WebSocket クライアント
        │
        │ HTTP（JSON）または WebSocket
        │ Authorization: Bearer <JWT>
        │
        ▼
┌──────────────────────────────────────────────────┐
│                 Adlaire サーバー層                │
│                                                  │
│  ┌──────────┐  ┌───────────┐  ┌───────────────┐ │
│  │ HTTP API │  │  WS API   │  │   管理 API    │ │
│  │(hrana-http│  │(hrana-ws) │  │ /admin/...   │ │
│  └────┬─────┘  └─────┬─────┘  └──────┬────────┘ │
│       │              │               │           │
│  ┌────▼──────────────▼───────────────▼────────┐  │
│  │           認証ミドルウェア（JWT 検証）       │  │
│  └────────────────────┬───────────────────────┘  │
│                       │                          │
│  ┌────────────────────▼───────────────────────┐  │
│  │       DB ルーター（パスベース マルチDB）    │  │
│  └────────────────────┬───────────────────────┘  │
│                       │                          │
└───────────────────────┼──────────────────────────┘
                        │
                        ▼
┌──────────────────────────────────────────────────┐
│              libSQL フォーク（sqld）              │
│  SQL パーサ / クエリ実行 / WAL / ページストレージ │
└──────────────────────────────────────────────────┘
```

### 3.2 データディレクトリ構成

```
{data-dir}/
├── .lock                         # プロセス排他ロック
├── config.toml                   # サーバー設定（起動フラグで上書き可）
├── databases/
│   ├── {db-name}/
│   │   ├── data.db               # SQLite 互換 DB（libSQL 管理）
│   │   └── data.db-wal           # WAL（libSQL 管理）
│   └── ...
└── meta/
    ├── databases.json            # DB メタデータ（名前・作成日時・状態）
    └── tokens.json               # 発行済みトークン一覧（失効管理用）
```

### 3.3 libSQL フォークとの統合方式

sqld（libSQL のサーバーコンポーネント）を **Rust ライブラリとして組み込む**。sqld をサブプロセスとして起動してプロキシする方式は採らない。

```
adlaire-db バイナリ（Rust）
├── Adlaire サーバー層（自前実装）
│   ├── HTTP ルーティング・認証・管理 API
│   └── マルチDB ルーター
└── sqld コア（libSQL フォークとして静的リンク）
    ├── SQL パーサ・クエリエグゼキューター
    ├── WAL 管理
    └── ページストレージ
```

**Phase 1 での sqld 改変範囲：**

| sqld の機能 | Adlaire での扱い |
|-------------|-----------------|
| SQL パーサ・クエリ実行 | そのまま使用 |
| WAL・ページストレージ | そのまま使用 |
| sqld 内蔵 HTTP サーバー | 無効化。Adlaire サーバー層が代替 |
| sqld 内蔵認証 | 無効化。Adlaire の JWT 認証が代替 |
| sqld 内蔵管理 API | 無効化。Adlaire 管理 API が代替 |
| hrana-http プロトコル実装 | sqld のものを再利用するか Adlaire で再実装するかは実装時に判断 |

**改変の基本方針：**
- Phase 1 では sqld への変更を最小限に留める
- sqld の `Connection` / `Database` 型を直接呼び出す形で統合する
- sqld の HTTP サーバーループは起動しない（Adlaire サーバーが HTTP を受け付ける）

### 3.4 マルチDB のデータ分離（Phase 2）

**ファイル分離：**

各 DB は独立した SQLite ファイルを持ち、他の DB のファイルとは完全に分離される。

**接続管理：**

| 項目 | Phase 2 実装方針 |
|------|----------------|
| DB ごとの接続数 | 接続 1 本（シンプルな実装から始める）|
| 同一 DB への並行アクセス | SQLite の WAL モードで複数リーダー・シングルライターを実現 |
| 異なる DB への並行アクセス | DB ごとに独立した接続のため干渉なし |
| 接続プール | Phase 2 は単一接続。Phase 3 以降でプール化を検討 |

**DB 作成フロー：**

1. `POST /admin/v1/databases` を受信
2. `databases/{name}/` ディレクトリを作成（既存なら `DB_ALREADY_EXISTS` エラー）
3. sqld で `data.db` を初期化（空の SQLite DB）
4. `meta/databases.json` にメタデータを追記
5. 成功レスポンスを返す

**DB 削除フロー：**

1. `DELETE /admin/v1/databases/{name}` を受信
2. 対象 DB への既存接続を閉じる
3. `databases/{name}/` ディレクトリを丸ごと削除
4. `meta/databases.json` からエントリを削除
5. 成功レスポンスを返す

---

## 4. 設定・起動

### 4.1 CLI

```
adlaire-db serve [OPTIONS]

OPTIONS:
  --data <PATH>          データディレクトリ（必須）
  --port <PORT>          HTTP リスニングポート（デフォルト: 8080）
  --admin-port <PORT>    管理 API ポート（デフォルト: 8081）
  --config <FILE>        設定ファイルパス（デフォルト: {data}/config.toml）
  --auth-jwt-secret <SECRET>
                         JWT 署名秘密鍵（HS256）。未指定時は認証無効（開発用）
  --auth-jwt-secret-file <FILE>
                         秘密鍵をファイルから読み込む
  --log-level <LEVEL>    ログレベル: error / warn / info / debug（デフォルト: info）

SUBCOMMANDS:
  adlaire-db token create --secret <SECRET> [--db <NAME>] [--expiry <DURATION>]
                           JWT トークンを生成して標準出力へ
```

### 4.2 設定ファイル（config.toml）

```toml
[server]
port       = 8080
admin_port = 8081
log_level  = "info"

[auth]
jwt_secret      = ""       # 空文字列 = 認証無効（開発用）
jwt_secret_file = ""       # ファイルから読む場合はこちら

[storage]
# data-dir は CLI フラグで指定（config.toml に書かない）
```

CLI フラグは config.toml を上書きする（フラグ > 設定ファイル > デフォルト値）。

---

## 5. 認証

### 5.1 方式

**JWT（JSON Web Token）HS256** を採用する。

- サーバー起動時に `--auth-jwt-secret` で共有秘密鍵を指定する
- クライアントは `Authorization: Bearer <JWT>` ヘッダでトークンを送信する
- 秘密鍵未指定時は認証を無効化する（開発・テスト用）

Turso Cloud の認証トークンと同じ JWT クレーム構造を採用し、既存ツールとの互換性を維持する。

### 5.2 JWT クレーム

```json
{
  "iss": "adlaire-db",
  "sub": "<token-id>",
  "iat": 1700000000,
  "exp": 1800000000,        // 省略時 = 無期限
  "a":  "rw"               // "rw" = 読み書き / "ro" = 読み取り専用
}
```

### 5.3 スコープ

| `a` 値 | 許可操作 |
|--------|---------|
| `rw` | 全 DB への読み書き（Phase 1 は全体一律）|
| `ro` | 全 DB への読み取りのみ |

DB 単位のスコープは Phase 2 で追加する。

### 5.4 トークン生成

```bash
# CLI でトークンを生成（Phase 1 では CLI のみ。Phase 2 で管理 API からも発行可能）
adlaire-db token create --secret "my-secret" --expiry 30d
# → eyJ...（標準出力）

# 読み取り専用トークン
adlaire-db token create --secret "my-secret" --access ro
```

### 5.5 トークン失効管理

**tokens.json の構造：**

```json
{
  "tokens": [
    {
      "id":         "tok_abc123",
      "access":     "rw",
      "created_at": "2026-09-10T12:00:00Z",
      "expires_at": "2026-10-10T12:00:00Z",
      "revoked":    false,
      "revoked_at": null
    },
    {
      "id":         "tok_def456",
      "access":     "ro",
      "created_at": "2026-09-01T00:00:00Z",
      "expires_at": null,
      "revoked":    true,
      "revoked_at": "2026-09-10T08:00:00Z"
    }
  ]
}
```

**JWT 検証フロー（リクエストごと）：**

```
1. Authorization: Bearer <JWT> ヘッダを取得
   → なし → 401 AUTH_REQUIRED

2. JWT 署名を HS256 で検証
   → 失敗 → 401 AUTH_INVALID

3. exp クレームを確認
   → 期限切れ → 401 AUTH_EXPIRED

4. sub クレーム（token_id）を tokens.json と照合
   → revoked=true → 401 AUTH_INVALID

5. a クレームと要求権限を照合
   → ro トークンで書き込み → 403 PERMISSION_DENIED

6. 検証通過 → リクエスト処理へ
```

**パフォーマンス：**
- サーバー起動時に `tokens.json` をメモリへロードする
- `DELETE /admin/v1/tokens/{id}` 受信時にメモリ上の失効リストを更新し `tokens.json` を書き直す
- メモリロード後は `tokens.json` の再読み込みは行わない（サーバー再起動で反映）

---

## 6. API 仕様

### 6.1 URL 設計

#### Phase 1（シングル DB）

```
POST http://localhost:8080/v2/pipeline
```

DB 名は URL に含まない。起動時に `--data` で指定した単一 DB を使用する。

#### Phase 2（マルチ DB・パスベースルーティング）

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

#### GET /v2/health

ヘルスチェックエンドポイント（認証不要）。

```
GET /v2/health

Response 200:
{ "status": "ok" }
```

### 6.3 WebSocket API（hrana-ws v3）

（Phase 3 実装。詳細は Phase 3 着手時に策定）

接続先：`ws://localhost:8080/v3/baton`

hrana WebSocket プロトコルに準拠する。libSQL クライアント SDK のインタラクティブトランザクション機能（`db.transaction()`）を有効にする。

### 6.4 管理 API

管理 API は独立したポート（デフォルト 8081）で提供する。外部に公開しないことを推奨する。

#### DB 管理（Phase 2）

```
GET    /admin/v1/databases               DB 一覧
POST   /admin/v1/databases               DB 作成
DELETE /admin/v1/databases/{name}        DB 削除
GET    /admin/v1/databases/{name}        DB 情報取得
```

**POST /admin/v1/databases リクエスト：**

```json
{ "name": "my-db" }
```

**GET /admin/v1/databases レスポンス：**

```json
{
  "databases": [
    {
      "name": "my-db",
      "created_at": "2026-09-10T12:00:00Z",
      "size_bytes": 4096
    }
  ]
}
```

#### トークン管理（Phase 2）

```
POST   /admin/v1/tokens          トークン発行
DELETE /admin/v1/tokens/{id}     トークン失効
GET    /admin/v1/tokens          発行済みトークン一覧
```

**POST /admin/v1/tokens リクエスト：**

```json
{
  "access": "rw",
  "expiry_seconds": 2592000
}
```

**レスポンス：**

```json
{
  "token_id": "tok_abc123",
  "jwt": "eyJ...",
  "expires_at": "2026-10-10T12:00:00Z"
}
```

---

## 7. エラーハンドリング

### 7.1 HTTP ステータスコード

| ステータス | 使用ケース |
|-----------|-----------|
| 200 OK | 正常処理（SQL エラーも 200 で results に含める）|
| 400 Bad Request | リクエスト JSON の形式エラー |
| 401 Unauthorized | JWT なし・JWT 検証失敗 |
| 403 Forbidden | 権限不足（ro トークンで書き込み等）|
| 404 Not Found | 存在しない DB・エンドポイント |
| 500 Internal Server Error | サーバー内部エラー |

### 7.2 エラーレスポンス形式

```json
{
  "error": "database not found: my-db",
  "code":  "DB_NOT_FOUND"
}
```

### 7.3 エラーコード一覧（主要なもの）

| コード | 説明 |
|--------|------|
| `SQLITE_ERROR` | SQL 実行エラー |
| `SQLITE_CONSTRAINT` | 制約違反 |
| `SQLITE_BUSY` | DB ロック中 |
| `AUTH_REQUIRED` | 認証トークンなし |
| `AUTH_INVALID` | JWT 検証失敗 |
| `AUTH_EXPIRED` | トークン期限切れ |
| `DB_NOT_FOUND` | DB が存在しない |
| `DB_ALREADY_EXISTS` | DB が既に存在する |
| `PERMISSION_DENIED` | アクセス権限なし |
| `INTERNAL_ERROR` | サーバー内部エラー |

---

## 8. 実装フェーズ

フェーズ単位で機能を積み上げる。各フェーズの内製化計画はフェーズ着手時に策定する。

### Phase 1：シングル DB・HTTP API（最小動作）

**目標**：libSQL クライアント SDK が Adlaire DB に接続して SQL を実行できる最小構成

**スコープ：**
- libSQL フォーク（sqld）のセットアップ・ビルド確認
- HTTP API `/v2/pipeline` の実装（hrana-http v2 準拠）
- GET `/v2/health`
- JWT 認証（HS256・`--auth-jwt-secret`）
- `adlaire-db token create` サブコマンド
- CLI: `--data` `--port` `--auth-jwt-secret`
- シングルバイナリ起動

**完了条件（検証可能な具体的テストケース）：**

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

TC-3: JWT 認証ありモード
  $ SECRET="test-secret"
  $ TOKEN=$(./adlaire-db token create --secret "$SECRET")
  $ ./adlaire-db serve --data ./testdb --port 8080 --auth-jwt-secret "$SECRET"
  （a）有効なトークンで SQL 実行 → 200 OK
  （b）Authorization ヘッダなし → 401 AUTH_REQUIRED
  （c）不正なトークン → 401 AUTH_INVALID

TC-4: TypeScript SDK 互換性
  const client = createClient({
    url: "http://localhost:8080",
    authToken: "<JWT>",          // 認証なしモードなら省略可
  });
  await client.execute("CREATE TABLE IF NOT EXISTS users (id INT, name TEXT)");
  await client.execute("INSERT INTO users VALUES (1, 'Alice')");
  const result = await client.execute("SELECT * FROM users");
  期待: result.rows[0] = { id: 1, name: "Alice" }

TC-5: データ永続性（I-4 検証）
  （a）INSERT 後にサーバーを Ctrl+C で停止
  （b）同じ --data で再起動
  （c）SELECT で挿入したデータが返ること

TC-6: 起動・停止
  （a）./adlaire-db serve で起動 → "Adlaire DB listening on ..." ログ
  （b）Ctrl+C でクリーンシャットダウン → .lock ファイルが解放される
  （c）再起動できる（.lock がゾンビ残留しない）
```

**対象外（Phase 2 以降）：**
- マルチ DB・管理 API・WebSocket・レプリケーション

---

### Phase 2：マルチ DB・管理 API

**目標**：1インスタンスで複数 DB を管理できる

- パスベース DB ルーティング（`/{db-name}/v2/pipeline`）
- 管理 API（DB CRUD・トークン CRUD）
- DB ごとのデータ分離
- DB 単位のアクセス制御（JWT クレーム拡張）

**完了条件（テストケース）：**

```
TC-2-1: マルチ DB SQL 実行
  （a）POST /admin/v1/databases {"name":"db_a"} → 201
  （b）POST /admin/v1/databases {"name":"db_b"} → 201
  （c）POST /db_a/v2/pipeline で db_a に CREATE TABLE t(v TEXT); INSERT
  （d）POST /db_b/v2/pipeline で db_b に CREATE TABLE t(v TEXT); 別データ INSERT
  （e）db_a の SELECT → db_a のデータのみ返る
  （f）db_b の SELECT → db_b のデータのみ返る（db_a のデータは見えない）

TC-2-2: 管理 API — DB CRUD
  （a）GET /admin/v1/databases → [] （初期は空リスト）
  （b）POST /admin/v1/databases {"name":"testdb"} → 201, {id,name,created_at}
  （c）GET /admin/v1/databases → [testdb] がリストに含まれる
  （d）GET /admin/v1/databases/testdb → 200, DB の詳細情報
  （e）DELETE /admin/v1/databases/testdb → 204
  （f）GET /admin/v1/databases/testdb → 404
  （g）POST /testdb/v2/pipeline（削除後） → 404 DB_NOT_FOUND

TC-2-3: DB 名バリデーション
  （a）POST /admin/v1/databases {"name":""} → 400 INVALID_DB_NAME
  （b）POST /admin/v1/databases {"name":"a b"} → 400 INVALID_DB_NAME（スペース不可）
  （c）POST /admin/v1/databases {"name":"../evil"} → 400 INVALID_DB_NAME（パストラバーサル不可）
  （d）POST /admin/v1/databases {"name":"validname"} → 201（英数字・ハイフン・アンダースコアは有効）
  （e）同名 DB を再作成 → 409 DB_ALREADY_EXISTS

TC-2-4: トークン CRUD
  （a）POST /admin/v1/tokens {"access":"rw","expiry":"30d"} → 201, {id,token,access,expires_at}
  （b）GET /admin/v1/tokens → 発行済みトークン一覧（secret は含まない）
  （c）GET /admin/v1/tokens/{id} → トークン詳細（revoked フラグ含む）
  （d）DELETE /admin/v1/tokens/{id} → 204（revoke 実行）
  （e）GET /admin/v1/tokens/{id} → revoked:true になっている

TC-2-5: トークン失効の即時反映
  （a）有効トークン T で POST /v2/pipeline → 200
  （b）DELETE /admin/v1/tokens/{T.id} で T を失効
  （c）同じトークン T で POST /v2/pipeline → 401 AUTH_INVALID（失効反映が即時であること）
  （d）新規トークン T2 で POST /v2/pipeline → 200（他のトークンは影響なし）

TC-2-6: データディレクトリ永続化（マルチ DB）
  （a）db_a / db_b を作成し各テーブルにデータ投入
  （b）サーバーを停止・再起動（同じ --data ディレクトリ）
  （c）db_a・db_b 両方のデータが復元されること
  （d）{data-dir}/databases/ 以下に db_a/ db_b/ ディレクトリが存在すること
  （e）{data-dir}/meta/databases.json に両 DB が記録されていること
```

---

### Phase 3：WebSocket API・埋め込みレプリカ

**目標**：Turso のインタラクティブトランザクション・埋め込みレプリカが動作する

- WebSocket エンドポイント `/v3/baton`（hrana-ws プロトコル）
- インタラクティブトランザクション（`BEGIN` / `COMMIT` / `ROLLBACK`）
- 埋め込みレプリカ同期プロトコル対応

**完了条件：**
- libSQL TypeScript SDK の `db.transaction()` が動作する
- embedded replica が Adlaire DB と同期できる

---

### Phase 4：レプリケーション

**目標**：プライマリ・レプリカ構成での運用

- WAL ベースのレプリカ同期（libSQL フォークの機能を利用）
- プライマリ書き込み・レプリカ読み取りルーティング
- プライマリ障害時の動作（詳細はフェーズ着手時に設計）

**完了条件：**
- プライマリ + レプリカ構成でデータが同期される
- レプリカへの書き込みがプライマリへリダイレクトされる

---

### Phase 5 以降

Phase 4 完了後に計画する。候補（優先度未確定）：

- ブランチ・ポイントインタイムリストア
- libSQL 内部コンポーネントの段階的内製化
- 高可用性・水平スケール
- 監視・メトリクス API

---

## 9. 配布・デプロイ

- シングルバイナリ（`adlaire-db`）として配布
- ターゲット：Linux x86_64 / aarch64
- 静的リンク（musl）によるランタイム依存ゼロを目標（Phase 1 完了後に検討）
- 配布チャネル：GitHub Releases
- リリース成果物には SHA-256 チェックサムを添付する

---

## 10. ログ仕様

### 10.1 フォーマット

構造化 JSON Lines（1 行 1 イベント）。

```json
{"ts":"2024-01-01T00:00:00.123Z","level":"INFO","msg":"request completed","method":"POST","path":"/v2/pipeline","status":200,"duration_ms":3,"db":null,"error":null}
```

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `ts` | string (RFC 3339, ms 精度) | ✓ | イベント発生時刻（UTC） |
| `level` | string | ✓ | `TRACE` / `DEBUG` / `INFO` / `WARN` / `ERROR` |
| `msg` | string | ✓ | 人間可読メッセージ |
| `method` | string | HTTP リクエスト時 | HTTP メソッド |
| `path` | string | HTTP リクエスト時 | リクエストパス |
| `status` | integer | HTTP レスポンス時 | HTTP ステータスコード |
| `duration_ms` | integer | HTTP リクエスト時 | 処理時間（ミリ秒） |
| `db` | string \| null | マルチ DB 時 | 対象 DB 名（Phase 2〜） |
| `error` | string \| null | エラー時 | エラーコードまたはメッセージ |

### 10.2 ログレベル

| レベル | 用途 |
|---|---|
| `ERROR` | リクエスト処理失敗・起動失敗・ファイル I/O エラー |
| `WARN` | 認証失敗・存在しない DB へのアクセス・設定非推奨 |
| `INFO` | 起動・停止・HTTP リクエスト完了（デフォルト） |
| `DEBUG` | SQL 実行詳細・WAL チェックポイント |
| `TRACE` | hrana プロトコル詳細・バイト列ダンプ |

デフォルトレベル：`INFO`。`--log-level` フラグまたは環境変数 `ADLAIRE_LOG_LEVEL` で変更可。

### 10.3 出力先

- デフォルト：stdout（コンテナ・systemd との親和性）
- `--log-file <PATH>` 指定時：ファイルへ書き出し（ローテーションは外部ツール任せ）
- stdout とファイルの同時出力は非サポート（Phase 1 時点）

### 10.4 起動・停止ログ例

```
{"ts":"...","level":"INFO","msg":"Adlaire DB starting","version":"0.1.0","data_dir":"/var/lib/adlaire","port":8080}
{"ts":"...","level":"INFO","msg":"Adlaire DB listening","addr":"0.0.0.0:8080","admin_addr":"0.0.0.0:8081"}
{"ts":"...","level":"INFO","msg":"shutdown signal received"}
{"ts":"...","level":"INFO","msg":"Adlaire DB stopped"}
```

---

## 11. WAL 設定

### 11.1 WAL モード

すべての SQLite DB は起動時に WAL モードを有効化する。

```sql
PRAGMA journal_mode = WAL;
```

- WAL により複数の同時読み取りと 1 書き込みが並行可能
- クラッシュ後の自動リカバリは SQLite が保証

### 11.2 設定パラメータ

| パラメータ | デフォルト | CLI フラグ | config.toml キー | 説明 |
|---|---|---|---|---|
| busy timeout | 5000 ms | `--busy-timeout` | `[storage] busy_timeout_ms` | ロック待機タイムアウト。超過時 503 BUSY |
| WAL checkpoint interval | 1000 pages | — | `[storage] wal_checkpoint_pages` | 自動チェックポイントのページ閾値 |
| WAL checkpoint mode | `PASSIVE` | — | `[storage] wal_checkpoint_mode` | `PASSIVE` / `FULL` / `RESTART` |
| synchronous | `NORMAL` | — | `[storage] synchronous` | `OFF` は非サポート（I-4 違反） |

### 11.3 チェックポイント挙動

- SQLite のデフォルト自動チェックポイント（1000 pages）をそのまま使用（Phase 1）
- Phase 1 では手動チェックポイントの API は提供しない
- Phase 4（レプリケーション）時に WAL チェックポイント制御を再設計する

### 11.4 busy timeout エラー

WAL ロック待機が `busy_timeout_ms` を超えた場合：

```json
{
  "error": {
    "message": "database is busy",
    "code": "STORAGE_BUSY"
  }
}
```

HTTP ステータス：503

---

## 付録：用語定義

| 用語 | 定義 |
|------|------|
| Turso Cloud | libSQL のマネージドホスティングサービス。Adlaire DB の機能パリティ参照先 |
| libSQL | SQLite フォーク。HTTP API・WAL レプリケーション等を追加した OSS DB ライブラリ |
| sqld | libSQL のサーバーコンポーネント。HTTP API・WebSocket API を提供する |
| libSQL フォーク | Adlaire DB 専用に改変した libSQL（sqld 含む）。本プロジェクトの全体基盤 |
| hrana | Turso / libSQL のワイヤプロトコル名。hrana-http（HTTP版）と hrana-ws（WebSocket版）がある |
| 埋め込みレプリカ | クライアント側ローカルに SQLite DB を持ち、リモート libSQL DB と同期する仕組み |
| baton | hrana プロトコルにおけるセッション継続識別子 |
