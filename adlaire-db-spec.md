# Adlaire DB 仕様書

**バージョン：** 0.33  
**ステータス：** 設計中  
**最終更新：** 2026-09-11  

---

## 1. 概要

### 1.1 プロジェクト概要

Adlaire DB は **libSQL ワイヤプロトコル（hrana-http v2 / hrana-ws v3）互換のサーバー特化 DB サーバー**である。

libSQL クライアント SDK（TypeScript・Rust・Go 等）から接続 URL を差し替えるだけで動作する。クライアント側の埋め込みレプリカ機能は対象外とし、サーバー側の HTTP/WebSocket API・マルチDB管理・レプリケーション・バックアップに特化する。

将来的には libSQL フォークの内部コンポーネント（WAL・ページストレージ・SQL エンジン等）を段階的に内製実装へ置き換えることを計画しているが、具体的な詳細・スケジュールはフェーズの進行とともに検討する。

### 1.2 ポジション

| 比較対象 | Adlaire DB との関係 |
|----------|---------------------|
| Turso Cloud | ワイヤプロトコル（hrana）互換の参照実装。埋め込みレプリカは対象外 |
| libSQL / sqld | フォーク元。Adlaire DB の全体基盤 |
| SQLite | libSQL 経由で互換性を維持 |

### 1.3 固定制約

| 項目 | 内容 |
|------|------|
| 実装言語 | Rust + 標準ライブラリ |
| ストレージ・SQL 基盤 | libSQL フォーク（sqld 含む）|
| 目標機能 | hrana プロトコル互換・サーバー特化機能 |
| 将来方針 | libSQL 内部の段階的内製化（詳細は各フェーズで検討）|
| デプロイ形態 | シングルバイナリ起動 |
| 対象 OS | Linux |

### 1.4 設計不変条件

実装のあらゆる判断においてこれらを最優先する。

**I-1：hrana プロトコル互換**  
既存の libSQL クライアント SDK（TypeScript・Rust・Go 等）が、Turso Cloud の URL を Adlaire DB の URL に差し替えるだけで動作しなければならない。ただし埋め込みレプリカ（`syncUrl` 指定）は対象外とし、通常の HTTP/WebSocket 接続のみを対象とする。

**I-2：外部 DB 依存は libSQL フォーク一本**  
SQLite・libSQL フォーク以外の外部 DB ライブラリ（PostgreSQL・MySQL ドライバ等）に依存しない。

**I-3：シングルバイナリ**  
サーバー起動は `./adlaire-db <flags>` 一コマンドで完結する。外部デーモン・サイドカーを必要としない（Phase 1）。

**I-4：データ永続化の先行保証**  
クライアントへ成功応答を返す前に、書き込みデータが永続化（fsync）されていることを保証する。

**I-5：内製化は段階的・計画的に**  
libSQL 内部コンポーネントの内製化はフェーズ完了後に計画・判断する。「実装が大変だから」という理由で無計画に外部依存を追加することは認めない。

---

## 2. 機能スコープ

### 2.1 クライアント接続

| 機能 | Phase | 説明 |
|------|-------|------|
| HTTP API（hrana-http） | 3 | libSQL クライアント SDK が利用する HTTP/JSON API |
| JWT 認証 | 4 | Bearer トークンによる認証 |
| マルチDB（パスベース） | 6 | URL パスで接続先 DB を指定 |
| WebSocket API（hrana-ws） | 8 | インタラクティブトランザクション用 |
| 埋め込みレプリカ同期 | — | **対象外**（サーバー特化のためスコープ外） |
| ATTACH DATABASE（クロス DB クエリ） | 9 | 管理下 DB 間のみ許可。任意パス指定は禁止 |
| メトリクス API | 9 | 接続数・クエリ数・ストレージ使用量の取得 |
| SQLite 拡張機能ロード | 15 | `.so` / Wasm 拡張（Vector Search 等）のロード |

### 2.2 データベース管理

| 機能 | Phase | 説明 |
|------|-------|------|
| DB 作成・削除・一覧 | 7 | 管理 API 経由での DB ライフサイクル管理 |
| トークン発行・失効 | 7 | DB ごと・全体のトークン管理 |
| バックアップ・エクスポート | 13 | オンラインバックアップ取得・リストア |
| ポイントインタイムリストア | 13 | WAL アーカイブから任意の時点への DB 復元 |
| ブランチ | 14 | DB のブランチ作成（WAL スナップショットから派生） |

### 2.3 レプリケーション

| 機能 | Phase | 説明 |
|------|-------|------|
| プライマリ・レプリカ構成 | 10 | 書き込みはプライマリ、読み取りはレプリカへ |
| WAL ベース同期 | 10 | libSQL の WAL レプリケーションを使用 |
| レプリカへの書き込みリダイレクト | 11 | 307 Temporary Redirect でプライマリへ転送 |

### 2.4 対象外（自己ホストでは不適用）

| 機能 | 理由 |
|------|------|
| データベースロケーション | Turso のエッジノード概念。自己ホストでは単一サーバーのため不要 |
| 組織・グループ管理 | マルチテナント SaaS 向け機能。単一運営者の自己ホストには不要 |
| ストレージクォータ | クラウド課金と連動した機能。自己ホストでは OS レベルで管理 |

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
│   │   ├── data.db-wal           # WAL（libSQL 管理）
│   │   └── wal-archive/          # PITR 用 WAL アーカイブ（Phase 12〜13, wal_retention_days > 0 時）
│   │       ├── snapshot-000000042.db
│   │       ├── frame-000000043.bin
│   │       └── manifest.json
│   ├── {db-name}___{branch-name}/  # ブランチ DB（Phase 14）
│   │   ├── data.db
│   │   └── data.db-wal
│   └── ...
└── meta/
    ├── databases.json            # DB メタデータ（名前・作成日時・状態）
    ├── tokens.json               # 発行済みトークン一覧（失効管理用）
    └── branches.json             # ブランチメタデータ（Phase 14）
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
sqld            = { path = "../../libsql/sqld", default-features = false, features = ["core"] }
tokio           = { version = "1", features = ["full"] }
axum            = "0.7"
tower           = "0.4"
serde           = { version = "1", features = ["derive"] }
serde_json      = "1"
jsonwebtoken    = "9"
thiserror       = "1"                                      # AppError derive
anyhow          = "1"                                      # 内部エラーラッパー・main() 戻り値
chrono          = { version = "0.4", features = ["serde"] } # DateTime<Utc>
crc32fast       = "1"                                      # WAL フレームチェックサム
dashmap         = "5"                                      # Metrics・ReplicationState
regex           = "1"                                      # DB 名バリデーション
url             = "2"                                      # ServerRole::Replica の primary_url
bytes           = "1"                                      # WalFrame::data
clap            = { version = "4", features = ["derive"] } # CLI パース
tracing         = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] } # 構造化ログ出力
tokio-tungstenite  = "0.21"                                # Phase 8: WebSocket
toml            = "0.8"                                    # config.toml パース
libc            = "0.2"                                    # flock による排他ロック
base64          = "0.22"                                   # Blob フィールドの Base64 エンコード
hex             = "0.4"                                    # generate_token_id() の tok_ プレフィックス生成
uuid            = { version = "1", features = ["v4"] }     # DbInfo::id 生成

[dev-dependencies]
reqwest         = { version = "0.12", features = ["json"] } # TestServer HTTP クライアント
tempfile        = "3"                                       # TestServer 一時ディレクトリ

[build-dependencies]
# libsql-sys が SQLite をコンパイルするため cc が必要
cc = "1"
```

#### 3.3.2 sqld との境界（呼び出しインターフェース）

Adlaire サーバー層が sqld に対して行う操作は以下の 3 種類に限定する（Phase 1 時点）。

**① DB オープン（起動時・Phase 6 は DB 作成時）**

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

#### 3.3.5 クレート一覧

**外部クレート一覧：**

| クレート | バージョン | 用途 | 導入フェーズ |
|---------|-----------|------|------------|
| `sqld`（libSQL fork） | submodule 固定 | SQL 実行 / WAL / ページストレージ | 1 |
| `tokio` | 1 | 非同期ランタイム | 1 |
| `axum` | 0.7 | HTTP フレームワーク・ルーティング | 1 |
| `tower` | 0.4 | ミドルウェアスタック | 1 |
| `serde` / `serde_json` | 1 | JSON シリアライズ・デシリアライズ | 1 |
| `jsonwebtoken` | 9 | JWT HS256 署名・検証 | 1 |
| `thiserror` | 1 | `AppError` derive | 1 |
| `anyhow` | 1 | 内部エラーラッパー・`main()` 戻り値 | 1 |
| `clap` | 4 | CLI パース（derive マクロ） | 1 |
| `tracing` | 0.1 | 構造化ログ計装 | 1 |
| `tracing-subscriber` | 0.3 | JSON Lines ログ出力 | 1 |
| `chrono` | 0.4 | `DateTime<Utc>`・タイムスタンプ処理 | 1 |
| `regex` | 1 | DB 名バリデーション（`LazyLock<Regex>`） | 1 |
| `tokio-tungstenite` | 0.21 | WebSocket フレーム送受信 | 8 |
| `toml` | 0.8 | `config.toml` デシリアライズ | 1 |
| `libc` | 0.2 | `flock` による排他プロセスロック | 1 |
| `base64` | 0.22 | Blob フィールドの Base64 エンコード | 1 |
| `hex` | 0.4 | `generate_token_id()` の hex エンコード | 1 |
| `dashmap` | 5 | `Metrics`・`ReplicationState` の並行マップ | 9 |
| `url` | 2 | `ServerRole::Replica` の `primary_url` 型 | 10 |
| `bytes` | 1 | WAL フレームバッファ（`WalFrame::data`） | 10 |
| `crc32fast` | 1 | WAL フレーム CRC32 チェックサム | 10 |
| `cc`（build-dep） | 1 | `libsql-sys` が SQLite をコンパイルするためのビルド依存 | 1 |

**内製クレート一覧（現行 + 計画）：**

| クレート名（予定） | 状態 | 置き換え対象の外部クレート | 内製化フェーズ |
|------------------|------|--------------------------|-------------|
| `adlaire-server` | 実装中（Phase 1〜） | —（新規実装。置き換えでなく追加） | Phase 1〜 |
| `adlaire-wal` | 計画 | libSQL WAL チェックポイント制御（`sqld`） | Phase 11 完了後 |
| `adlaire-storage` | 計画 | libSQL SQLite ページャー（`sqld` / `libsql-sys`） | `adlaire-wal` 内製後 |
| `adlaire-sql-parser` | 計画 | libSQL SQLite パーサ（`libsql-sys`） | 最後（最難関） |

内製クレートへの移行は §3.5.3 のロードマップ・§将来の内製化方針に従い段階的に行う。

### 3.4 マルチDB のデータ分離（Phase 6）

**ファイル分離：**

各 DB は独立した SQLite ファイルを持ち、他の DB のファイルとは完全に分離される。

**接続管理：**

| 項目 | Phase 6 実装方針 |
|------|----------------|
| DB ごとの接続数 | 接続 1 本（シンプルな実装から始める）|
| 同一 DB への並行アクセス | SQLite の WAL モードで複数リーダー・シングルライターを実現 |
| 異なる DB への並行アクセス | DB ごとに独立した接続のため干渉なし |
| 接続プール | Phase 6 は単一接続。Phase 8 以降でプール化を検討 |

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

### 3.5 開発・保守方針

#### 3.5.1 hrana-http 変換層の方針

sqld は hrana-http の型（ステートメント・カラム・行・エラー）を Rust の struct として持つ。Adlaire ではこの**型だけを借用**し、sqld の HTTP サーバーは起動しない。

**採用する方式：sqld 型流用 + Adlaire 独自シリアライズ**

```
POST /v2/pipeline
  ↓ Adlaire: JSON → sqld の Statement 型にデシリアライズ
  ↓ sqld: Connection::execute() を呼び出す
  ↓ sqld: QueryResult 型を返す
  ↓ Adlaire: QueryResult → hrana-http v2 JSON にシリアライズ
  ↓ Adlaire: HTTP レスポンスを返す
```

sqld の hrana HTTP ハンドラ関数（axum router 等）は使わない。JSON ⇔ sqld 型のシリアライズコードが sqld に存在する場合は `pub use` で再利用することを許容するが、sqld の tokio ランタイムや axum インスタンスには依存しない。

理由：sqld の HTTP サーバーを起動すると認証・管理 API の無効化が困難になり、Adlaire の制御から外れるリスクがある。

#### 3.5.2 libSQL フォーク管理方針

| 項目 | 方針 |
|------|------|
| **fork タイミング** | Phase 1 着手直前に `github.com/tursodatabase/libsql` を fork する |
| **fork リポジトリ名** | `fqwink/libsql`（予定）|
| **upstream リモート** | `git remote add upstream https://github.com/tursodatabase/libsql` を登録し追従を可能にする |
| **upstream 追従頻度** | 月 1 回、upstream の `main` をレビューして取り込む。セキュリティパッチは随時 |
| **独自変更の範囲（Phase 1〜7）** | 最小限。sqld の feature flag 追加のみ。SQL パーサ・WAL・ストレージには触れない |
| **独自変更の記録** | `ADLAIRE_PATCHES.md` を fork リポジトリに置き、変更の理由と対象コミットを記録する |
| **upstream との diff 管理** | `git diff upstream/main..HEAD -- sqld/` を CI で常時確認し、意図しない乖離を検出する |

#### 3.5.3 内製化ロードマップ（Phase 15 以降）

内製化の優先順位は「Adlaire の差別化に直結するか」と「upstream との依存切り離し効果が大きいか」で決める。

| 優先 | 対象コンポーネント | 理由 |
|------|-------------------|----|
| 1 | HTTP / 認証 / 管理 API | Phase 1〜7 で既に Adlaire 実装済み。sqld 依存なし |
| 2 | WAL チェックポイント制御 | レプリケーション（Phase 10）に直結。sqld の WAL コードは比較的分離されている |
| 3 | hrana-http/ws プロトコル変換 | 変換レイヤーを自前化すれば sqld の型依存を完全に排除できる |
| 4 | クエリエグゼキューター | SQLite との境界。libsql-sys（C バインディング）を直接呼ぶ形に移行 |
| 5 | SQL パーサ | 最もリスクが高い。Phase 15 後半以降に検討 |

内製化は I-5（段階的・計画的）に従い、**各フェーズで動作するテストスイートが通ることを確認してから**次のコンポーネントに進む。

#### 3.5.4 テスト・CI 方針

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
  upstream: git diff upstream/main..HEAD -- libsql/sqld/ | wc -l  # diff 行数を記録
```

**テストカバレッジ方針：**
- JWT 検証の全 6 ステップ（正常・各エラー）は必ずユニットテストを書く
- hrana-http v2 の JSON シリアライズ・デシリアライズはラウンドトリップテストを書く
- TC-1〜TC-6 の統合テストは `cargo test --test integration` で自動実行する
- TypeScript SDK E2E は Node.js 環境依存のため手動確認を基本とし、CI は任意とする

### 3.6 データ整合性・保全方針

#### 3.6.1 書き込み耐久性（Design Invariant I-4 の詳細）

すべての書き込み操作はクライアントに成功を返す前に **fsync** を完了しなければならない。

```
クライアント POST /v2/pipeline INSERT
  → sqld: WAL フレームをバッファに書く
  → sqld: fsync（WAL ファイルをディスクに同期）
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

レプリケーション（Phase 10）で転送する WAL フレームには CRC32 チェックサムを付与する（Phase 10 レプリケーション API GET /replication/v1/log の `checksum` フィールド）。レプリカ側でフレーム受信後にチェックサムを検証し、不一致の場合はそのフレームを破棄してプライマリへ再送要求する。

Phase 1〜9 ではチェックサム検証はローカル DB への SQLite 書き込みで行われる（WAL の組み込みチェックサム機構を使用）。

#### 3.6.4 レプリケーション書き込み確認モード（Phase 10）

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

#### 3.6.6 WAL リテンションと PITR（Phase 12〜13）

PITR のために WAL フレームを一定期間保持する。

```toml
[storage]
wal_retention_days = 7   # 0 = 無効（デフォルト）
                         # フレームは {data-dir}/databases/{name}/wal-archive/ に保存
```

WAL リテンションが有効な場合、チェックポイントで消去される前に WAL フレームをアーカイブへコピーする。詳細は Phase 12〜13 参照。

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
  --skip-integrity-check 起動時の PRAGMA integrity_check をスキップ（非推奨。WARN ログ出力）
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
port            = 8080     # HTTP API ポート
admin_port      = 8081     # 管理 API ポート（Phase 1〜5 は 127.0.0.1 固定。--admin-bind は Phase 6 以降）
log_level       = "info"   # trace / debug / info / warn / error
log_file        = ""       # 空 = stdout。パス指定でファイル出力
shutdown_timeout = 30      # グレースフルシャットダウン最大待機秒数

