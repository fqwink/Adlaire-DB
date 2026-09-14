# 実行基盤仕様

サーバー構成、データ配置、libsql 統合、設定、起動停止、リカバリを固定する仕様である。

## 4. 実行基盤責務

サーバー構成、データ配置、libsql 統合、設定、起動停止、リカバリを固定する責務である。production path はこの責務に従い、起動時の不整合は黙認せず、仕様に定めた失敗または復旧を選ぶ。

### 4.1 アーキテクチャ責務

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
│           libsql crate 0.6（embedded SQLite）     │
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
│   │   ├── data.db-wal           # WAL（libSQL 管理）
│   │   └── wal-archive/          # PITR 用 WAL アーカイブ（Phase 13〜14, wal_retention_days > 0 時）
│   │       ├── snapshot-000000042.db
│   │       ├── frame-000000043.bin
│   │       └── manifest.json
│   ├── {db-name}___{branch-name}/  # ブランチ DB（Phase 15）
│   │   ├── data.db
│   │   └── data.db-wal
│   └── ...
└── meta/
    ├── databases.json            # DB メタデータ（名前・作成日時・状態）
    ├── tokens.json               # 発行済みトークン一覧（失効管理用）
    └── branches.json             # ブランチメタデータ（Phase 15）
```

### 3.3 libsql crate との統合方式

`libsql` crate（crates.io, embedded SQLite モード）を **Rust ライブラリとして組み込む**。sqld サブプロセスの起動・git submodule の利用は行わない。

```
adlaire-db バイナリ（Rust）
├── Adlaire サーバー層（自前実装）
│   ├── HTTP ルーティング・認証・管理 API
│   └── マルチDB ルーター
└── libsql crate 0.6（embedded SQLite）
    ├── SQL パーサ・クエリエグゼキューター
    ├── WAL 管理
    └── ページストレージ
```

**libsql crate の利用範囲：**

| libsql の機能 | Adlaire での扱い |
|--------------|-----------------|
| SQL パーサ・クエリ実行 | `libsql::Connection::query / execute / execute_batch` を使用 |
| WAL・ページストレージ | libsql が内部処理。`PRAGMA` で設定（journal_mode / synchronous / busy_timeout） |
| HTTP サーバー | libsql は持たない。Adlaire が hyper で実装 |
| 認証 | libsql は持たない。Adlaire が JWT で実装 |
| hrana-http プロトコル | libsql は持たない。Adlaire 独自型（hrana/types.rs）で実装 |

**統合の基本方針：**
- `libsql::Builder::new_local(path).build().await?` でデータベースをオープンする
- `db.connect()?` でコネクションを取得し、クエリを実行する
- HTTP・認証・管理 API は Adlaire が完全に実装し libsql には依存しない

#### 3.3.1 Cargo ワークスペース構成

Adlaire DB のリポジトリはシングルワークスペースで管理する。libSQL は crates.io の `libsql` crate（embedded SQLite モード）を使用する（git submodule は使わない）。

```
adlaire-db/              ← このリポジトリ
├── Cargo.toml           ← workspace root（[workspace.dependencies] で依存一括管理）
├── adlaire-server/      ← Adlaire サーバー層 crate
│   ├── Cargo.toml       ← { workspace = true } 参照のみ
│   └── src/
└── Cargo.lock           ← リポジトリにコミットして依存をロック
```

**workspace Cargo.toml：**

```toml
[workspace]
members = ["adlaire-server"]
resolver = "2"

[workspace.dependencies]
# ── Phase 1〜3（常時有効） ──────────────────────────────────────────────────
anyhow             = "1"
async-trait        = "0.1"
base64             = "0.22"
bytes              = "1"
chrono             = { version = "0.4",  features = ["serde"] }
clap               = { version = "4",    features = ["derive"] }
http-body-util     = "0.1"
hyper              = { version = "1",    features = ["http1", "server"] }
hyper-util         = { version = "0.1",  features = ["tokio"] }
libc               = "0.2"
libsql             = "0.6"              # embedded SQLite（WAL モード）
regex              = "1"
serde              = { version = "1",    features = ["derive"] }
serde_json         = "1"
thiserror          = "2"
tokio              = { version = "1",    features = ["full"] }
toml               = "0.8"
tracing            = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
uuid               = { version = "1",   features = ["v4"] }

