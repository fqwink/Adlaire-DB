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

#### 3.3.1 Cargo ワークスペース構成

Adlaire DB のリポジトリは libSQL フォークを Git submodule として管理し、Cargo workspace で参照する。

```
adlaire-db/              ← このリポジトリ
├── Cargo.toml           ← workspace root
├── crates/
│   └── adlaire-server/  ← Adlaire サーバー層
│       ├── Cargo.toml
│       └── src/
└── libsql/              ← git submodule (fqwink/libsql fork)
    ├── sqld/            ← sqld crate
    └── libsql-sys/      ← SQLite バインディング
```

**workspace Cargo.toml：**

```toml
[workspace]
members = [
    "crates/adlaire-server",
    "libsql/sqld",
]
resolver = "2"
```

**adlaire-server/Cargo.toml 主要依存：**

```toml
[dependencies]
sqld = { path = "../../libsql/sqld", default-features = false, features = ["core"] }
tokio  = { version = "1", features = ["full"] }
axum   = "0.7"
tower  = "0.4"
serde  = { version = "1", features = ["derive"] }
serde_json = "1"
jsonwebtoken = "9"

[build-dependencies]
# libsql-sys が SQLite をコンパイルするため cc が必要
cc = "1"
```

#### 3.3.2 sqld との境界（呼び出しインターフェース）

Adlaire サーバー層が sqld に対して行う操作は以下の 3 種類に限定する（Phase 1 時点）。

**① DB オープン（起動時・Phase 2 は DB 作成時）**

```rust
// 擬似コード。実際の型名は libSQL フォーク実装時に確定する
let db: sqld::Database = sqld::Database::open(path, sqld::Config {
    journal_mode: JournalMode::Wal,
    busy_timeout: Duration::from_millis(5000),
    ..Default::default()
})?;
```

**② SQL 実行（hrana-http v2 pipeline リクエストごと）**

```rust
let conn: sqld::Connection = db.connect()?;
let result: sqld::QueryResult = conn.execute_batch(&statements)?;
// result を hrana-http v2 レスポンス形式に変換して返す
```

**③ DB クローズ（DB 削除時・サーバーシャットダウン時）**

```rust
drop(conn);
drop(db); // Drop で WAL チェックポイント + ファイルクローズ
```

Adlaire サーバー層は sqld の HTTP サーバー・認証・レプリケーション機能を一切呼び出さない。これらは sqld の `core` feature フラグで無効化する（フラグが存在しない場合は Phase 1 着手時に feature 分割を実施する）。

#### 3.3.3 hrana-http v2 プロトコル変換層

sqld の `QueryResult` → hrana-http v2 レスポンス JSON への変換は Adlaire サーバー層で実装する。

```
POST /v2/pipeline
  ↓ リクエスト JSON をパース（Adlaire）
  ↓ statements[] を sqld::Connection に渡す（Adlaire → sqld 境界）
  ↓ sqld::QueryResult を受け取る（sqld → Adlaire 境界）
  ↓ hrana-http v2 results[] 形式に変換（Adlaire）
  ↓ JSON レスポンスを返す（Adlaire）
```

sqld が独自の hrana 実装を持つ場合、その型をそのまま流用することも可とする（実装時判断）。ただし sqld の HTTP サーバーを起動する形にはしない。

#### 3.3.4 Phase 1 の依存ロックダウン方針

- libSQL フォークのコミットハッシュを submodule で固定する
- Phase 1 着手時に `Cargo.lock` をリポジトリにコミットし、依存バージョンをロックする
- フォーク内部の変更は必ず diff レビューを行い、意図しない upstream 取り込みを防ぐ

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
port       = 8080          # HTTP API ポート
admin_port = 8081          # 管理 API ポート
log_level  = "info"        # trace / debug / info / warn / error
log_file   = ""            # 空 = stdout。パス指定でファイル出力

[auth]
jwt_secret      = ""       # 空文字列 = 認証無効（開発用）
jwt_secret_file = ""       # ファイルから読む場合はこちら（jwt_secret より優先）

[admin]
auth_token = ""            # 管理 API 認証トークン（空 = 認証無効）
                           # 本番では必ず設定する