[auth]
jwt_secret      = ""       # 空文字列 = 認証無効（開発用）
jwt_secret_file = ""       # ファイルから読む場合はこちら（jwt_secret より優先）

[admin]
auth_token = ""            # 管理 API 認証トークン（空 = 認証無効）
                           # 本番では必ず設定する

[storage]
# data-dir は CLI フラグで指定（config.toml に書かない）
busy_timeout_ms               = 5000     # WAL ロック待機タイムアウト（ミリ秒）
wal_checkpoint_pages          = 1000     # 自動チェックポイントのページ閾値
wal_checkpoint_mode           = "PASSIVE"  # PASSIVE / FULL / RESTART
synchronous                   = "NORMAL"   # OFF は非サポート（I-4 違反）
wal_retention_days            = 0        # PITR 用 WAL アーカイブ保持日数（0 = 無効）
integrity_check_interval_hours = 0       # 定期整合性チェック間隔（0 = 無効）

[replication]
write_mode      = "async"  # async / sync
sync_timeout_ms = 5000     # sync モード時のタイムアウト（ミリ秒）
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

Phase 1〜5 ではトークンのスコープは全体一律。DB 単位の制御は Phase 7 で追加する。

### 5.4 DB スコープ（Phase 7）

Phase 7 から JWT に省略可能な `dbs` クレームを追加する。

**グローバルトークン（Phase 1〜5 互換・Phase 6 以降も有効）：**

```json
{
  "iss": "adlaire-db",
  "sub": "tok_abc123",
  "iat": 1700000000,
  "exp": 1800000000,
  "a":  "rw"
}
```

`dbs` が存在しない場合は全 DB に `a` クレームのアクセスを適用する（Phase 1〜5 挙動と同じ）。

**DB スコープトークン（Phase 7〜）：**

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

**Phase 7 JWT 検証フロー（DB スコープ対応版）：**

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

### 5.5 トークン生成

```bash
# グローバル rw トークン（Phase 1 と同じ）
adlaire-db token create --secret "my-secret" --expiry 30d

# グローバル ro トークン
adlaire-db token create --secret "my-secret" --access ro

# DB スコープトークン（Phase 7〜）
adlaire-db token create --secret "my-secret" \
  --access ro \
  --db analytics:rw \
  --db reports:ro
# → eyJ...（標準出力）
```

### 5.6 トークン失効管理

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

### 6.3 WebSocket API（hrana-ws v3、Phase 8）

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

#### バックアップ・エクスポート（Phase 12〜13）

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

#### ブランチ管理（Phase 14）

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
3. 新 DB を通常の DB として登録し sqld でオープンする

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

#### メトリクス（Phase 9）

```
GET /admin/v1/metrics     全 DB のメトリクス取得
```

**GET /admin/v1/metrics レスポンス（200 OK）：**

```json
{
  "uptime_seconds": 3600,
  "databases": [
    {
      "name": "my-db",
      "queries_total": 1234,
      "rows_read": 5678,
      "rows_written": 91,
      "connections_active": 2,
      "size_bytes": 4096,
      "integrity_errors": 0
    }
  ]
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
| `REPLICATION_TIMEOUT` | 503 | sync モードでレプリカ ACK タイムアウト |
| `PITR_NOT_ENABLED` | 503 | PITR 試行時に `wal_retention_days = 0` |
| `FRAME_NOT_FOUND` | 404 | PITR/ブランチ作成で指定フレームが存在しない |
| `RESTORE_INTEGRITY_FAILED` | 409 | リストア後の `integrity_check` 失敗 |
| `RESTORE_FRAME_CORRUPT` | 409 | WAL フレームの CRC32 検証失敗 |
| `DB_RESERVED_NAME` | 400 | `___` を含む DB 名の直接作成試行 |
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

Step 5: メタデータ読み込み（Phase 1〜5 はシングル DB のためスキップ可）
  5-1. {data-dir}/meta/databases.json が存在すれば読み込みメモリに展開
       なければ空のリスト `{"databases":[]}` として初期化し書き出す
  5-2. {data-dir}/meta/tokens.json が存在すれば読み込みメモリに展開
       なければ空のリスト `{"tokens":[]}` として初期化し書き出す
  5-3. {data-dir}/meta/branches.json が存在すれば読み込みメモリに展開（Phase 14〜）
       なければ空のリスト `{"branches":[]}` として初期化し書き出す

Step 6: DB オープン
  【Phase 1〜5 — シングル DB 固定】
  6-1. {data-dir}/databases/default/ が存在しなければ作成（初回起動時）
  6-2. {data-dir}/databases/default/data.db を sqld::Database::open()
  6-3. WAL モードを設定（PRAGMA journal_mode = WAL）
  6-4. busy timeout を設定（busy_timeout_ms）
  6-5. synchronous を設定（PRAGMA synchronous = NORMAL）
  6-6. skip_integrity_check フラグが false の場合は PRAGMA integrity_check を実行
       → ok 以外の場合は Error: database integrity check failed で終了
  ※ いずれかで失敗した場合は Error: failed to open database 'default': {err} で終了

  【Phase 6 以降 — マルチ DB】
  6-1. {data-dir}/databases/ 以下の各 DB ディレクトリを列挙
  6-2. 各 DB の data.db を sqld::Database::open()（整合性チェック含む）
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
--auth-jwt-secret-file > --auth-jwt-secret > ADLAIRE_JWT_SECRET (env) > config.toml [auth] jwt_secret
--data               > config.toml [storage] data_dir  （config.toml に書かないことを推奨）
--port               > config.toml [server] port        (default: 8080)
--admin-port         > config.toml [server] admin_port  (default: 8081)
--log-level          > ADLAIRE_LOG_LEVEL (env) > config.toml [server] log_level  (default: info)
--busy-timeout       > config.toml [storage] busy_timeout_ms  (default: 5000)
```

---

## 9. 実装フェーズ

フェーズ単位で機能を積み上げる。各フェーズの内製化計画はフェーズ完了後に検討する（§3.5.3）。

| フェーズ | 内容 | テストケース | 実装タスク |
|----------|------|------------|----------|
| **Phase 1** | ビルド基盤・CLI | — | T-1〜T-2 (2件) |
| **Phase 2** | データディレクトリ・sqld 統合 | — | T-3〜T-4 (2件) |
| **Phase 3** | HTTP サーバー・hrana パイプライン | TC-1, TC-2, TC-6 (3件) | T-5〜T-6 (2件) |
| **Phase 4** | JWT 認証・token コマンド | TC-3 (1件) | T-7〜T-9 (3件) |
| **Phase 5** | ログ・統合テスト | TC-4, TC-5 (2件) | T-10〜T-11 (2件) |
| **Phase 6** | マルチ DB ルーター・DB マネージャ | — | T2-1〜T2-2 (2件) |
| **Phase 7** | 管理 API・トークン CRUD・DB スコープ JWT | TC-2-1〜TC-2-6（TC-2-5b 含む）(7件) | T2-3〜T2-6 (4件) |
| **Phase 8** | WebSocket（hrana-ws v3） | TC-3-1〜TC-3-4 (4件) | T3-1〜T3-5 (5件) |
| **Phase 9** | ATTACH DB・メトリクス | TC-3-5, TC-3-6 (2件) | T3-6〜T3-8 (3件) |
| **Phase 10** | レプリケーション基盤（WAL ストリーム・スナップショット） | — | T4-1〜T4-3 (3件) |
| **Phase 11** | レプリカ同期・書き込みリダイレクト | TC-4-1〜TC-4-5 (5件) | T4-4〜T4-7 (4件) |
| **Phase 12** | WAL アーカイブ・manifest 管理 | TC-5-8 (1件) | T5-1〜T5-4 (4件) |
| **Phase 13** | バックアップ・リストア・PITR API | TC-5-1〜TC-5-7 (7件) | T5-5〜T5-9 (5件) |
| **Phase 14** | ブランチ作成・一覧・削除 | TC-6-1〜TC-6-7 (7件) | T6-1〜T6-7 (7件) |
| **Phase 15** | SQLite 拡張・内製化・HA | — | — |


### 9.0 共通実装詳細

全フェーズで共有される型定義・モジュール構成を以下に示す。

#### 14.1 モジュール構成

```
crates/adlaire-server/src/
├── main.rs              ← エントリポイント・tokio ランタイム起動・CLI パース
├── config.rs            ← Config struct・CLI フラグと config.toml のマージ
├── error.rs             ← AppError enum（thiserror）・HTTP レスポンス変換
├── state.rs             ← AppState struct・Arc<> ラッパー定義
├── db/
│   ├── mod.rs           ← DB 名バリデーション・DbInfo 型
│   ├── manager.rs       ← DbManager struct・open/close/create/delete ロジック
│   ├── meta.rs          ← databases.json / tokens.json / branches.json 読み書き
│   └── sqld_adapter.rs  ← SqldAdapter トレイト・RealSqldAdapter 実装（§14.19）
├── auth/
│   ├── mod.rs           ← JWT 検証ロジック・Claims / AuthState struct
│   └── middleware.rs    ← axum extractor: Authenticated
├── http/
│   ├── mod.rs           ← axum Router 組み立て（build_router / build_admin_router）
│   ├── pipeline.rs      ← POST /v2/pipeline ハンドラ
│   ├── health.rs        ← GET /v2/health ハンドラ
│   └── admin/
│       ├── mod.rs       ← 管理 API Router・AdminAuth extractor
│       ├── databases.rs ← DB CRUD ハンドラ（Phase 6）
│       ├── tokens.rs    ← トークン CRUD ハンドラ（Phase 7）
│       ├── metrics.rs   ← GET /admin/v1/metrics（Phase 9）
│       ├── backup.rs    ← バックアップ・リストア・PITR（Phase 12〜13）
│       └── branches.rs  ← ブランチ管理（Phase 14）
├── hrana/
│   ├── mod.rs           ← hrana-http v2 型の re-export
│   ├── types.rs         ← PipelineRequest / PipelineResponse / Value 等
│   └── convert.rs       ← sqld::QueryResult → hrana 型変換
├── ws/
│   ├── mod.rs           ← hrana-ws v3 WebSocket ハンドラ（Phase 8）
│   ├── session.rs       ← WsSession・stream_id ごとの状態管理
│   └── types.rs         ← ClientMsg / ServerMsg 型定義
├── replication/
│   ├── mod.rs           ← WAL レプリケーション共通型（Phase 10）
│   ├── primary.rs       ← SSE /replication/v1/log・snapshot ハンドラ
│   └── replica.rs       ← フレーム受信・CRC32 検証・適用ループ
├── wal/
│   ├── mod.rs           ← WAL アーカイブ公開 API（Phase 12〜13）
│   ├── archive.rs       ← フレーム書き込み・fsync・manifest 更新
│   └── manifest.rs      ← Manifest / FrameMeta struct・アトミック保存
└── metrics.rs           ← AtomicU64 カウンター・DashMap（Phase 9）
```


#### 14.2 主要型定義

#### AppState

```rust
// state.rs
#[derive(Clone)]
pub struct AppState {
    pub config:      Arc<Config>,
    pub db_mgr:      Arc<DbManager>,
    pub auth:        Arc<AuthState>,
    pub metrics:     Arc<Metrics>,                    // Phase 9～
    pub role:        ServerRole,                      // Phase 10～（デフォルト Standalone）
    pub replication: Option<Arc<ReplicationState>>,   // Phase 10～（Replica のみ Some）
}

pub type SharedState = Arc<AppState>;

#[derive(Clone, Debug, PartialEq)]
pub enum ServerRole {
    Standalone,
    Primary { primary_port: u16 },
    Replica { primary_url: url::Url },
}
```

#### Config

```rust
// config.rs
#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir:          PathBuf,
    pub port:              u16,              // デフォルト 8080
    pub admin_port:        u16,              // デフォルト 8081
    pub log_level:         String,           // "trace" | "debug" | "info" | "warn" | "error"
    pub admin_auth_token:  Option<String>,   // None = 認証無効（開発用）
    pub jwt_secret_bytes:  Option<Vec<u8>>, // 32 バイト以上。None = 認証無効
    pub shutdown_timeout:  u64,             // グレースフルシャットダウン最大秒数（デフォルト 30）
    pub skip_integrity_check: bool,         // 起動時整合性チェックをスキップ（デフォルト false）
    pub storage:           StorageConfig,
    pub replication:       ReplicationConfig,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub busy_timeout_ms:              u64,  // デフォルト 5000
    pub wal_checkpoint_pages:         u32,  // デフォルト 1000
    pub wal_checkpoint_mode:          WalCheckpointMode,
    pub wal_retention_days:           u32,  // 0 = PITR 無効
    pub integrity_check_interval_hrs: u64,  // 0 = 無効
}

#[derive(Debug, Clone, Default)]
pub enum WalCheckpointMode { #[default] Passive, Full, Restart }

#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub write_mode:      ReplicationWriteMode,
    pub sync_timeout_ms: u64,  // デフォルト 5000
}

#[derive(Debug, Clone, Default)]
pub enum ReplicationWriteMode { #[default] Async, Sync }
```

#### DbManager

```rust
// db/manager.rs
pub struct DbManager {
    data_dir: PathBuf,
    dbs:      tokio::sync::RwLock<HashMap<String, Arc<sqld::Database>>>,
    meta:     tokio::sync::RwLock<DatabasesMeta>,
    config:   Arc<StorageConfig>,
}

impl DbManager {
    /// 起動時: databases/ 以下を全件オープン
    pub async fn open_all(data_dir: &Path, config: Arc<StorageConfig>) -> anyhow::Result<Self>;

    /// DB 名 → sqld::Database を返す（存在しない場合 None）
    pub async fn get(&self, name: &str) -> Option<Arc<sqld::Database>>;

    /// DB 作成: ディレクトリ作成 → sqld オープン → meta 更新
    pub async fn create(&self, name: &str) -> Result<DbInfo, AppError>;

    /// DB 削除: sqld クローズ → ディレクトリ削除 → meta 更新
    pub async fn delete(&self, name: &str) -> Result<(), AppError>;

    /// DB 一覧（size_bytes は data.db のファイルサイズ）
    pub async fn list(&self) -> Vec<DbInfo>;

    /// シャットダウン時: 全 sqld::Database を drop
    pub async fn close_all(self);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DbInfo {
    pub id:         String,  // UUID v4（作成時に uuid::Uuid::new_v4().to_string() で生成）
    pub name:       String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub size_bytes: u64,
}
```

#### JWT Claims と AuthState

```rust
// auth/mod.rs
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub iss: Option<String>,
    pub sub: String,                             // token_id（tok_xxx）
    pub iat: i64,
    pub exp: Option<i64>,
    pub a:   AccessLevel,
    pub dbs: Option<HashMap<String, AccessLevel>>, // Phase 7～
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessLevel { Rw, Ro }

impl Claims {
    /// DB 名に対するアクセスレベルを解決する
    pub fn resolve_access(&self, db_name: &str) -> AccessLevel {
        match &self.dbs {
            Some(dbs) => dbs.get(db_name).cloned().unwrap_or(self.a.clone()),
            None      => self.a.clone(),
        }
    }

    /// 書き込み権限の有無を返す（AccessLevel::Rw の場合のみ true）
    pub fn can_write(&self) -> bool {
        matches!(self.a, AccessLevel::Rw)
    }

    /// 認証無効モード用（jwt_secret 未設定時のみ使用）
    pub fn unauthenticated() -> Self {
        Self {
            iss: None,
            sub: String::new(),
            iat: 0,
            exp: None,
            a:   AccessLevel::Rw,
            dbs: None,
        }
    }
}

pub struct AuthState {
    secret_bytes: Option<Vec<u8>>,                                    // 発行・テスト用生バイト
    secret:       Option<jsonwebtoken::DecodingKey>,
    revoked:      tokio::sync::RwLock<std::collections::HashSet<String>>,  // token_id
    tokens:       tokio::sync::RwLock<Vec<TokenRecord>>,
}

impl AuthState {
    /// 起動時: tokens.json からメモリへ展開
    pub fn load(config: &Config, tokens: Vec<TokenRecord>) -> Self;

    /// テスト用: バイト列シークレットを直接受け取って生成
    pub fn load_with(secret_bytes: &[u8], tokens: Vec<TokenRecord>) -> Self;

    /// JWT 検証（6 ステップフロー §5.6）
    pub async fn verify(&self, raw_token: &str) -> Result<Claims, AppError>;

    /// トークン発行: JWT 生成 + tokens.json 追記
    /// dbs: None = 全 DB アクセス、Some = DB スコープ付き
    /// 戻り値: (token_id, jwt_string)
    pub async fn issue(
        &self,
        access: AccessLevel,
        exp:    Option<chrono::DateTime<chrono::Utc>>,
        dbs:    Option<HashMap<String, AccessLevel>>,
    ) -> Result<(String, String), AppError>;

    /// トークン失効: revoked フラグ更新 + tokens.json 書き直し
    pub async fn revoke(&self, token_id: &str, meta_path: &Path) -> Result<(), AppError>;

    /// 発行済みトークン一覧（JWT シークレット値は含まない）
    pub async fn list_tokens(&self) -> Vec<TokenRecord>;

    /// 指定 ID のトークンを返す（存在しない場合は TokenNotFound）
    pub async fn get_token(&self, token_id: &str) -> Result<TokenRecord, AppError>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenRecord {
    pub id:         String,
    pub access:     AccessLevel,
    pub dbs:        Option<HashMap<String, AccessLevel>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked:    bool,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}
```