# ── Phase 4〜（JWT 認証） ──────────────────────────────────────────────────
jsonwebtoken       = "9"

# ── Phase 9〜（WebSocket） ────────────────────────────────────────────────
tokio-tungstenite  = "0.24"

# ── Phase 10〜（マルチ DB・ATTACH） ────────────────────────────────────────
dashmap            = "6"

# ── Phase 11〜（レプリケーション） ────────────────────────────────────────
url                = { version = "2",   features = ["serde"] }  # ServerRole::Replica の primary_url
crc32fast          = "1"                                          # WAL フレーム整合性チェック

# ── dev のみ ───────────────────────────────────────────────────────────────
# [dev-dependencies] は adlaire-server/Cargo.toml で管理（workspace 共有なし）
```

**adlaire-server/Cargo.toml：**

```toml
[package]
name    = "adlaire-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "adlaire-db"
path = "src/main.rs"

[dependencies]
# Phase 1〜3
anyhow             = { workspace = true }
async-trait        = { workspace = true }
base64             = { workspace = true }
bytes              = { workspace = true }
chrono             = { workspace = true }
clap               = { workspace = true }
http-body-util     = { workspace = true }
hyper              = { workspace = true }
hyper-util         = { workspace = true }
libc               = { workspace = true }
libsql             = { workspace = true }
regex              = { workspace = true }
serde              = { workspace = true }
serde_json         = { workspace = true }
thiserror          = { workspace = true }
tokio              = { workspace = true }
toml               = { workspace = true }
tracing            = { workspace = true }
tracing-subscriber = { workspace = true }
uuid               = { workspace = true }
# Phase 4〜
jsonwebtoken       = { workspace = true }
# Phase 9〜
tokio-tungstenite  = { workspace = true }
# Phase 10〜
dashmap            = { workspace = true }
# Phase 11〜
url                = { workspace = true }
crc32fast          = { workspace = true }

[dev-dependencies]
reqwest  = { version = "0.12", features = ["json"] }
tempfile = "3"
```

#### 3.3.2 libsql crate との境界（呼び出しインターフェース）

Adlaire サーバー層が `libsql` crate に対して行う操作は以下の 3 種類に限定する（Phase 3 時点）。

**① DB オープン（起動時・Phase 6 は DB 作成時）**

```rust
// db/sqld_adapter.rs の RealSqldAdapter::open() 参照（§14.19）
let db = libsql::Builder::new_local(path).build().await?;
let conn = db.connect()?;
let _ = conn.query("PRAGMA journal_mode=WAL", ()).await?;
conn.execute("PRAGMA synchronous=NORMAL", ()).await?;
let _ = conn.query(&format!("PRAGMA busy_timeout={busy_timeout_ms}"), ()).await?;
// skip_integrity_check=false のとき:
// let mut rows = conn.query("PRAGMA integrity_check", ()).await?;
// → ok 以外なら Error: database integrity check failed
```

**② SQL 実行（hrana-http v2 pipeline リクエストごと）**

```rust
// execute() は呼び出しのたびに新規 Connection を生成する（§14.19 注記参照）
let conn = self.db.connect()?;
let mut rows = conn.query(sql, params).await?;       // SELECT 系
conn.execute(sql, params).await?;                    // DML 系
conn.execute_batch(sql).await?;                      // Sequence リクエスト（複文）
// rows → hrana-http v2 results[] 形式に変換して返す
```

**③ DB クローズ（DB 削除時・サーバーシャットダウン時）**

```rust
drop(db); // Arc<libsql::Database> の最後の参照が drop されると WAL チェックポイント + ファイルクローズ
```

Adlaire サーバー層は libsql crate の HTTP サーバー・認証・レプリケーション機能を一切使用しない。組み込み SQLite エンジン部分のみを利用する。

#### 3.3.3 hrana-http v2 プロトコル変換層

`libsql` の行・カラム型 → hrana-http v2 レスポンス JSON への変換は Adlaire サーバー層で実装する。

```
POST /v2/pipeline
  ↓ リクエスト JSON をパース（Adlaire）
  ↓ statements[] を libsql::Connection に渡す（Adlaire → libsql 境界）
  ↓ libsql の行・カラム型を受け取る（libsql → Adlaire 境界）
  ↓ hrana-http v2 results[] 形式に変換（Adlaire）
  ↓ JSON レスポンスを返す（Adlaire）