[storage]
# data-dir は CLI フラグで指定（config.toml に書かない）
busy_timeout_ms      = 5000     # WAL ロック待機タイムアウト（ミリ秒）
wal_checkpoint_pages = 1000     # 自動チェックポイントのページ閾値
wal_checkpoint_mode  = "PASSIVE"  # PASSIVE / FULL / RESTART
synchronous          = "NORMAL"   # OFF は非サポート
```

優先順位：CLI フラグ > 設定ファイル > デフォルト値。

`jwt_secret` と `jwt_secret_file` を両方指定した場合は `jwt_secret_file` を優先する。

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

### 5.3 スコープ（Phase 1）

| `a` 値 | 許可操作 |
|--------|---------|
| `rw` | 全 DB への読み書き |
| `ro` | 全 DB への読み取りのみ |

Phase 1 ではトークンのスコープは全体一律。DB 単位の制御は Phase 2 で追加する。

### 5.3b DB スコープ（Phase 2）

Phase 2 から JWT に省略可能な `dbs` クレームを追加する。

**グローバルトークン（Phase 1 互換・Phase 2 以降も有効）：**

```json
{
  "iss": "adlaire-db",
  "sub": "tok_abc123",
  "iat": 1700000000,
  "exp": 1800000000,
  "a":  "rw"
}
```

`dbs` が存在しない場合は全 DB に `a` クレームのアクセスを適用する（Phase 1 挙動と同じ）。

**DB スコープトークン（Phase 2〜）：**

```json
{
  "iss": "adlaire-db",
  "sub": "tok_def456",
  "iat": 1700000000,
  "exp": 1800000000,
  "a":  "ro",
  "dbs": {
    "analytics": "rw",
    "reports":   "ro"
  }
}
```

`dbs` クレームが存在する場合の権限解決ルール：

| 条件 | 適用アクセス |
|------|-------------|
| `dbs[db_name]` が存在する | `dbs[db_name]` の値を使用 |
| `dbs[db_name]` が存在しない | `a` クレームを使用 |

つまり `dbs` は個別 DB のデフォルト（`a`）を上書きする。全 DB を拒否するには `"a": "ro"` のうえ書き込みが必要な DB のみ `"dbs": {"target": "rw"}` で許可するパターンを使う。

**POST /admin/v1/tokens の DB スコープ指定：**

```json
{
  "access": "ro",
  "expiry": "30d",
  "dbs": {
    "analytics": "rw"
  }
}
```

`dbs` 省略時はグローバルトークン（`dbs` クレームなし）を発行する。

**Phase 2 JWT 検証フロー（DB スコープ対応版）：**

```
1. Authorization: Bearer <JWT> ヘッダを取得
   → なし → 401 AUTH_REQUIRED

2. JWT 署名を HS256 で検証
   → 失敗 → 401 AUTH_INVALID

3. exp クレームを確認
   → 期限切れ → 401 AUTH_EXPIRED

4. sub クレーム（token_id）を tokens.json と照合
   → revoked=true → 401 AUTH_INVALID

5. リクエスト対象 DB のアクセスレベルを解決
   dbs[db_name] が存在する → その値を使用
   存在しない              → a クレームを使用

6. 解決したアクセスレベルと要求操作を照合
   → ro で書き込み操作 → 403 PERMISSION_DENIED

7. 検証通過 → リクエスト処理へ
```

### 5.4 トークン生成

```bash
# グローバル rw トークン（Phase 1 と同じ）
adlaire-db token create --secret "my-secret" --expiry 30d

# グローバル ro トークン
adlaire-db token create --secret "my-secret" --access ro

# DB スコープトークン（Phase 2〜）
adlaire-db token create --secret "my-secret" \
  --access ro \
  --db analytics:rw \
  --db reports:ro
# → eyJ...（標準出力）
```

### 5.5 トークン失効管理

**tokens.json の構造：**

```json
{
  "tokens": [
    {
      "id":         "tok_abc123",
      "access":     "rw",
      "dbs":        null,
      "created_at": "2026-09-10T12:00:00Z",
      "expires_at": "2026-10-10T12:00:00Z",
      "revoked":    false,
      "revoked_at": null
    },
    {
      "id":         "tok_def456",
      "access":     "ro",
      "dbs":        {"analytics": "rw", "reports": "ro"},
      "created_at": "2026-09-01T00:00:00Z",
      "expires_at": null,
      "revoked":    false,
      "revoked_at": null
    }
  ]
}
```

`dbs` フィールド：`null` = グローバルトークン、オブジェクト = DB スコープトークン。

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

#### 管理 API 認証

管理ポートへのすべてのリクエストに `Authorization: Bearer <admin-token>` を要求する。

- `admin-token` は config.toml の `[admin] auth_token` または `--admin-auth-token` フラグで設定する
- 未設定時は認証を無効化する（開発・ローカル用。本番では必ず設定すること）
- 認証失敗時: `401 {"error":"unauthorized","code":"AUTH_REQUIRED"}`
- 管理トークンは JWT ではなく任意の文字列で良い（内部的には Bearer 文字列の完全一致で検証）

#### DB 管理（Phase 2）

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
  "name": "my-db",
  "created_at": "2026-09-10T12:00:00Z"
}
```