#### AppError（全エラーコード対応）

```rust
// error.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("authentication required")]
    AuthRequired,
    #[error("invalid or revoked token")]
    AuthInvalid,
    #[error("token expired")]
    AuthExpired,
    #[error("permission denied")]
    PermissionDenied,
    #[error("database not found: {0}")]
    DbNotFound(String),
    #[error("token not found: {0}")]
    TokenNotFound(String),
    #[error("database already exists: {0}")]
    DbAlreadyExists(String),
    #[error("invalid database name")]
    InvalidDbName,
    #[error("reserved database name")]
    DbReservedName,
    #[error("invalid request")]
    InvalidRequest,
    #[error("database is busy")]
    StorageBusy,
    #[error("replication timeout")]
    ReplicationTimeout,
    #[error("PITR not enabled")]
    PitrNotEnabled,
    #[error("frame not found")]
    FrameNotFound,
    #[error("restore integrity check failed")]
    RestoreIntegrityFailed,
    #[error("WAL frame corrupt")]
    RestoreFrameCorrupt,
    #[error("authentication is disabled")]
    AuthDisabled,
    #[error("config error: {0}")]
    ConfigError(String),
    #[error("sqld error: {0}")]
    Sqld(sqld::Error),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        let (status, code) = match &self {
            Self::AuthRequired          => (StatusCode::UNAUTHORIZED,            "AUTH_REQUIRED"),
            Self::AuthInvalid           => (StatusCode::UNAUTHORIZED,            "AUTH_INVALID"),
            Self::AuthExpired           => (StatusCode::UNAUTHORIZED,            "AUTH_EXPIRED"),
            Self::PermissionDenied      => (StatusCode::FORBIDDEN,               "PERMISSION_DENIED"),
            Self::DbNotFound(_)         => (StatusCode::NOT_FOUND,               "DB_NOT_FOUND"),
            Self::TokenNotFound(_)      => (StatusCode::NOT_FOUND,               "TOKEN_NOT_FOUND"),
            Self::DbAlreadyExists(_)    => (StatusCode::CONFLICT,                "DB_ALREADY_EXISTS"),
            Self::InvalidDbName         => (StatusCode::BAD_REQUEST,             "INVALID_DB_NAME"),
            Self::DbReservedName        => (StatusCode::BAD_REQUEST,             "DB_RESERVED_NAME"),
            Self::InvalidRequest        => (StatusCode::BAD_REQUEST,             "INVALID_REQUEST"),
            Self::StorageBusy           => (StatusCode::SERVICE_UNAVAILABLE,     "STORAGE_BUSY"),
            Self::ReplicationTimeout    => (StatusCode::SERVICE_UNAVAILABLE,     "REPLICATION_TIMEOUT"),
            Self::PitrNotEnabled        => (StatusCode::SERVICE_UNAVAILABLE,     "PITR_NOT_ENABLED"),
            Self::FrameNotFound         => (StatusCode::NOT_FOUND,               "FRAME_NOT_FOUND"),
            Self::RestoreIntegrityFailed=> (StatusCode::CONFLICT,                "RESTORE_INTEGRITY_FAILED"),
            Self::RestoreFrameCorrupt   => (StatusCode::CONFLICT,                "RESTORE_FRAME_CORRUPT"),
            Self::AuthDisabled          => (StatusCode::UNAUTHORIZED,            "AUTH_DISABLED"),
            Self::ConfigError(_)        => (StatusCode::INTERNAL_SERVER_ERROR,   "CONFIG_ERROR"),
            Self::Sqld(_)              => (StatusCode::INTERNAL_SERVER_ERROR,   "INTERNAL_ERROR"),
            Self::Internal(_)           => (StatusCode::INTERNAL_SERVER_ERROR,   "INTERNAL_ERROR"),
        };
        let body = axum::Json(serde_json::json!({
            "error": self.to_string(),
            "code":  code,
        }));
        (status, body).into_response()
    }
}
```

#### hrana 型（hrana-http v2 完全定義）

```rust
// hrana/types.rs

// ─── リクエスト ───────────────────────────────────────────
#[derive(Debug, serde::Deserialize)]
pub struct PipelineRequest {
    pub baton:    Option<String>,
    pub requests: Vec<StreamRequest>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamRequest {
    Execute  { stmt: Stmt },
    Close,
    Sequence { sql: String },
}

#[derive(Debug, serde::Deserialize)]
pub struct Stmt {
    pub sql:        String,
    pub args:       Vec<Value>,
    #[serde(default)]
    pub named_args: Vec<NamedArg>,
    #[serde(default)]
    pub want_rows:  bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Value {
    Integer { value: String },     // 整数を文字列で表現（i64 の範囲）
    Real    { value: f64 },
    Text    { value: String },
    Blob    { value: String },     // base64 エンコード
    Null,
}

#[derive(Debug, serde::Deserialize)]
pub struct NamedArg {
    pub name:  String,
    pub value: Value,
}

// ─── レスポンス ──────────────────────────────────────────
#[derive(Debug, serde::Serialize)]
pub struct PipelineResponse {
    pub baton:    Option<String>,
    pub base_url: Option<String>,
    pub results:  Vec<StreamResult>,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StreamResult {
    Ok    { response: StreamResponse },
    Error { error: HranaError },
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamResponse {
    Execute { result: StmtResult },
    Close,
}

#[derive(Debug, serde::Serialize)]
pub struct StmtResult {
    pub cols:              Vec<Col>,
    pub rows:              Vec<Vec<Value>>,
    pub rows_affected:     u64,
    pub last_insert_rowid: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct Col {
    pub name:     Option<String>,
    pub decltype: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct HranaError {
    pub message: String,
    pub code:    String,
}
```

#### WAL アーカイブ manifest 型（Phase 12〜13）

```rust
// wal/manifest.rs
#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Manifest {
    pub version:    u32,             // フォーマットバージョン = 1
    pub base_frame: u64,             // 最新スナップショット時点の WAL フレーム番号
    pub snapshot:   Option<String>,  // "snapshot-{frame_no:012}.db"
    pub frames:     Vec<FrameMeta>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrameMeta {
    pub frame_no:   u64,
    pub file:       String,                       // "frame-{frame_no:012}.bin"
    pub size:       u64,
    pub checksum:   u32,                          // CRC32
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Manifest {
    pub async fn load(path: &std::path::Path) -> anyhow::Result<Self>;

    /// 一時ファイルへ書き込み → rename（POSIX アトミック）
    pub async fn save_atomic(&self, path: &std::path::Path) -> anyhow::Result<()>;

    /// timestamp 以前の全フレームを返す
    pub fn frames_before(&self, ts: chrono::DateTime<chrono::Utc>) -> Vec<&FrameMeta>;

    /// frame_no 以下の全フレームを返す
    pub fn frames_at_or_before(&self, frame_no: u64) -> Vec<&FrameMeta>;

    /// 保持期間切れフレームをリストから除去し、除去したメタデータを返す
    pub fn prune_before(&mut self, cutoff: chrono::DateTime<chrono::Utc>) -> Vec<FrameMeta>;
}
```

#### Metrics（Phase 9）

```rust
// metrics.rs
pub struct Metrics {
    pub started_at:      std::time::Instant,
    pub databases:       dashmap::DashMap<String, DbMetrics>,
    pub tokens_total:    std::sync::atomic::AtomicU64,
    pub tokens_revoked:  std::sync::atomic::AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            started_at:     std::time::Instant::now(),
            databases:      dashmap::DashMap::new(),
            tokens_total:   std::sync::atomic::AtomicU64::new(0),
            tokens_revoked: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

#[derive(Default)]
pub struct DbMetrics {
    pub queries_total:      std::sync::atomic::AtomicU64,
    pub rows_read_total:    std::sync::atomic::AtomicU64,
    pub rows_written_total: std::sync::atomic::AtomicU64,
    pub connections_active: std::sync::atomic::AtomicI64,
    pub integrity_errors:   std::sync::atomic::AtomicU64,
    pub wal_size_bytes:     std::sync::atomic::AtomicU64,  // WAL ファイルサイズ（バイト）
}
```


### Phase 1：ビルド基盤・CLI

**目標**：Cargo ワークスペースと libSQL サブモジュールを確立し、CLI の骨格を動かす

**スコープ：**
- Cargo workspace 初期化（adlaire-server crate + libsql submodule）
- clap による `serve` / `token` サブコマンド骨格
- config.toml 3-way マージ（CLI > TOML > デフォルト）
- CI: cargo build / cargo test が通る状態を維持

**実装タスク：**

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
```

---


#### 実装詳細

#### 14.12 CLI 構造体

```rust
// main.rs
#[derive(Parser)]
#[command(name = "adlaire-db", version, about = "Self-hosted libSQL-compatible DB server")]
pub struct Cli { #[command(subcommand)] pub command: CliCommand }

#[derive(Subcommand)]
pub enum CliCommand {
    Serve(ServeArgs),
    Token { #[command(subcommand)] cmd: TokenSubcommand },
}

#[derive(Subcommand)]
pub enum TokenSubcommand { Create(TokenCreateArgs) }

#[derive(Parser)]
pub struct ServeArgs {
    #[arg(long, required = true)] pub data:                    PathBuf,
    #[arg(long)] pub port:                                     Option<u16>,
    #[arg(long)] pub admin_port:                               Option<u16>,
    #[arg(long)] pub config:                                   Option<PathBuf>,
    #[arg(long)] pub auth_jwt_secret:                          Option<String>,
    #[arg(long)] pub auth_jwt_secret_file:                     Option<PathBuf>,
    #[arg(long)] pub admin_auth_token:                         Option<String>,
    #[arg(long)] pub log_level:                                Option<String>,
    #[arg(long, default_value_t = false)] pub skip_integrity_check: bool,
    #[arg(long)] pub replication_write_mode:                   Option<String>,
    #[arg(long)] pub busy_timeout:                             Option<u64>,
    #[arg(long)] pub shutdown_timeout:                         Option<u64>,
}

#[derive(Parser)]
pub struct TokenCreateArgs {
    #[arg(long, required = true)] pub secret: String,
    #[arg(long, default_value = "rw")] pub access: String,
    #[arg(long)] pub expiry: Option<String>,
    #[arg(long, value_name = "DB:ACCESS")] pub db: Vec<String>,
}
```

`--data` のみ required。その他はすべて `Option<T>` にして 3-way マージで解決する。

---


#### 14.5 エントリポイント（main.rs）

CLI 構造体（`Cli`, `ServeArgs`, `TokenCreateArgs` 等）の定義は §14.12 を参照。

```rust
// main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CliCommand::Serve(args) => run_serve(args).await,
        CliCommand::Token { cmd: TokenSubcommand::Create(args) } => run_token_create(args),
    }
}

async fn run_serve(args: ServeArgs) -> anyhow::Result<()> {
    // Step 1: 設定マージ（CLI > config.toml > デフォルト）
    let config = Arc::new(Config::resolve(&args)?);

    // Step 2: ログ初期化（tracing + tracing-subscriber JSON）
    init_tracing(&config.log_level);

    // Step 3: データディレクトリ初期化
    DataDir::init(&config.data_dir)?;

    // Step 4: プロセス排他ロック（flock LOCK_EX | LOCK_NB）
    let _lock = ProcessLock::acquire(&config.data_dir)?;

    // Step 5: メタデータ読み込み + AuthState 初期化
    let tokens_meta = meta::load_tokens(&config.data_dir)?;
    let auth = Arc::new(AuthState::load(&config, tokens_meta.tokens));

    // Step 6: DB 全件オープン（起動時整合性チェック込み）
    let db_mgr = Arc::new(
        DbManager::open_all(&config.data_dir, Arc::new(config.storage.clone())).await?
    );

    // Step 7: AppState 構築
    let state: SharedState = Arc::new(AppState {
        config:      Arc::clone(&config),
        db_mgr,
        auth,
        metrics:     Arc::new(Metrics::new()),
        role:        ServerRole::Standalone,
        replication: None,  // Phase 10 で Some(Arc::new(ReplicationState::new())) に更新
    });

    // Step 8: TCP ソケット bind
    let api_listener   = tokio::net::TcpListener::bind(("0.0.0.0",       config.port)).await?;
    let admin_listener = tokio::net::TcpListener::bind(("127.0.0.1", config.admin_port)).await?;

    tracing::info!(port = config.port, admin_port = config.admin_port, "Adlaire DB listening");

    // Step 9: サーバー起動 + グレースフルシャットダウン
    let shutdown = shutdown_signal();
    tokio::select! {
        r = axum::serve(api_listener,   build_router(Arc::clone(&state)))       => r?,
        r = axum::serve(admin_listener, build_admin_router(Arc::clone(&state))) => r?,
        _ = shutdown => { tracing::info!("shutdown signal received"); }
    }

    // Step 10: DB クローズ（WAL flush + checkpoint）
    Arc::try_unwrap(state).ok()
        .map(|s| Arc::try_unwrap(s.db_mgr).ok())
        .flatten()
        .expect("db_mgr still referenced")
        .close_all()
        .await;

    tracing::info!("Adlaire DB stopped");
    Ok(())
}

fn run_token_create(args: TokenCreateArgs) -> anyhow::Result<()> {
    let secret = args.secret.as_bytes().to_vec();
    anyhow::ensure!(secret.len() >= 32, "--secret は 32 バイト以上の文字列を指定してください");
    let access: AccessLevel = match args.access.as_str() {
        "rw" => AccessLevel::Rw,
        "ro" => AccessLevel::Ro,
        other => anyhow::bail!("unknown access level: {other}. Use 'rw' or 'ro'"),
    };
    let exp = args.expiry.as_deref()
        .map(parse_expiry)
        .transpose()?
        .map(|d| chrono::Utc::now() + d);
    let auth = AuthState::load_with(&secret, vec![]);
    // dbs は token create サブコマンドでは None（全 DB アクセス）
    let (_token_id, jwt) = tokio::runtime::Handle::current()
        .block_on(auth.issue(access, exp, None))?;
    println!("{jwt}");
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint  = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv()  => {},
        _ = sigterm.recv() => {},
    }
}
```


#### 14.13 Config 解決ロジック

```rust
// config.rs