```

#### 3.3.4 依存ロックダウン方針

- `Cargo.lock` をリポジトリにコミットし、依存バージョンをロックする
- `libsql` crate は crates.io からバージョン固定で取得する（git submodule 不使用）

#### 3.3.5 クレート一覧

**外部クレート一覧：**

| クレート | バージョン | 用途 | 導入フェーズ |
|---------|-----------|------|------------|
| `libsql` | 0.6 | 組み込み SQLite（WAL モード）| 1 |
| `tokio` | 1 | 非同期ランタイム | 1 |
| `hyper` | 1 | 低レベル HTTP ライブラリ | 1 |
| `http-body-util` | 0.1 | リクエストボディ読み取りユーティリティ | 1 |
| `hyper-util` | 0.1 | tokio IO アダプタ（`TokioIo`）| 1 |
| `serde` / `serde_json` | 1 | JSON シリアライズ・デシリアライズ | 1 |
| `jsonwebtoken` | 9 | JWT HS256 署名・検証 | 4 |
| `thiserror` | 2 | `AppError` derive | 1 |
| `anyhow` | 1 | 内部エラーラッパー・`main()` 戻り値 | 1 |
| `clap` | 4 | CLI パース（derive マクロ） | 1 |
| `tracing` | 0.1 | 構造化ログ計装 | 1 |
| `tracing-subscriber` | 0.3 | JSON Lines ログ出力 | 1 |
| `chrono` | 0.4 | `DateTime<Utc>`・タイムスタンプ処理 | 1 |
| `regex` | 1 | DB 名バリデーション（`LazyLock<Regex>`） | 1 |
| `tokio-tungstenite` | 0.24 | WebSocket フレーム送受信 | 9 |
| `toml` | 0.8 | `config.toml` デシリアライズ | 1 |
| `libc` | 0.2 | `flock` による排他プロセスロック | 1 |
| `base64` | 0.22 | Blob フィールドの Base64 エンコード | 1 |
| `uuid` | 1 | DB ID・トークン ID 生成（v4） | 1 |
| `async-trait` | 0.1 | `SqldAdapter` トレイトの async fn | 1 |
| `dashmap` | 6 | `Metrics`・`ReplicationState` の並行マップ | 10 |
| `url` | 2 | `ServerRole::Replica` の `primary_url` 型 | 11 |
| `bytes` | 1 | WAL フレームバッファ（`WalFrame::data`）・hyper レスポンスボディ | 1 |
| `crc32fast` | 1 | WAL フレーム CRC32 チェックサム | 11 |
| `cc`（推移的ビルド依存） | 1 | `libsql-sys` → `libsql` の推移的依存。`libsql-sys` が SQLite C ソースをコンパイルするために使用。`Cargo.toml` には書かない | 1 |

**内製クレート一覧（現行 + 計画）：**

| クレート名（予定） | 状態 | 置き換え対象の外部クレート | 内製化フェーズ |
|------------------|------|--------------------------|-------------|
| `adlaire-server` | 実装中（Phase 1〜） | —（新規実装。置き換えでなく追加） | Phase 1〜 |
| `adlaire-wal` | 計画 | libSQL WAL チェックポイント制御（`sqld`） | Phase 19 |
| `adlaire-storage` | 計画 | libSQL SQLite ページャー（`sqld` / `libsql-sys`） | Phase 19 では readonly adapter まで。write path 切替は Phase 20 以降の仕様変更 PR が承認されるまで禁止 |
| `adlaire-sql-parser` | 計画 | libSQL SQLite パーサ（`libsql-sys`） | Phase 19 では実装禁止。Phase 20 以降の仕様変更 PR が承認されるまで着手禁止 |

内製クレートへの移行は §3.5.4 のロードマップ・§Phase 19 内製化方針に従い段階的に行う。

### 3.4 マルチDB のデータ分離（Phase 6）

**ファイル分離：**

各 DB は独立した SQLite ファイルを持ち、他の DB のファイルとは完全に分離される。

**接続管理：**

| 項目 | Phase 6 実装方針 |
|------|----------------|
| DB ごとの接続数 | 接続 1 本（シンプルな実装から始める）|
| 同一 DB への並行アクセス | SQLite の WAL モードで複数リーダー・シングルライターを実現 |
| 異なる DB への並行アクセス | DB ごとに独立した接続のため干渉なし |
| 接続プール | Phase 6〜18 は単一接続。プール化は Phase 20 以降の仕様変更 PR が承認されるまで実装禁止 |

**DB 作成フロー：**

1. `POST /admin/v1/databases` を受信
2. `databases/{name}/` ディレクトリを作成（既存なら `DB_ALREADY_EXISTS` エラー）
3. `libsql::Builder::new_local()` で `data.db` を初期化（空の SQLite DB）
4. `meta/databases.json` にメタデータを追記
5. 成功レスポンスを返す

**DB 削除フロー：**

1. `DELETE /admin/v1/databases/{name}` を受信
2. 対象 DB への既存接続を閉じる
3. `databases/{name}/` ディレクトリを丸ごと削除
4. `meta/databases.json` からエントリを削除
5. 成功レスポンスを返す

### 3.5 開発・保守方針

#### 3.5.1 hrana-http 変換層の方針

Adlaire は hrana-http の型（ステートメント・カラム・行・エラー）を**自前で定義**し、JSON の受け取りから libsql への受け渡し、結果の返却までを独自実装する。

**採用する方式：Adlaire 独自型 + libsql 直接呼び出し**

```
POST /v2/pipeline
  ↓ Adlaire: JSON → Adlaire 独自型（hrana/types.rs の Stmt 等）にデシリアライズ
  ↓ Adlaire: libsql::Connection::execute() / query() / execute_batch() を呼び出す
  ↓ Adlaire: libsql の行・カラム型を hrana-http v2 JSON にシリアライズ
  ↓ Adlaire: HTTP レスポンスを返す