**GET /admin/v1/databases レスポンス（200 OK）：**

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

**GET /admin/v1/databases/{name} レスポンス（200 OK）：**

```json
{
  "name": "my-db",
  "created_at": "2026-09-10T12:00:00Z",
  "size_bytes": 4096
}
```

**DELETE /admin/v1/databases/{name} レスポンス：** `204 No Content`（ボディなし）

DB 名バリデーション規則：

- 正規表現: `^[a-zA-Z0-9_-]{1,127}$`
- パストラバーサル文字（`/` `.` `..`）は不可
- 予約語：`meta`・`admin` は使用不可

#### トークン管理（Phase 2）

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

### 7.3 エラーコード一覧

| コード | HTTP | 説明 |
|--------|------|------|
| `AUTH_REQUIRED` | 401 | Authorization ヘッダがない |
| `AUTH_INVALID` | 401 | JWT 署名検証失敗・失効済みトークン |
| `AUTH_EXPIRED` | 401 | JWT exp 切れ |
| `PERMISSION_DENIED` | 403 | ro トークンで書き込み操作 |
| `DB_NOT_FOUND` | 404 | 指定 DB が存在しない |
| `TOKEN_NOT_FOUND` | 404 | 指定トークン ID が存在しない |
| `DB_ALREADY_EXISTS` | 409 | 同名 DB が既に存在する |
| `INVALID_DB_NAME` | 400 | DB 名がバリデーションを通過しない |
| `INVALID_REQUEST` | 400 | リクエスト JSON が不正 |
| `SQLITE_ERROR` | 400 | SQL 構文・実行エラー |
| `SQLITE_CONSTRAINT` | 400 | 制約違反（UNIQUE 等） |
| `STORAGE_BUSY` | 503 | WAL ロック待機タイムアウト |
| `INTERNAL_ERROR` | 500 | サーバー内部エラー |

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

ETC-3: DB 未存在エラー（Phase 2〜）
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

ETC-6: DB 名バリデーション（Phase 2〜）
  （a）空文字列 POST /admin/v1/databases {"name":""}
      → 400 {"error":"...","code":"INVALID_DB_NAME"}

  （b）スペース含む POST /admin/v1/databases {"name":"my db"}
      → 400 {"code":"INVALID_DB_NAME"}

  （c）パストラバーサル POST /admin/v1/databases {"name":"../etc"}
      → 400 {"code":"INVALID_DB_NAME"}

  （d）127 文字以内・英数字・ハイフン・アンダースコアのみ有効
      "valid-name_123" → 201
      長さ 128 文字の文字列 → 400 {"code":"INVALID_DB_NAME"}

ETC-7: 重複エラー（Phase 2〜）
  POST /admin/v1/databases {"name":"dup"}
  POST /admin/v1/databases {"name":"dup"}（同名再作成）
  → 409 {"error":"database already exists: dup","code":"DB_ALREADY_EXISTS"}

ETC-8: ストレージビジーエラー
  （意図的な長時間トランザクション保持中に別接続で書き込み試行、
    --busy-timeout を短く設定してテスト）
  → 503 {"error":"database is busy","code":"STORAGE_BUSY"}
```

---

## 8. 起動・停止シーケンス

### 8.1 起動シーケンス（`adlaire-db serve`）

```
Step 1: 設定解決
  1-1. CLI フラグをパース
  1-2. --config で指定された config.toml を読み込む（なければスキップ）
  1-3. 優先順位に従い設定値をマージ（フラグ > config.toml > デフォルト）
  1-4. --data が未指定なら Error: --data is required で終了