// TOML 構造体はすべてのフィールドを Option<T> にする
#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlConfig {
    pub server:      Option<TomlServer>,
    pub auth:        Option<TomlAuth>,
    pub admin:       Option<TomlAdmin>,
    pub storage:     Option<TomlStorage>,
    pub replication: Option<TomlReplication>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlServer {
    pub port:             Option<u16>,
    pub admin_port:       Option<u16>,
    pub log_level:        Option<String>,
    pub busy_timeout_ms:  Option<u64>,
    pub shutdown_timeout: Option<u64>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlAuth {
    pub jwt_secret:      Option<String>,
    pub jwt_secret_file: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlAdmin {
    pub auth_token: Option<String>,  // None = 認証無効（開発用）
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlStorage {
    pub wal_mode:              Option<String>,
    pub skip_integrity_check:  Option<bool>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlReplication {
    pub write_mode: Option<String>,
}

impl Config {
    /// tokens.json のパスを返す
    pub fn tokens_path(&self) -> std::path::PathBuf {
        self.data_dir.join("meta").join("tokens.json")
    }

    pub fn resolve(args: &ServeArgs) -> anyhow::Result<Arc<Config>> {
        // 1. config.toml 読み込み（存在しなければ Default）
        let toml_path = args.config.clone()
            .unwrap_or_else(|| args.data.join("config.toml"));
        let toml: TomlConfig = if toml_path.exists() {
            let s = std::fs::read_to_string(&toml_path)?;
            toml::from_str(&s)?
        } else {
            TomlConfig::default()
        };
        let srv  = toml.server.unwrap_or_default();
        let auth = toml.auth.unwrap_or_default();
        let adm  = toml.admin.unwrap_or_default();   // auth_token のみ
        let sto  = toml.storage.unwrap_or_default();
        let rep  = toml.replication.unwrap_or_default();

        // 2. JWT シークレット解決（優先度順）
        //    --auth-jwt-secret-file > --auth-jwt-secret
        //    > ADLAIRE_JWT_SECRET 環境変数
        //    > toml jwt_secret > toml jwt_secret_file
        let raw_secret: Option<Vec<u8>> = if let Some(p) = &args.auth_jwt_secret_file {
            Some(std::fs::read(p)?)
        } else if let Some(s) = &args.auth_jwt_secret {
            Some(s.as_bytes().to_vec())
        } else if let Ok(s) = std::env::var("ADLAIRE_JWT_SECRET") {
            Some(s.into_bytes())
        } else if let Some(s) = &auth.jwt_secret {
            Some(s.as_bytes().to_vec())
        } else if let Some(p) = &auth.jwt_secret_file {
            Some(std::fs::read(p)?)
        } else {
            None
        };

        // 3. シークレット長チェック（32 バイト未満は拒否）
        if let Some(ref b) = raw_secret {
            anyhow::ensure!(b.len() >= 32, "JWT secret must be at least 32 bytes");
        }

        // 4. 3-way マージ（CLI > TOML > デフォルト）
        let port       = args.port.or(srv.port).unwrap_or(8080);
        let admin_port = args.admin_port.or(srv.admin_port).unwrap_or(8081);
        let log_level  = args.log_level.as_deref()
            .or(srv.log_level.as_deref())
            .unwrap_or("info")
            .to_string();
        let busy_timeout_ms: u64  = args.busy_timeout.or(srv.busy_timeout_ms).unwrap_or(5000);
        let shutdown_timeout: u64 = args.shutdown_timeout.or(srv.shutdown_timeout).unwrap_or(30);
        let skip_integrity_check = args.skip_integrity_check
            || sto.skip_integrity_check.unwrap_or(false);
        let wal_mode  = parse_wal_mode(sto.wal_mode.as_deref())?;
        let write_mode = match &args.replication_write_mode {
            Some(s) => parse_write_mode(s)?,
            None    => rep.write_mode.as_deref()
                           .map(parse_write_mode)
                           .transpose()?
                           .unwrap_or(ReplicationWriteMode::Async),
        };

        let admin_auth_token = args.admin_auth_token.clone()
            .or(adm.auth_token)
            .or_else(|| std::env::var("ADLAIRE_ADMIN_TOKEN").ok());

        Ok(Arc::new(Config {
            data_dir:             args.data.clone(),
            port,
            admin_port,
            log_level,
            admin_auth_token,
            jwt_secret_bytes:     raw_secret,
            shutdown_timeout,
            skip_integrity_check,
            storage: StorageConfig {
                busy_timeout_ms:              busy_timeout_ms,
                wal_checkpoint_pages:         1000,
                wal_checkpoint_mode:          wal_mode,
                wal_retention_days:           0,
                integrity_check_interval_hrs: 0,
            },
            replication: ReplicationConfig {
                write_mode,
                sync_timeout_ms: 5000,
            },
        }))
    }
}

fn parse_wal_mode(s: Option<&str>) -> anyhow::Result<WalCheckpointMode> {
    match s.unwrap_or("passive") {
        "passive" => Ok(WalCheckpointMode::Passive),
        "full"    => Ok(WalCheckpointMode::Full),
        "restart" => Ok(WalCheckpointMode::Restart),
        other     => anyhow::bail!("unknown wal_mode: {other}. Use 'passive', 'full', or 'restart'"),
    }
}

fn parse_write_mode(s: &str) -> anyhow::Result<ReplicationWriteMode> {
    match s {
        "async" => Ok(ReplicationWriteMode::Async),
        "sync"  => Ok(ReplicationWriteMode::Sync),
        other   => anyhow::bail!("unknown replication write_mode: {other}. Use 'async' or 'sync'"),
    }
}
```

---


### Phase 2：データディレクトリ・sqld 統合

**目標**：データディレクトリを初期化し、sqld でシングル DB を開ける状態にする

**スコープ：**
- `--data` パスのディレクトリ作成・パーミッション設定
- flock による排他プロセスロック
- sqld::Database::open()・WAL モード設定

**実装タスク：**

```
T-3: データディレクトリ初期化
  [ ] --data パスの作成（mkdir -p）
  [ ] .lock ファイルによる排他ロック（flock）
  [ ] databases/ meta/ サブディレクトリ作成
  [ ] ディレクトリパーミッション警告（700 未満で WARN）
  参照: §3.2, §8.1 Step 3〜4, §10.3

T-4: sqld 統合・DB オープン
  [ ] sqld::Database::open() でシングル DB を開く
  [ ] PRAGMA journal_mode = WAL を起動時に適用
  [ ] busy_timeout を設定
  [ ] PRAGMA synchronous = NORMAL を設定
  [ ] サーバーシャットダウン時に drop（WAL flush + close）
  参照: §3.3.2, §13, §8.1 Step 6, §8.2 Step 3
  検証: TC-5（データ永続性）
```

---


#### 実装詳細

#### 14.14 DataDir・ProcessLock 実装

```rust
// data_dir.rs

impl DataDir {
    pub fn init(data_dir: &Path) -> anyhow::Result<()> {
        for sub in &["", "databases", "meta"] {
            let p = data_dir.join(sub);
            std::fs::create_dir_all(&p)?;
            std::fs::set_permissions(&p, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;

            // 実際のモードを確認し、0o700 より広ければ警告
            let mode = std::fs::metadata(&p)?.permissions().mode() & 0o777;
            if mode > 0o700 {
                tracing::warn!(path = %p.display(), mode = format!("{:04o}", mode),
                    "data directory permissions are broader than 0700");
            }
        }
        Ok(())
    }
}

// プロセス多重起動防止
pub struct ProcessLock {
    _file: std::fs::File,  // Drop 時に flock が自動解放される
}

impl ProcessLock {
    pub fn acquire(data_dir: &Path) -> anyhow::Result<Self> {
        use std::os::unix::io::AsRawFd;
        let file = std::fs::OpenOptions::new()
            .create(true).write(true)
            .open(data_dir.join(".lock"))?;
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            anyhow::bail!("another adlaire-db process is already running in {:?}", data_dir);
        }
        Ok(Self { _file: file })
    }
}
```

---


#### 14.19 SqldAdapter トレイト（db/sqld_adapter.rs）

sqld の内部型への依存を 1 ファイルに集約し、アップストリーム変更の影響範囲を限定する。

```rust
// db/sqld_adapter.rs

/// sqld::Database を薄くラップして Adlaire 内部で使用する抽象トレイト。
/// sqld の型変更が生じた場合はこのファイルのみを修正すれば済む。
pub trait SqldAdapter: Send + Sync {
    /// SQL ステートメントを実行し、行列を返す
    fn execute(
        &self,
        stmt: &sqld::hrana::proto::Stmt,
    ) -> impl std::future::Future<Output = Result<sqld::hrana::proto::StmtResult, crate::error::AppError>> + Send;

    /// パイプラインリクエストを実行する
    fn execute_pipeline(
        &self,
        pipeline: &sqld::hrana::proto::PipelineReqBody,
    ) -> impl std::future::Future<Output = Result<sqld::hrana::proto::PipelineRespBody, crate::error::AppError>> + Send;
}

/// 本番実装: sqld::Database を保持する newtype
pub struct RealSqldAdapter(pub Arc<sqld::Database>);

impl SqldAdapter for RealSqldAdapter {
    async fn execute(
        &self,
        stmt: &sqld::hrana::proto::Stmt,
    ) -> Result<sqld::hrana::proto::StmtResult, crate::error::AppError> {
        self.0.execute(stmt).await.map_err(crate::error::AppError::Sqld)
    }

    async fn execute_pipeline(
        &self,
        pipeline: &sqld::hrana::proto::PipelineReqBody,
    ) -> Result<sqld::hrana::proto::PipelineRespBody, crate::error::AppError> {
        self.0.execute_pipeline(pipeline).await.map_err(crate::error::AppError::Sqld)
    }
}
```

> **設計メモ**: `db/manager.rs` の `DbManager` は `Arc<dyn SqldAdapter>` を保持する。
> テストでは `MockSqldAdapter` を差し込んで sqld バイナリなしで単体テストが可能になる。

### Phase 3：HTTP サーバー・hrana パイプライン

**目標**：libSQL クライアント SDK が Adlaire DB に接続して SQL を実行できる最小構成

**スコープ：**
- axum HTTP サーバー起動・SIGINT/SIGTERM ハンドラ
- GET `/v2/health`
- POST `/v2/pipeline`（hrana-http v2 完全実装）

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
  検証: TC-1（SQL 実行）
```

---


#### 実装詳細

#### 14.4 axum Router 設計

```rust
// http/mod.rs
pub fn build_router(state: SharedState) -> axum::Router {
    axum::Router::new()
        // Phase 1: シングル DB
        .route("/v2/pipeline",           axum::routing::post(pipeline::handle))
        .route("/v2/health",             axum::routing::get(health::handle))
        // Phase 6: パスベース DB ルーティング
        .route("/:db_name/v2/pipeline",  axum::routing::post(pipeline::handle_db))
        // Phase 8: WebSocket
        .route("/v3/baton",              axum::routing::get(ws::handle))
        .route("/:db_name/v3/baton",     axum::routing::get(ws::handle_db))
        .with_state(state)
}

pub fn build_admin_router(state: SharedState) -> axum::Router {
    use axum::routing::{delete, get, post};
    // AdminAuth は Layer ではなく各ハンドラの引数 Extractor として使用する
    // （axum 0.7 では from_extractor_with_state が削除されたため）
    axum::Router::new()
        .nest("/admin/v1", axum::Router::new()
            // Phase 6: DB CRUD
            .route("/databases",
                get(admin::databases::list).post(admin::databases::create))
            .route("/databases/:name",
                get(admin::databases::get).delete(admin::databases::delete))
            // Phase 7: トークン CRUD
            .route("/tokens",
                get(admin::tokens::list).post(admin::tokens::create))
            .route("/tokens/:id",
                get(admin::tokens::get).delete(admin::tokens::revoke))
            // Phase 9: メトリクス
            .route("/metrics",           get(admin::metrics::get))
            // Phase 12〜13: バックアップ・PITR
            .route("/databases/:name/backup",                    get(admin::backup::backup))
            .route("/databases/:name/restore",                   post(admin::backup::restore))
            .route("/databases/:name/restore/point-in-time",     post(admin::backup::pitr))
            // Phase 14: ブランチ
            .route("/databases/:name/branches",
                get(admin::branches::list).post(admin::branches::create))
            .route("/databases/:name/branches/:branch",          delete(admin::branches::delete))
        )
        .with_state(state)
}

// 各管理ハンドラは先頭引数に _auth: AdminAuth を必須とする。例：
// pub async fn list(
//     _auth: AdminAuth,
//     State(state): State<SharedState>,
// ) -> Result<Json<...>, AppError> { ... }
```


#### 14.6 hrana 変換ロジック

```rust
// hrana/convert.rs

pub fn to_pipeline_response(
    results: Vec<Result<sqld::QueryResult, sqld::Error>>,
) -> PipelineResponse {
    PipelineResponse {
        baton:    None,
        base_url: None,
        results:  results.into_iter().map(to_stream_result).collect(),
    }
}

fn to_stream_result(r: Result<sqld::QueryResult, sqld::Error>) -> StreamResult {
    match r {
        Ok(qr)  => StreamResult::Ok {
            response: StreamResponse::Execute { result: to_stmt_result(qr) },
        },
        Err(e) => StreamResult::Error {
            error: HranaError {
                message: e.to_string(),
                code:    sqld_error_code(&e),
            },
        },
    }
}

fn to_stmt_result(qr: sqld::QueryResult) -> StmtResult {
    StmtResult {
        cols: qr.columns.into_iter().map(|c| Col {
            name:     c.name,
            decltype: c.decl_type,
        }).collect(),
        rows: qr.rows.into_iter().map(|row| {
            row.values.into_iter().map(sqld_val_to_hrana).collect()
        }).collect(),
        rows_affected:     qr.rows_affected as u64,
        last_insert_rowid: qr.last_insert_rowid.map(|n| n.to_string()),
    }
}

fn sqld_error_code(e: &sqld::Error) -> String {
    // sqld のエラー型は実装時に確定する。典型的なマッピングを示す
    if e.to_string().contains("UNIQUE constraint") {
        "SQLITE_CONSTRAINT".into()
    } else {
        "SQLITE_ERROR".into()
    }
}

fn sqld_val_to_hrana(v: sqld::Value) -> Value {
    match v {
        sqld::Value::Null       => Value::Null,
        sqld::Value::Integer(n) => Value::Integer { value: n.to_string() },
        sqld::Value::Real(f)    => Value::Real    { value: f },
        sqld::Value::Text(s)    => Value::Text    { value: s },
        sqld::Value::Blob(b)    => Value::Blob    {
            value: base64::engine::general_purpose::STANDARD.encode(&b),
        },
    }
}
```


#### 14.15 HTTP ハンドラ実装

```rust
// handlers/pipeline.rs

// 単一 DB ハンドラ（Phase 1）
pub async fn handle(
    State(state): State<Arc<AppState>>,
    Authenticated(claims): Authenticated,
    Json(req): Json<PipelineRequest>,
) -> Result<Json<PipelineResponse>, AppError> {
    let db = state.db_mgr.get("default").await
        .ok_or_else(|| AppError::DbNotFound("default".to_string()))?;
    let results = execute_pipeline(&db, &claims, &req.requests).await?;
    Ok(Json(PipelineResponse { baton: None, base_url: None, results }))
}

// マルチ DB ハンドラ（Phase 6）
pub async fn handle_db(
    State(state): State<Arc<AppState>>,
    Authenticated(claims): Authenticated,
    Path(db_name): Path<String>,
    Json(req): Json<PipelineRequest>,
) -> Result<Json<PipelineResponse>, AppError> {
    let db = state.db_mgr.get(&db_name).await
        .ok_or_else(|| AppError::DbNotFound(db_name.clone()))?;
    let results = execute_pipeline(&db, &claims, &req.requests).await?;
    Ok(Json(PipelineResponse { baton: None, base_url: None, results }))
}

async fn execute_pipeline(
    db: &sqld::Database,
    claims: &Claims,
    requests: &[StreamRequest],
) -> Result<Vec<StreamResult>, AppError> {
    let conn = db.connect().map_err(|e| AppError::Sqld(e))?;
    let mut results = Vec::with_capacity(requests.len());
    for req in requests {
        match req {
            StreamRequest::Execute { stmt } => {
                if is_write_stmt(&stmt.sql) && !claims.can_write() {
                    results.push(StreamResult::Error {
                        error: HranaError { message: "write not permitted".into(), code: "PERMISSION_DENIED".into() },
                    });
                } else {
                    let params = hrana_values_to_params(&stmt.args);
                    let result = conn.execute(&stmt.sql, params).await
                        .map_err(|e| AppError::Sqld(e))?;
                    results.push(StreamResult::Ok {
                        response: StreamResponse::Execute { result: to_stmt_result(result) },
                    });
                }
            }
            StreamRequest::Sequence { sql } => {
                // セミコロン分割して逐次実行（いずれかが失敗したら即座に返す）
                for stmt_sql in split_sql_statements(sql) {
                    let result = conn.execute(&stmt_sql, libsql::params![]).await
                        .map_err(|e| AppError::Sqld(e))?;
                    results.push(StreamResult::Ok {
                        response: StreamResponse::Execute { result: to_stmt_result(result) },
                    });
                }
            }
            StreamRequest::Close => break,
        }
    }
    Ok(results)
}

/// hrana Value 列を libsql Params へ変換する（hrana/convert.rs に定義し ws/session.rs からも use する）
pub fn hrana_values_to_params(args: &[Value]) -> libsql::Params {
    let values: Vec<libsql::Value> = args.iter().map(|v| match v {
        Value::Null                => libsql::Value::Null,
        Value::Integer { value }   => libsql::Value::Integer(value.parse().unwrap_or(0)),
        Value::Real    { value }   => libsql::Value::Real(*value),
        Value::Text    { value }   => libsql::Value::Text(value.clone()),
        Value::Blob    { value }   => {
            use base64::Engine as _;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(value).unwrap_or_default();
            libsql::Value::Blob(bytes)
        }
    }).collect();
    libsql::Params::Positional(values)
}

/// SQL 文字列をセミコロンで分割し、空文字列を除去する
fn split_sql_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
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
// handlers/health.rs
pub async fn handle() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
```

---


### Phase 4：JWT 認証・token コマンド

**目標**：JWT HS256 認証と `adlaire-db token create` が動作する

**スコープ：**
- Authorization: Bearer ヘッダ抽出・6 ステップ検証フロー
- tokens.json 読み込み・revoke リスト照合
- `token create` サブコマンド（JWT 生成・stdout 出力）

**完了条件（テストケース）：**

```
TC-3: JWT 認証ありモード
  $ SECRET="test-secret"
  $ TOKEN=$(./adlaire-db token create --secret "$SECRET")
  $ ./adlaire-db serve --data ./testdb --port 8080 --auth-jwt-secret "$SECRET"
  （a）有効なトークンで SQL 実行 → 200 OK
  （b）Authorization ヘッダなし → 401 AUTH_REQUIRED
  （c）不正なトークン → 401 AUTH_INVALID
```

**実装タスク：**

```
T-7: JWT 認証ミドルウェア
  [ ] jsonwebtoken crate で HS256 検証
  [ ] Authorization: Bearer <JWT> ヘッダ抽出
  [ ] 6 ステップ検証フロー実装（§5.6）
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
  [ ] Phase 1〜5 では tokens.json の更新は CLI のみ（管理 API は Phase 7）
  参照: §5.6

T-9: `token create` サブコマンド
  [ ] --secret <VALUE>, --expiry <DURATION>, --access ro|rw フラグ
  [ ] JWT を HS256 で署名して stdout に出力
  [ ] tok_<random> 形式の token_id を生成（sub クレーム）
  [ ] tokens.json に新規トークンを追記
  参照: §4.1, §5.2, §5.5
  検証: TC-3（TOKEN=$(./adlaire-db token create ...)）
```

---


#### 実装詳細

#### 14.3 axum 認証 Extractor

```rust
// auth/middleware.rs

/// リクエストごとに JWT を検証し、Claims を抽出する axum Extractor
pub struct Authenticated(pub Claims);
// 利用側: Authenticated(claims): Authenticated

impl<S> axum::extract::FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
    SharedState: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let state = SharedState::from_ref(state);

        // 認証無効モード（jwt_secret 未設定）はスキップ
        if state.auth.is_disabled() {
            return Ok(Authenticated(Claims::unauthenticated()));
        }

        let header = parts.headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::AuthRequired)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::AuthInvalid)?;

        let claims = state.auth.verify(token).await?;
        Ok(Authenticated(claims))
    }
}

/// 管理 API 専用 Extractor（Bearer 文字列完全一致）
pub struct AdminAuth;

impl<S> axum::extract::FromRequestParts<S> for AdminAuth
where
    S: Send + Sync,
    SharedState: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let state = SharedState::from_ref(state);
        let expected = match &state.config.admin_auth_token {
            None    => return Ok(AdminAuth),  // 認証無効
            Some(t) => t,
        };
        let header = parts.headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::AuthRequired)?;
        let token = header.strip_prefix("Bearer ").ok_or(AppError::AuthInvalid)?;
        if token != expected { return Err(AppError::AuthInvalid); }
        Ok(AdminAuth)
    }
}
```


#### 14.16 AuthState 実装

```rust
// auth/state.rs

impl AuthState {
    pub fn is_disabled(&self) -> bool { self.secret.is_none() }

    pub fn load(config: &Config, tokens: Vec<TokenRecord>) -> Self {
        let (secret_bytes, secret) = match &config.jwt_secret_bytes {
            Some(b) => {
                let key = jsonwebtoken::DecodingKey::from_secret(b);
                (Some(b.clone()), Some(key))
            }
            None => (None, None),
        };
        Self {
            secret_bytes,
            secret,
            revoked: tokio::sync::RwLock::new(std::collections::HashSet::new()),
            tokens:  tokio::sync::RwLock::new(tokens),
        }
    }

    pub async fn verify(&self, raw_token: &str) -> Result<Claims, AppError> {
        let key = self.secret.as_ref().ok_or(AppError::AuthDisabled)?;
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = false;  // 手動で exp を検証する

        let data = jsonwebtoken::decode::<Claims>(raw_token, key, &validation)
            .map_err(|_| AppError::AuthInvalid)?;
        let claims = data.claims;

        // exp チェック
        if let Some(exp) = claims.exp {
            if exp < chrono::Utc::now().timestamp() {
                return Err(AppError::AuthExpired);
            }
        }

        // 失効チェック（tokio RwLock: read().await）
        let revoked = self.revoked.read().await;
        if revoked.contains(&claims.sub) {
            return Err(AppError::AuthInvalid);
        }

        Ok(claims)
    }

    pub async fn revoke(&self, token_id: &str, meta_path: &Path) -> Result<(), AppError> {
        {
            let mut revoked = self.revoked.write().await;
            revoked.insert(token_id.to_string());
        }
        // tokens リストの該当エントリを論理削除（revoked=true）して永続化
        {
            let mut tokens = self.tokens.write().await;
            let record = tokens.iter_mut()
                .find(|t| t.id == token_id)
                .ok_or_else(|| AppError::TokenNotFound(token_id.to_string()))?;
            record.revoked    = true;
            record.revoked_at = Some(chrono::Utc::now());
            let meta      = TokensMeta { tokens: tokens.clone() };
            let path_copy = meta_path.to_path_buf();
            tokio::task::spawn_blocking(move || save_atomic(&path_copy, &meta))
                .await
                .map_err(|e| AppError::Internal(e.into()))??;
        }
        Ok(())
    }

    // 戻り値: (token_id, jwt_string)
    pub async fn issue(
        &self,
        access: AccessLevel,
        exp:    Option<chrono::DateTime<chrono::Utc>>,
        dbs:    Option<HashMap<String, AccessLevel>>,
    ) -> Result<(String, String), AppError> {
        let secret = self.secret_bytes.as_ref().ok_or(AppError::AuthDisabled)?;
        let token_id = generate_token_id();
        let claims = Claims {
            iss: Some("adlaire-db".into()),
            sub: token_id.clone(),
            iat: chrono::Utc::now().timestamp(),
            exp: exp.map(|e| e.timestamp()),
            a:   access,
            dbs,
        };
        let jwt = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(secret),
        ).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok((token_id, jwt))
    }

    // テスト用ヘルパー（#[cfg(test)]）
    pub fn load_with(secret_bytes: &[u8], tokens: Vec<TokenRecord>) -> Self {
        let b = secret_bytes.to_vec();
        let key = jsonwebtoken::DecodingKey::from_secret(&b);
        Self {
            secret_bytes: Some(b),
            secret: Some(key),
            revoked: tokio::sync::RwLock::new(std::collections::HashSet::new()),
            tokens:  tokio::sync::RwLock::new(tokens),
        }
    }

    #[cfg(test)]
    pub fn issue_test_token(&self, access: AccessLevel) -> String {
        self.issue_test_token_exp(access, None)
    }

    #[cfg(test)]
    pub fn issue_test_token_exp(&self, access: AccessLevel, exp: Option<chrono::DateTime<Utc>>) -> String {
        let bytes = self.secret_bytes.as_ref().expect("secret not set");
        let key = jsonwebtoken::EncodingKey::from_secret(bytes);
        let claims = Claims {
            iss: Some("adlaire-db".into()),
            sub: generate_token_id(),
            iat: Utc::now().timestamp(),
            exp: exp.map(|t| t.timestamp()),
            a:   access,
            dbs: None,
        };
        jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &key).unwrap()
    }

    /// 発行済みトークン一覧を返す（JWT 値は含まない）
    pub async fn list_tokens(&self) -> Vec<TokenRecord> {
        self.tokens.read().await.clone()
    }

    /// 指定 ID のトークンを返す
    pub async fn get_token(&self, token_id: &str) -> Result<TokenRecord, AppError> {
        self.tokens.read().await
            .iter()
            .find(|t| t.id == token_id)
            .cloned()
            .ok_or_else(|| AppError::TokenNotFound(token_id.to_string()))
    }

    #[cfg(test)]
    pub fn revoke_sync(&self, token_id: &str) {
        self.revoked.blocking_write().insert(token_id.to_string());
    }
}
```

---


#### 14.18 Expiry パース・トークン ID 生成

```rust
// token/util.rs

/// "30d" / "24h" / "3600s" / "90m" 形式の期限文字列を chrono::Duration に変換する
pub fn parse_expiry(s: &str) -> anyhow::Result<chrono::Duration> {
    // 最初のアルファベット文字の位置で数値部分とサフィックスを分割
    let split_pos = s.find(|c: char| c.is_alphabetic())
        .ok_or_else(|| anyhow::anyhow!("expiry must end with a unit (s/m/h/d): {s}"))?;
    let (num_str, unit) = s.split_at(split_pos);
    let n: i64 = num_str.parse()
        .map_err(|_| anyhow::anyhow!("invalid expiry number: {num_str}"))?;
    let dur = match unit {
        "s" => chrono::Duration::seconds(n),
        "m" => chrono::Duration::minutes(n),
        "h" => chrono::Duration::hours(n),
        "d" => chrono::Duration::days(n),
        _   => anyhow::bail!("unknown expiry unit '{}' (use s/m/h/d)", unit),
    };
    Ok(dur)
}

/// /dev/urandom から 8 バイト読み取り "tok_{hex16}" 形式の ID を生成する
pub fn generate_token_id() -> String {
    let mut buf = [0u8; 8];
    let mut f = std::fs::File::open("/dev/urandom").expect("cannot open /dev/urandom");
    use std::io::Read;
    f.read_exact(&mut buf).expect("cannot read /dev/urandom");
    format!("tok_{}", hex::encode(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_expiry_days() {
        assert_eq!(parse_expiry("30d").unwrap(), chrono::Duration::days(30));
    }

    #[test]
    fn parse_expiry_hours() {
        assert_eq!(parse_expiry("24h").unwrap(), chrono::Duration::hours(24));
    }

    #[test]
    fn parse_expiry_seconds() {
        assert_eq!(parse_expiry("3600s").unwrap(), chrono::Duration::seconds(3600));
    }

    #[test]
    fn parse_expiry_invalid_unit() {
        assert!(parse_expiry("10y").is_err());
    }

    #[test]
    fn parse_expiry_no_unit() {
        assert!(parse_expiry("3600").is_err());
    }
}
```

---


### Phase 5：ログ・統合テスト

**目標**：構造化ログを整備し Phase 1〜5 全テストを完走させる

**スコープ：**
- tracing + JSON Lines 出力
- HTTP リクエストログ
- TC-4（TypeScript SDK）・TC-5（データ永続性）を含む全件完走

**完了条件（テストケース）：**

```
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
```

**実装タスク：**

```
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

**タスク依存グラフ（Phase 1〜5）：**

```
T-1 → T-2 → T-3 → T-4 ─┐
                          ├→ T-5 → T-6 → T-7 → T-8 → T-9 → T-10 → T-11
                          └→ T-5（並行可）
```

T-4〜T-5 は並行実装可。T-6（hrana 変換）は T-4・T-5 の両方が揃った後。

**対象外（Phase 6 以降）：**
- マルチ DB・管理 API・WebSocket・レプリケーション

---


#### 実装詳細

#### 14.11 テスト実装パターン

**ユニットテスト例（JWT 検証・全ケース）：**

```rust
// auth/mod.rs — #[cfg(test)] mod tests
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn secret() -> Vec<u8> { "a".repeat(32).into_bytes() }

    fn make_auth() -> AuthState {
        AuthState::load_with(&secret(), vec![])
    }

    #[tokio::test]
    async fn valid_rw_token_passes() {
        let auth  = make_auth();
        let token = auth.issue_test_token(AccessLevel::Rw);
        let c = auth.verify(&token).await.unwrap();
        assert_eq!(c.a, AccessLevel::Rw);
    }

    #[tokio::test]
    async fn expired_token_is_rejected() {
        let auth  = make_auth();
        let token = auth.issue_test_token_exp(AccessLevel::Rw, Some(Utc::now() - Duration::seconds(1)));
        assert!(matches!(auth.verify(&token).await, Err(AppError::AuthExpired)));
    }

    #[tokio::test]
    async fn wrong_signature_is_rejected() {
        let auth = make_auth();
        assert!(matches!(auth.verify("eyJ.eyJ.badsig").await, Err(AppError::AuthInvalid)));
    }

    #[tokio::test]
    async fn revoked_token_is_rejected() {
        let auth  = make_auth();
        let token = auth.issue_test_token(AccessLevel::Rw);
        let sub   = auth.verify(&token).await.unwrap().sub;
        auth.revoke_sync(&sub);
        assert!(matches!(auth.verify(&token).await, Err(AppError::AuthInvalid)));
    }

    #[test]
    fn db_scope_rw_overrides_global_ro() {
        let claims = Claims {
            sub: "tok_x".into(),
            a:   AccessLevel::Ro,
            dbs: Some([("db_a".into(), AccessLevel::Rw)].into()),
            iss: None, iat: 0, exp: None,
        };
        assert_eq!(claims.resolve_access("db_a"), AccessLevel::Rw);
        assert_eq!(claims.resolve_access("db_b"), AccessLevel::Ro);
    }
}
```

**テストヘルパー（`tests/helpers.rs`）：**

```rust
// tests/helpers.rs

pub struct TestConfig {
    pub jwt_secret:  Option<String>,
    pub admin_token: Option<String>,
}

impl Default for TestConfig {
    fn default() -> Self { Self { jwt_secret: None, admin_token: None } }
}

pub struct TestServer {
    pub base_url:  String,
    pub admin_url: String,
    client:        reqwest::Client,
    _dir:          tempfile::TempDir,
    _handle:       tokio::task::AbortHandle,
}

impl Drop for TestServer {
    fn drop(&mut self) { self._handle.abort(); }
}

impl TestServer {
    pub async fn spawn(cfg: TestConfig) -> Self {
        let dir   = tempfile::TempDir::new().unwrap();
        let port  = pick_unused_port();
        let aport = pick_unused_port();
        let args = ServeArgs {
            data:              dir.path().to_path_buf(),
            port:              Some(port),
            admin_port:        Some(aport),
            auth_jwt_secret:   cfg.jwt_secret,
            admin_auth_token:  cfg.admin_token,
            ..Default::default()
        };
        let handle = tokio::spawn(async move { run_serve(args).await.unwrap() });
        wait_for_ready(&format!("http://127.0.0.1:{port}/v2/health")).await;
        Self {
            base_url:  format!("http://127.0.0.1:{port}"),
            admin_url: format!("http://127.0.0.1:{aport}"),
            client:    reqwest::Client::new(),
            _dir:      dir,
            _handle:   handle.abort_handle(),
        }
    }

    pub async fn pipeline(&self, body: serde_json::Value) -> reqwest::Response {
        self.client.post(format!("{}/v2/pipeline", self.base_url))
            .json(&body).send().await.unwrap()
    }

    pub async fn get(&self, path: &str) -> reqwest::Response {
        self.client.get(format!("{}{path}", self.base_url)).send().await.unwrap()
    }

    pub async fn admin_post(&self, path: &str, body: serde_json::Value) -> reqwest::Response {
        self.client.post(format!("{}{path}", self.admin_url))
            .json(&body).send().await.unwrap()
    }

    pub async fn admin_delete(&self, path: &str) -> reqwest::Response {
        self.client.delete(format!("{}{path}", self.admin_url)).send().await.unwrap()
    }

    pub async fn admin_get(&self, path: &str) -> reqwest::Response {
        self.client.get(format!("{}{path}", self.admin_url)).send().await.unwrap()
    }

    /// 管理トークンを発行して JWT 文字列を返す
    pub async fn create_token(&self, access: &str) -> String {
        let resp = self.admin_post("/admin/v1/tokens",
            serde_json::json!({"access": access})).await;
        resp.json::<serde_json::Value>().await.unwrap()["token"]
            .as_str().unwrap().to_string()
    }
}

fn pick_unused_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap()
        .local_addr().unwrap().port()
}

async fn wait_for_ready(url: &str) {
    let client = reqwest::Client::new();
    for _ in 0..100 {
        if client.get(url).send().await.map(|r| r.status().is_success()).unwrap_or(false) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("TestServer did not become ready at {url}");
}
```

**統合テスト例（TC-1）：**

```rust
// tests/integration/phase1.rs
//! cargo test --test integration

mod helpers;
use helpers::TestServer;

#[tokio::test]
async fn tc1_sql_execution() {
    let srv = TestServer::spawn(Default::default()).await;

    let resp = srv.pipeline(serde_json::json!({
        "baton": null,
        "requests": [
            {"type":"execute","stmt":{"sql":"CREATE TABLE t(id INT, v TEXT)","args":[],"want_rows":false}},
            {"type":"execute","stmt":{"sql":"INSERT INTO t VALUES(1,'hello')","args":[],"want_rows":false}},
            {"type":"execute","stmt":{"sql":"SELECT * FROM t","args":[],"want_rows":true}},
            {"type":"close"}
        ]
    })).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let row = &body["results"][2]["response"]["result"]["rows"][0];
    assert_eq!(row[0], serde_json::json!({"type":"integer","value":"1"}));
    assert_eq!(row[1], serde_json::json!({"type":"text","value":"hello"}));
}

#[tokio::test]
async fn tc2_health_check() {
    let srv = TestServer::spawn(Default::default()).await;
    let resp = srv.get("/v2/health").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<serde_json::Value>().await.unwrap(), serde_json::json!({"status":"ok"}));
}

#[tokio::test]
async fn tc3_jwt_auth() {
    let secret = "test-secret-32bytes-minimum-len!";
    let srv = TestServer::spawn(TestConfig {
        jwt_secret: Some(secret.to_string()),
        ..Default::default()
    }).await;

    let valid_token = srv.create_token(secret, "rw", None);

    // (a) 有効トークン → 200
    assert_eq!(srv.pipeline_with_token(&valid_token, minimal_select()).await.status(), 200);

    // (b) Authorization ヘッダなし → 401 AUTH_REQUIRED
    let no_auth = srv.pipeline_no_auth(minimal_select()).await;
    assert_eq!(no_auth.status(), 401);
    assert_eq!(no_auth.json::<serde_json::Value>().await.unwrap()["code"], "AUTH_REQUIRED");

    // (c) 不正トークン → 401 AUTH_INVALID
    let bad = srv.pipeline_with_token("eyJ.eyJ.badsig", minimal_select()).await;
    assert_eq!(bad.status(), 401);
    assert_eq!(bad.json::<serde_json::Value>().await.unwrap()["code"], "AUTH_INVALID");
}
```

---


### Phase 6：マルチ DB ルーター・DB マネージャ

**目標**：1インスタンスで複数 DB をルーティングできる

- パスベース DB ルーティング（`/{db-name}/v2/pipeline`）
- DB ごとのデータ分離

**実装タスク：**

```
T2-1: パスベース DB ルーター
  [ ] axum Router を /{db-name}/v2/pipeline にマッチするように拡張
  [ ] パスセグメントから db-name を抽出し、DB 名バリデーションを適用
  [ ] 存在しない db-name → 404 DB_NOT_FOUND
  [ ] Phase 1〜5 の単一 DB ルート（/v2/pipeline）との共存（後方互換）
  参照: §6.1, §3.4

T2-2: マルチ DB マネージャ
  [ ] 起動時に databases.json を読み込み、全 DB を sqld でオープン
  [ ] DB 名 → sqld::Database のマップをメモリ上で管理（RwLock<HashMap>）
  [ ] 新規 DB 作成時にマップへ追加・databases.json を更新
  [ ] DB 削除時にマップから除去・ファイル削除・databases.json を更新
  参照: §3.4, §8.1 Step 5〜6
```

---


#### 実装詳細

#### 14.7 DB 名バリデーション

```rust
// db/mod.rs
use std::sync::LazyLock;
use regex::Regex;

static DB_NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9_-]{1,127}$").unwrap()
});