```

libsql crate の HTTP サーバーや hrana ハンドラは使用しない（libsql embedded モードにはそれらは存在しない）。

#### 3.5.2 依存バージョン管理方針

| 項目 | 方針 |
|------|------|
| **libsql バージョン** | `libsql = "0.6"` を `[workspace.dependencies]` に固定する |
| **Cargo.lock** | リポジトリにコミットし、依存バージョンをロックする |
| **バージョン更新** | マイナーアップデートは changelog を確認し、テストが通ることを確認してから更新する |

#### 3.5.3 Turso Cloud 追従方針

Turso Cloud は Adlaire DB の互換参照実装である。Turso Cloud の挙動、libSQL SDK、hrana 仕様、管理 API、metadata、JWT claim、エラー形式、quota/location/organization/group モデルに変更がある場合は、実装前に互換性差分を仕様書へ反映する。

追従判断は以下の順で行う。

1. 既存 libSQL SDK が利用する wire format と client-visible behavior は最優先で追従する
2. Turso Cloud の管理 API・metadata・エラー形式は、自己ホストで危険な外部依存を要求しない限り追従する
3. 自己ホストでは安全でない挙動は、差分理由、代替仕様、SDK 影響、error code を仕様書に固定してから実装する
4. 追従によって既存 Adlaire DB の metadata migration が必要な場合は、migration 仕様、rollback 方針、後方互換テストを先に定義する

互換性レビューでは、少なくとも次を確認する。

- libSQL TypeScript SDK / Rust SDK / Go SDK からの通常接続が壊れないこと
- hrana-http v2 / hrana-ws v3 の request/response schema が後方互換であること
- JWT claim、token scope、admin auth の意味が Turso Cloud 互換から逸脱していないこと
- DB、location、organization、group、quota の metadata が既存データと共存できること
- エラーの HTTP status、`code`、message 粒度が §7.3 と矛盾しないこと

#### 3.5.4 内製化ロードマップ（Phase 19 以降）

内製化の優先順位は「Turso Cloud 互換を維持したまま Adlaire の差別化に直結するか」と「libsql crate への依存切り離し効果が大きいか」で決める。内製化は §1.5 の活用・進化方針に従い、外部契約を固定したまま adapter 境界の内側を育てる作業として扱う。

| Phase | 対象コンポーネント | 実装可否 | 理由 |
|-------|-------------------|----------|----|
| Phase 1〜18 | HTTP / 認証 / 管理 API / hrana 変換 | 実装可 | Turso Cloud 互換レイヤーとして Adlaire が直接実装する |
| Phase 19 | WAL チェックポイント制御 | 実装可 | レプリケーション、PITR、HA の内部安定性に直結する |
| Phase 19 | libSQL adapter 境界整理 | 実装可 | 外部 API を変えずに内部差し替え可能な境界を固定する |
| Phase 19 | ストレージ層差し替え | readonly adapter と互換テスト追加のみ可 | production write path への切り替えは Phase 20 以降の仕様変更 PR が承認されるまで禁止 |
| Phase 19 | SQL パーサ | 実装禁止 | Turso Cloud / SQLite 互換リスクが高いため、Phase 20 以降の仕様変更 PR が承認されるまで着手禁止 |

内製化は I-5（段階的・計画的）と §1.5 に従い、**各フェーズで動作するテストスイートと Turso Cloud / libSQL SDK 互換テストが通ることを確認してから**次のコンポーネントに進む。内製化 PR は API、認証、metadata、エラー形式、SDK 互換挙動を変更してはならない。変更が必要な場合は、先に Turso Cloud 追従差分として仕様書を改訂する。Adlaire 独自拡張が必要な場合は、Turso 互換 mode と Adlaire 拡張 mode を分け、既定 mode の互換 snapshot に差分を出してはならない。

#### 3.5.5 テスト・CI 方針

**テストの種類と比率（目標）：**

| 種類 | 内容 | 比率目標 |
|------|------|---------|
| ユニットテスト | JWT 検証・hrana JSON 変換・DB 名バリデーション・エラーコード変換 | 60% |
| 統合テスト | `adlaire-db serve` を起動して curl / TypeScript SDK で叩く（TC-1〜TC-6） | 35% |
| E2E テスト | libSQL TypeScript SDK の全 API を実際に通す（TC-4・TC-3-1〜TC-3-6） | 5% |

**CI 構成（GitHub Actions）：**

```yaml
# 実行タイミング: PR 作成・push
jobs:
  build:    cargo build --release
  test:     cargo test
  lint:     cargo clippy -- -D warnings
  fmt:      cargo fmt --check
  integ:    cargo test --test integration  # adlaire-db を起動して叩く