Step 2: JWT シークレット検証
  2-1. jwt_secret_file が指定されていればファイルを読み込む
  2-2. いずれも未指定なら WARN "auth is disabled (no jwt_secret configured)"
  2-3. secret が指定されている場合、長さを検証
       → 32 バイト未満なら Error: jwt_secret must be at least 32 bytes で終了

Step 3: データディレクトリ準備
  3-1. {data-dir} が存在しなければ mkdir -p で作成（パーミッション 700）
  3-2. {data-dir}/meta/ が存在しなければ作成
  3-3. {data-dir}/databases/ が存在しなければ作成
  3-4. {data-dir} のパーミッションを確認
       → 700 未満なら WARN "data directory permissions are too open: {mode}"

Step 4: プロセスロック取得
  4-1. {data-dir}/.lock の排他ロック（flock LOCK_EX | LOCK_NB）を試みる
  4-2. 失敗（EWOULDBLOCK）なら Error: another adlaire-db process is running で終了
  4-3. 成功したら .lock を保持したまま続行

Step 5: メタデータ読み込み（Phase 1 はシングル DB のためスキップ可）
  5-1. {data-dir}/meta/databases.json が存在すれば読み込みメモリに展開
       なければ空のリスト `{"databases":[]}` として初期化し書き出す
  5-2. {data-dir}/meta/tokens.json が存在すれば読み込みメモリに展開
       なければ空のリスト `{"tokens":[]}` として初期化し書き出す

Step 6: DB オープン（Phase 1 はシングル DB）
  6-1. {data-dir}/databases/ 以下の各 DB ディレクトリを列挙
  6-2. 各 DB の data.db を sqld::Database::open()
  6-3. WAL モードを設定（PRAGMA journal_mode = WAL）
  6-4. busy timeout を設定（busy_timeout_ms）
  6-5. synchronous を設定（PRAGMA synchronous = NORMAL）
  ※ いずれかで失敗した場合は Error: failed to open database '{name}': {err} で終了

Step 7: HTTP サーバー起動
  7-1. API ポート（デフォルト 0.0.0.0:8080）でソケットを bind
  7-2. 管理ポート（デフォルト 127.0.0.1:8081）でソケットを bind
  7-3. いずれかで失敗した場合は Error: failed to bind port {n}: {err} で終了

Step 8: 起動完了
  INFO {"msg":"Adlaire DB starting","version":"<semver>","data_dir":"...","port":8080}
  INFO {"msg":"Adlaire DB listening","addr":"0.0.0.0:8080","admin_addr":"127.0.0.1:8081"}
```

### 8.2 停止シーケンス（SIGINT / SIGTERM 受信時）

```
Step 1: シャットダウン開始
  INFO {"msg":"shutdown signal received","signal":"SIGTERM"}

Step 2: 新規リクエスト受付を停止
  HTTP リスナーを閉じる。処理中のリクエストは最大 --shutdown-timeout（デフォルト 5s）待機する。
  タイムアウト超過の場合は強制終了する（WARN ログを出力）。

Step 3: DB クローズ
  各 sqld::Connection を drop する（WAL をフラッシュ）
  各 sqld::Database を drop する（チェックポイント + ファイルクローズ）

Step 4: プロセスロック解放
  {data-dir}/.lock の flock を解放する（プロセス終了で自動解放されるが明示的に行う）

Step 5: 停止完了
  INFO {"msg":"Adlaire DB stopped"}
  exit code 0
```

### 8.3 異常終了・リカバリ

| シナリオ | 挙動 |
|----------|------|
| クラッシュ（SIGKILL 等） | SQLite WAL がコミット済みデータを保護する。次回起動時に WAL ロールフォワードが自動実行される |
| `.lock` ゾンビ残留 | flock はプロセス死亡で自動解放される。手動削除は不要 |
| `databases.json` 破損 | Error: failed to parse databases.json で起動失敗。バックアップから復元する |
| `tokens.json` 破損 | Error: failed to parse tokens.json で起動失敗。バックアップから復元するか空リストで初期化 |
| data.db 破損 | SQLite の PRAGMA integrity_check を実行し異常なら起動失敗 |

### 8.4 初期化フラグ優先順位まとめ

```
--auth-jwt-secret-file > ADLAIRE_JWT_SECRET (env) > --auth-jwt-secret > config.toml [auth] jwt_secret
--data               > config.toml [storage] data_dir  （config.toml に書かないことを推奨）
--port               > config.toml [server] port        (default: 8080)
--admin-port         > config.toml [server] admin_port  (default: 8081)
--log-level          > ADLAIRE_LOG_LEVEL (env) > config.toml [server] log_level  (default: info)
--busy-timeout       > config.toml [storage] busy_timeout_ms  (default: 5000)
```

---

## 9. 実装フェーズ


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

**Phase 1 実装タスク一覧：**

依存関係に沿った順序で示す。括弧内は対応する spec セクション・テストケース。

```
T-1: リポジトリ・ビルド基盤
  [ ] Cargo workspace 初期化（adlaire-server crate + libsql submodule）
  [ ] libSQL フォークを git submodule として追加
  [ ] sqld crate が core feature でビルドできることを確認
  [ ] CI: cargo build / cargo test が通る状態を維持
  参照: §3.3.1