const RESERVED_NAMES: &[&str]  = &["meta", "admin"];
const BRANCH_SEP:     &str     = "___";

pub fn validate_db_name(name: &str) -> Result<(), AppError> {
    if !DB_NAME_RE.is_match(name) {
        return Err(AppError::InvalidDbName);
    }
    if RESERVED_NAMES.contains(&name) {
        return Err(AppError::InvalidDbName);
    }
    if name.contains(BRANCH_SEP) {
        return Err(AppError::DbReservedName);
    }
    Ok(())
}

/// ブランチ DB の内部名を生成する
pub fn branch_db_name(source: &str, branch: &str) -> String {
    format!("{}{}{}", source, BRANCH_SEP, branch)
}
```


#### 14.17 db/meta.rs 実装

```rust
// db/meta.rs

pub struct DatabasesMeta { pub databases: Vec<DbInfo> }
pub struct TokensMeta    { pub tokens: Vec<TokenRecord> }

pub fn load_databases(data_dir: &Path) -> anyhow::Result<DatabasesMeta> {
    load_or_init(data_dir.join("meta").join("databases.json"))
}

pub fn load_tokens(data_dir: &Path) -> anyhow::Result<TokensMeta> {
    load_or_init(data_dir.join("meta").join("tokens.json"))
}