```

**テストカバレッジ方針：**
- JWT 検証の全 6 ステップ（正常・各エラー）は必ずユニットテストを書く
- hrana-http v2 の JSON シリアライズ・デシリアライズはラウンドトリップテストを書く
- TC-1〜TC-6 の統合テストは `cargo test --test integration` で自動実行する
- TypeScript SDK E2E は Phase 5 以降の regression gate とする。CI 環境で Node.js が利用できない場合は、Docker test image 内に Node.js を固定して実行する

### 3.6 データ整合性・保全方針

#### 3.6.1 書き込み耐久性（Design Invariant I-4 の詳細）

すべての書き込み操作はクライアントに成功を返す前に **fsync** を完了しなければならない。

```
クライアント POST /v2/pipeline INSERT
  → libsql: WAL フレームをバッファに書く
  → libsql: fsync（WAL ファイルをディスクに同期）
  → Adlaire: 200 OK を返す
```

`PRAGMA synchronous = NORMAL` の場合、WAL ヘッダへの書き込みはチェックポイント時に fsync される。クラッシュ後は SQLite が自動ロールフォワードして整合性を回復する。`synchronous = OFF` は設定不可（I-4 違反）。

#### 3.6.2 起動時整合性チェック

起動シーケンス Step 6（DB オープン）で各 DB に対して以下を実行する：

```sql
PRAGMA integrity_check;
```

結果が `ok` 以外の場合：

```
ERROR {"msg":"database integrity check failed","db":"mydb","detail":"..."}
→ 起動失敗。破損 DB はオープンしない
```

`--skip-integrity-check` フラグで無効化可（本番での使用は非推奨。ログに WARN を出す）。

#### 3.6.3 WAL フレームチェックサム

レプリケーション（Phase 11）で転送する WAL フレームには CRC32 チェックサムを付与する（Phase 11 レプリケーション API GET /replication/v1/log の `checksum` フィールド）。レプリカ側でフレーム受信後にチェックサムを検証し、不一致の場合はそのフレームを破棄してプライマリへ再送要求する。

Phase 1〜10 ではチェックサム検証はローカル DB への SQLite 書き込みで行われる（WAL の組み込みチェックサム機構を使用）。

#### 3.6.4 レプリケーション書き込み確認モード（Phase 11）

プライマリへの書き込み時に、レプリカへの同期完了を待つかどうかを `--replication-write-mode` で制御する。

| モード | 挙動 | 整合性 | レイテンシ |
|--------|------|--------|-----------|
| `async`（デフォルト） | WAL 書き込み完了で即 200 返却。レプリカ同期はバックグラウンド | 結果整合性 | 低 |
| `sync` | 1 台以上のレプリカが ACK した後に 200 返却 | 強い整合性 | 高 |

```toml
[replication]
write_mode = "async"   # async / sync
sync_timeout_ms = 5000 # sync モード時のタイムアウト。超過時 503 REPLICATION_TIMEOUT
```

sync モードでレプリカが 0 台の場合（スタンドアロンプライマリ）は async と同じ挙動にフォールバックし WARN ログを出す。

#### 3.6.5 定期整合性チェック（オプション）

```toml
[storage]
integrity_check_interval_hours = 0  # 0 = 無効（デフォルト）
```

設定時はバックグラウンドスレッドが指定間隔で各 DB に `PRAGMA integrity_check` を実行する。問題検出時は ERROR ログを出力し、`GET /admin/v1/metrics` の `integrity_errors` カウンターを増加させる。DB はオープンのまま（自動シャットダウンしない）。

#### 3.6.6 WAL リテンションと PITR（Phase 13〜14）

PITR のために WAL フレームを一定期間保持する。

```toml
[storage]
wal_retention_days = 7   # 0 = 無効（デフォルト）
                         # フレームは {data-dir}/databases/{name}/wal-archive/ に保存