T-2: CLI フレームワーク
  [ ] clap による `serve` / `token` サブコマンドの骨格実装
  [ ] `serve` フラグ: --data, --port, --admin-port, --auth-jwt-secret,
      --auth-jwt-secret-file, --log-level, --busy-timeout, --shutdown-timeout
  [ ] config.toml 読み込み（フラグ > config.toml > デフォルト）
  [ ] --data 未指定時の起動エラー
  参照: §4.1, §4.2, §8.4

T-3: データディレクトリ初期化
  [ ] --data パスの作成（mkdir -p）
  [ ] .lock ファイルによる排他ロック（flock）
  [ ] databases/ meta/ サブディレクトリ作成
  [ ] ディレクトリパーミッション警告（700 未満で WARN）
  参照: §3.2, §8.1 Step 3〜4, §9.3

T-4: sqld 統合・DB オープン
  [ ] sqld::Database::open() でシングル DB を開く
  [ ] PRAGMA journal_mode = WAL を起動時に適用
  [ ] busy_timeout を設定
  [ ] PRAGMA synchronous = NORMAL を設定
  [ ] サーバーシャットダウン時に drop（WAL flush + close）
  参照: §3.3.2, §11, §8.1 Step 6, §8.2 Step 3
  検証: TC-5（データ永続性）

T-5: HTTP サーバー骨格（axum）
  [ ] tokio ランタイム起動
  [ ] axum Router: POST /v2/pipeline, GET /v2/health
  [ ] --port でバインドアドレスを指定
  [ ] SIGINT / SIGTERM ハンドラ登録（graceful shutdown）
  [ ] "Adlaire DB listening" INFO ログ出力
  参照: §6.1, §8.1 Step 7〜8, §8.2
  検証: TC-2（ヘルスチェック）, TC-6（起動・停止）

T-6: hrana-http v2 パイプライン実装
  [ ] POST /v2/pipeline のリクエスト JSON デシリアライズ
      （baton, requests[].type, requests[].stmt.sql/args/want_rows）
  [ ] requests を sqld::Connection.execute_batch() に渡す
  [ ] sqld::QueryResult を hrana-http v2 results[] 形式に変換
      （cols, rows, rows_affected, last_insert_rowid）
  [ ] SQL エラーを results[i].type="error" として返す（HTTP 200 のまま）
  [ ] "close" type リクエストを正しく処理する
  参照: §6.2, §3.3.3
  検証: TC-1（SQL 実行）, TC-4（TypeScript SDK 互換）

T-7: JWT 認証ミドルウェア
  [ ] jsonwebtoken crate で HS256 検証
  [ ] Authorization: Bearer <JWT> ヘッダ抽出
  [ ] 6 ステップ検証フロー実装（§5.5）
      ①ヘッダ有無, ②署名, ③exp, ④revoke リスト照合,
      ⑤アクセスレベル, ⑥通過
  [ ] --auth-jwt-secret 未設定時は認証をスキップ（WARN ログ）
  [ ] 各エラーの HTTP ステータス・コード返却（§7.3）
  参照: §5.1〜5.5, §7.3
  検証: TC-3（JWT 認証）, ETC-1（認証エラー）, ETC-2（権限エラー）

T-8: tokens.json 読み込み・revoke リスト
  [ ] 起動時に meta/tokens.json をメモリに展開（§8.1 Step 5）
  [ ] なければ空リストで初期化・書き出し
  [ ] JWT 検証ステップ④での revoke 照合
  [ ] Phase 1 では tokens.json の更新は CLI のみ（管理 API は Phase 2）
  参照: §5.5