fn load_or_init<T>(path: PathBuf) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize + Default,
{
    if path.exists() {
        let s = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&s)?)
    } else {
        let val = T::default();
        save_atomic(&path, &val)?;
        Ok(val)
    }
}

// tmp → fsync → rename によるアトミック保存（WAL マニフェストと同一パターン）
pub fn save_atomic<T: serde::Serialize>(path: &Path, val: &T) -> anyhow::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut f = std::fs::File::create(&tmp)?;
    let json = serde_json::to_vec_pretty(val)?;
    use std::io::Write;
    f.write_all(&json)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)?;
    Ok(())
}
```

---


### Phase 7：管理 API・トークン CRUD・DB スコープ JWT

**目標**：管理 API と DB スコープアクセス制御を実装する

- 管理 API（DB CRUD・トークン CRUD）
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

**実装タスク：**

```
T2-3: 管理 API — DB CRUD
  [ ] GET /admin/v1/databases → databases.json の一覧を返す
  [ ] POST /admin/v1/databases — DB 名バリデーション・ディレクトリ作成・sqld オープン
  [ ] GET /admin/v1/databases/{name} → 個別情報（name・created_at・size_bytes）
  [ ] DELETE /admin/v1/databases/{name} — sqld クローズ・ディレクトリ削除
  [ ] size_bytes は data.db のファイルサイズを返す
  参照: §6.4（DB 管理）

T2-4: 管理 API — トークン CRUD
  [ ] POST /admin/v1/tokens — JWT 生成・tokens.json への追記・201 返却
  [ ] GET /admin/v1/tokens / GET /admin/v1/tokens/{id} — tokens.json から読み込み
  [ ] DELETE /admin/v1/tokens/{id} — tokens.json の revoked を true に更新（冪等）
  [ ] expiry パース（30d / 24h / 3600s 等）→ JWT exp クレームへの変換
  参照: §6.4（トークン管理）, §5.5, §5.6

T2-5: DB スコープ JWT（dbs クレーム）
  [ ] Phase 4 の JWT 検証を拡張（7 ステップフロー §5.4）
  [ ] dbs クレームが存在する場合、対象 DB 名でアクセス権を解決
  [ ] POST /admin/v1/tokens に dbs フィールドを追加
  [ ] tokens.json の dbs フィールドを保存
  参照: §5.4

T2-6: 統合テスト TC-2-1〜TC-2-6（TC-2-5b 含む）
  [ ] 各テストケースを実行し全て PASS することを確認
  [ ] Phase 1〜6 の TC がリグレッションしないことを確認
```

#### 実装詳細

```rust
// handlers/admin/databases.rs

/// POST /admin/v1/databases  → DB 作成
pub async fn create_db(
    State(state): State<Arc<AppState>>,
    _auth: AdminAuth,
    Json(req): Json<CreateDbRequest>,
) -> Result<(StatusCode, Json<DbInfo>), AppError> {
    validate_db_name(&req.name)?;
    let info = state.db_mgr.create(&req.name).await?;
    Ok((StatusCode::CREATED, Json(info)))
}

/// GET /admin/v1/databases  → DB 一覧
pub async fn list_dbs(
    State(state): State<Arc<AppState>>,
    _auth: AdminAuth,
) -> Result<Json<Vec<DbInfo>>, AppError> {
    Ok(Json(state.db_mgr.list().await?))
}

/// GET /admin/v1/databases/:name  → DB 詳細
pub async fn get_db(
    State(state): State<Arc<AppState>>,
    _auth: AdminAuth,
    Path(name): Path<String>,
) -> Result<Json<DbInfo>, AppError> {
    state.db_mgr.get_info(&name).await.map(Json)
}