```

WAL リテンションが有効な場合、チェックポイントで消去される前に WAL フレームをアーカイブへコピーする。詳細は Phase 13〜14 参照。

---

### 4.2 設定・起動責務

### 4.1 CLI

```
adlaire-db serve [OPTIONS]

OPTIONS:
  --data <PATH>          データディレクトリ（必須）
  --port <PORT>          HTTP リスニングポート（デフォルト: 8080）
  --admin-port <PORT>    管理 API ポート（デフォルト: 8081）
  --config <FILE>        設定ファイルパス（デフォルト: {data}/config.toml）
  --auth-jwt-secret <SECRET>
                         JWT 署名秘密鍵（HS256）。未指定時は認証無効（開発用）。環境変数 ADLAIRE_JWT_SECRET も使用可
  --auth-jwt-secret-file <FILE>
                         秘密鍵をファイルから読み込む
  --admin-auth-token <TOKEN>
                         管理 API 固定認証トークン（未指定時は認証無効）。環境変数 ADLAIRE_ADMIN_TOKEN も使用可
  --log-level <LEVEL>    ログレベル: error / warn / info / debug / trace（デフォルト: info）。環境変数 ADLAIRE_LOG_LEVEL も使用可
  --skip-integrity-check 起動時の PRAGMA integrity_check をスキップ（非推奨。WARN ログ出力）
  --role <ROLE>          サーバーロール: standalone（デフォルト）/ primary / replica
  --primary-port <PORT>  プライマリが WAL ストリームを公開するポート（デフォルト: 8082）
  --primary-url <URL>    レプリカが接続するプライマリの URL（--role replica 時に必須）
  --replication-auth-token <TOKEN>
                         プライマリ・レプリカ間の認証トークン
  --replication-write-mode <MODE>
                         レプリケーション書き込みモード: async / sync（デフォルト: async）
  --busy-timeout <MS>    WAL ロック待機タイムアウト（ミリ秒、デフォルト: 5000）
  --shutdown-timeout <SECS>
                         グレースフルシャットダウン最大待機時間（デフォルト: 30）