T-9: `token create` サブコマンド
  [ ] --secret <VALUE>, --expiry <DURATION>, --access ro|rw フラグ
  [ ] JWT を HS256 で署名して stdout に出力
  [ ] tok_<random> 形式の token_id を生成（sub クレーム）
  [ ] tokens.json に新規トークンを追記
  参照: §4.1, §5.2, §5.4
  検証: TC-3（TOKEN=$(./adlaire-db token create ...)）

T-10: ログ実装
  [ ] tracing crate + tracing-subscriber（JSON Lines 出力）
  [ ] --log-level フラグ対応
  [ ] HTTP リクエストごとの INFO ログ（method, path, status, duration_ms）
  [ ] 起動・停止の INFO ログ
  参照: §12

T-11: 統合テスト・TC 完走確認
  [ ] TC-1: curl で SQL 実行（CREATE / INSERT / SELECT）
  [ ] TC-2: GET /v2/health → {"status":"ok"}
  [ ] TC-3: JWT 認証（有効・ヘッダなし・不正トークン）
  [ ] TC-4: TypeScript SDK (@libsql/client) による CRUD
  [ ] TC-5: データ永続性（再起動後に SELECT）
  [ ] TC-6: 起動・停止・.lock 解放
```

**タスク依存グラフ（最短クリティカルパス）：**

```
T-1 → T-2 → T-3 → T-4 ─┐
                          ├→ T-5 → T-6 → T-7 → T-8 → T-9 → T-10 → T-11
                          └→ T-5（並行可）
```

T-4〜T-5 は並行実装可。T-6（hrana 変換）は T-4・T-5 の両方が揃った後。

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

TC-2-5b: DB スコープトークン
  （a）POST /admin/v1/tokens {"access":"ro","dbs":{"db_a":"rw"}} → 201
  （b）発行トークンで POST /db_a/v2/pipeline INSERT → 200（db_a は rw 許可）
  （c）発行トークンで POST /db_b/v2/pipeline INSERT → 403 PERMISSION_DENIED
       （db_b は dbs に含まれないため a="ro" が適用）
  （d）発行トークンで POST /db_b/v2/pipeline SELECT → 200（読み取りは ro で許可）
  （e）GET /admin/v1/tokens/{id} → dbs フィールドに {"db_a":"rw"} が含まれる

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

#### sqld との統合（Phase 3）

Phase 1〜2 と同様、sqld の WebSocket サーバーループは起動しない。sqld の WebSocket ハンドラを Rust ライブラリとして呼び出すか、hrana-ws プロトコル変換レイヤーを Adlaire で実装するかは Phase 3 着手時に判断する（sqld の hrana-ws 実装の再利用可否を確認）。

#### 埋め込みレプリカ同期

libSQL クライアント SDK の embedded replica 機能は、サーバー側で WAL フレームを HTTP ストリームで提供する同期 API を必要とする。

```
GET /v2/replication/log              WAL フレームのストリーム取得
GET /v2/replication/snapshot         スナップショット取得
POST /v2/replication/heartbeat       接続維持
```

詳細プロトコルは Phase 3 着手時に sqld の実装を参照して確定する。

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

TC-3-5: 埋め込みレプリカ同期
  TypeScript SDK:
  const db = createClient({
    url: "file:local.db",
    syncUrl: "http://localhost:8080",
    authToken: "<JWT>",
  });
  await db.sync();
  const r = await db.execute("SELECT * FROM t");
  期待: サーバー側のデータが local.db に同期される
```

**Phase 3 実装タスク（骨格）：**

```
T3-1: WebSocket サーバー追加（axum の WebSocket upgrade）
T3-2: hrana-ws v3 hello ハンドシェイク + JWT 認証
T3-3: ストリーム多重化レイヤー実装
T3-4: execute / batch / sequence リクエスト処理（sqld 境界再利用）
T3-5: インタラクティブトランザクション状態管理
T3-6: 埋め込みレプリカ同期 API 実装
T3-7: 統合テスト TC-3-1〜TC-3-5
```

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

## 10. セキュリティ考慮事項

### 9.1 JWT シークレット管理

**優先順位：** `--auth-jwt-secret-file` > `--auth-jwt-secret` > 環境変数 `ADLAIRE_JWT_SECRET` > 設定ファイル `[auth] jwt_secret`