/// DELETE /admin/v1/databases/:name  → DB 削除
pub async fn delete_db(
    State(state): State<Arc<AppState>>,
    _auth: AdminAuth,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    state.db_mgr.delete(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

// handlers/admin/tokens.rs

#[derive(serde::Deserialize)]
pub struct IssueTokenRequest {
    pub access: AccessLevel,
    pub expiry: Option<String>,          // "30d" / "24h" / "3600s" 形式
    pub dbs:    Option<HashMap<String, AccessLevel>>,
}

#[derive(serde::Serialize)]
pub struct TokenResponse {
    pub id:         String,
    pub token:      String,
    pub access:     AccessLevel,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// POST /admin/v1/tokens  → トークン発行
pub async fn issue_token(
    State(state): State<Arc<AppState>>,
    _auth: AdminAuth,
    Json(req): Json<IssueTokenRequest>,
) -> Result<(StatusCode, Json<TokenResponse>), AppError> {
    // "30d" / "24h" / "3600s" → chrono::DateTime<Utc>
    let exp: Option<chrono::DateTime<chrono::Utc>> = req.expiry.as_deref()
        .map(|s| parse_expiry(s).map(|d| chrono::Utc::now() + d))
        .transpose()
        .map_err(|e| AppError::Internal(e))?;
    let (token_id, jwt) = state.auth.issue(req.access.clone(), exp, req.dbs).await?;
    Ok((StatusCode::CREATED, Json(TokenResponse {
        id:         token_id,
        token:      jwt,
        access:     req.access,
        expires_at: exp,
    })))
}

/// GET /admin/v1/tokens  → トークン一覧（JWT シークレット非公開）
pub async fn list_tokens(
    State(state): State<Arc<AppState>>,
    _auth: AdminAuth,
) -> Result<Json<Vec<TokenRecord>>, AppError> {
    Ok(Json(state.auth.list_tokens().await))
}

/// GET /admin/v1/tokens/:id  → トークン詳細
pub async fn get_token(
    State(state): State<Arc<AppState>>,
    _auth: AdminAuth,
    Path(id): Path<String>,
) -> Result<Json<TokenRecord>, AppError> {
    state.auth.get_token(&id).await.map(Json)
}

/// DELETE /admin/v1/tokens/:id  → トークン失効（冪等）
pub async fn revoke_token(
    State(state): State<Arc<AppState>>,
    _auth: AdminAuth,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.auth.revoke(&id, &state.config.tokens_path()).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

---


### Phase 8：WebSocket（hrana-ws v3）

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

#### sqld との統合（Phase 8）

Phase 1〜7 と同様、sqld の WebSocket サーバーループは起動しない。**Adlaire 独自の hrana-ws プロトコル変換レイヤーを実装する**（§3.3.3 の hrana-http 変換層と同じ設計方針）。

採用理由：

- sqld の WebSocket ハンドラはセッション管理・認証と密結合しており、ライブラリとして分離が困難
- Phase 1〜7 で構築した hrana-http 変換レイヤー（§3.3.3）の延長として実装でき、アーキテクチャの一貫性を保てる
- WebSocket コネクションのライフサイクル（hello / stream_id / baton 管理）を Adlaire が完全制御できる

WebSocket フレームの受受信・送信には `tokio-tungstenite` クレートを使用する。クエリ実行は Phase 1〜7 と同じ `sqld::Connection::execute_batch()` を経由する（§3.3.2）。

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
```

---


#### 実装詳細

#### 14.9 WebSocket セッション管理（Phase 8）

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
use crate::hrana::convert::hrana_values_to_params;

pub struct WsSession {
    db:      std::sync::Arc<sqld::Database>,
    streams: HashMap<u32, WsStream>,  // stream_id → WsStream
    auth:    Claims,
}

pub struct WsStream {
    conn:    sqld::Connection,
    tx_mode: TransactionMode,
}

#[derive(PartialEq)]
pub enum TransactionMode { None, ReadOnly, ReadWrite }

impl WsSession {
    pub fn new(db: std::sync::Arc<sqld::Database>, auth: Claims) -> Self {
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
                let params = hrana_values_to_params(&stmt.args);
                let result = stream.conn.execute(&stmt.sql, params).await
                    .map_err(|e| AppError::Sqld(e))?;
                Ok(ResponseBody::Execute { result: to_stmt_result(result) })
            }
            RequestBody::CloseStream => {
                self.streams.remove(&stream_id);
                Ok(ResponseBody::CloseStream)
            }
            RequestBody::Sequence { sql } => {
                let stream = self.streams.get_mut(&stream_id).ok_or(AppError::InvalidRequest)?;
                for stmt_sql in split_sql_statements(&sql) {
                    stream.conn.execute(&stmt_sql, libsql::params![]).await
                        .map_err(|e| AppError::Sqld(e))?;
                }
                Ok(ResponseBody::Sequence)
            }
            RequestBody::Describe { stmt } => {
                let stream = self.streams.get_mut(&stream_id).ok_or(AppError::InvalidRequest)?;
                let desc = stream.conn.prepare(&stmt.sql).await.map_err(|e| AppError::Sqld(e))?;
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
                    let params = hrana_values_to_params(&stmt.args);
                    match stream.conn.execute(&stmt.sql, params).await {
                        Ok(r)  => { step_results.push(Some(to_stmt_result(r))); step_errors.push(None); }
                        Err(e) => { step_results.push(None); step_errors.push(Some(HranaError { message: e.to_string(), code: "SQLITE_ERROR".into() })); }
                    }
                }
                Ok(ResponseBody::Batch { step_results, step_errors })
            }
            // store_sql / close_sql（SQL テキストキャッシュ）は Phase 8 未実装
            RequestBody::StoreSql { .. } | RequestBody::CloseSql { .. } => {
                Err(AppError::InvalidRequest)
            }
        }
    }
}
```


### Phase 9：ATTACH DB・メトリクス

**目標**：クロス DB クエリとインメモリメトリクス API を実装する

#### ATTACH DATABASE（クロス DB クエリ）

Turso Cloud と同様に、Adlaire が管理する DB 間に限り `ATTACH DATABASE` を許可する。

**セキュリティモデル：**
- クライアントが `ATTACH DATABASE 'other-db' AS alias` を送信した場合、Adlaire は `'other-db'` を DB 名として解釈し、`databases/other-db/data.db` のパスを解決する
- 任意のファイルパス（`/etc/passwd` 等）は DB 名バリデーション（`^[a-zA-Z0-9_-]{1,127}$`）で事前に拒否する
- 存在しない DB 名の場合は `404 DB_NOT_FOUND` を返す

**実装方針：**
- hrana-http v2 の `execute` リクエストで ATTACH SQL を受け取った際、Adlaire 側でインターセプトして DB 名を解決する
- sqld の Connection に対してパス解決済みの ATTACH を発行する
- 対象 DB の接続が未オープンの場合はその場でオープンする

**追加テストケース：**

```
TC-3-5: ATTACH DATABASE（クロス DB クエリ）
  （a）db_a・db_b を作成し、それぞれにテーブルとデータを投入
  （b）db_a への pipeline で:
      ATTACH DATABASE 'db_b' AS b;
      SELECT * FROM b.t;
      期待: db_b のデータが返る
  （c）ATTACH DATABASE '/etc/passwd' AS evil
      → 400 INVALID_DB_NAME（バリデーション拒否）
  （d）ATTACH DATABASE 'nonexistent' AS x
      → 404 DB_NOT_FOUND
```

#### メトリクス API

```
GET /admin/v1/metrics
```

認証: 管理トークン必須（§6.4 管理 API 認証と同じ）

**レスポンス（200 OK）：**

```json
{
  "uptime_seconds": 3600,
  "databases": [
    {
      "name": "mydb",
      "size_bytes": 4096,
      "wal_size_bytes": 1024,
      "connections_active": 2,
      "queries_total": 1500,
      "rows_read_total": 8000,
      "rows_written_total": 200
    }
  ],
  "tokens_total": 5,
  "tokens_revoked": 1
}
```

カウンター（`queries_total` 等）はプロセス起動からの累積値。再起動でリセットされる（Phase 9 時点では永続化しない）。

**追加テストケース：**

```
TC-3-6: メトリクス API
  （a）GET /admin/v1/metrics（管理トークンあり）→ 200、databases 配列に管理下 DB が含まれる
  （b）クエリ実行後に queries_total が増加していること
  （c）GET /admin/v1/metrics（管理トークンなし）→ 401
```

**Phase 8 実装タスク：**

```
T3-1: WebSocket サーバー追加（axum の WebSocket upgrade）
T3-2: hrana-ws v3 hello ハンドシェイク + JWT 認証
T3-3: ストリーム多重化レイヤー実装（stream_id ごとの接続状態管理）
T3-4: execute / batch / sequence / describe リクエスト処理（sqld 境界再利用）
T3-5: インタラクティブトランザクション状態管理（BEGIN/COMMIT/ROLLBACK）
```

**Phase 9 実装タスク：**

```
T3-6: ATTACH DATABASE インターセプト・DB 名バリデーション・パス解決
T3-7: メトリクス収集（インメモリカウンター）+ GET /admin/v1/metrics
T3-8: 統合テスト TC-3-1〜TC-3-6
```

#### 実装詳細

```rust
// ATTACH DATABASE インターセプト
// execute_pipeline で呼び出す前に SQL を検査し、ATTACH 文なら DB 名を解決して書き換える

/// "ATTACH DATABASE 'foo' AS alias" を検出して解決済みパスに書き換える。
/// foo が Adlaire 管理外（バリデーション失敗 or 未登録）なら Err を返す。
pub async fn resolve_attach(
    sql: &str,
    db_mgr: &dyn DbManager,
    data_dir: &std::path::Path,
) -> Result<String, AppError> {
    let re = regex::Regex::new(
        r#"(?i)ATTACH\s+(?:DATABASE\s+)?'([^']+)'\s+AS\s+(\w+)"#
    ).unwrap();
    if let Some(caps) = re.captures(sql) {
        let db_name = &caps[1];
        let alias   = &caps[2];
        validate_db_name(db_name)?;
        // DB が存在するか確認（存在しない場合は DbNotFound）
        db_mgr.get_info(db_name).await?;
        let db_path = data_dir
            .join("databases")
            .join(db_name)
            .join("data.db");
        Ok(format!(
            "ATTACH DATABASE '{}' AS {}",
            db_path.display(), alias
        ))
    } else {
        Ok(sql.to_string())
    }
}

// メトリクス収集
use std::sync::atomic::Ordering;

/// execute_pipeline の実行後に呼び出してカウンターを更新する
pub fn record_metrics(metrics: &DbMetrics, results: &[StreamResult]) {
    metrics.queries_total.fetch_add(results.len() as u64, Ordering::Relaxed);
    for r in results {
        if let StreamResult::Ok { response: StreamResponse::Execute { result: ref row } } = r {
            metrics.rows_read_total.fetch_add(row.rows_read, Ordering::Relaxed);
            metrics.rows_written_total.fetch_add(row.rows_affected, Ordering::Relaxed);
        }
    }
}

// handlers/admin/metrics.rs

/// GET /admin/v1/metrics  → プロセス起動からの累積カウンター
pub async fn get_metrics(
    State(state): State<Arc<AppState>>,
    _auth: AdminAuth,
) -> Json<serde_json::Value> {
    use std::sync::atomic::Ordering::Relaxed;
    let uptime = state.metrics.started_at.elapsed().as_secs();
    let databases: Vec<serde_json::Value> = state.metrics.databases.iter().map(|e| {
        let (name, m) = (e.key(), e.value());
        // WAL サイズはファイルシステムから取得
        let wal_path = state.config.data_dir
            .join("databases").join(name).join("data.db-wal");
        let wal_size = std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
        // 現在値をカウンターに反映
        m.wal_size_bytes.store(wal_size, Relaxed);
        serde_json::json!({
            "name":              name,
            "size_bytes":        std::fs::metadata(
                                     state.config.data_dir.join("databases").join(name).join("data.db")
                                 ).map(|m| m.len()).unwrap_or(0),
            "wal_size_bytes":    wal_size,
            "connections_active": m.connections_active.load(Relaxed),
            "queries_total":     m.queries_total.load(Relaxed),
            "rows_read_total":   m.rows_read_total.load(Relaxed),
            "rows_written_total":m.rows_written_total.load(Relaxed),
        })
    }).collect();
    Json(serde_json::json!({
        "uptime_seconds":  uptime,
        "databases":       databases,
        "tokens_total":    state.metrics.tokens_total.load(Relaxed),
        "tokens_revoked":  state.metrics.tokens_revoked.load(Relaxed),
    }))
}
```

---


### Phase 10：レプリケーション基盤（WAL ストリーム・スナップショット）

**目標**：プライマリ・レプリカ構成での運用

#### アーキテクチャ

```
クライアント
  │
  ├─ 書き込み → プライマリ（:8080）─ WAL 同期 ─→ レプリカ 1（:8080）
  │                                             └→ レプリカ 2（:8080）
  └─ 読み取り → レプリカ（ロードバランサー経由）
```

- プライマリとレプリカは同じバイナリ。起動フラグでロールを決定する
- レプリカはプライマリの WAL フレームを HTTP ストリームで受信して自身の DB に適用する
- レプリカへの書き込みは `307 Temporary Redirect` でプライマリへ転送する

#### 起動フラグ（Phase 10 追加）

```
# プライマリとして起動
adlaire-db serve --data ./data --role primary --primary-port 8082

# レプリカとして起動
adlaire-db serve --data ./data --role replica --primary-url http://primary:8082
```

| フラグ | 説明 |
|--------|------|
| `--role` | `standalone`（デフォルト）/ `primary` / `replica` |
| `--primary-port` | プライマリが WAL ストリームを公開するポート（デフォルト: 8082）|
| `--primary-url` | レプリカが接続するプライマリの URL |
| `--replication-auth-token` | プライマリ・レプリカ間の認証トークン |

#### レプリケーション API

プライマリが `--primary-port`（デフォルト 8082）で公開する内部エンドポイント。クライアント SDK は直接使わない。すべてのリクエストに `Authorization: Bearer <replication-auth-token>` が必要。

**GET /replication/v1/log?from_frame=\<N\>**

WAL フレームを Server-Sent Events でストリーム配信する。

```
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache

data: {"frame_no":0,"db":"mydb","data":"<base64 WAL frame>","checksum":3294921183}

data: {"frame_no":1,"db":"mydb","data":"<base64 WAL frame>","checksum":1928374652}
```

- `frame_no`: WAL フレームの通し番号（0 始まり）
- `db`: 対象 DB 名（Phase 10 はマルチ DB 対応）
- `data`: WAL フレームのバイナリを Base64 エンコードしたもの
- `checksum`: フレームの CRC32 チェックサム

フレームが追いついた場合は接続を保持し、新しいフレームが来次第送信する（long-poll SSE）。

**GET /replication/v1/snapshot**

レプリカの初回参加時に全スナップショットを取得する。

```
HTTP/1.1 200 OK
Content-Type: application/octet-stream
X-Replication-Frame-No: 42
X-Replication-Db: mydb

<SQLite ページダンプ バイナリ>
```

`X-Replication-Frame-No` が示す frame_no 以降の差分を `/log?from_frame=43` で取得することで同期を完成させる。

**POST /replication/v1/heartbeat**

レプリカの生存確認と進捗報告。

```json
// リクエスト
{"replica_id": "replica-1", "synced_frame": 42}

// レスポンス 200 OK
{"primary_frame": 42, "lag_frames": 0}
```

プライマリは `synced_frame` 以前の WAL フレームを将来的に GC できる（Phase 10 では GC は未実装・受付のみ）。

**GET /replication/v1/status**

プライマリの現在状態。

```json
// 200 OK
{
  "role": "primary",
  "current_frame": 42,
  "replicas": [
    {"id": "replica-1", "synced_frame": 42, "lag_frames": 0, "last_seen": "2026-09-10T12:00:00Z"},
    {"id": "replica-2", "synced_frame": 39, "lag_frames": 3, "last_seen": "2026-09-10T11:59:55Z"}
  ]
}
```

#### ヘルスチェック拡張

Phase 11 から `GET /v2/health` のレスポンスにロール情報を追加する：

```json
{
  "status": "ok",
  "role": "primary",
  "replication_lag_frames": 0
}
```

レプリカの場合：

```json
{
  "status": "ok",
  "role": "replica",
  "primary_url": "http://primary:8080",
  "replication_lag_frames": 3
}
```

#### 書き込みリダイレクト

レプリカが書き込みリクエストを受信した場合：

```
HTTP/1.1 307 Temporary Redirect
Location: http://primary:8080/{db-name}/v2/pipeline
```

クライアント（libSQL SDK）は自動的にプライマリへ再送する。

---


#### 実装詳細

#### 14.10 WAL レプリケーション（Phase 10）

**プライマリ側 WAL フレーム管理：**

```rust
// replication/primary.rs
use tokio::sync::broadcast;

pub struct ReplicationState {
    /// 書き込みコミット時にフレームを broadcast する
    pub frame_tx:      broadcast::Sender<WalFrame>,
    pub current_frame: std::sync::atomic::AtomicU64,
    pub replicas:      dashmap::DashMap<String, ReplicaStatus>,
    /// レプリカのみ: primary との差分フレーム数（primary_frame - synced_frame）
    pub lag_frames:    std::sync::atomic::AtomicU64,
}

#[derive(Clone, Debug)]
pub struct WalFrame {
    pub frame_no: u64,
    pub db_name:  String,
    pub data:     bytes::Bytes,
    pub checksum: u32,  // CRC32
}

#[derive(Debug)]
pub struct ReplicaStatus {
    pub synced_frame: u64,
    pub last_seen:    chrono::DateTime<chrono::Utc>,
}
```

**レプリカ側フレーム受信・適用ループ：**

```rust
// replication/replica.rs
pub async fn run_replica_loop(
    primary_url:  url::Url,
    db_mgr:       std::sync::Arc<DbManager>,
    auth_token:   String,
) {
    let mut from_frame = db_mgr.current_max_frame().await;

    loop {
        match fetch_frames(&primary_url, from_frame, &auth_token).await {
            Ok(frames) if frames.is_empty() => {
                // フレームなし: バックオフして再試行
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            Ok(frames) => {
                for frame in frames {
                    // CRC32 検証
                    let actual = crc32fast::hash(&frame.data);
                    if actual != frame.checksum {
                        tracing::error!(frame_no = frame.frame_no, "checksum mismatch, skipping");
                        continue;
                    }
                    if let Err(e) = db_mgr.apply_wal_frame(&frame.db_name, &frame.data).await {
                        tracing::error!(error = %e, "failed to apply WAL frame");
                    }
                    from_frame = frame.frame_no + 1;
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "replica fetch error, retrying in 1s");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}
```


### Phase 11：レプリカ同期・書き込みリダイレクト

**目標**：レプリカが WAL フレームを受信・適用し書き込みをプライマリへ転送する

#### 完了条件（テストケース）

```
TC-4-1: WAL 同期（基本）
  （a）プライマリで INSERT 実行
  （b）レプリカで SELECT → プライマリのデータが反映されている
  （c）GET /v2/health（レプリカ）→ replication_lag_frames = 0 または小さい値

TC-4-2: 書き込みリダイレクト
  （a）レプリカのエンドポイントに直接 POST /v2/pipeline（INSERT）を送信
  （b）307 Redirect でプライマリへ転送される
  （c）プライマリで SELECT → データが存在する

TC-4-3: レプリカ障害・復帰
  （a）レプリカを停止
  （b）プライマリで INSERT を複数回実行
  （c）レプリカを再起動（同じ --primary-url で）
  （d）レプリカが差分 WAL フレームを取得して追いつく
  （e）SELECT → 最新データが返る

TC-4-4: プライマリ停止時のレプリカ挙動
  （a）プライマリを停止
  （b）レプリカへの SELECT → 200（既存データは返せる）
  （c）レプリカへの INSERT → 503 または 307（プライマリ到達不能）
  （d）GET /v2/health（レプリカ）→ status:"degraded" 等の警告

TC-4-5: マルチレプリカ同期
  プライマリ 1 台 + レプリカ 2 台の構成で TC-4-1 を実施
  両レプリカで同じデータが返ること
```

**Phase 10 実装タスク：**

```
T4-1: --role フラグ対応（standalone / primary / replica の起動分岐）
T4-2: WAL フレームストリーム API（GET /replication/v1/log SSE）
T4-3: スナップショット API（GET /replication/v1/snapshot）
```

**Phase 11 実装タスク：**

```
T4-4: レプリカ側 WAL フレーム受信・適用ループ
T4-5: 書き込みリダイレクト（307 → primary-url）
T4-6: GET /v2/health にロール・ lag 情報を追加
T4-7: 統合テスト TC-4-1〜TC-4-5
```

#### 実装詳細

```rust
// middleware/replica_redirect.rs

/// レプリカモードで書き込みリクエストを受けた際に primary-url へ 307 リダイレクト
pub async fn maybe_redirect_write(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if let ServerRole::Replica { primary_url } = &state.role {
        if is_mutating_request(&req) {
            let target = format!(
                "{}{}",
                primary_url.as_str().trim_end_matches('/'),
                req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("")
            );
            return axum::response::Response::builder()
                .status(axum::http::StatusCode::TEMPORARY_REDIRECT)
                .header(axum::http::header::LOCATION, target)
                .body(axum::body::Body::empty())
                .unwrap();
        }
    }
    next.run(req).await
}

/// POST / PUT / DELETE は書き込みリクエストとみなす
fn is_mutating_request(req: &axum::extract::Request) -> bool {
    matches!(
        req.method(),
        &axum::http::Method::POST | &axum::http::Method::PUT | &axum::http::Method::DELETE
    )
}

// handlers/health.rs（Phase 11 拡張）

#[derive(serde::Serialize)]
pub struct HealthResponse {
    pub status:               &'static str,
    pub role:                 ServerRole,
    pub replication_lag_frames: Option<u64>,  // replica のみ
}

pub async fn handle(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let lag = state.replication.as_ref()
        .map(|r| r.lag_frames.load(std::sync::atomic::Ordering::Relaxed));
    let status = match lag {
        Some(lag) if lag > 1000 => "degraded",
        _ => "ok",
    };
    Json(HealthResponse {
        status,
        role: state.role.clone(),
        replication_lag_frames: lag,
    })
}
```

---


### Phase 12：WAL アーカイブ・manifest 管理

**目標**：WAL フレームのアーカイブと manifest.json による管理を実装する

**スコープ：**
- WAL アーカイブ書き込み（チェックポイント前フック）
- manifest.json による WAL フレーム管理
- `wal_retention_days` 設定によるアーカイブ保持期間の管理

**完了条件（テストケース）：**

```
TC-5-8: 保持期間超過フレームのクリーンアップ
  設定: wal_retention_days = 1
  （a）2 日前のタイムスタンプを持つフレームを作成
  （b）クリーンアップ実行（または 24h 経過後）
  （c）該当フレームが削除され、manifest.json から除去されている
```

**Phase 12 実装タスク：**

```
T5-1: WAL フレームアーカイブ書き込み
  [ ] sqld チェックポイント前フックで WAL フレームを wal-archive/ へコピー
  [ ] フレームごとに CRC32 チェックサムを計算・付与
  [ ] manifest.json へフレームメタデータを追記
  参照: §3.2, §3.6.3, §6.4（PITR）

T5-2: スナップショット保存
  [ ] チェックポイント完了後に data.db を snapshot-{frame_no}.db へコピー
  [ ] スナップショットは最新 1 件のみ保持（古い snapshot ファイルを削除）
  [ ] manifest.json の base_frame / snapshot フィールドを更新
  参照: §3.6.3

T5-3: manifest.json 管理
  [ ] manifest.json の読み込み・書き込みロジック（アトミック更新）
  [ ] 整合性確認: frames[] と実ファイルの突合
  [ ] manifest.json 破損時の起動エラー処理
  参照: §3.2, §3.6.3, §3.6.6

T5-4: クリーンアップスレッド
  [ ] wal_retention_days 設定を config.toml から読み込み
  [ ] 24h ごとに manifest.json をスキャンし期限超過フレームを削除
  [ ] 削除後に manifest.json を更新
  参照: §4.2, §3.6.6
  検証: TC-5-8
```

---


#### 実装詳細

#### 14.8 WAL アーカイブ処理（Phase 12〜13）

チェックポイント前フックで WAL フレームを `wal-archive/` へコピーし、`manifest.json` をアトミックに更新する。

```rust
// wal/archive.rs
use crc32fast::Hasher as Crc32Hasher;
use tokio::io::AsyncWriteExt;

pub async fn archive_frames(
    db_name:    &str,
    data_dir:   &std::path::Path,
    new_frames: &[WalFrame],
) -> anyhow::Result<()> {
    let archive_dir   = data_dir.join("databases").join(db_name).join("wal-archive");
    let manifest_path = archive_dir.join("manifest.json");

    tokio::fs::create_dir_all(&archive_dir).await?;
    let mut manifest = Manifest::load(&manifest_path).await.unwrap_or_default();

    for frame in new_frames {
        // CRC32 計算
        let mut h = Crc32Hasher::new();
        h.update(&frame.data);
        let checksum = h.finalize();

        let filename  = format!("frame-{:012}.bin", frame.frame_no);
        let frame_path = archive_dir.join(&filename);

        // 書き込み + fsync（I-4 保証）
        let mut f = tokio::fs::File::create(&frame_path).await?;
        f.write_all(&frame.data).await?;
        f.sync_all().await?;

        manifest.frames.push(FrameMeta {
            frame_no:   frame.frame_no,
            file:       filename,
            size:       frame.data.len() as u64,
            checksum,
            created_at: chrono::Utc::now(),
        });
    }

    // manifest をアトミック更新（tmp → rename）
    manifest.save_atomic(&manifest_path).await?;
    Ok(())
}

/// manifest.json のロード・アトミック保存
impl Manifest {
    /// manifest.json を読み込む。ファイルが存在しない場合は Err を返す（呼び出し側で unwrap_or_default）
    pub async fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let data = tokio::fs::read(path).await?;
        Ok(serde_json::from_slice(&data)?)
    }

    /// manifest.json をアトミック更新（tmp → fsync → rename、I-4 保証）
    pub async fn save_atomic(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self)?;
        tokio::fs::write(&tmp, &json).await?;
        tokio::fs::rename(&tmp, path).await?;
        Ok(())
    }
}
```


### Phase 13：バックアップ・リストア・PITR API

**目標**：WAL アーカイブからのオンラインバックアップと任意時点リストア（PITR）が動作する

**スコープ：**
- バックアップ API：`GET /admin/v1/databases/{name}/backup`
- リストア API：`POST /admin/v1/databases/{name}/restore`
- PITR API：`POST /admin/v1/databases/{name}/restore/point-in-time`

**完了条件（テストケース）：**

```
TC-5-1: バックアップと同時書き込み
  （a）GET /admin/v1/databases/{name}/backup を開始（大きな DB でストリーミング）
  （b）バックアップ中に POST /{name}/v2/pipeline で INSERT を実行
  （c）バックアップは整合性を保って完了し、書き込みリクエストも 200 で成功

TC-5-2: バックアップからリストア
  （a）GET /admin/v1/databases/{name}/backup でバックアップファイルを取得
  （b）POST /admin/v1/databases/{name}/restore でリストア
  （c）リストア後 SELECT → 元のデータが参照できる

TC-5-3: 不正ファイルでリストア
  （a）POST /admin/v1/databases/{name}/restore（不正な SQLite ファイルをアップロード）
  期待: 409 RESTORE_INTEGRITY_FAILED

TC-5-4: PITR 無効時の試行
  設定: wal_retention_days = 0（または未設定）
  （a）POST /admin/v1/databases/{name}/restore/point-in-time
  期待: 503 PITR_NOT_ENABLED

TC-5-5: タイムスタンプ指定 PITR
  （a）t=T1 に INSERT A、t=T2 に INSERT B
  （b）POST .../restore/point-in-time {"timestamp": "T1+1s"}
  （c）SELECT → A が存在し B が存在しない

TC-5-6: アーカイブ範囲外タイムスタンプ
  （a）POST .../restore/point-in-time（アーカイブに存在しない timestamp）
  期待: 404 FRAME_NOT_FOUND

TC-5-7: CRC32 不一致フレームで PITR
  （a）フレームファイルを手動で破壊
  （b）POST .../restore/point-in-time
  期待: 409 RESTORE_FRAME_CORRUPT、元 DB が復元されている
```

**対象外（Phase 14 以降）：**
- ブランチ機能（Phase 14）
- 外部ストレージへのアーカイブ転送

**Phase 13 実装タスク：**

```
T5-5: バックアップ API
  [ ] GET /admin/v1/databases/{name}/backup → sqlite3_backup_* API でオンラインバックアップ
  [ ] バックアップ中の書き込みをブロックしない（Online Backup API の並行性保証）
  [ ] レスポンス: SQLite ファイルをストリーミング送信（Content-Type: application/octet-stream）
  参照: §6.4（バックアップ / PITR / ブランチ）
  検証: TC-5-1, TC-5-2

T5-6: リストア API
  [ ] POST /admin/v1/databases/{name}/restore → アップロードされた SQLite ファイルを適用
  [ ] PRAGMA integrity_check でファイル整合性検証
  [ ] 検証失敗時: 元 DB を復元し 409 RESTORE_INTEGRITY_FAILED を返す
  [ ] 成功時: sqld をリロードしてサービス再開
  参照: §6.4, §7.3
  検証: TC-5-2, TC-5-3

T5-7: PITR API
  [ ] POST /admin/v1/databases/{name}/restore/point-in-time（timestamp / frame_no 指定）
  [ ] wal_retention_days = 0 の場合は即座に 503 PITR_NOT_ENABLED
  [ ] manifest.json から対象フレームを特定
  [ ] スナップショット + WAL フレームリプレイ処理を実装
  [ ] CRC32 検証失敗時: 元 DB 復元 + 409 RESTORE_FRAME_CORRUPT
  参照: §6.4, §7.3, §3.6.3
  検証: TC-5-4〜TC-5-7

T5-8: エラーハンドリング・冪等性
  [ ] 各 API エラーコードを §7.3 の定義に沿って実装
  [ ] リストア・PITR の中断時に元 DB を必ず復元すること（ロールバック保証）
  [ ] 管理 API への Admin JWT 検証をバックアップ・リストアエンドポイントにも適用
  参照: §7.3, §10

T5-9: 統合テスト
  [ ] TC-5-1〜TC-5-8 を全て実行し PASS することを確認
  [ ] Phase 1〜11 の TC がリグレッションしないことを確認
```

---


### Phase 14：ブランチ

**目標**：DB の任意時点からブランチを作成し、独立した DB として読み書き可能にする

**スコープ：**
- ブランチ DB 作成 API（`from: "current"` / `from: {timestamp}` / `from: {frame_no}`）
- ブランチ一覧・削除 API
- ブランチ DB 命名規則と予約名バリデーション（`___`）
- 再起動後のブランチ DB 自動復元

**完了条件（テストケース）：**

```
TC-6-1: current から新規ブランチ作成
  （a）POST /admin/v1/databases/my-db/branches {"branch_name":"feature-x","from":"current"} → 201
  （b）GET /my-db___feature-x/v2/pipeline SELECT → 元 DB と同じデータが返る
  （c）GET /admin/v1/databases/my-db/branches → feature-x が含まれる

TC-6-2: ブランチ DB への独立書き込み
  （a）ブランチ DB に INSERT A
  （b）元 DB を SELECT → A が存在しない
  （c）ブランチ DB を SELECT → A が存在する

TC-6-3: タイムスタンプ指定でブランチ作成
  （a）t=T1 に my-db へ INSERT A、t=T2 に INSERT B
  （b）POST .../branches {"branch_name":"snap","from":"T1+1s"} → 201
  （c）ブランチ DB を SELECT → A が存在し B が存在しない

TC-6-4: ブランチ一覧取得
  （a）feature-x・snap の 2 ブランチを作成
  （b）GET /admin/v1/databases/my-db/branches → 両ブランチが含まれる

TC-6-5: ブランチ削除
  （a）DELETE /admin/v1/databases/my-db/branches/feature-x → 204
  （b）GET /admin/v1/databases/my-db/branches → feature-x が含まれない
  （c）/my-db___feature-x/v2/pipeline → 404 DB_NOT_FOUND
  （d）{data-dir}/databases/my-db___feature-x/ ディレクトリが削除されている

TC-6-6: 予約名バリデーション
  （a）POST /admin/v1/databases {"name":"a___b"} → 400 DB_RESERVED_NAME

TC-6-7: 再起動後のブランチ自動復元
  （a）feature-x ブランチを作成
  （b）サーバーを再起動（同じ --data）
  （c）/my-db___feature-x/v2/pipeline SELECT → データが復元されている
```

**対象外（Phase 15 以降）：**
- ブランチのマージ
- ブランチ間 diff

**Phase 14 実装タスク一覧：**

```
T6-1: branches.json 読み書きロジック
  [ ] {data-dir}/meta/branches.json の読み込み・書き込み（アトミック更新）
  [ ] 起動時に branches.json を読み込み（§8.1 Step 5-3）
  [ ] branches.json がない場合は空で初期化
  参照: §3.2, §8.1 Step 5-3

T6-2: ブランチ DB 命名・バリデーション
  [ ] `___` を含む DB 名を予約名として判定
  [ ] POST /admin/v1/databases で `___` 含む名前 → 400 DB_RESERVED_NAME
  [ ] ブランチ DB 内部名の生成: {source}___{branch-name}
  参照: §7.3
  検証: TC-6-6

T6-3: `from: "current"` ブランチ作成
  [ ] sqlite3_backup_* API で source DB のオンラインスナップショットを取得
  [ ] {data-dir}/databases/{name}___{branch}/ ディレクトリ作成
  [ ] スナップショットを data.db として配置
  [ ] branches.json へメタデータを追加
  [ ] 新 DB を sqld でオープン・マルチ DB マネージャへ登録
  参照: §6.4（ブランチ）
  検証: TC-6-1, TC-6-2

T6-4: `from: {timestamp}/{frame_no}` ブランチ作成
  [ ] wal_retention_days = 0 の場合は 503 PITR_NOT_ENABLED
  [ ] T5-7 の PITR ロジックを再利用してブランチ DB を構築
  [ ] 構築先: {data-dir}/databases/{name}___{branch}/data.db
  [ ] branches.json へメタデータを追加（from_frame を記録）
  参照: §6.4, T5-7
  検証: TC-6-3

T6-5: ブランチ一覧・削除 API
  [ ] GET /admin/v1/databases/{name}/branches → branches.json からフィルタして返す
  [ ] DELETE /admin/v1/databases/{name}/branches/{branch-name}
        → sqld クローズ・ディレクトリ削除・branches.json 更新
  参照: §6.4（ブランチ）
  検証: TC-6-4, TC-6-5

T6-6: 起動時ブランチ自動復元ロジック
  [ ] branches.json を読み込み、各 db_name の DB ディレクトリが存在すれば sqld でオープン
  [ ] ディレクトリが存在しないエントリは WARN ログを出力してスキップ
  参照: §8.1 Step 5-3
  検証: TC-6-7

T6-7: 統合テスト
  [ ] TC-6-1〜TC-6-7 を全て実行し PASS することを確認
  [ ] Phase 1〜13 の TC がリグレッションしないことを確認
```

---


### Phase 15：SQLite 拡張・内製化・HA

Phase 14 完了後に計画する。候補（優先度未確定）：

- SQLite 拡張機能ロード（`.so` / Wasm）
- libSQL 内部コンポーネントの段階的内製化（§3.5.3 のロードマップに従う）
- 高可用性・自動フェイルオーバー
- メトリクス永続化・外部監視連携（Prometheus 等）

---


## 10. セキュリティ考慮事項

### 10.1 JWT シークレット管理

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

### 10.2 管理ポート（8081）のアクセス制御

**デフォルト動作：**

```
bind: 127.0.0.1:8081   ← localhost のみ待機（Phase 1 固定）
```

Phase 1〜5 では管理ポートのバインドアドレスは `127.0.0.1` 固定であり、変更できない（`--admin-bind` フラグは Phase 6 以降で追加する）。
外部ネットワークへの公開が必要な場合は Phase 6 以降でリバースプロキシ経由で行うこと。公開する場合は必ず `[admin] auth_token` を設定し、TLS ターミネーション（Nginx・Caddy 等）を前段に置くこと。

**推奨構成（本番）：**

```
Internet → Reverse Proxy (TLS) → :8080 (API)
                                → :8081 (Admin) ← VPN / internal network only
```

### 10.3 データディレクトリのファイルパーミッション

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

### 10.4 TLS

Phase 1〜7 では TLS をネイティブ実装しない。リバースプロキシ（Nginx・Caddy 等）による TLS ターミネーションを推奨する。

```
Client → [TLS] → Nginx/Caddy → [plain HTTP] → adlaire-db :8080
```

TLS ネイティブ対応は Phase 13 以降の検討事項とする。

### 10.5 トークン情報の漏洩防止

- `GET /admin/v1/tokens` および `GET /admin/v1/tokens/{id}` は JWT 文字列（`token` フィールド）を返さない（§6.4 参照）
- JWT 文字列は `POST /admin/v1/tokens` の発行時レスポンスでのみ返す（以降は再取得不可）
- `tokens.json` に JWT 文字列は保存しない（`id` と `access` と有効期限のみ保存）

### 10.6 パストラバーサル対策

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

### 12.1 フォーマット

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
| `db` | string \| null | マルチ DB 時 | 対象 DB 名（Phase 6〜） |
| `error` | string \| null | エラー時 | エラーコードまたはメッセージ |

### 12.2 ログレベル

| レベル | 用途 |
|---|---|
| `ERROR` | リクエスト処理失敗・起動失敗・ファイル I/O エラー |
| `WARN` | 認証失敗・存在しない DB へのアクセス・設定非推奨 |
| `INFO` | 起動・停止・HTTP リクエスト完了（デフォルト） |
| `DEBUG` | SQL 実行詳細・WAL チェックポイント |
| `TRACE` | hrana プロトコル詳細・バイト列ダンプ |

デフォルトレベル：`INFO`。`--log-level` フラグまたは環境変数 `ADLAIRE_LOG_LEVEL` で変更可。

### 12.3 出力先

- デフォルト：stdout（コンテナ・systemd との親和性）
- `--log-file <PATH>` 指定時：ファイルへ書き出し（ローテーションは外部ツール任せ）
- stdout とファイルの同時出力は非サポート（Phase 1 時点）

### 12.4 起動・停止ログ例

```
{"ts":"...","level":"INFO","msg":"Adlaire DB starting","version":"0.1.0","data_dir":"/var/lib/adlaire","port":8080}
{"ts":"...","level":"INFO","msg":"Adlaire DB listening","addr":"0.0.0.0:8080","admin_addr":"127.0.0.1:8081"}
{"ts":"...","level":"INFO","msg":"shutdown signal received"}
{"ts":"...","level":"INFO","msg":"Adlaire DB stopped"}
```

---

## 13. WAL 設定

### 13.1 WAL モード

すべての SQLite DB は起動時に WAL モードを有効化する。

```sql
PRAGMA journal_mode = WAL;
```

- WAL により複数の同時読み取りと 1 書き込みが並行可能
- クラッシュ後の自動リカバリは SQLite が保証

### 13.2 設定パラメータ

| パラメータ | デフォルト | CLI フラグ | config.toml キー | 説明 |
|---|---|---|---|---|
| busy timeout | 5000 ms | `--busy-timeout` | `[storage] busy_timeout_ms` | ロック待機タイムアウト。超過時 503 BUSY |
| WAL checkpoint interval | 1000 pages | — | `[storage] wal_checkpoint_pages` | 自動チェックポイントのページ閾値 |
| WAL checkpoint mode | `PASSIVE` | — | `[storage] wal_checkpoint_mode` | `PASSIVE` / `FULL` / `RESTART` |
| synchronous | `NORMAL` | — | `[storage] synchronous` | `OFF` は非サポート（I-4 違反） |

### 13.3 チェックポイント挙動

- SQLite のデフォルト自動チェックポイント（1000 pages）をそのまま使用（Phase 1）
- Phase 1 では手動チェックポイントの API は提供しない
- Phase 10（レプリケーション）時に WAL チェックポイント制御を再設計する

### 13.4 busy timeout エラー

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
| Turso Cloud | libSQL のマネージドホスティングサービス。Adlaire DB の hrana プロトコル互換の参照実装 |
| libSQL | SQLite フォーク。HTTP API・WAL レプリケーション等を追加した OSS DB ライブラリ |
| sqld | libSQL のサーバーコンポーネント。HTTP API・WebSocket API を提供する |
| libSQL フォーク | Adlaire DB 専用に改変した libSQL（sqld 含む）。本プロジェクトの全体基盤 |
| hrana | Turso / libSQL のワイヤプロトコル名。hrana-http（HTTP版）と hrana-ws（WebSocket版）がある |
| baton | hrana プロトコルにおけるセッション継続識別子 |

---

## 将来の内製化方針

### 基本方針
- 内製化の単位はクレートとする
- 外部クレートを内製クレートに段階的に差し替えることで内製化を進める
- 既存の外部クレートで要件を満たせる場合は積極的に採用する
- 既存クレートで不足する機能は、最初から内製クレートとして開発する
- **外部クレートと内製クレートの併用パターンを初期段階から採用する**
- 内製化の順序は実装難易度が低いものから優先する

### 併用パターン

初期段階から外部クレートと内製クレートを併用する。
外部クレートで不足する機能を内製クレートで補い、
成熟次第に外部クレートを内製クレートへ差し替える。

### 内製化順序（難易度低い順）

| 優先度 | 対象 | 現行クレート | 備考 |
|--------|------|------------|------|
| 1 | WAL チェックポイント制御 | libSQL | Phase 10 と直結 |
| 2 | ストレージ層 | libSQL（SQLite ページャー）| WAL 内製後に着手 |
| 3 | SQL パーサ | libSQL（SQLite）| 最難関・最後 |

### 実施時期

Phase 1〜7 と並行して着手可能なものから開始する。

---

## テストカバレッジ方針

### 原則

- **意味のあるテストのみ書く**：カバレッジ数値のためだけのテストは禁止
- 各テストは「何が壊れたら検出できるか」を明確にする
- テストのないコードより、誤ったテストのほうが危険

### テスト種別と対象

| 種別 | 対象 | 必須条件 |
|------|------|---------|
| **ユニットテスト** | フィルタ評価・JWT 検証・エラー処理 | 境界値・異常系を必ず含む |
| **統合テスト** | TC-* 全件 | CI で全件通過必須。1件でも失敗はリリース不可 |
| **プロパティテスト** | クエリ演算子・WAL 整合性 | 任意入力で不変条件が崩れないことを検証 |
| **クラッシュテスト** | fsync 保証（I-4）・自動 ROLLBACK（I-5） | プロセス強制終了 → 再起動 → データ検証。WAL 内製時は必須 |

### カバレッジ目標

| 層 | 目標 | 備考 |
|----|------|------|
| WAL・ストレージ（内製クレート） | **95%以上** | データ整合性に直結。妥協しない |
| API・認証層 | **85%以上** | TC-* で大部分をカバー |
| エラーハンドリング | **100%** | 全エラーコードに対応するテストを必須とする |

### CI ゲート（厳格）

| チェック | 条件 |
|---------|------|
| 統合テスト（TC-*） | 全件グリーン必須。スキップ禁止 |
| カバレッジ下限 | 内製クレート 95%・API 層 85% 未満でビルド失敗 |
| クラッシュテスト | Phase 1 完了時点から CI 必須 |
| プロパティテスト | 1000 ケース以上で回帰チェック |

---