SUBCOMMANDS:
  adlaire-db token create --secret <SECRET> [--access ro|rw] [--expiry <DURATION>]
                           [--db DB:ACCESS ...]
                           JWT トークンを生成して標準出力へ
```

**Phase 1 シングル DB の固定パス：**

Phase 1 では DB は `{data-dir}/databases/default/data.db` を固定で使用する。
`/v2/pipeline` は常にこの `default` DB を対象とする。
起動時に `databases/default/` が存在しなければ自動作成する。
Phase 6 移行後も `/v2/pipeline`（DB 名なし）は `default` DB にフォールバックする（後方互換）。

### 4.2 設定ファイル（config.toml）

```toml
[server]
port             = 8080      # HTTP API ポート
admin_port       = 8081      # 管理 API ポート（Phase 1〜5 は 127.0.0.1 固定。--admin-bind は Phase 6 以降）
log_level        = "info"    # trace / debug / info / warn / error
# log_file       = ""        # 空 = stdout。パス指定でファイル出力（未実装）
shutdown_timeout = 30        # グレースフルシャットダウン最大待機秒数
busy_timeout_ms  = 5000      # WAL ロック待機タイムアウト（ミリ秒）

[auth]
jwt_secret      = ""       # 空文字列 = 認証無効（開発用）
jwt_secret_file = ""       # ファイルから読む場合はこちら（jwt_secret が指定されている場合は jwt_secret が優先）

[admin]
auth_token = ""            # 管理 API 認証トークン（空 = 認証無効）
                           # 本番では必ず設定する

[storage]
# data-dir は CLI フラグで指定（config.toml に書かない）
wal_mode                           = "passive"  # passive / full / restart
# wal_checkpoint_pages             = 1000       # 自動チェックポイントのページ閾値（未実装・Phase 11 以降）
# synchronous                      = "NORMAL"   # OFF は非サポート（I-4 違反）（未実装・Phase 11 以降）
# wal_retention_days               = 0          # PITR 用 WAL アーカイブ保持日数（未実装・Phase 13 以降）
# integrity_check_interval_hours   = 0          # 定期整合性チェック間隔（未実装・Phase 13 以降）