| 方法 | 推奨度 | 用途 |
|------|--------|------|
| `--auth-jwt-secret-file <PATH>` | 本番推奨 | ファイルから読み込み。ファイルパーミッション 600 で保護 |
| 環境変数 `ADLAIRE_JWT_SECRET` | 本番可 | コンテナ・systemd での秘密注入に適す |
| `--auth-jwt-secret <VALUE>` | 開発のみ | プロセスリストに secret が露出するため本番不可 |
| config.toml `jwt_secret` | 非推奨 | 設定ファイルが平文で読まれる。git に入れないこと |

secret が未設定の場合は認証を完全に無効化する（起動時に `WARN` ログを出力する）。

**シークレットの最小要件（実装で検証する）：**
- 長さ 32 バイト以上
- 未満の場合: 起動失敗 `Error: jwt_secret must be at least 32 bytes`

**ローテーション方針（Phase 1 時点）：**
- 旧 secret でのトークンは即時無効化される（新 secret で再発行が必要）
- ローテーション手順: 新 secret で新トークン発行 → クライアント切り替え → 旧 secret 廃止
- ゼロダウンタイムローテーション（複数 secret の同時受理）は Phase 1 対象外

### 9.2 管理ポート（8081）のアクセス制御

**デフォルト動作：**

```
bind: 127.0.0.1:8081   ← localhost のみ待機（Phase 1 デフォルト）
```

外部ネットワークへの公開には `--admin-bind 0.0.0.0:8081` が必要。公開する場合は必ず `[admin] auth_token` を設定し、TLS ターミネーション（リバースプロキシ）を前段に置くこと。

**推奨構成（本番）：**

```
Internet → Reverse Proxy (TLS) → :8080 (API)
                                → :8081 (Admin) ← VPN / internal network only
```

### 9.3 データディレクトリのファイルパーミッション

起動時に `--data` で指定したディレクトリのパーミッションを検証・適用する。

| パス | 推奨パーミッション | 内容 |
|------|--------------------|------|
| `{data-dir}/` | `700` | ルートディレクトリ |
| `{data-dir}/meta/` | `700` | tokens.json・databases.json |
| `{data-dir}/meta/tokens.json` | `600` | JWT secret と同等の機密 |
| `{data-dir}/databases/{name}/` | `700` | DB ファイルディレクトリ |
| `{data-dir}/databases/{name}/data.db` | `600` | SQLite 本体 |
| `{data-dir}/databases/{name}/data.db-wal` | `600` | WAL ファイル |

実装方針：
- 起動時に `data-dir` が `700` 未満の場合は `WARN` ログを出力する（強制変更はしない）
- 新規作成するファイル・ディレクトリは上記パーミッションで作成する

### 9.4 TLS

Phase 1〜2 では TLS をネイティブ実装しない。リバースプロキシ（Nginx・Caddy 等）による TLS ターミネーションを推奨する。

```
Client → [TLS] → Nginx/Caddy → [plain HTTP] → adlaire-db :8080
```

TLS ネイティブ対応は Phase 5 以降の検討事項とする。

### 9.5 トークン情報の漏洩防止

- `GET /admin/v1/tokens` および `GET /admin/v1/tokens/{id}` は JWT 文字列（`token` フィールド）を返さない（§6.4 参照）
- JWT 文字列は `POST /admin/v1/tokens` の発行時レスポンスでのみ返す（以降は再取得不可）
- `tokens.json` に JWT 文字列は保存しない（`id` と `access` と有効期限のみ保存）

### 9.6 パストラバーサル対策

DB 名・ファイルパス生成時に以下を必ず適用する：

1. DB 名バリデーション（§6.4 の正規表現 `^[a-zA-Z0-9_-]{1,127}$`）
2. `databases/{name}/` パス構築時に `Path::new(data_dir).join("databases").join(name)` を使用し、`..` を含む入力をバリデーションで事前排除する
3. 予約語（`meta`・`admin`）のブロック

---

## 11. 配布・デプロイ

- シングルバイナリ（`adlaire-db`）として配布
- ターゲット：Linux x86_64 / aarch64
- 静的リンク（musl）によるランタイム依存ゼロを目標（Phase 1 完了後に検討）
- 配布チャネル：GitHub Releases
- リリース成果物には SHA-256 チェックサムを添付する

---

## 12. ログ仕様

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

## 13. WAL 設定

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