[replication]
write_mode      = "async"  # async / sync
sync_timeout_ms = 5000     # sync モード時のタイムアウト（ミリ秒）
```

優先順位：CLI フラグ > 設定ファイル > デフォルト値。

`jwt_secret` と `jwt_secret_file` を両方指定した場合は `jwt_secret` を優先する。

---

### 4.3 起動・停止・リカバリ責務

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

Step 5: メタデータ読み込み（Phase 1〜5 はシングル DB のためスキップ可）
  5-1. {data-dir}/meta/databases.json が存在すれば読み込みメモリに展開
       なければ空のリスト `{"databases":[]}` として初期化し書き出す
  5-2. {data-dir}/meta/tokens.json が存在すれば読み込みメモリに展開
       なければ空のリスト `{"tokens":[]}` として初期化し書き出す
  5-3. {data-dir}/meta/branches.json が存在すれば読み込みメモリに展開（Phase 15〜）
       なければ空のリスト `{"branches":[]}` として初期化し書き出す
  ※ Phase 3 では tokens.json / branches.json の内容は利用しないが、
     Phase 7 以降の管理 API および Phase 15 の branch metadata と同じ配置にするためファイル自体は初期化する

Step 6: DB オープン
  【Phase 1〜5 — シングル DB 固定】
  6-1. {data-dir}/databases/default/ が存在しなければ作成（初回起動時）
  6-2. {data-dir}/databases/default/data.db を libsql::Builder::new_local() でオープン（§14.19 参照）
  6-3. WAL モードを設定（PRAGMA journal_mode = WAL）
  6-4. busy timeout を設定（busy_timeout_ms）
  6-5. synchronous を設定（PRAGMA synchronous = NORMAL）
  6-6. skip_integrity_check フラグが false の場合は PRAGMA integrity_check を実行
       → ok 以外の場合は Error: database integrity check failed で終了
  ※ いずれかで失敗した場合は Error: failed to open database 'default': {err} で終了

  【Phase 6 以降 — マルチ DB】
  6-1. {data-dir}/databases/ 以下の各 DB ディレクトリを列挙
  6-2. 各 DB の data.db を libsql::Builder::new_local() でオープン（整合性チェック含む）
  ※ databases/default/ が存在しない場合も自動作成して後方互換を維持

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
  HTTP リスナーを閉じる。処理中のリクエストは最大 --shutdown-timeout（デフォルト 30s）待機する。
  タイムアウト超過の場合は接続タスクを中断し、WARN ログを出力する。

Step 3: DB クローズ
  Arc<libsql::Database> の最後の参照が drop される（WAL チェックポイント + ファイルクローズ）
  ※ Connection は execute() 呼び出しのたびに都度生成・即 drop するため、シャットダウン時点では保持していない

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
--auth-jwt-secret-file > --auth-jwt-secret > ADLAIRE_JWT_SECRET (env) > config.toml [auth] jwt_secret
--data               > config.toml [storage] data_dir  （config.toml に書かないことを推奨）
--port               > config.toml [server] port        (default: 8080)
--admin-port         > config.toml [server] admin_port  (default: 8081)
--log-level          > ADLAIRE_LOG_LEVEL (env) > config.toml [server] log_level  (default: info)
--busy-timeout       > config.toml [server] busy_timeout_ms  (default: 5000)
--admin-auth-token   > ADLAIRE_ADMIN_TOKEN (env) > config.toml [admin] auth_token
--shutdown-timeout   > config.toml [server] shutdown_timeout  (default: 30)

# Phase 11 レプリケーション設定
--role                      standalone（デフォルト）     CLI のみ（TOML 対応なし）
--primary-port              8082（デフォルト）           CLI のみ（TOML 対応なし）
--primary-url               必須（--role replica 時のみ）CLI のみ（TOML 対応なし）
--replication-auth-token    なし（デフォルト）           CLI のみ（TOML 対応なし）
--replication-write-mode    async（デフォルト）          CLI + config.toml [replication] write_mode
```

---
