# Adlaire DB 仕様書

**バージョン：** V.116
**ステータス：** 設計中  
**最終更新：** 2026-09-13

---

## 0. 仕様書バージョン管理固定契約

本仕様書のバージョンは `V.{累積番号}` 形式で表記する。現在の仕様書バージョンは `V.116` である。

仕様書バージョンは累積単調増加とし、リセットしてはならない。大規模改訂、Phase 再編、リポジトリ移行、仕様書構成変更、実装方針変更、Turso Cloud 互換方針の更新があっても、`V.1`、`0.x`、日付ベース、Phase 番号ベースへ戻してはならない。

仕様書を更新する PR は、変更内容が仕様本文に影響する場合、必ず現在値より大きい次の累積番号へ進める。`V.99` の次は `V.100` とし、以後 `V.101`、`V.102` のように 1 ずつ増加させる。

**禁止事項：**

| 状態 | 判定 |
|------|------|
| `V.{累積番号}` 以外の仕様書バージョン表記へ変更する | review failure |
| 仕様書バージョンを過去番号へ戻す | merge 不可 |
| Phase 番号に合わせて仕様書バージョンをリセットする | merge 不可 |
| リポジトリ移行や仕様再編を理由に `V.1` から再開する | merge 不可 |
| 仕様本文を変更したのに仕様書バージョンを上げない | review failure |

## 1. 概要

### 1.1 プロジェクト概要

Adlaire DB は **libSQL ワイヤプロトコル（hrana-http v2 / hrana-ws v3）互換のサーバー特化 DB サーバー**である。

libSQL クライアント SDK（TypeScript・Rust・Go 等）から接続 URL を差し替えるだけで動作する。クライアント側の埋め込みレプリカ機能は対象外とし、サーバー側の HTTP/WebSocket API・マルチDB管理・レプリケーション・バックアップに特化する。

libSQL フォークの内部コンポーネント（WAL・ページストレージ・SQL エンジン等）の内製化は Phase 19 から開始する。Phase 18 以前は Turso Cloud 互換レイヤー、管理 API、レプリケーション、バックアップ、ブランチ、HA の完成を優先し、production path の内部差し替えは行わない。

### 1.2 ポジション

| 比較対象 | Adlaire DB との関係 |
|----------|---------------------|
| Turso Cloud | ワイヤプロトコル（hrana）互換の参照実装。埋め込みレプリカは対象外 |
| libSQL / libsql crate | ワイヤプロトコルと embedded SQLite の実装参照。libsql 0.6（crates.io）を組み込み利用 |
| SQLite | libsql crate 経由で互換性を維持 |

### 1.3 固定制約

| 項目 | 内容 |
|------|------|
| 実装言語 | Rust + 標準ライブラリ |
| ストレージ・SQL 基盤 | libsql crate 0.6（embedded SQLite / WAL モード）|
| 外部フレームワーク | 使用禁止。外部クレートは使用可能だが、Web フレームワーク（axum・actix-web・rocket 等）は採用しない |
| 目標機能 | Turso Cloud 互換・hrana プロトコル互換・サーバー特化機能 |
| Phase 19 方針 | Turso Cloud 追従を継続し、互換レイヤーを維持したまま libSQL 内部を段階的に内製化する |
| デプロイ形態 | シングルバイナリ起動 |
| 対象 OS | Linux |

### 1.4 設計不変条件

実装のあらゆる判断においてこれらを最優先する。

**I-1：hrana プロトコル互換**  
既存の libSQL クライアント SDK（TypeScript・Rust・Go 等）が、Turso Cloud の URL を Adlaire DB の URL に差し替えるだけで動作しなければならない。ただし埋め込みレプリカ（`syncUrl` 指定）は対象外とし、通常の HTTP/WebSocket 接続のみを対象とする。

**I-1a：Turso Cloud 互換の継続追従**  
Adlaire DB は Turso Cloud 互換を継続追従する。Turso Cloud の API、metadata、認証、権限、エラー、管理モデル、SDK 互換挙動に変更が確認された場合は互換性レビュー対象とし、Adlaire DB 側の仕様差分を明示する。自己ホスト都合で Turso Cloud と異なる仕様を採用する場合は、差分理由、代替仕様、既存 SDK への影響、後方互換性を本仕様書に明記してから実装する。

**I-2：外部 DB 依存は libSQL フォーク一本**  
SQLite・libSQL フォーク以外の外部 DB ライブラリ（PostgreSQL・MySQL ドライバ等）に依存しない。

**I-3：シングルバイナリ**  
サーバー起動は `./adlaire-db <flags>` 一コマンドで完結する。外部デーモン・サイドカーを必要としない（Phase 1）。

**I-4：データ永続化の先行保証**  
クライアントへ成功応答を返す前に、書き込みデータが永続化（fsync）されていることを保証する。

**I-5：内製化は段階的・計画的に**  
libSQL 内部コンポーネントの内製化はフェーズ完了後に計画・判断する。内製化は Turso Cloud 互換を捨てるためではなく、互換レイヤーを保ったまま内部実装を段階的に置き換えるために行う。「実装が大変だから」という理由で無計画に外部依存を追加することは認めない。

**I-6：外部 Web フレームワーク不使用**  
HTTP サーバー層に Web フレームワーク（axum・actix-web・rocket 等）を使用しない。hyper 等の低レベル HTTP ライブラリ（クレート）は使用可能だが、ルーティング・ミドルウェア・リクエスト解析の構造はフレームワークに依存せず自前で実装する。

### 1.5 活用・進化方針

Adlaire DB の活用目的は、SQLite / libSQL 系の軽量さを保ちながら、Turso Cloud と同等の管理体験を自己ホスト環境へ持ち込むことである。単なる SQLite wrapper ではなく、Turso Cloud 互換の API、SDK 接続、認証、metadata、organization / group / location / quota、backup、restore、PITR、branch、replication、metrics、HA、運用証跡を段階的に備える DB 管理基盤として実装する。

**活用対象：**

| 活用対象 | 目的 | 必須方針 |
|----------|------|----------|
| 小規模 SaaS / 個人開発 | user / organization / project ごとの DB を軽量に管理する | Turso Cloud 互換 API と libSQL SDK 互換を優先する |
| 社内・オンプレ環境 | 外部クラウドへデータを出せない環境で DB 管理 API を提供する | 自己ホスト運用でも organization / group / location / quota を正式対象にする |
| 開発・検証環境 | Turso Cloud を使わずに互換 API、SDK、migration、backup、branch を検証する | snapshot / oracle / compatibility test を仕様化する |
| アプリ別独立 DB | CMS、業務ツール、管理画面、tenant ごとに DB を分離する | DB identity、path boundary、auth scope、quota を破らない |
| 運用復旧基盤 | backup、restore、PITR、branch、rollback により戻せる運用を実現する | success-before-fsync、partial commit、metadata/file 不一致を禁止する |

**進化方針：Turso Cloud 互換優先、内部は段階的に内製化**

| 項目 | 固定方針 |
|------|----------|
| 外部契約 | API、wire format、SDK 挙動、認証、metadata、error、管理モデルは Turso Cloud 互換を優先する |
| 自己ホスト差分 | 単一サーバーやオンプレ都合の差分は許可するが、差分理由、代替仕様、SDK 影響、後方互換性を仕様本文へ明記してから実装する |
| 内部実装 | 初期は libsql crate と外部 crate を利用し、運用機能を満たす。成熟後に adapter 境界の内側から段階的に内製 crate へ置き換える |
| 内製化禁止線 | 内製化を理由に API、response wrapper、JWT claim、metadata schema、error code、SDK 互換挙動を破ってはならない |
| モード分離 | Adlaire 独自拡張が必要な場合は、Turso 互換 mode と Adlaire 拡張 mode を仕様上分離し、既定は Turso 互換 mode とする |
| 完了判定 | 内製化 PR は Turso Cloud / libSQL SDK compatibility、Phase regression、oracle、invariant ledger が通るまで完了扱いにしない |

**最終目標：**

Adlaire DB の最終目標は、Turso Cloud 互換の自己ホスト DB 管理基盤として実用化したうえで、外部 contract を固定したまま内部実装を Adlaire 独自基盤へ段階移行することである。外部 contract とは、HTTP/WebSocket wire format、Platform API、admin API、SDK 互換挙動、JWT claim、metadata schema、error code、response wrapper、persistence compatibility、operator-facing behavior を指す。内部実装とは、WAL、checkpoint、storage、query executor adapter、archive、branch engine、quota engine、scheduler、metrics、HA coordination、recovery engine を指す。

**絶対ルール：**

| Rule | 内容 | 違反時の扱い |
|------|------|--------------|
| External contract freeze | Turso Cloud 互換 mode の外部 contract は、内製化都合で変更しない | merge 不可 |
| Internal replacement only | 内製化 PR は外部 contract ではなく adapter 境界の内側だけを置き換える | Phase 未完了 |
| Compatibility first | Turso Cloud / libSQL SDK 互換 snapshot が壊れる変更は、先に仕様差分、mode 分離、migration、rollback を定義する | 実装開始禁止 |
| Default compatibility mode | 既定挙動は常に Turso 互換 mode とする | Adlaire 拡張 mode を既定にした場合は merge 不可 |
| No silent divergence | 自己ホスト都合の差分を暗黙仕様にしない。差分理由、代替仕様、client impact、evidence を本文に残す | review failure |
| Evidence before Done | compatibility、oracle、invariant、regression、rollback evidence が揃うまで Done receipt を作成しない | Phase 未完了 |

**禁止事項：**

| 禁止事項 | 例 | 判定 |
|----------|----|------|
| 内製化都合で API path / method / request / response を変更する | adapter 差し替えのため `/v1/*` wrapper を変える | merge 不可 |
| SDK 互換を壊す | `@libsql/client` が既存 URL 差し替えで動かない | merge 不可 |
| metadata schema を理由なく破壊する | migration / rollback なしに field rename / delete を行う | merge 不可 |
| error code を差し替える | 既存 `DB_NOT_FOUND` を別 code に変える | merge 不可 |
| Turso 互換 mode に Adlaire 独自挙動を混ぜる | 既定 mode で独自 field 必須、独自 auth 必須にする | merge 不可 |
| `INTERNAL_ERROR` で互換差分を隠す | 本来 `INVALID_REQUEST` / `QUOTA_EXCEEDED` の差分を 500 にする | Phase 未完了 |
| production path を rollback flag なしで内製 crate へ切り替える | shadow 検証なしに write path を置換する | merge 不可 |

**許可事項：**

| 許可事項 | 条件 | 必須 evidence |
|----------|------|---------------|
| 内部 adapter の追加 | 既定 path の外部 contract に差分を出さない | shadow diff、compat snapshot |
| libsql crate 内側の置換準備 | production write path を変えない、または rollback flag を持つ | rollback test、crash recovery test |
| shadow mode 実装 | client-visible response に影響しない | shadow/active diff、performance baseline |
| Adlaire 拡張 mode 追加 | Turso 互換 mode と config / API / metadata を仕様上分離する | mode matrix、SDK regression |
| 自己ホスト差分の採用 | Turso Cloud と同一にできない理由が security / persistence / operation 上明確 | compatibility diff、client impact |
| 内製 crate の新規開発 | 外部 contract を固定し、adapter 境界に閉じる | unit coverage、oracle、invariant result |

**実装判断表：**

| 変更種別 | 判断 | 仕様に必要な固定事項 |
|----------|------|----------------------|
| Turso Cloud 互換に影響する変更 | 原則 Turso Cloud に追従。差分は例外扱い | snapshot source、差分理由、SDK impact、error mapping |
| libSQL SDK 互換に影響する変更 | SDK 互換を壊さない。壊す場合は Turso 互換 mode では不可 | SDK transcript、regression command、migration note |
| 自己ホスト差分 | security、persistence、operation の理由がある場合のみ可 | 代替仕様、operator impact、compatibility diff |
| 内部最適化 | 外部 contract に差分がなければ可 | performance baseline、rollback condition |
| Adlaire 独自拡張 | Turso 互換 mode と分離する場合のみ可 | mode flag、API boundary、metadata boundary |
| Phase 19 以降の内製化 | external contract freeze / internal replacement only を満たす場合のみ可 | shadow diff、oracle、invariant、full regression |

実装判断で迷う場合は、`Turso Cloud 互換 > libSQL SDK 互換 > 既存 Adlaire 後方互換 > 自己ホスト最適化 > 内製化都合 > Adlaire 独自拡張` の順で優先する。内部実装を育てることは目的であるが、外部契約を壊してまで内製化を進めてはならない。

---

## 2. 機能スコープ

### 2.1 クライアント接続

| 機能 | Phase | 説明 |
|------|-------|------|
| HTTP API（hrana-http） | 3 | libSQL クライアント SDK が利用する HTTP/JSON API |
| JWT 認証 | 4 | Bearer トークンによる認証 |
| マルチDB（パスベース） | 6 | URL パスで接続先 DB を指定 |
| WebSocket API（hrana-ws） | 9 | インタラクティブトランザクション用 |
| 埋め込みレプリカ同期 | — | **対象外**（サーバー特化のためスコープ外） |
| ATTACH DATABASE（クロス DB クエリ） | 10 | 管理下 DB 間のみ許可。任意パス指定は禁止 |
| メトリクス API | 10 | 接続数・クエリ数・ストレージ使用量の取得 |
| SQLite 拡張機能ロード | 16 | `.so` / Wasm 拡張（Vector Search 等）のロード |

### 2.2 データベース管理

| 機能 | Phase | 説明 |
|------|-------|------|
| DB 作成・削除・一覧 | 7 | 管理 API 経由での DB ライフサイクル管理 |
| トークン発行・失効 | 7 | DB ごと・全体のトークン管理 |
| データベースロケーション | 8 | Turso Cloud の location/region 概念と互換の DB 配置・所属属性 |
| 組織・グループ管理 | 8 | Turso Cloud の organization/group 管理モデルと互換の管理境界 |
| ストレージクォータ | 8 | Turso Cloud の quota/usage 概念と互換の容量制限・使用量管理 |
| バックアップ・エクスポート | 14 | オンラインバックアップ取得・リストア |
| ポイントインタイムリストア | 14 | WAL アーカイブから任意の時点への DB 復元 |
| ブランチ | 15 | DB のブランチ作成（WAL スナップショットから派生） |

### 2.3 レプリケーション

| 機能 | Phase | 説明 |
|------|-------|------|
| プライマリ・レプリカ構成 | 11 | 書き込みはプライマリ、読み取りはレプリカへ |
| WAL ベース同期 | 11 | libSQL の WAL レプリケーションを使用 |
| レプリカへの書き込みリダイレクト | 12 | 307 Temporary Redirect でプライマリへ転送 |

### 2.4 Turso Cloud 互換として対象に含める管理機能

以下の機能は自己ホスト環境でも対象外にしない。Adlaire DB は Turso Cloud 互換を価値の中心に置くため、単一サーバー構成であっても API・メタデータ・権限判定・エラー応答上は Turso Cloud と互換の管理モデルを提供する。

| 機能 | Phase | 互換方針 |
|------|-------|----------|
| データベースロケーション | 8 | Turso Cloud の location/region 概念と互換にする。自己ホストでも DB の `location` は正式属性として扱い、DB 作成・一覧・詳細・replication・backup/restore・branch の仕様と整合させる |
| 組織・グループ管理 | 8 | Turso Cloud の organization/group モデルと互換にする。DB、token、location、quota、admin 権限の所属・管理境界として扱う |
| ストレージクォータ | 8 | Turso Cloud の quota/usage 概念と互換にする。DB/group/organization 単位の制限値、使用量、超過時エラー、write/import/restore/replication apply への影響を仕様化する |

Phase 8 では、上記 3 機能について Turso Cloud 互換 API、永続化形式、権限モデル、エラーコード、既存 admin API への影響、テストゲートを具体化してから実装を開始する。

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

> **§5.4 vs §5.6 の使い分け**：§5.6 の 6 ステップフローは Phase 1〜6 の実装基準（`dbs` クレーム不使用）。§5.4 の 7 ステップフローは Phase 7 以降の拡張版（`dbs` クレームによる DB スコープ権限を追加）。実装フェーズに応じて参照するセクションを切り替える。

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

5. resolve_access(db_name) でアクセスレベルを解決
     dbs[db_name] が存在する → その値を使用（Phase 7〜）
     存在しない / dbs なし → a クレームを使用
   → Ro かつ書き込み操作 → 403 PERMISSION_DENIED

6. 検証通過 → リクエスト処理へ
```

**パフォーマンス：**
- サーバー起動時に `tokens.json` をメモリへロードする
- 失効済みトークン ID の集合はメモリ上に `RwLock<HashSet<String>>` で保持する（`AuthState` 内）
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

## 7. エラーハンドリング

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

## 9. 実装フェーズ

フェーズ単位で機能を積み上げる。内製化は Phase 19 から開始し、Phase 18 以前は production path の内部差し替えを行わない（§3.5.4）。

| フェーズ | 内容 | テストケース | 実装タスク |
|----------|------|------------|----------|
| **Phase 1** | ビルド基盤・CLI | — | T-1〜T-2 (2件) |
| **Phase 2** | データディレクトリ・libsql 統合 | — | T-3〜T-4 (2件) |
| **Phase 3** | HTTP サーバー・hrana パイプライン | TC-1, TC-2, TC-6 (3件) | T-5〜T-6 (2件) |
| **Phase 4** | JWT 認証・token コマンド | TC-3 (1件) | T-7〜T-9 (3件) |
| **Phase 5** | ログ・統合テスト | TC-4, TC-5 (2件) | T-10〜T-11 (2件) |
| **Phase 6** | マルチ DB ルーター・DB マネージャ | — | T2-1〜T2-2 (2件) |
| **Phase 7** | 管理 API・トークン CRUD・DB スコープ JWT | TC-2-1〜TC-2-6（TC-2-5b 含む）(7件) | T2-3〜T2-6 (4件) |
| **Phase 8** | Turso Cloud 互換管理モデル | TC-8-1〜TC-8-24 (24件) | T8-1〜T8-24 (24件) |
| **Phase 9** | WebSocket（hrana-ws v3） | TC-3-1〜TC-3-4 (4件) | T3-1〜T3-5 (5件) |
| **Phase 10** | ATTACH DB・メトリクス | TC-3-5, TC-3-6 (2件) | T3-6〜T3-8 (3件) |
| **Phase 11** | レプリケーション基盤（WAL ストリーム・スナップショット） | — | T4-1〜T4-3 (3件) |
| **Phase 12** | レプリカ同期・書き込みリダイレクト | TC-4-1〜TC-4-5 (5件) | T4-4〜T4-7 (4件) |
| **Phase 13** | WAL アーカイブ・manifest 管理 | TC-5-8 (1件) | T5-1〜T5-4 (4件) |
| **Phase 14** | バックアップ・リストア・PITR API | TC-5-1〜TC-5-7 (7件) | T5-5〜T5-9 (5件) |
| **Phase 15** | ブランチ作成・一覧・削除 | TC-6-1〜TC-6-7 (7件) | T6-1〜T6-7 (7件) |
| **Phase 16** | SQLite 拡張ロード | TC-16-1〜TC-16-6 (6件) | T16-1〜T16-6 (6件) |
| **Phase 17** | メトリクス永続化・外部監視連携 | TC-17-1〜TC-17-5 (5件) | T17-1〜T17-5 (5件) |
| **Phase 18** | HA・自動フェイルオーバー | TC-18-1〜TC-18-7 (7件) | T18-1〜T18-7 (7件) |
| **Phase 19** | libSQL 内部コンポーネント段階的内製化 | TC-19-1〜TC-19-6 (6件) | T19-1〜T19-6 (6件) |


### 9.0 実装判断ルール

実装中に仕様の解釈で迷った場合は、以下の順で判断する。

1. 本仕様書の「固定制約」「設計不変条件」「API 仕様」「エラー定義」を優先する
2. 同じ Phase 内の「スコープ」「対象外」「完了ゲート」を優先する
3. 既存実装と仕様が違う場合は仕様を優先する。ただし仕様変更が必要な場合は作業ルールの変更承認フローに従う
4. Phase に明記されていない機能は、その Phase では実装しない。必要なら次 Phase の対象として仕様に追記してから実装する
5. 互換性判断で迷う場合は §1.5 に従い、`Turso Cloud 互換 > libSQL SDK 互換 > 既存 Adlaire 後方互換 > 自己ホスト最適化 > 内製化都合 > Adlaire 独自拡張` の順で優先する
6. libSQL/Turso の hrana ワイヤ互換、Turso Cloud の管理 API・metadata・auth・error 互換を優先する。ただしセルフホスト運用・データ永続性・セキュリティ制約を破ってはならない
7. 内製化都合で API、レスポンス、認証、metadata、永続化形式、エラー形式を変更してはならない。変更が必要な場合は、先に Turso Cloud 追従差分として仕様書を改訂する
8. エラー形式で迷う場合は §7.3 の `code` を使う。新しいエラーが必要な場合は先に §7.3 へ追加する
9. JSON schema で迷う場合は「省略可能」「null 可」「空配列可」を本文に明記する。明記がないフィールドは必須とする
10. 永続化を伴う処理は、成功応答前にファイル内容とメタデータの両方が整合していることを必須とする

**曖昧語の扱い：**

| 表現 | 実装上の扱い |
|------|--------------|
| 「必須」 | Phase 完了条件に含める。未実装なら Phase 未完了 |
| 「任意」 | 実装してよいが、完了条件には含めない。未実装でも API 契約を壊してはならない |
| 「対象外」 | その Phase では実装しない。route を生やす場合は 501 または仕様で定めた stub 応答に限定する |
| 「将来」「検討」 | 実装禁止。対象 Phase が明記されるまでコードに入れない |
| 「スタブ」 | レスポンス形式・ステータス・ログ有無を本仕様に従って固定する |
| 「よい」「できる」「可能」 | 明示された条件下でだけ許可。条件、Phase、完了ゲートが同じ節にない場合は実装根拠にしない |
| 「原則」 | 例外条件が同じ節または参照先表に明記されている場合だけ例外を許可。例外条件がなければ MUST と同じ |
| 「推奨」「非推奨」 | 運用 guidance であり、実装完了条件にはしない。ただし security / persistence / compatibility に関係する場合は §9 の契約表を優先する |
| 「未実装」 | 指定 Phase に到達するまで成功応答を返してはならない。設定値や route を受け取る場合の挙動は §9.0.1 と §9.13.1 に従う |
| 「など」 | 例示であり、実装範囲を拡張しない。列挙外の機能は仕様未定義として扱う |

**仕様の正本性：**

| 種別 | 実装上の優先度 | 扱い |
|------|----------------|------|
| 本文の MUST / MUST NOT 相当の記述 | 最優先 | 実装・テスト・レビューで必ず満たす |
| §9 の判断ルール・契約表・完了ゲート | 最優先 | 各 Phase の merge 判定に使用する |
| API schema / error code / 永続化 schema の表 | 高 | コード例より優先する |
| Rust コード例 | 中 | 実装方針の参考。本文・表と矛盾する場合は本文・表を正とする |
| 将来方針・候補・検討事項 | 低 | 実装根拠にしてはならない。実装する場合は先に仕様変更 PR を作る |

**コード例・stub の扱い：**

| 対象 | 実装上の扱い |
|------|--------------|
| Rust コード例の `unwrap()` / `expect()` | 説明用に限定する。request、disk、network、metadata、config、auth、replication、backup、extension、HA 由来の失敗を production path で panic にしてはならない |
| Rust コード例の `not_implemented()` | 対象 Phase 前の stub 例であり、対象 Phase 完了後の production path に残してはならない |
| コメント内の Phase 名 | 実装境界の説明であり、本文・表の Phase 契約と矛盾する場合は本文・表を優先する |
| sample config の commented key | その key の実装許可ではない。§9.13 に Phase と挙動がない限り実装禁止 |

**未定義に遭遇した場合の処理：**

1. API の method/path/auth/status/body/error が未定義なら、実装を開始しない
2. 永続化ファイル、更新順序、rollback、破損時挙動が未定義なら、実装を開始しない
3. security boundary、secret handling、ログ秘匿対象が未定義なら、実装を開始しない
4. 既存仕様にない success response を返す route は追加しない
5. やむを得ず先行実装が必要な場合は、同じ PR の先頭 commit で仕様を更新し、その後に実装 commit を積む
6. 仕様変更を伴う PR は、PR description に「変更した契約」「影響 Phase」「追加テスト」を明記する

#### 9.0.1 stub / NOT_IMPLEMENTED 固定契約

未来 Phase の route、CLI、config、内部 hook を先に配置する場合は、本節を必ず満たす。stub は success response ではなく、互換性・探索性・routing 明確化のための失敗応答である。

| 対象 | 許可条件 | 必須応答 | 禁止 |
|------|----------|----------|------|
| HTTP route stub | §9.5 または該当 Phase 節に method/path が定義済み | `501 NOT_IMPLEMENTED` と `{"error":"not implemented","code":"NOT_IMPLEMENTED"}`。body なしが明記された Turso 互換 endpoint は該当節を優先 | 2xx、部分成功、metadata 更新、DB file 更新 |
| unsupported method | path は既知だが method が未定義 | `405 METHOD_NOT_ALLOWED` と `METHOD_NOT_ALLOWED` | 501 で method 不一致を隠す |
| unknown path | path が仕様未定義 | `404 ENDPOINT_NOT_FOUND` | 501 で未知 path を許可済みに見せる |
| CLI stub | subcommand 名と Phase が定義済み | 起動前 validation 後に非 0 終了。stderr に対象 Phase と未実装理由を出す | 0 終了、token/secret/metadata 生成 |
| config stub | §9.13.1 で対象 Phase 前の扱いが定義済み | 起動失敗または明示 WARN のどちらか。silent ignore 禁止 | 成功扱いで値を保存、将来 Phase の挙動を部分実行 |
| internal hook stub | production path から到達しない、または到達時に仕様済み error を返す | `NOT_IMPLEMENTED` または対象 error code | panic、unwrap、silent no-op |

stub の log は WARN `not implemented endpoint` / `not implemented command` / `not implemented config` のいずれかとし、request body、Authorization、JWT、admin/platform/replication/HA token、生 SQL args、backup body、extension path を出力してはならない。

対象 Phase の完了 PR では、その Phase の completion path にある stub をすべて実装済み処理へ置き換える。stub が残る場合は、対象外節または unsupported 固定表に残存理由、status、body、log、test を明記する。

#### 9.0.2 実装前仕様固定プロトコル

各 Phase の実装開始前に、実装者は該当 Phase について以下の契約表を作成または既存節から確認し、全項目が仕様書本文に存在することを確認する。1 つでも欠ける場合、その Phase の実装を開始してはならない。

| 契約カテゴリ | 固定必須項目 | 欠けている場合の扱い |
|--------------|--------------|----------------------|
| API surface | method、path、path parameter、query parameter、request body、success status、success body、error status、error body、auth 種別 | 仕様未確定。実装禁止 |
| Permission | admin/JWT/platform/replication/HA token のどれを使うか、ro/rw/scope/org/group/quota の判定順序 | 仕様未確定。実装禁止 |
| Persistence | 更新ファイル、atomic update 手順、fsync 対象、rollback 手順、起動時 recovery、破損時挙動 | 仕様未確定。実装禁止 |
| Compatibility | Turso Cloud との差分、libSQL SDK 影響、legacy metadata 互換、snapshot artifact の正規化方法 | 仕様未確定。実装禁止 |
| Observability | INFO/WARN/ERROR の発火条件、metric 名、secret/log redaction、audit 相当の記録有無 | 仕様未確定。実装禁止 |
| Test evidence | 正常系、異常系、権限系、永続化系、再起動系、互換系、concurrency 系の test ID | 仕様未確定。実装禁止 |

**実装前チェックリスト：**

1. 該当 Phase の「対象外」に書かれた機能を実装予定に含めていない
2. 追加 endpoint は §9.5 または該当 Phase 節に method/path 単位で存在する
3. 追加 error code は §7.3 に存在し、HTTP status と message 粒度が固定されている
4. 追加 metadata field は §9.6 または該当 Phase 節に型、必須/任意、default、migration、破損時挙動が固定されている
5. 追加 config/flag/env は §4 と §8.4 に default、優先順位、不正値エラーが固定されている
6. 追加外部 crate は §3.3.5 に用途、導入 Phase、セキュリティ影響が固定されている
7. Turso Cloud 互換に関係する変更は snapshot artifact と差分理由が固定されている
8. 実装 PR の完了条件として実行する test command と期待結果が固定されている

上記チェックリストの「固定」は、仕様書本文に追記済みであることを意味する。PR description だけに書かれた判断は仕様として扱わない。

### 9.1 全 Phase 共通の完了条件

各 Phase は、個別ゲートに加えて以下をすべて満たすこと。

1. `cargo build` と `cargo test` が成功する
2. その Phase で追加・変更した API の正常系、異常系、権限系、永続化系テストが追加されている
3. 既存 Phase のテストケースがリグレッションしない
4. API レスポンスの HTTP status、JSON body、`code` が §6・§7 と一致している
5. 永続化ファイルは tmp 書き込み、fsync、rename の順でアトミックに更新する。ただし対象 FS で fsync が利用できない場合は起動時に WARN ではなくエラーとする
6. メタデータ破損、DB 破損、設定不正は起動成功扱いにしない
7. ログには秘密情報、JWT、生 SQL 引数の値、管理トークンを出さない
8. 新しい設定値を追加した場合は、CLI/config/env の優先順位、デフォルト値、不正値エラーを §4 と §8.4 に追記する
9. 新しい永続化ファイルを追加した場合は、ディレクトリ構成、初期値、破損時挙動、バックアップ対象かどうかを §3.2 または該当 Phase に追記する
10. 新しい外部 crate を追加する場合は、採用理由、代替案、対象 Phase、セキュリティ影響を仕様書に明記する

#### 9.1.1 バグ修正ゼロ化ゲート

各 Phase の実装 PR は、以下を満たすまで完了扱いにしてはならない。1 つでも未実施または失敗がある場合は、その PR 内で修正し、後続の「バグ修正 PR」へ持ち越さない。

| ゲート | 必須判定 |
|--------|----------|
| Contract exhaustiveness | §9.5 の該当 endpoint 全てについて、method/path/auth/request/success/error/persistence/idempotency のテストが存在する |
| Error exhaustiveness | §7.3 / §9.7 に定義された該当 Phase の error code 全てについて、発火テストが存在する |
| Unknown/null validation | 管理 API は unknown field、余分 body、不正 null、不正 query、不正 path decode を `INVALID_REQUEST` として拒否する |
| Persistence atomicity | metadata 更新中の失敗注入で、partial write、空上書き、metadata/file 不整合が起きない |
| Restart recovery | 正常更新後、異常終了後、metadata 破損後の起動結果が §9.6 と一致する |
| Auth boundary | 認証なし、不正 token、scope 外、ro/rw の境界を全 endpoint で確認する |
| Secret leakage | stdout/stderr/log file/HTTP response に secret、JWT、admin token、replication token、HA token、生 SQL args、backup body、extension 絶対 path が出ない |
| Concurrency | 同一 resource への同時 create/delete/update で二重作成、lost update、破損 metadata が起きない |
| Regression | 当該 Phase より前の Phase の TC がすべて通る |
| Compatibility | Phase 5 以降は TypeScript `@libsql/client` の CRUD regression、Phase 9 以降は WebSocket regression、Phase 12 以降は replication regression が通る |
| Turso Platform compatibility | Phase 8 以降は `/v1/*` の response wrapper、field casing、status code、query mapping が Turso 互換 snapshot と一致する |
| Legacy fallback | Phase 8 以降は Phase 1〜7 で作られた legacy DB/token metadata が読み取り・接続・削除でき、新規作成 validation と混同されない |

**失敗時の扱い：**

- ゲート失敗はすべて当該 Phase の未完了として扱う
- test skip で通過扱いにしてはならない
- flaky test は `retry` で隠さず、原因を修正してから完了扱いにする
- 外部環境依存で自動化できない検証は、手順、期待値、実行ログ保存先を仕様書または PR description に固定する

#### 9.1.1a バグ修正持ち越し禁止リスト

以下の状態を含む PR は、実装が一見動作していても Phase 完了として扱わない。該当する場合は同一 PR 内で修正し、後続 PR へ「バグ修正」として分離してはならない。

| 禁止状態 | 判定 |
|----------|------|
| TODO/FIXME による契約未実装 | Phase 未完了 |
| error mapping の暫定 `INTERNAL_ERROR` 代用 | Phase 未完了 |
| request validation の一部未実装 | Phase 未完了 |
| auth/scope/quota/org/group 判定の一部 endpoint 未適用 | Phase 未完了 |
| metadata migration の片方向のみ実装 | Phase 未完了 |
| rollback 不能な destructive operation | Phase 未完了 |
| crash/restart 後の挙動未検証 | Phase 未完了 |
| concurrency test 未実施 | Phase 未完了 |
| Turso Platform snapshot 未更新または未比較 | Phase 8 以降は Phase 未完了 |
| TypeScript SDK regression 未実行 | Phase 5 以降は Phase 未完了 |
| WebSocket regression 未実行 | Phase 9 以降は Phase 未完了 |
| replication regression 未実行 | Phase 12 以降は Phase 未完了 |
| backup/restore rollback 未検証 | Phase 14 以降は Phase 未完了 |
| HA split-brain test 未実行 | Phase 18 以降は Phase 未完了 |
| internal adapter rollback flag 未検証 | Phase 19 は Phase 未完了 |

「後で検証する」「既知の軽微な不具合」「現時点では通る想定」は完了根拠として認めない。完了根拠は、実行済み command、保存された snapshot、または自動テスト結果のいずれかでなければならない。

#### 9.1.2 全 Phase 共通実装固定契約

本節は Phase 1〜19 の全実装に適用する。個別 Phase 節がより厳しい条件を定義する場合は個別 Phase 節を優先する。個別 Phase 節が沈黙している場合は下表を正とする。

| 項目 | 固定契約 |
|------|----------|
| JSON object | request body が JSON API の場合、body は object 必須。array/scalar/null は `400 INVALID_REQUEST` |
| unknown field | 管理 API、Turso Platform API、HA、extension、backup/restore/branch API では拒否。hrana protocol body は hrana 仕様に従う |
| field 省略 | schema で `?` が付いた field だけ省略可。明記なし field は必須 |
| `null` | schema で `null 可` と明記された field だけ許可。省略可能 field に `null` を送っても省略扱いにしない |
| 空文字 | token、name、id、path parameter、query value の空文字は禁止。明記された free-form text field だけ許可 |
| 空配列 | schema で空配列可と明記された field だけ許可。filter 配列、scope 配列、batch 配列は空を `INVALID_REQUEST` とする |
| query | unknown key、duplicate key、percent decode 不能、型変換不能は `INVALID_REQUEST` |
| pagination | `limit` は 1〜500。未指定は 100。`cursor` は base64url 文字列とし、decode 不能・期限切れ・対象 resource 不一致は `INVALID_REQUEST` |
| timestamp | response と metadata は RFC3339 UTC 秒精度。比較用 snapshot では placeholder 正規化。request の未来 timestamp は endpoint が許可しない限り `INVALID_REQUEST` |
| UUID/id | 外部互換で UUID が必要な field は UUID v4 文字列。内部 prefix id は `^[a-z]+_[a-zA-Z0-9_-]{1,80}$` |
| Content-Type | JSON body は `application/json` 必須。`charset=utf-8` は許可。octet-stream endpoint は `application/octet-stream` 必須 |
| Accept | 未指定、`*/*`、`application/json` は許可。その他を厳密拒否する場合は endpoint 節に明記する |
| response body | 204 は body なし。JSON response は object/array の最上位型を schema に固定し、成功時に error field を混在させない |
| error body | HTTP API の error は `{"error": "...", "code": "..."}` のみ。追加 debug field は返さない |
| idempotency | GET は副作用禁止。DELETE は表に明記された挙動に従う。POST は idempotency key を仕様化していない限り非冪等 |
| concurrency | 同一 resource の create/update/delete は resource 単位 lock を取る。lost update と二重作成は禁止 |
| atomic write | tmp file 書き込み、file fsync、rename、parent directory fsync の順を必須とする |
| multi-file commit | 複数 file 更新は prepare、fsync、rename、commit marker の順で行う。途中失敗時は旧状態または明示 rollback 状態だけを起動可能にする |
| startup corruption | 必須 metadata の JSON parse 失敗、schema 違反、unique constraint 違反は起動失敗。空初期値で上書きしない |
| cleanup | temp/backup/orphan file の cleanup は起動成功後に WARN を出して行う。cleanup 失敗は元データを破壊しない限り起動失敗にしない |
| logging | request body、SQL args、JWT、admin/platform/replication/HA token、backup binary、extension 絶対 path は出力禁止 |
| test artifact | snapshot は動的値を正規化し、status/header subset/body を含める。生成場所は Phase 節で固定する |

**状態遷移固定契約：**

resource の状態を持つ Phase 9 以降の機能は、以下の状態名を使う。未定義状態を追加する場合は、先に本表へ追記する。

| Resource | 状態 | 許可遷移 | 禁止 |
|----------|------|----------|------|
| WebSocket session | `new` → `hello_ok` → `closing` → `closed` | `hello_ok` 後だけ request 処理可 | `new` で SQL 実行、`closed` から復帰 |
| WebSocket stream | `open` → `tx_active` → `open` → `closed` | BEGIN で `tx_active`、COMMIT/ROLLBACK で `open` | close 後 execute、接続 close 後 tx 放置 |
| replication frame | `generated` → `served` → `acked` → `archived` | frame_no は単調増加 | checksum 未検証 ack、番号巻き戻り |
| restore job | `prepared` → `verifying` → `committed` または `rolled_back` | committed 前は元 DB を保持 | 元 DB 直接上書き、rollback 不能 |
| branch | `creating` → `active` → `deleting` → `deleted` | active だけ pipeline 接続可 | metadata 先行 active、削除済み接続 |
| extension | `registered` → `loaded` → `disabled` または `deleted` | sha256 検証後のみ loaded | 未検証 load、SQL から直接 load |
| HA node | `standalone` / `replica` / `candidate` / `primary` | candidate から primary は operator promote 必須 | 自動 primary 昇格、古い term 採用 |
| internal adapter | `disabled` → `shadow` → `active` → `rollback` | shadow で互換 snapshot 通過後だけ active | API/metadata 差分を伴う active |

**ゼロバグ横断テスト：**

各 Phase 実装 PR は既存 TC に加えて、該当する横断テストを実施する。

| ID | 対象 Phase | 検証内容 |
|----|------------|----------|
| ZB-1 | all | unknown/null/empty/duplicate query/body validation が固定契約通り |
| ZB-2 | 2+ | metadata atomic write 失敗注入で旧状態または rollback 状態に戻る |
| ZB-3 | 3+ | error body が `error` と `code` だけで、secret/debug field を含まない |
| ZB-4 | 7+ | 同一 resource の concurrent create/delete/update で unique constraint と metadata 整合性が壊れない |
| ZB-5 | 8+ | Turso snapshot が dynamic placeholder 正規化後に一致する |
| ZB-6 | 9+ | connection/session/job の途中切断で未完了 write が成功扱いにならない |
| ZB-7 | 11+ | frame/checksum/manifest の不整合を検出し、silent success しない |
| ZB-8 | 14+ | restore/branch/adapter rollback が再起動後も整合する |

#### 9.1.3 実装 PR 証跡フォーマット

各 Phase の実装 PR は、PR description または同梱された test artifact に以下を残す。記録がない項目は未実施として扱う。

| 項目 | 必須内容 |
|------|----------|
| Phase | 対象 Phase、影響する前後 Phase、対象外として確認した機能 |
| Contract changed | 追加/変更した endpoint、metadata、config、error code、permission、migration |
| Compatibility | Turso Cloud / libSQL SDK / legacy metadata への影響と確認結果 |
| Test commands | 実行した command、終了コード、失敗時の修正内容 |
| Snapshot artifacts | API snapshot、Turso Platform snapshot、Prometheus snapshot、WebSocket transcript など該当 Phase の artifact |
| Persistence evidence | atomic write、rollback、restart recovery、corruption handling の検証結果 |
| Security evidence | secret redaction、auth failure、scope denial、quota denial の検証結果 |
| Regression evidence | 当該 Phase より前の TC が通ったこと |

証跡は「何を実装したか」ではなく「仕様のどの契約を満たしたか」で記述する。仕様書にない判断を PR description で補って完了扱いにしてはならない。

#### 9.1.4 Phase 完了不可条件

以下に該当する PR は、該当機能が動作して見える場合でも Phase 完了として扱わない。修正は同一 PR 内で完了させ、後続の「バグ修正」へ分離してはならない。

| 完了不可条件 | 判定 |
|--------------|------|
| 対象 Phase の endpoint、metadata、config、error、permission、test artifact のいずれかが仕様未定義 | 実装 PR ではなく仕様修正 PR とする |
| success response は返るが異常系、権限系、quota/scope 系、破損系、再起動系のいずれかが未検証 | Phase 未完了 |
| TODO/FIXME/stub/unimplemented/panic/unwrap により production path が残る | Phase 未完了 |
| `INTERNAL_ERROR`、`INVALID_REQUEST`、`PERMISSION_DENIED` を仕様未定義エラーの代用にしている | Phase 未完了 |
| 仕様上の `対象外` 機能に成功応答を返している | Phase 未完了 |
| metadata migration が旧形式、新形式、破損形式、rollback 形式のいずれかを検証していない | Phase 未完了 |
| destructive operation に rollback、restart recovery、concurrency test がない | Phase 未完了 |
| Turso Cloud 互換対象 endpoint の snapshot がない、または snapshot 差分理由が仕様本文にない | Phase 未完了 |
| TypeScript SDK / WebSocket / replication / backup / HA / internal adapter の該当 regression が未実行 | Phase 未完了 |
| log redaction の確認なしに token、JWT、SQL args、backup body、extension path を扱う | Phase 未完了 |
| flaky test、手動確認、目視確認、想定結果だけを完了根拠にしている | Phase 未完了 |

「軽微」「一時的」「後で直す」「既知課題」「仕様上問題ないはず」という記述は完了根拠として無効である。Phase 完了は、仕様本文、実装、テスト、証跡が揃った場合だけ認める。

#### 9.1.5 実装差分リスク分類

実装 PR は、変更内容を下表のリスク分類に必ず割り当てる。複数に該当する場合はすべての必須証跡を満たす。分類できない変更は仕様未定義として扱い、先に本表へ分類を追加する。

| リスク分類 | 該当例 | 必須証跡 |
|------------|--------|----------|
| API contract | endpoint 追加、status/body/header/error 変更、query/path/body validation 変更 | endpoint snapshot、正常系/異常系/unknown/null/duplicate query test |
| Auth/permission | JWT claim、admin/platform token、DB scope、org/group scope、quota gate、ro/rw 判定 | auth matrix、scope denial、secret redaction、対象外 resource rejection |
| Metadata/migration | metadata field 追加、schema 変更、legacy migration、unique constraint 変更 | old/new/corrupt metadata fixture、migration log、rollback/restart recovery |
| Persistence/rollback | DB 作成削除、restore、branch、WAL archive、extension binary、HA state 更新 | atomic write failure injection、fsync/rename evidence、rollback evidence、concurrency test |
| Compatibility | Turso Platform API、hrana schema、SDK 挙動、legacy DB 名、response wrapper 変更 | Turso snapshot、TypeScript SDK regression、legacy fixture、差分理由 |
| Replication/HA | WAL frame、snapshot、redirect、leader election、promotion/demotion、term 更新 | frame/checksum transcript、lag/health snapshot、split-brain rejection、restart recovery |
| Observability | log field、metric、Prometheus output、request id、audit 相当 record | log redaction sample、metric snapshot、secret 非含有確認 |
| Internal adapter | WAL/storage/executor adapter、shadow/active/rollback flag、performance baseline | Phase 1〜18 regression、adapter diff snapshot、rollback flag test、baseline comparison |

PR は「変更なし」として分類を省略してはならない。仕様書のみの PR であっても、影響する分類と後続実装で必要になる証跡を明記する。

#### 9.1.6 Phase 別証跡チェックリスト

各 Phase 実装 PR は、§9.1.3 の共通証跡に加えて下表の証跡を残す。該当 Phase の証跡が欠ける場合は Phase 未完了とする。

| Phase | 必須証跡 |
|-------|----------|
| Phase 1 | CLI help snapshot、invalid flag stderr、workspace build result |
| Phase 2 | data-dir 初期化 tree、二重起動拒否、integrity_check 成功/失敗、metadata 破損起動失敗 |
| Phase 3 | `/v2/health` snapshot、pipeline success/error snapshot、malformed JSON、hrana error body、Web framework 不使用確認 |
| Phase 4 | JWT valid/expired/revoked/bad signature、ro write denial、token secret redaction、tokens.json atomic update |
| Phase 5 | JSON Lines log snapshot、Authorization/SQL args 非出力、TypeScript SDK CRUD、restart persistence |
| Phase 6 | default fallback、`/{db}/v2/pipeline`、DB 名 validation、複数 DB 分離、databases.json recovery |
| Phase 7 | admin auth matrix、DB CRUD、token CRUD、DB scope ro/rw、revoke 即時反映、concurrent create/delete |
| Phase 8 | Turso Platform snapshot、organization/group/location/quota migration、legacy metadata fixture、quota exceeded denial、scope denial |
| Phase 9 | WebSocket hello/subprotocol transcript、stream transaction、store_sql/close_sql、close rollback、multi stream |
| Phase 10 | ATTACH allow/deny、任意 path 拒否、metrics counter/gauge snapshot、HTTP/WS/pipeline 更新点 |
| Phase 11 | replication log/snapshot/heartbeat/status snapshot、frame_no 単調増加、CRC32、replication token denial |
| Phase 12 | replica catch-up、307 redirect、primary down behavior、lag health、checksum mismatch recovery |
| Phase 13 | manifest/frame 双方向整合、retention cleanup、orphan/missing/corrupt frame、restart recovery |
| Phase 14 | backup artifact、restore rollback、PITR 範囲外、CRC 破壊、restore lock、元 DB 保持 |
| Phase 15 | branch create/list/delete、timestamp/frame branch、独立書き込み、source delete denial、restart recovery |
| Phase 16 | extension manifest、sha256/署名検証、allowlist、任意 path 拒否、load/unload/restart |
| Phase 17 | metrics snapshot 永続化、Prometheus text snapshot、quota usage 整合、破損 snapshot recovery、secret 非含有 |
| Phase 18 | leader election、promotion/demotion、network partition、split-brain rejection、term 単調増加、redirect/health |
| Phase 19 | adapter shadow/active/rollback、Phase 1〜18 regression、SDK 互換、WAL consistency、performance baseline |

証跡は repository 内の test fixture、snapshot、CI log、または PR description の実行結果として追跡可能でなければならない。ローカルで確認しただけの説明は証跡として扱わない。

#### 9.1.7 契約トレーサビリティ固定契約

実装 PR は、変更した仕様契約、対応するテスト、保存された証跡を 1 対 1 以上で追跡できなければならない。追跡不能な契約は未検証として扱い、Phase 完了不可とする。

| 契約種別 | ID 形式 | 例 | 必須対応 |
|----------|---------|----|----------|
| API 契約 | `API-P{phase}-{kebab-name}` | `API-P8-turso-database-create` | endpoint test、snapshot、error test |
| 永続化契約 | `PERSIST-P{phase}-{kebab-name}` | `PERSIST-P14-restore-rollback` | atomic write test、restart/rollback fixture |
| エラー契約 | `ERR-{code}` | `ERR-QUOTA_EXCEEDED` | 発火 test、HTTP status/body snapshot |
| 設定契約 | `CFG-P{phase}-{kebab-name}` | `CFG-P11-replication-write-mode` | valid/invalid/default/priority test |
| セキュリティ境界 | `SEC-{boundary}` | `SEC-admin-api` | auth denial、secret redaction、scope denial |
| 互換契約 | `COMPAT-P{phase}-{kebab-name}` | `COMPAT-P8-turso-wrapper` | SDK regression または Turso snapshot |
| 横断ゼロバグ | `ZB-{number}` | `ZB-4` | 該当 Phase の横断 test |

契約 ID は PR description、test 名、snapshot/fixture path のいずれかに含める。完全一致が難しい場合は PR description の traceability table で対応を明示する。

**PR 完了時の追跡表フォーマット：**

| Contract ID | 実装対象 | Test ID / command | Evidence path | Regression | N/A 理由 |
|-------------|----------|-------------------|---------------|------------|----------|
| `API-Px-name` | endpoint / config / metadata / auth | `TC-*` / `ZB-*` / command | snapshot / fixture / log | Phase 1〜x | 該当なしの場合だけ理由を書く |

追跡表の各行は、少なくとも `Contract ID`、`Test ID / command`、`Evidence path` を持つ。`N/A` は、その契約種別が変更対象外である場合だけ許可する。失敗した test、未生成 artifact、手元確認だけの項目を `N/A` にしてはならない。

**テスト種別の最低要件：**

| 契約種別 | 単体テストのみ | snapshot のみ | 手動確認のみ | 必須最低ライン |
|----------|----------------|---------------|--------------|----------------|
| API 契約 | 不可 | 不可 | 不可 | integration test + response snapshot |
| 永続化契約 | 不可 | 不可 | 不可 | failure injection または fixture + restart test |
| エラー契約 | 不可 | 可。ただし発火 test とセット | 不可 | 発火 test + status/body assertion |
| 設定契約 | 不可 | 不可 | 不可 | valid/invalid/default/priority test |
| セキュリティ境界 | 不可 | 不可 | 不可 | auth/scope denial + redaction test |
| Turso 互換 | 不可 | 可。ただし strict compare 必須 | 不可 | Turso snapshot + regression |
| 破壊的操作 | 不可 | 不可 | 不可 | rollback/restart/concurrency test |
| 内部 adapter | 不可 | 不可 | 不可 | shadow/rollback/regression/performance artifact |

手動確認は、自動化できない外部環境依存の補助証跡としてのみ許可する。認証、永続化、破壊的操作、Turso 互換、secret redaction、rollback、concurrency は手動確認だけで完了扱いにしてはならない。

#### 9.1.8 契約カバレッジ固定契約

§9.5 API endpoint 契約表、§9.6 永続化ファイル契約表、§9.7 エラーコード使用契約表、§9.13 設定値契約表、§9.14 セキュリティ境界表は契約 ID の発生源である。実装 PR は、対象 Phase の該当行すべてに契約 ID を割り当て、テストと証跡でカバーしなければならない。

| Source section | Contract ID | Contract summary | Required tests | Required evidence | Owner phase |
|----------------|-------------|------------------|----------------|-------------------|-------------|
| `§9.5` | `API-P{phase}-{kebab-name}` | method/path/auth/status/body/error/persistence/idempotency | normal / invalid / auth / error / idempotency | response snapshot, request fixture, log redaction sample | endpoint の Phase |
| `§9.6` | `PERSIST-P{phase}-{kebab-name}` | file path/schema/update/fsync/corruption/backup | atomic write / corruption / restart / rollback | metadata fixture, crash fixture, recovery log | first write Phase |
| `§9.7` | `ERR-{code}` | code/status/retry/client action |発火 test / status-body assertion / retry decision | error snapshot, triggering fixture | first use Phase |
| `§9.13` | `CFG-P{phase}-{kebab-name}` | CLI/env/TOML/default/invalid handling | default / override priority / invalid / target Phase before-after | config fixture, stderr/log sample | config Phase |
| `§9.14` | `SEC-{boundary}` | untrusted input / required control / forbidden behavior | allow / deny / redaction / bypass attempt | auth matrix, redaction log, denied response | first exposed Phase |

既存表の行に明示 ID が書かれていない場合でも、実装 PR では上表の形式で ID を割り当てる。割り当てた ID は PR description だけでなく、該当する test 名、snapshot path、fixture path、または仕様本文のいずれかに残す。

**契約 ID 欠落時の扱い：**

| 欠落状態 | 判定 |
|----------|------|
| 対象 Phase の §9.5 endpoint に API 契約 ID がない | Phase 未完了 |
| 更新する §9.6 persistence 行に PERSIST 契約 ID がない | Phase 未完了 |
| 発火する §9.7 error code に ERR 契約 ID の test がない | Phase 未完了 |
| 追加/変更する §9.13 config に CFG 契約 ID がない | Phase 未完了 |
| 触れる §9.14 security boundary に SEC 契約 ID の deny/redaction test がない | Phase 未完了 |
| PR description の一時 ID だけで、仕様本文・test・artifact のどこにも残らない | Phase 未完了 |

Phase 完了時は、対象 Phase の API / persistence / error / config / security / compatibility 契約に未カバー行が 0 件でなければならない。`N/A` は、その契約が対象外である理由が §9.2、§9.4、該当 Phase 節、または unsupported 固定表に明記されている場合だけ許可する。

#### 9.1.9 Phase 実装 PR ライフサイクル固定契約

Phase 実装 PR は下表の順序で進める。順序を飛ばした PR は Phase 完了として扱わない。

| Step | Gate | 必須状態 | 失敗時 |
|------|------|----------|--------|
| 1 | Ready | §9.11 の Definition of Ready と §9.17 の実装前チェックリストを満たす | 実装開始禁止。仕様修正 PR に戻す |
| 2 | Contract mapping | §9.1.7 / §9.1.8 の Contract ID、test ID、evidence path を先に割り当てる | 実装開始禁止 |
| 3 | Implementation | 対象 Phase のみ実装し、対象外機能に成功応答を返さない | Phase 未完了 |
| 4 | Evidence generation | snapshot、fixture、log、CI output を生成し secret scan を通す | Phase 未完了 |
| 5 | Regression | 対象 Phase 以前の regression と該当 SDK/Turso/replication/HA/internal tests を通す | Phase 未完了 |
| 6 | Review | §9.12、§9.1.7、§9.1.8 の traceability と coverage を確認する | Phase 未完了 |
| 7 | Merge | 未カバー契約、未検証、既知不具合、未生成証跡が 0 件 | merge 不可 |

**順序違反時の扱い：**

| 違反 | 判定 |
|------|------|
| 実装後に Contract ID を後付けし、test/evidence へ反映していない | Phase 未完了 |
| Contract ID はあるが evidence path が存在しない | Phase 未完了 |
| regression 未完了のまま review/merge へ進む | merge 不可 |
| 仕様未確定のまま実装を開始した | 実装 PR ではなく仕様修正 PR として扱う |
| snapshot 差分を実装都合だけで更新した | review failure |
| `N/A` 理由が仕様本文に存在しない | review failure |

**レビュー自動判定チェック：**

PR review / CI / release-check は最低限、以下を機械的に確認する。自動化されていない場合は Phase 完了不可であり、手動確認だけで代替してはならない。

| Check | 必須判定 |
|-------|----------|
| traceability table | PR description に Contract ID / Test ID / Evidence path が存在する |
| coverage | 対象 Phase の契約 ID に未カバーが 0 件 |
| N/A validation | `N/A` の理由が仕様本文の対象外・unsupported・該当なしに対応している |
| artifact existence | snapshot / fixture / log / CI output の path が存在する |
| secret scan | artifact に JWT、Bearer token、admin/platform/replication/HA token、生 SQL args、backup body が含まれない |
| regression result | 対象 Phase 以前の regression が全件成功している |
| zero-bug gate | 既知不具合、未検証、未生成証跡、TODO/FIXME production path が 0 件 |

Phase 完了 PR は、上記 check がすべて成功しなければならない。1 件でも失敗した場合は、その PR 内で修正し、後続のバグ修正 PR に持ち越さない。

#### 9.1.10 Phase 受入 manifest 固定契約

各 Phase 実装 PR は、実装開始前に Phase 受入 manifest を作成し、PR description または `docs/phase-evidence/phase-{phase}.md` に固定する。manifest は「この Phase が何を実装し、何を実装しないか」「どの契約をどの test / artifact で証明するか」を 1 箇所で読める形にする。

manifest がない Phase 実装 PR は、§9.1.9 の Step 2 `Contract mapping` 未完了として扱う。実装者がコードを読まないと完了条件を判断できる状態は不可とする。

**必須 manifest fields：**

| Field | 必須内容 | 欠落時 |
|-------|----------|--------|
| `Phase` | 対象 Phase 番号と Phase 名 | 実装開始禁止 |
| `Scope in` | この PR で成功応答まで実装する機能一覧 | 実装開始禁止 |
| `Scope out` | 未来 Phase、unsupported、明示対象外の機能一覧と根拠 section | 実装開始禁止 |
| `Contract map` | §9.1.7 / §9.1.8 の Contract ID、対象仕様 section、test ID、evidence path | Phase 未完了 |
| `Endpoint map` | method、path、auth、status、body、error code、idempotency | route 公開禁止 |
| `Persistence map` | file path、schema version、write timing、fsync、corruption、rollback | 書き込み処理実装禁止 |
| `Config map` | CLI/env/TOML/default/invalid/priority/対象 Phase 前挙動 | config 実装禁止 |
| `Security map` | auth boundary、scope、secret redaction、denial case、bypass attempt | success response 公開禁止 |
| `Compatibility map` | Turso/libSQL SDK 互換確認、差分理由、snapshot source | 互換完了不可 |
| `Regression set` | 対象 Phase 以前の regression command と期待結果 | merge 不可 |
| `Manual exception` | 自動化できない確認の理由、owner、期限、代替自動化 Phase | 手動確認のみ不可 |
| `Zero-bug declaration` | 既知不具合、未検証、未生成証跡、TODO/FIXME production path が 0 件である宣言 | merge 不可 |

**manifest の固定フォーマット：**

```markdown
## Phase {phase} Acceptance Manifest

| Field | Value |
|-------|-------|
| Phase | P{phase}: {name} |
| Scope in | ... |
| Scope out | ... |
| Regression set | ... |
| Manual exception | none / ... |
| Zero-bug declaration | known bugs: 0 / unverified: 0 / missing evidence: 0 / production TODO-FIXME: 0 |

| Contract ID | Source section | Implementation target | Test ID / command | Evidence path | Status |
|-------------|----------------|-----------------------|-------------------|---------------|--------|
| API-Px-name | §9.5 / §6.x | endpoint / behavior | TC-* | artifacts/... | planned/pass |
```

`Status` は実装開始前は `planned`、PR 完了時は `pass` または仕様本文に根拠がある `N/A` のみ許可する。`todo`、`later`、`manual only`、`unknown`、空欄は Phase 完了不可である。

**manifest 更新ルール：**

| 変更 | 必須対応 |
|------|----------|
| 実装中に endpoint / schema / error / config / security 境界が変わった | 先に manifest と仕様本文を更新し、Contract ID と test/evidence を再割当する |
| 実装中に対象外機能が必要になった | `Scope out` から `Scope in` へ移す前に仕様変更 PR を先行する |
| artifact path が変わった | manifest、PR description、test output を同時に更新する |
| 手動確認が増えた | `Manual exception` に理由、owner、期限、代替自動化 Phase を追加する |
| regression を削った | 削除理由と代替 test を仕様本文に明記するまで merge 不可 |

Phase 完了 review では、manifest、仕様本文、test 名、artifact path、PR description の 5 点が一致していなければならない。1 つでも不一致がある場合は、実装の正しさではなく受入条件の未確定として扱い、Phase 未完了に戻す。

#### 9.1.11 Evidence artifact 固定契約

各 Phase 実装 PR は、§9.1.10 の Phase 受入 manifest に記載した `Evidence path` に、実際の artifact を生成しなければならない。artifact は「実装が仕様を満たしたこと」を後から再確認できる証跡であり、PR description の文章や手元ログだけでは代替できない。

artifact path は deterministic に固定し、実装者、実行環境、実行時刻によって変わってはならない。timestamp、random ID、host 名、absolute path、local username を path に含めることは禁止する。

**artifact 保存規則：**

| 種別 | 保存先 | 必須命名 | 必須内容 |
|------|--------|----------|----------|
| request fixture | `tests/fixtures/phase-{phase}/{contract-id}/request.json` | Contract ID を directory に含める | request method/path/header subset/body |
| response snapshot | `tests/snapshots/phase-{phase}/{contract-id}/response.json` | Contract ID を directory に含める | status、header subset、body、error code |
| persistence fixture | `tests/fixtures/phase-{phase}/{contract-id}/persistence.json` | Contract ID を directory に含める | file path、schema version、before/after、fsync/rollback 結果 |
| recovery / rollback log | `tests/artifacts/phase-{phase}/{contract-id}/recovery.log` | Contract ID を directory に含める | 起動、破損検出、復旧、rollback の要点 |
| compatibility diff | `tests/artifacts/phase-{phase}/{contract-id}/compat.diff` | Contract ID を directory に含める | Turso/libSQL SDK との差分、正規化後比較結果 |
| CI output | `tests/artifacts/phase-{phase}/{contract-id}/ci.txt` | Contract ID を directory に含める | 実行 command、exit code、pass/fail summary |
| secret scan result | `tests/artifacts/phase-{phase}/{contract-id}/secret-scan.txt` | Contract ID を directory に含める | scan command、対象 path、検出 0 件の結果 |
| performance baseline | `tests/artifacts/phase-{phase}/{contract-id}/performance.json` | Contract ID を directory に含める | p95、RSS、DB size、WAL size、測定条件 |

`contract-id` は小文字化せず、§9.1.7 / §9.1.8 で割り当てた Contract ID と完全一致させる。ファイルシステム都合で大文字小文字が不安定になる環境を考慮し、同一 directory 内に大文字小文字だけが異なる Contract ID を作ってはならない。

**正規化必須 field：**

| Field | 正規化値 | 理由 |
|-------|----------|------|
| timestamp / datetime | `<normalized-time>` | 実行時刻差分を禁止 |
| request id / trace id | `<normalized-id>` | 実行ごとの差分を禁止 |
| absolute path | `<normalized-path>` | local 環境差分と user name 混入を禁止 |
| host / port | `<normalized-host>` / `<normalized-port>` | CI と local 差分を禁止 |
| JWT / Bearer token / secret | `<redacted-secret>` | secret 混入禁止 |
| SQL bind value が機微情報の場合 | `<redacted-arg>` | user data 混入禁止 |
| backup body / replication payload の機微 field | `<redacted-payload>` | 大容量 secret / data 混入禁止 |

正規化前の raw artifact は repository にコミットしてはならない。raw artifact が必要な検証は CI 内一時領域だけに保存し、repository に残す artifact は正規化後のみとする。

**artifact 欠落・不一致時の扱い：**

| 状態 | 判定 |
|------|------|
| manifest の `Evidence path` に実ファイルが存在しない | Phase 未完了 |
| artifact path に Contract ID が含まれない | Phase 未完了 |
| artifact 内容が該当 Contract ID と無関係 | review failure |
| artifact に secret / token / raw SQL args / local username が含まれる | merge 不可 |
| snapshot が実装都合で更新され、仕様差分理由がない | review failure |
| artifact が timestamp / random ID を含む path に保存される | Phase 未完了 |
| PR description だけに証跡があり repository に artifact がない | Phase 未完了 |
| 手動確認ログのみで自動 test / snapshot がない | Phase 未完了。ただし §9.1.10 の Manual exception が仕様本文に根拠付きである場合のみ補助証跡として許可 |

artifact は Phase 完了 PR と同じ commit に含める。後続 PR で artifact だけを追加して Phase 完了扱いにすることは禁止する。artifact を更新する場合は、対応する仕様 section、Contract ID、test ID、manifest、PR description を同時に更新しなければならない。

#### 9.1.12 仕様矛盾解消固定契約

仕様書内の複数 section が同じ対象について異なる挙動、status、error code、永続化 schema、設定優先順位、認証境界、Phase 境界、証跡要件を示す場合、その状態を仕様矛盾とみなす。仕様矛盾がある対象は、実装者の判断で実装してはならない。

**仕様優先順位：**

| 優先順位 | Source | 適用範囲 |
|----------|--------|----------|
| 1 | §1.4 設計不変条件 | 全 Phase、全機能の最上位制約 |
| 2 | §9.1〜§9.17 の実装固定契約 | Phase 完了条件、契約 ID、証跡、ゼロバグ判定 |
| 3 | §9.2 Phase 別完了ゲート / §9.4 unsupported 固定表 | Phase 境界、対象外、前倒し可否 |
| 4 | §9.5 API endpoint 契約表 / §9.6 永続化ファイル契約表 / §9.7 エラーコード使用契約表 / §9.13 設定値契約表 / §9.14 セキュリティ境界表 | 実装対象の具体契約 |
| 5 | 各 Phase 詳細節 | Phase 内の補足仕様 |
| 6 | Rust コード例、JSON 例、CLI 例、説明文中の例 | 実装参考。上位契約と矛盾する場合は上位契約を正とする |

上位 Source と下位 Source が矛盾する場合は、上位 Source を一時的な正とする。ただし、下位 Source を放置したまま実装 PR を進めてはならない。矛盾を発見した PR は、実装前に仕様修正 PR として矛盾箇所を解消する。

**矛盾の種類と必須対応：**

| 矛盾 | 必須対応 |
|------|----------|
| Phase 境界と詳細節が異なる | §9.2 / §9.4 と詳細節を同じ PR で修正する |
| API status/body と error code 表が異なる | §9.5、§9.7、該当 API 節、snapshot 期待値を同時更新する |
| persistence schema と Phase 詳細が異なる | §9.6、該当 Phase 節、migration / rollback 契約を同時更新する |
| config default / priority が複数箇所で異なる | §9.13、CLI/env/TOML 例、manifest の Config map を同時更新する |
| security boundary と API 説明が異なる | §9.14、該当 API 節、deny/redaction test を同時更新する |
| Turso Cloud 互換方針と自己ホスト差分が異なる | §1.4、§9.4、該当 Phase 節、compatibility diff を同時更新する |
| コード例 / JSON 例 / CLI 例だけが本文と異なる | 例を本文に合わせる。本文を変える場合は上位契約も更新する |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 矛盾を PR description だけで説明し、仕様本文を直さない | Phase 未完了 |
| 上位 Source だけを直し、下位 Source の矛盾を残す | review failure |
| test 期待値だけを直し、仕様本文を直さない | review failure |
| manifest だけを直し、Contract ID / artifact path / 仕様 section を直さない | Phase 未完了 |
| 「実装上はこちらを採用」として仕様矛盾を残す | merge 不可 |
| 未来 Phase で直す前提で現在 Phase の成功応答を公開する | merge 不可 |

矛盾解消後は、該当 Contract ID、manifest、test name、artifact path、PR description の追跡表が同じ挙動を指していなければならない。1 つでも古い仕様を参照している場合は、矛盾未解消として扱う。

#### 9.1.13 Verification command 固定契約

各 Phase 実装 PR は、完了判定に使う verification command を実装開始前に Phase 受入 manifest の `Regression set` と PR description に固定する。固定されていない command の成功は完了根拠として扱わない。

verification command は deterministic に実行できなければならない。実行者のローカル環境、手順記憶、IDE、手動クリック、外部サービスの一時状態に依存する確認は、§9.1.10 の `Manual exception` に根拠がない限り完了根拠として使えない。

**必須 command 分類：**

| 分類 | 必須 command | 適用 Phase | 必須 artifact |
|------|--------------|------------|---------------|
| format / lint | repository 標準の format / lint command。存在しない場合は manifest に `N/A` 理由を明記 | all | `tests/artifacts/phase-{phase}/{contract-id}/ci.txt` |
| unit | 対象 crate/module の unit test | all implementation phases | `ci.txt` |
| integration | HTTP / WebSocket / CLI / persistence の integration test | Phase 3 以降、または外部 API を持つ Phase | `ci.txt`、request fixture、response snapshot |
| SDK compatibility | libSQL client SDK 互換 test | hrana / client API を変更する Phase | SDK transcript、compat diff |
| Turso compatibility | Turso Cloud snapshot / behavior compare | Turso Cloud 互換 API / metadata / auth / quota / branch / platform API を変更する Phase | `compat.diff` |
| persistence / restart / rollback | fsync、restart、corruption、rollback、migration test | 永続化 schema / metadata / backup / branch / HA を変更する Phase | persistence fixture、recovery log |
| security / secret scan | auth denial、scope denial、redaction、artifact secret scan | 認証、認可、secret、artifact を扱う Phase | secret-scan result、denied response |
| release-check | release-check script または同等の repository 標準 release validation | release / packaging / CI 完了判定を行う Phase | `ci.txt` |

該当する分類が存在するのに command が未定義の場合、その Phase は未完了である。`N/A` は、対象機能が §9.2、§9.4、該当 Phase 節、または unsupported 固定表で明示対象外の場合だけ許可する。

**実行順序：**

| Order | Gate | 失敗時 |
|-------|------|--------|
| 1 | format / lint | 実装修正。後続 gate に進まない |
| 2 | unit | 実装修正。integration に進まない |
| 3 | integration | 実装または仕様矛盾を修正する |
| 4 | SDK / Turso compatibility | 互換差分を仕様化するか実装修正する |
| 5 | persistence / restart / rollback | 永続化・migration・rollback 契約を修正する |
| 6 | security / secret scan | secret 混入を除去し、deny/redaction test を追加する |
| 7 | release-check | release / packaging / CI 契約を修正する |

上記順序を飛ばして後段 command だけを成功させても Phase 完了扱いにしない。前段 command の失敗を `known issue`、`flaky`、`後で確認` として残すことは禁止する。

**exit code と証跡：**

| 状態 | 判定 |
|------|------|
| command の exit code が `0` | 完了根拠として利用可 |
| command の exit code が非 `0` | Phase 未完了 |
| command 未実行 | Phase 未完了 |
| command 名、引数、対象 path が artifact に残っていない | Phase 未完了 |
| CI output artifact と manifest の `Regression set` が一致しない | review failure |
| ローカル実行の口頭説明だけで artifact がない | Phase 未完了 |
| flaky test を再実行で通したが失敗ログを残していない | review failure |
| `--ignored`、`--skip`、filter で対象 test を外した | Phase 未完了。ただし除外理由が仕様本文にある場合のみ `N/A` 可 |

verification command の追加・削除・引数変更は、manifest、PR description、CI output artifact、該当 Contract ID の追跡表を同時に更新しなければならない。検証 command を変更しただけで仕様本文を更新しない PR は Phase 完了不可である。

#### 9.1.14 Failure closure 固定契約

各 Phase 実装 PR は、検証中に発見した失敗、flaky、未検証、artifact 欠落、secret 混入、仕様矛盾、TODO/FIXME production path を同一 PR 内で解消しなければならない。後続 PR、別 issue、運用メモ、口頭説明へ持ち越した時点で、その Phase は未完了である。

failure closure は「失敗を隠す」ことではなく、失敗の原因、修正、再検証、証跡を同じ追跡単位で閉じることを意味する。修正後に pass した結果だけでなく、発見した失敗の分類と再発防止の evidence を残す。

**closure 必須対象：**

| 対象 | 必須 closure | 未完了判定 |
|------|--------------|------------|
| verification command failure | 原因、修正 commit、再実行 command、exit code 0 の artifact | 非 0 exit code、未再実行、artifact 欠落 |
| flaky test | flaky 原因、deterministic 化修正、失敗ログ、再実行 pass artifact | 再実行成功だけ、失敗ログなし、原因未特定 |
| unverified contract | Contract ID、追加 test、evidence path、manifest 更新 | `N/A` で隠す、手動確認のみ |
| missing artifact | 正規化済み artifact 生成、manifest / PR description path 更新 | PR description の説明のみ |
| secret leakage | 漏洩 artifact 削除、redaction 修正、secret scan 再実行 | secret を含む artifact が残る |
| spec conflict | §9.1.12 に従う同時仕様修正、test / artifact 更新 | 上位仕様だけ修正、下位矛盾放置 |
| TODO/FIXME/stub/unimplemented/panic/unwrap production path | production path から除去、または対象 Phase 前 stub 契約へ移動 | 完了 PR に残存 |
| regression failure | 失敗原因、修正、対象 Phase 以前の regression 再実行 | regression 範囲縮小、skip、後続対応 |

**禁止語句と扱い：**

| PR / manifest / artifact に残る語句 | 判定 |
|-------------------------------------|------|
| `known issue` / `known bug` | merge 不可 |
| `later` / `follow-up` / `next PR` | Phase 未完了 |
| `temporary` / `workaround` | 仕様本文に期限・owner・解消 Phase がない限り Phase 未完了 |
| `flaky but passed` | merge 不可 |
| `manual checked` | §9.1.10 の Manual exception がない限り Phase 未完了 |
| `TODO` / `FIXME` / `unimplemented` / `panic!` / unchecked `unwrap` in production path | Phase 未完了 |
| `後で修正` / `後で検証` / `一旦 merge` | merge 不可 |

**closure artifact 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `Failure ID` | `FAIL-P{phase}-{number}` の形式 |
| `Related Contract ID` | 失敗に対応する Contract ID。横断失敗は `ZB-*` も併記 |
| `Failure class` | command failure / flaky / unverified / artifact / secret / spec conflict / production TODO / regression |
| `Root cause` | 実装、仕様、test、fixture、環境差分のいずれか |
| `Fix summary` | 同一 PR 内での修正内容 |
| `Re-run command` | §9.1.13 の command と完全一致 |
| `Evidence path` | pass artifact、失敗ログ、修正後 snapshot / fixture |
| `Closure status` | `closed` のみ許可。`open`、`deferred`、`accepted risk` は Phase 未完了 |

failure closure の追跡表は PR description または `docs/phase-evidence/phase-{phase}.md` に残す。closure artifact が存在しない失敗は、修正済みであっても Phase 完了根拠として扱わない。失敗を見つけた後に仕様変更で対象外へ移す場合も、§9.1.12 に従って仕様本文、manifest、Contract ID、test、artifact を同時更新しなければならない。

#### 9.1.15 Backward compatibility / migration 固定契約

既存 endpoint、wire schema、metadata schema、config default、error code、JWT claim、SDK 互換挙動、永続化 file path を変更する Phase 実装 PR は、実装開始前に後方互換と migration の契約を固定しなければならない。既存データ、既存 config、既存 client request が存在する状態で起動・接続・操作できることを完了条件に含める。

後方互換は「新規環境で動く」ことではない。旧仕様で生成された metadata / config / token / DB path / request fixture を読み、必要な migration を行い、失敗時に安全に rollback または起動失敗できることを意味する。

**互換影響の分類：**

| 変更対象 | 必須互換契約 | 必須 evidence |
|----------|--------------|---------------|
| API endpoint | 旧 method/path/query/body/header の扱い、deprecated field、unknown field 方針、status/error mapping | old/new request fixture、response snapshot、SDK regression |
| hrana wire schema | 旧 SDK request、unknown field、baton/stream/args 互換、error response 互換 | libSQL SDK transcript、wire snapshot |
| metadata schema | schema version、追加 field default、旧形式 migration、破損時挙動、rollback | old/new/corrupt fixture、migration log、restart test |
| config default / priority | 旧 config 読み込み、default 変更理由、CLI/env/TOML 優先順位、invalid value | old/new config fixture、stderr/log snapshot |
| error code / status | 旧 client が期待する status/code、retry 可否、Turso Cloud 差分 | error snapshot、client action matrix |
| JWT / token claims | 旧 token の扱い、claim 追加時 default、scope 解決順、revoke 互換 | old/new token fixture、auth denial test |
| persistence path | 旧 directory/file 名、移動手順、fsync、rollback、partial migration 検出 | path fixture、failure injection、recovery log |
| SDK / Turso compatibility | 旧 SDK version、Turso snapshot 差分、自己ホスト差分理由 | SDK transcript、compat diff |

**migration plan 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `Migration ID` | `MIG-P{phase}-{name}` の形式 |
| `Source schema` | 旧 schema version、旧 file path、旧 field 一覧 |
| `Target schema` | 新 schema version、新 field、default、必須/任意 |
| `Trigger` | 起動時、API 実行時、admin command 実行時のいずれで migration するか |
| `Atomicity` | tmp write、fsync、rename、commit marker、multi-file commit 順序 |
| `Rollback` | commit 前失敗、commit 後失敗、再起動後検出時の扱い |
| `Idempotency` | 再実行時に二重変換・二重削除・二重課金を起こさない条件 |
| `Compatibility window` | 旧形式を読み続ける Phase、または削除禁止理由 |
| `Evidence` | old/new/corrupt fixture、restart test、rollback log、CI output |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 旧 metadata / config / token を手動修正前提にする | Phase 未完了 |
| 新規環境だけで test し、旧形式 fixture がない | Phase 未完了 |
| migration 失敗時に部分更新済み状態で起動成功する | merge 不可 |
| rollback plan なしで destructive operation を行う | merge 不可 |
| 旧 endpoint / field を仕様本文なしに削除する | review failure |
| error status / code を変更し、旧 client action を定義しない | Phase 未完了 |
| Turso Cloud 互換差分を自己ホスト都合だけで隠す | merge 不可 |
| migration artifact が Contract ID / Migration ID と紐づかない | Phase 未完了 |

破壊的変更が必要な場合は、同じ PR で §9.2、§9.5、§9.6、§9.7、§9.13、§9.14、該当 Phase 詳細、manifest、test、artifact を更新し、既存利用者への互換維持策または段階的移行策を明記する。互換維持策がない破壊的変更は、実装都合があっても Phase 完了として扱わない。

#### 9.1.16 Concurrency / idempotency / shutdown 固定契約

状態変更を伴う Phase 実装 PR は、同時 request、resource lock、idempotency、shutdown 中の扱いを実装開始前に固定しなければならない。並行実行で lost update、二重作成、二重削除、二重 revoke、partial commit、成功応答後 rollback が発生する可能性が残る場合、その Phase は未完了である。

**resource lock 境界：**

| 対象 | lock 単位 | 同時実行時の固定挙動 | 必須 evidence |
|------|-----------|----------------------|---------------|
| DB create / delete / update | DB name + organization/group scope | 同一 DB への create/update/delete は直列化。競合 create は `409 DB_ALREADY_EXISTS`、削除中 update は `404 DB_NOT_FOUND` または `409 STORAGE_BUSY` | concurrency test、metadata before/after fixture |
| token create / revoke | token id + tokens.json | revoke は冪等 204。create は token id 重複禁止。revoke と auth check の順序を固定 | auth race test、tokens fixture |
| quota / usage update | organization/group/DB quota key | usage 計算と write 判定の間で quota 超過を見逃さない。超過時は commit 前に拒否 | quota race test、denied response |
| backup | source DB | read lock または Online Backup API 相当の一貫 snapshot。backup 中 write の可否を Phase 節で固定 | backup consistency artifact |
| restore / PITR | target DB exclusive lock | restore 中 write は `503 STORAGE_BUSY`。read は旧 DB 継続または `503` のどちらかを Phase 節で固定 | restore lock test、rollback log |
| branch create / delete | source DB + branch DB | 同名 branch create は `409 DB_ALREADY_EXISTS`。delete は明記された場合のみ冪等 204 | branch race fixture |
| migration | migration id + metadata set | commit marker 前の中断は rollback または起動失敗。commit marker 後は再実行しない | interrupted migration fixture |
| WebSocket transaction | connection id + stream id | stream ごとに transaction 状態を分離。connection close 時の open tx は rollback | WebSocket transcript |
| metrics / counters | metric key | lost increment 禁止。restart 永続化対象は snapshot write と shutdown write の競合を固定 | counter race artifact |

**HTTP method idempotency：**

| Method | 既定 | 例外を許可する条件 |
|--------|------|--------------------|
| `GET` | 副作用禁止。同一 request の再試行で状態を変更しない | metrics read 時の内部 read counter など、仕様本文に副作用が明記される場合のみ |
| `POST` | 非冪等。重複 request は二重作成または二重実行を防ぐ契約が必要 | endpoint ごとに idempotency key、natural key、または conflict response が仕様化される場合 |
| `PATCH` | resource version または lock により lost update を防ぐ | 全 field が上書きではなく merge semantics として仕様化され、競合 test がある場合 |
| `DELETE` | endpoint 表で冪等 204 と明記された場合だけ冪等。未記載なら存在しない resource は 404 | Turso Cloud 互換 endpoint が冪等削除を要求する場合 |

**shutdown 中の固定挙動：**

| 状態 | 必須挙動 |
|------|----------|
| shutdown signal 受信後の新規 request | listener を閉じる。受信済みで処理未開始の request は `503 STORAGE_BUSY` または connection close のどちらかを Phase 節で固定 |
| commit 前の write request | 成功応答を返してはならない。rollback 可能なら rollback、不能なら recovery marker を残す |
| commit 後・応答前の write request | 再試行時に二重実行しない。idempotency / conflict / read-after-write で結果を確認できること |
| WebSocket open transaction | connection close または shutdown timeout で rollback。commit 完了前の close を成功扱いにしない |
| restore / migration / backup 中 | restore/migration は commit marker と rollback を優先。backup は一貫 snapshot 以外を返さない |
| shutdown timeout 超過 | 途中成功応答を作らない。次回起動時 recovery が完了するまで該当 resource を成功扱いにしない |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 同一 resource の並行 create で二重 metadata が作成される | merge 不可 |
| read-modify-write に lock または version check がない | Phase 未完了 |
| commit 前に success response を返す | merge 不可 |
| shutdown 中の partial write を次回起動で成功扱いにする | merge 不可 |
| retry により token revoke、quota charge、restore、branch create が二重適用される | Phase 未完了 |
| concurrency test が手動確認のみ | Phase 未完了 |
| lock timeout / busy error が §9.7 の error code に紐づかない | review failure |

並行性に関係する仕様変更は、manifest の `Endpoint map`、`Persistence map`、`Regression set`、該当 Contract ID、concurrency artifact を同時更新する。並行性 test がない状態で「単体では動く」ことを Phase 完了根拠にしてはならない。

#### 9.1.17 Observability / audit / redaction 固定契約

各 Phase 実装 PR は、ログ、request id、trace id、監査相当記録、metric、秘匿、運用証跡の扱いを実装開始前に固定しなければならない。状態変更、認証、権限拒否、quota、replication、backup/restore、branch、extension、HA、internal adapter に関係する変更は、観測不能または secret 漏洩の可能性が残る場合、その Phase は未完了である。

**request id / trace id：**

| 項目 | 固定仕様 |
|------|----------|
| request id 生成 | inbound に `x-request-id` がある場合は安全な長さ・文字種に正規化して使用する。ない場合は server が生成する |
| trace id | Phase 内で分散 trace を実装しない場合でも、log / artifact では `request_id` で同一 request を追跡できること |
| response header | public/admin API は `x-request-id` を返す。WebSocket は transcript artifact に connection id と stream id を残す |
| snapshot 正規化 | request id / trace id / connection id は artifact 上で `<normalized-id>` に置換する |
| 禁止 | request id に Authorization、JWT、SQL、path の絶対値、user input body 全体を入れてはならない |

**ログ level 固定表：**

| Level | 発火条件 | 禁止内容 |
|-------|----------|----------|
| `INFO` | 起動、停止、request 完了、admin 操作成功、migration 完了、backup/restore/branch 成功 | token、SQL args、backup body、secret value |
| `WARN` | auth disabled、unsupported/stub、非推奨 config、recoverable corruption、retryable replication lag、Phase 前 config 無効化 | request body 全体、secret、絶対 extension path |
| `ERROR` | 起動失敗、永続化破損、migration/restore rollback 不能、secret scan failure、HA split-brain、unrecoverable adapter error | panic backtrace に secret/raw path/body を含めること |
| `DEBUG` / `TRACE` | 開発補助。production default では無効 | INFO/WARN/ERROR で禁止された値すべて |

**出力禁止 field：**

| 種別 | 禁止対象 | 代替表現 |
|------|----------|----------|
| auth | Authorization header、JWT、Bearer token、admin/platform/replication/HA token | `<redacted-secret>` |
| SQL | SQL bind args、生 SQL args、user data row value | count、type、statement kind |
| backup / restore | backup body、uploaded DB body、replication frame bytes | byte length、checksum、frame_no |
| filesystem | extension 絶対 path、data-dir 絶対 path、local username | logical resource name、`<normalized-path>` |
| config | secret file contents、env secret value | key name、source type、redacted marker |
| error | upstream error message 内の secret / path / SQL args | sanitized error code + redacted message |

**audit 相当記録：**

| 操作 | 必須記録 | Phase 対象外の場合 |
|------|----------|-------------------|
| token create / revoke | actor scope、target token id、result、request id。secret は記録しない | manifest に対象外理由を記載 |
| DB create / delete / configuration update | actor scope、organization/group/db、operation、result | Phase 前 route は 501 stub log のみ |
| quota / usage enforcement | db/org/group、limit、usage、denied/allowed、request id | quota 未実装 Phase は成功応答禁止 |
| backup / restore / PITR | db、operation、size、checksum、commit/rollback result | backup body は記録禁止 |
| replication / HA | role、node id、frame_no、term、leader、result | token/frame bytes は記録禁止 |
| extension load / unload | extension name、version、sha256、state、result | absolute path は記録禁止 |
| internal adapter switch | flag、adapter name、mode、compat result、rollback result | silent fallback 禁止 |

audit 相当記録は、Phase 8 の `/v1/organizations/{org}/audit-logs` API を実装することを意味しない。audit logs API が unsupported の Phase では、内部 log / artifact として記録し、API は該当 unsupported 固定表に従う。

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| log snapshot | INFO/WARN/ERROR の代表 log、request id、status、duration、result |
| redaction sample | token、JWT、SQL args、backup body、extension path が redacted されること |
| secret scan result | log artifact / snapshot / fixture に secret がないこと |
| audit matrix | 対象操作、記録 field、禁止 field、対象外理由 |
| failure log | rollback、migration failure、auth denial、quota denial の sanitized log |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| secret / token / SQL args / backup body / absolute extension path が log または artifact に残る | merge 不可 |
| state-changing operation に request id 付き log / audit 相当記録がない | Phase 未完了 |
| auth denial / scope denial / quota denial の log redaction test がない | Phase 未完了 |
| unsupported/stub log に request body、token、upload binary が出る | merge 不可 |
| DEBUG/TRACE なら secret を出してよい扱いにする | merge 不可 |
| audit logs API 未実装を理由に内部記録も省略する | Phase 未完了 |

observability に関係する仕様変更は、manifest の `Security map`、`Regression set`、該当 SEC 契約 ID、log artifact、secret scan artifact を同時更新する。ログが「見やすい」だけで、秘匿・追跡・失敗調査の契約を満たさない場合は Phase 完了扱いにしない。

#### 9.1.18 Auth / permission / quota precedence 固定契約

各 Phase 実装 PR は、認証、認可、scope、ro/rw、organization/group、quota、usage、block policy、delete protection、replication/HA token の判定順を実装開始前に固定しなければならない。複数の拒否条件が同時に成立する場合でも、endpoint ごとに返す status/code が揺れてはならない。

**共通判定順：**

| Order | Gate | 失敗時の既定 error | 備考 |
|-------|------|--------------------|------|
| 1 | request parsing / size / method / content type | `400 INVALID_REQUEST` または `405 METHOD_NOT_ALLOWED` | 認証不要で公開してよい validation だけを先に行う |
| 2 | required auth presence | `401 AUTH_REQUIRED` | Authorization header なし、必要 token 未設定時 |
| 3 | auth format / signature / exact match / expiry / revoke | `401 AUTH_INVALID` または `401 AUTH_EXPIRED` | token 値の補正、trim、大小文字補正は禁止 |
| 4 | actor scope | `403 ORG_SCOPE_DENIED` または `403 PERMISSION_DENIED` | org/group/db/replica/HA scope 外 |
| 5 | resource existence | `404 DB_NOT_FOUND` 等 | scope 外 resource の存在を漏らす endpoint は §9.5 で例外明記 |
| 6 | operation permission | `403 PERMISSION_DENIED` | ro token write、admin 権限不足、block_reads/block_writes |
| 7 | protection policy | `403 ORG_SCOPE_DENIED` または `403 PERMISSION_DENIED` | delete_protection、allow_attach=false 等 |
| 8 | usage availability / quota | `503 USAGE_UNAVAILABLE` または `403/402 QUOTA_EXCEEDED` | usage 不明は quota 超過より先に返す |
| 9 | storage / lock / concurrency | `503 STORAGE_BUSY` | restore lock、DB busy、shutdown 中 |
| 10 | operation execution | endpoint 固有 error | SQL / replication / backup / extension / HA 固有 error |

上記順序は既定であり、Turso Cloud 互換 endpoint が異なる status を要求する場合だけ、該当 endpoint 節に差分理由と snapshot を明記して上書きできる。上書きしても `code` は §7.3 / §9.7 と一致させる。

**API 種別ごとの auth source：**

| API surface | Required auth | Scope / permission | Quota / block 適用 |
|-------------|---------------|--------------------|--------------------|
| hrana HTTP `/v2/pipeline` | JWT。Phase 4 前は auth disabled のみ | `a`、`dbs`、org/group/db scope、ro/rw | write/import 相当 SQL は quota/block_writes。read SQL は block_reads |
| hrana WebSocket `/v3/baton` | JWT | connection DB と stream operation ごとに判定 | transaction 内でも operation ごとに判定 |
| `/admin/v1/*` | Admin token | Admin token は管理 API 全体。ただし org/group scope 拡張が仕様化された Phase では scope を適用 | DB write、restore、branch、quota update に適用 |
| `/v1/*` Turso Platform API | Platform token または Phase 8 の Admin token 代用 | organizationSlug / groupName / databaseName の scope を先に判定 | `/v1/*` quota 超過は Turso 互換で 402 を優先 |
| replication API | replication token | primary/replica role、replica_id、frame range | apply/write 側のみ quota/block_writes |
| HA API | Admin token + HA token | node_id、role、term、leader state | quota は適用しない |
| health / metrics read | endpoint ごとの auth | read-only scope または admin scope | metrics read は quota 超過中も許可 |

**複数拒否条件時の優先 error：**

| 条件 | 返す error |
|------|------------|
| auth header なし + resource 不存在 | `401 AUTH_REQUIRED` |
| token 不正 + scope 外 | `401 AUTH_INVALID` |
| token 期限切れ + revoke 済み | `401 AUTH_EXPIRED` |
| org scope 外 + DB 不存在 | `403 ORG_SCOPE_DENIED`。存在漏洩を避ける |
| ro token write + quota 超過 | `403 PERMISSION_DENIED` |
| block_writes=true + quota 超過 | `403 PERMISSION_DENIED`。ただし `/v1/*` で quota status 互換が明記される場合は `402 QUOTA_EXCEEDED` |
| usage unavailable + quota 超過推定 | `503 USAGE_UNAVAILABLE` |
| delete_protection=true + DB not found | `404 DB_NOT_FOUND` |
| restore lock + auth failure | auth failure を優先 |
| shutdown 中 + auth failure | auth failure を優先。auth 通過後は shutdown/storage busy |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| auth matrix | no auth / bad format / invalid / expired / revoked / valid |
| scope matrix | global、dbs、org、group、platform、replication、HA の allow/deny |
| permission matrix | ro/rw、read/write SQL、admin operation、restore、branch、extension |
| quota matrix | allowed、quota exceeded、usage unavailable、Turso 402、admin/hrana 403 |
| block policy matrix | block_reads、block_writes、delete_protection、allow_attach=false |
| precedence snapshot | 複数拒否条件が同時に成立した時の status/code |
| redaction evidence | denied log に token、JWT、SQL args、quota internals が出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| endpoint ごとに auth / scope / quota の判定順が仕様なしに異なる | Phase 未完了 |
| scope 外 resource の存在有無が error で漏れる | merge 不可。ただし仕様本文に例外がある場合のみ可 |
| ro/rw、block、quota の優先順位を test していない | Phase 未完了 |
| `/admin/v1/*` と `/v1/*` の status 差分理由が仕様本文にない | review failure |
| replication / HA token を admin token と混同する | merge 不可 |
| auth denial log に token や JWT claim raw value を出す | merge 不可 |

認証・認可に関係する仕様変更は、manifest の `Security map`、§9.14 security boundary、該当 SEC 契約 ID、auth/scope/quota evidence、redaction artifact を同時更新する。正常系だけが通っても、拒否条件の precedence が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.19 Configuration resolution / validation 固定契約

各 Phase 実装 PR は、追加・変更する CLI flag、環境変数、TOML key、default、secret file、対象 Phase 前 config の扱いを実装開始前に固定しなければならない。設定解決が実行環境や実装者の判断で変わる場合、その Phase は未完了である。

**設定解決の必須項目：**

| 項目 | 必須内容 |
|------|----------|
| config key | CLI flag、env name、TOML section/key、内部 field 名 |
| type / unit | string、bool、integer、duration、bytes、path、enum。ms/sec/bytes など単位も固定 |
| default | 省略時の値。default なしの場合は必須扱いか対象 Phase 前無効を明記 |
| precedence | CLI > env > TOML > default など、全 source の順位 |
| invalid value | 範囲外、型違い、空文字、未知 enum、存在しない file/path の扱い |
| target Phase before behavior | 対象 Phase 前に指定された時、起動失敗か WARN 無効化か |
| redaction | log / stderr / artifact に値を出してよいか、redacted marker |
| evidence | config fixture、env fixture、stderr snapshot、redaction sample、priority test |

**既定 precedence：**

| 種別 | 既定順位 |
|------|----------|
| 通常設定 | CLI flag > env > TOML > default |
| `jwt_secret` | `--auth-jwt-secret-file` > `--auth-jwt-secret` > `ADLAIRE_JWT_SECRET` > TOML `[auth] jwt_secret_file` > TOML `[auth] jwt_secret` > auth disabled |
| admin token | `--admin-auth-token` > `ADLAIRE_ADMIN_TOKEN` > TOML `[admin] auth_token` > admin auth disabled |
| replication token | `--replication-auth-token` > `ADLAIRE_REPLICATION_TOKEN` > TOML `[replication] auth_token` > replication auth disabled only if Phase 節で許可 |
| HA token | `--ha-auth-token` > `ADLAIRE_HA_TOKEN` > TOML `[ha] auth_token` > HA API 起動不可 |
| data dir | `--data` > TOML `[storage] data_dir`。本番では CLI 指定を推奨し、未指定は起動失敗 |

既定順位と異なる設定は、§4、§8.4、§9.13、該当 Phase 節、manifest の `Config map` に差分理由を明記する。差分理由なしに順位を変更してはならない。

**secret / path validation：**

| 設定 | 必須 validation |
|------|-----------------|
| secret value | 最小長、空文字の扱い、前後空白禁止、log/stderr/artifact redaction |
| secret file | 存在、通常 file、readable、directory 禁止、symlink 方針、permission warning/error、内容末尾改行の扱い |
| data dir | create 可否、permission、absolute/canonical path、既存 file 衝突、lock 取得 |
| extension path | data-dir 配下固定、absolute path log 禁止、symlink 拒否 |
| primary URL | scheme、host、port、path、TLS 方針、secret を URL に含めることの禁止 |
| duration / timeout | 0、負数、上限、単位、省略時 default |
| bytes / size limit | suffix、整数 overflow、負数、0 の意味 |
| enum | 大小文字、未知値、deprecated value、fallback 禁止 |

**未実装 config の扱い：**

| 状態 | 必須挙動 |
|------|----------|
| 対象 Phase 前の key が指定された | §9.13.1 に従い、起動失敗または明示 WARN。silent ignore 禁止 |
| sample config に commented key がある | 実装許可ではない。対象 Phase 前に有効値として扱わない |
| unknown TOML key | 互換のため許可するか、起動失敗にするかを section 単位で固定。未定義なら起動失敗 |
| unknown env | 無視可。ただし Adlaire prefix の unknown env を warning するかは仕様化する |
| CLI unknown flag | clap 標準 error で非 0 終了 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| config priority fixture | CLI/env/TOML/default の上書き順を示す fixture |
| invalid config stderr | 型違い、範囲外、unknown enum、存在しない secret file の stderr snapshot |
| target Phase before fixture | 対象 Phase 前 config 指定時の起動失敗または WARN |
| redaction sample | secret、secret file contents、absolute path が log/stderr/artifact に出ないこと |
| restart fixture | config 変更後の起動成功/失敗が deterministic であること |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 未実装 config を silent ignore する | Phase 未完了 |
| invalid value を default に fallback する | merge 不可。ただし仕様本文に fallback 可と明記された場合のみ可 |
| secret value を stderr/log/artifact に出す | merge 不可 |
| CLI/env/TOML の優先順位 test がない | Phase 未完了 |
| sample config の comment だけで実装契約を代替する | review failure |
| Phase ごとに同じ key の意味や単位が変わる | merge 不可。migration / compatibility 契約がある場合のみ可 |

設定に関係する仕様変更は、§4、§8.4、§9.13、manifest の `Config map`、CFG 契約 ID、config fixture、stderr snapshot、redaction artifact を同時更新する。設定の正常系だけが通っても、不正値、優先順位、対象 Phase 前挙動、秘匿が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.20 API schema / validation / serialization 固定契約

各 Phase 実装 PR は、追加・変更する API の method、path、path parameter、query、header、Content-Type、Accept、body limit、request body schema、success response schema、error response schema、serialization、snapshot 正規化を実装開始前に固定しなければならない。schema が曖昧な API は route を公開してはならない。

**request validation 既定ルール：**

| 対象 | 既定仕様 | 例外条件 |
|------|----------|----------|
| method | §9.5 または該当 Phase 節に明記された method のみ許可。未対応 method は `405 METHOD_NOT_ALLOWED` | Turso 互換で異なる status が必要な場合は snapshot と差分理由を明記 |
| path parameter | URL decode 後に validation。空文字、`..`、`/`、NUL、未正規化 unicode は `INVALID_REQUEST` または専用 error | hrana path DB 名など既存互換がある場合のみ該当節で緩和 |
| query | 明記された key のみ許可。duplicate query key、空 query value、未知 query は `INVALID_REQUEST` | cursor など opaque value は型だけ固定 |
| Content-Type | JSON body は `application/json` 必須。`; charset=utf-8` は許可。octet-stream は `application/octet-stream` 必須 | body なし endpoint は Content-Type 不要 |
| Accept | response media type が JSON の場合は未指定、`*/*`、`application/json` を許可。それ以外は `406 NOT_ACCEPTABLE` | SDK 互換で Accept を無視する endpoint は明記 |
| body limit | endpoint ごとに byte 上限を明記。超過は `413 PAYLOAD_TOO_LARGE` | streaming response は response 側上限を別途固定 |
| JSON top-level | request body は object 必須。array/scalar/null は `INVALID_REQUEST` | hrana wire protocol で配列等が仕様化される場合のみ |
| unknown field | 管理 API、Turso Platform API、backup/restore/branch/extension/HA API は拒否 | hrana wire protocol は hrana 仕様に従い、該当節で許可 |
| omitted field | `?` または「省略可」と明記された field だけ省略可。明記なしは必須 | default が仕様化された field は省略可 |
| null | `null 可` と明記された field だけ許可。省略可能 field の `null` を省略扱いにしない | Turso 互換 DTO で null が必要な field |
| empty array / object | 空を許可する field だけ許可。filter、scope、batch、permission 配列の空は既定で `INVALID_REQUEST` | 空 object が Turso 互換上必要な場合 |

**response serialization 既定ルール：**

| 対象 | 固定仕様 |
|------|----------|
| success JSON | schema に定義された wrapper、field casing、必須 field、nullable field を厳密に返す |
| error JSON | `{"error": string, "code": string}` を基本形とし、追加 field は該当 error 契約に明記する |
| 204 response | body なし。`Content-Type` を付けない |
| hrana SQL error | HTTP 200 のまま `results[i].type="error"`。pipeline 全体の JSON 不正だけ HTTP 400 |
| timestamp | RFC3339 UTC 秒精度。ミリ秒、local time、timezone offset は返さない |
| numeric string | SQLite integer / Turso size など文字列指定の field は数値型へ勝手に変えない |
| field order | JSON object order は意味を持たない。snapshot 比較前に key を辞書順正規化する |
| dynamic field | UUID、timestamp、request id、JWT、host、path は placeholder へ正規化する |
| secret field | token / JWT / secret は作成時 1 回だけ返す endpoint を除き response に含めない |

**Turso / SDK 互換 schema：**

| Surface | 必須固定 |
|---------|----------|
| `/v1/*` Turso Platform API | wrapper 名、field casing、status、query mapping、unsupported response を snapshot 化 |
| `/admin/v1/*` | Adlaire 管理 API schema として `/v1/*` と混同しない。差分理由を明記 |
| hrana HTTP | libSQL SDK の request/response schema を優先。unknown field 方針は hrana 節に従う |
| hrana WebSocket | message type、request_id、stream_id、response_ok/error の対応を transcript 化 |
| binary endpoint | `Content-Type`、`Content-Disposition`、body size、checksum/header を固定 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| request fixture | method、path、query、header subset、body、Content-Type |
| response snapshot | status、header subset、body、error code、wrapper、field casing |
| validation error snapshot | unknown field、null、missing field、bad query、bad Content-Type、body limit |
| compatibility snapshot | Turso / SDK 互換 endpoint の正規化後 snapshot |
| serialization roundtrip | request deserialize、internal type、response serialize の roundtrip test |
| redaction snapshot | response / error に secret、token、raw path、SQL args が出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| success response schema が仕様本文にない | route 公開禁止 |
| unknown field を endpoint ごとに仕様なしで黙って無視する | Phase 未完了 |
| `null` と省略を同一扱いにする | review failure |
| 204 response に JSON body を返す | merge 不可 |
| Turso wrapper / field casing を実装都合で変更する | merge 不可 |
| snapshot 正規化なしに dynamic 値を保存する | Phase 未完了 |
| SDK 互換 endpoint を手動確認だけで完了扱いにする | Phase 未完了 |

API schema に関係する仕様変更は、§6、§7、§9.5、manifest の `Endpoint map`、API 契約 ID、request fixture、response snapshot、validation error snapshot を同時更新する。正常系だけが通っても、invalid / unknown / null / body limit / serialization の証跡がない場合は Phase 完了扱いにしない。

#### 9.1.21 Persistence atomicity / fsync / recovery 固定契約

各 Phase 実装 PR は、追加・変更する永続化 file、directory、metadata JSON、SQLite DB、WAL archive、backup artifact、branch artifact、lock file、manifest の書き込み順序、fsync 境界、atomic rename、破損検出、復旧可否を実装開始前に固定しなければならない。永続化成功の定義が曖昧な状態で client へ成功応答を返してはならない。

**atomic write 既定手順：**

| Step | 必須処理 | 失敗時の扱い |
|------|----------|--------------|
| 1 | 対象 directory を canonicalize し、data dir 配下であることを確認する | 起動時または request 時に失敗。path を secret/path redaction 付きで記録 |
| 2 | 同一 directory に一意な temporary file を作成する | 既存 file overwrite 禁止。失敗時は元 file を変更しない |
| 3 | 完全な新内容を書き込む。部分更新、in-place truncate、append で metadata JSON を更新しない | 書き込み失敗時は temp を削除し、元 file を維持 |
| 4 | temporary file を flush し、file fsync を行う | fsync 失敗時は成功応答禁止。元 file を維持 |
| 5 | temporary file を target file へ atomic rename する | rename 失敗時は成功応答禁止。元 file または temp の状態を recovery 対象にする |
| 6 | parent directory fsync を行う | fsync 失敗時は成功応答禁止。次回起動で recovery scan 必須 |
| 7 | lock release 前に in-memory state と disk state の version / checksum を照合する | 不一致なら panic ではなく内部 error とし、成功応答禁止 |

SQLite DB、WAL、backup binary のように library が内部で durability を管理する file は、library の commit 成功条件、checkpoint / sync mode、追加で必要な directory fsync、metadata 更新との順序を Phase 節に明記する。metadata だけ atomic でも、対応する DB/WAL/backup file が永続化されていない場合は成功扱いにしない。

**metadata JSON 既定 schema 契約：**

| 項目 | 固定仕様 |
|------|----------|
| schema_version | すべての metadata JSON に必須。初期値、増加条件、migration path を §9.6 に明記 |
| generation | atomic update ごとに単調増加。restart 後に逆行してはならない |
| checksum | file 内容または参照 artifact の checksum を保存する場合、対象 byte 範囲と algorithm を明記 |
| created_at / updated_at | RFC3339 UTC 秒精度。更新順序の証跡に使う場合は snapshot 正規化する |
| unknown field | 旧 version 互換で読み飛ばすか、破損扱いにするかを file ごとに固定 |
| missing required field | 起動失敗または該当 resource disabled。default 補完は禁止。ただし migration 契約がある場合のみ可 |
| duplicate logical key | DB 名、token id、backup id、branch id の重複は破損扱い。先勝ち/後勝ちは禁止 |

**lock / concurrency 既定ルール：**

| 対象 | 必須挙動 |
|------|----------|
| process lock | `--data` 単位で単一 writer を保証する。lock 取得不能時は起動失敗 |
| resource lock | DB create/delete、token revoke、backup restore、branch create は対象 resource の排他範囲を仕様化 |
| read during write | 読み取りが旧 state を見るか新 state を見るかを固定し、中間 temp state を見せない |
| concurrent update | generation / compare-and-swap / mutex のいずれで競合検出するかを明記 |
| shutdown during write | lock release、temp cleanup、未完了 operation の response 方針を固定 |
| retry | 同じ request を retry した時に idempotent か、重複 error かを API 契約と揃える |

**recovery 判定：**

| 状態 | 必須挙動 |
|------|----------|
| temp file のみ残存 | target file が正しければ temp を削除。target 不在なら operation 種別ごとの recovery 可否で判断 |
| target と temp が両方存在 | generation / checksum / manifest を比較し、勝者を deterministic に決める。判断不能なら起動失敗 |
| metadata JSON parse 失敗 | 自動上書き修復禁止。backup copy または manifest から復旧できる場合のみ復旧 |
| schema_version 未来値 | 起動失敗。downgrade / ignore 禁止 |
| schema_version 過去値 | migration 契約がある場合だけ migrate。なければ起動失敗 |
| checksum 不一致 | 対象 artifact を使用禁止にし、error code と recovery log を出す。成功応答禁止 |
| DB integrity_check 失敗 | 対象 DB を degraded / unavailable とし、通常 read/write を許可しない |
| recovery 失敗 | 部分的に成功したように見せず、起動失敗または対象 resource unavailable とする |

**Phase 別の追加固定対象：**

| Phase 範囲 | 対象 | 必須固定 |
|------------|------|----------|
| Phase 2 | data dir、default DB、lock、integrity marker | 初期化途中 crash、既存 file 衝突、integrity_check 失敗 |
| Phase 4-7 | tokens.json、databases.json、revoke state | token 作成/失効、DB create/delete の atomicity と restart 後状態 |
| Phase 8 | location / org / group / quota metadata | migration、quota usage 更新順序、Turso wrapper との整合 |
| Phase 11-13 | WAL archive、snapshot、replication manifest | frame number、CRC、retention cleanup、manifest/files 整合 |
| Phase 14-16 | backup、restore、PITR、branch | restore rollback、source snapshot 固定、branch metadata と DB file の commit 順序 |
| Phase 17-18 | extension、HA state | signed artifact、term/leader persistence、split-brain recovery |
| Phase 19+ | internal WAL/storage/executor state | libSQL 互換 path と内製 path の rollback、format marker |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| atomic trace | temp write、file fsync、rename、directory fsync、lock release の順序 |
| crash fixture | 各 Step 中断時の再起動後状態。旧 state / 新 state / 起動失敗のいずれかを固定 |
| corrupt fixture | JSON parse 失敗、missing field、future schema_version、checksum mismatch |
| restart fixture | 成功応答後の再起動で state が保持されること |
| concurrency fixture | 同一 resource への同時更新、reader during write、lock 取得不能 |
| recovery log snapshot | recovery 実施 / 失敗時の log。secret、absolute path、raw SQL args を出さない |
| manifest consistency | metadata が参照する artifact が存在し、checksum / size / generation と一致すること |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| metadata JSON を in-place truncate/write で更新する | merge 不可 |
| file fsync または directory fsync の扱いが仕様にない | Phase 未完了 |
| parse 失敗した metadata を空 default で再作成する | merge 不可 |
| checksum 不一致 artifact を警告だけで使用する | merge 不可 |
| success response 後の restart で state が消える | merge 不可 |
| recovery の勝者判定が実装依存または timestamp だけに依存する | review failure |
| temp file や absolute path を API response に出す | merge 不可 |
| crash / corrupt fixture なしに Phase 完了扱いにする | Phase 未完了 |

永続化に関係する仕様変更は、§9.6、§9.15、manifest の `Persistence map`、該当 Phase の storage 契約 ID、atomic trace、crash fixture、corrupt fixture、restart fixture を同時更新する。正常系の書き込みだけが通っても、fsync、rename、directory sync、破損検出、recovery 失敗時挙動が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.22 Error taxonomy / retry / client action 固定契約

各 Phase 実装 PR は、追加・変更する失敗条件について、error category、HTTP status、wire surface、`code`、message 粒度、retry 可否、client action、redaction、evidence を実装開始前に固定しなければならない。未分類の失敗を `INTERNAL_ERROR` に丸めて Phase 完了扱いにしてはならない。

**error category 既定表：**

| Category | 代表条件 | HTTP status / wire | Retry | Client action |
|----------|----------|--------------------|-------|---------------|
| request validation | JSON 不正、未知 field、型違い、body limit、bad query | 400 / 413 / 406 / 405 | No | request を修正 |
| auth required / invalid | token 欠落、不正形式、署名不一致、期限切れ、失効済み | 401 | No | token を再取得または設定修正 |
| permission denied | scope 不足、ro token write、block_reads / block_writes | 403 | No | 権限または設定を変更 |
| not found | DB、token、backup、branch、frame が存在しない | 404 | No | resource 名または selector を修正 |
| conflict | duplicate name、generation mismatch、同時更新競合、delete protection | 409 | Conditional | 最新 state を再取得して再試行 |
| locked / busy | SQLite busy、resource lock timeout、process lock 競合 | 423 または 503 | Yes | backoff retry。process lock は運用者対応 |
| quota / limit | quota 超過、usage unavailable、rate / size limit | 402 / 403 / 429 / 503 | Conditional | quota 調整、時間経過、usage 確認 |
| unavailable | replica lag、leader unavailable、primary 到達不能、maintenance | 503 / 307 | Yes | redirect follow または backoff retry |
| corruption / integrity | checksum 不一致、metadata parse 失敗、integrity_check 失敗 | 500 / 503 | No | recovery / restore / operator action |
| unsupported / not implemented | 未来 Phase、自己ホスト未対応、Turso 互換外 endpoint | 501 / 404 | No | feature availability を確認 |
| internal bug | 仕様上分類済みでない invariant violation | 500 | No | bug として扱い、redacted log を添付 |

上表と §9.7 が衝突する場合は、§9.7、該当 API 節、snapshot 期待値を同じ PR で更新し、差分理由を明記する。Turso Cloud 互換 endpoint が異なる status を必要とする場合でも、`code`、retry、client action は本節または該当 API 節で固定する。

**wire surface 別 error 変換：**

| Surface | 固定仕様 |
|---------|----------|
| admin HTTP | HTTP status と `{"error": string, "code": string}` を返す。debug field、stack trace、path は返さない |
| Turso Platform `/v1/*` | Turso 互換 status / wrapper / casing を優先し、差分理由と compatibility snapshot を残す |
| hrana HTTP `/v2/pipeline` | pipeline JSON 不正は HTTP 400。SQL 実行、permission、constraint は HTTP 200 の `results[i].type="error"` |
| hrana WebSocket | hello 前の認証失敗は `hello_error`。request 単位の失敗は `response_error`。connection close 条件を明記 |
| CLI / startup | HTTP error code に変換しない。stderr message、exit code、redaction snapshot を固定 |
| replication / HA | redirect、timeout、lag、term mismatch、split-brain rejection の status / code / retry を個別に固定 |

**`INTERNAL_ERROR` 使用条件：**

| 状態 | 判定 |
|------|------|
| OS / filesystem / library error でも仕様済み category に分類できる | 専用 code に写像する。`INTERNAL_ERROR` 禁止 |
| serialization、validation、auth、permission、quota、not found、conflict | `INTERNAL_ERROR` 禁止 |
| invariant violation、到達不能分岐、未分類 bug | `INTERNAL_ERROR` 可。ただし redacted log と bug evidence 必須 |
| secret scan failure、redaction failure | `INTERNAL_ERROR` にせず merge/blocking failure として扱う |
| panic / unwrap による process abort | Phase 未完了。HTTP 500 の代替として扱わない |

**複数 error 同時発生時の優先順位：**

| Order | 条件 | 理由 |
|-------|------|------|
| 1 | request body / route / method / content-type が不正 | resource 存在や auth 状態を推測させないため、構文不正を先に閉じる |
| 2 | auth required / invalid / expired / revoked | 未認証 caller に resource 状態を返さない |
| 3 | scope / permission / block policy / quota | 認証済み caller の操作可否を先に確定 |
| 4 | resource not found | 認可後に存在判定する |
| 5 | conflict / idempotency / generation mismatch | 対象 resource の現在 state と request の競合 |
| 6 | busy / unavailable / timeout | 操作可能だが一時的に完了できない |
| 7 | corruption / integrity / recovery failure | operator action が必要な状態 |
| 8 | internal bug | 上記分類に該当しない場合のみ |

§9.1.18 の認証・認可 precedence、§9.1.20 の validation 既定ルール、§9.1.21 の recovery 判定がより具体的な順序を定義する場合は、該当節を優先し、error matrix に差分を記録する。

**retry 既定ルール：**

| Retry | 条件 | Client behavior |
|-------|------|-----------------|
| No | validation、auth、permission、not found、unsupported、corruption | 同一 request をそのまま再送しない |
| Yes | storage busy、replication timeout、leader unavailable、temporary network / upstream failure | exponential backoff + jitter。idempotency 契約を確認 |
| Conditional | conflict、quota、rate limit、usage unavailable、redirect | state 再取得、quota/limit 確認、redirect follow 後に再試行 |
| Unknown | 仕様未定義 | 実装禁止。`INTERNAL_ERROR` で代用しない |

retry 可の error は、同一 request を再送した場合に二重作成、二重課金、二重 revoke、二重 restore、branch 重複が起きないよう、§9.1.16 の idempotency 契約または endpoint 固有契約を必ず参照する。

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| error matrix | 各 error code の category、status、wire surface、retry、client action |
| triggering fixture | 各 error code を発火させる最小 request / config / metadata |
| precedence snapshot | 複数 error 条件が同時に成立した時の status/code/body |
| retry matrix | retry 可 / 不可 / conditional の理由と idempotency 契約 ID |
| SDK/client action snapshot | libSQL SDK、Turso Platform 互換 client、CLI での観測結果 |
| redaction snapshot | error body、stderr、log に secret、JWT、raw SQL args、absolute path が出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 仕様済み error を `INTERNAL_ERROR` に丸める | merge 不可 |
| retry 可否が §9.7 または endpoint 契約にない | Phase 未完了 |
| client action が未定義のまま error code を追加する | review failure |
| hrana SQL error を HTTP 500 に変換する | merge 不可 |
| auth 前に resource not found を返して存在有無を漏らす | merge 不可 |
| error message に token、JWT claim raw value、SQL args、absolute path を出す | merge 不可 |
| 失敗条件の snapshot が正常系 snapshot だけで代替されている | Phase 未完了 |

error に関係する仕様変更は、§7.3、§9.5、§9.7、manifest の `Error map`、ERR 契約 ID、error matrix、triggering fixture、retry matrix、redaction snapshot を同時更新する。正常系だけが通っても、status、code、retry、client action、precedence、redaction が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.23 Turso Cloud tracking / compatibility diff / snapshot update 固定契約

各 Phase 実装 PR は、Turso Cloud 互換対象に関係する API、wire schema、metadata、auth、quota、location、organization/group、error/status、SDK 挙動、snapshot を追加・変更する場合、Turso Cloud 追従状態、差分分類、snapshot 更新理由、既存 Adlaire 後方互換への影響を実装開始前に固定しなければならない。Turso Cloud 互換差分を実装者判断で暗黙に作ってはならない。

**追従対象 surface：**

| Surface | 追従対象 | 必須 evidence |
|---------|----------|---------------|
| hrana HTTP | request/response schema、SQL error surface、baton、pipeline semantics | libSQL SDK transcript、wire snapshot |
| hrana WebSocket | message type、hello/open_stream/execute/sequence/store_sql、error message | WebSocket transcript、SDK regression |
| Turso Platform `/v1/*` | method、path、query、request body、wrapper、field casing、status、error body | Turso snapshot、compat diff |
| auth / JWT | claim 名、scope、ro/rw、expiration、revocation、token presentation | auth matrix、SDK token test |
| location / org / group | slug/name/id、membership、routing metadata、ownership boundary | metadata fixture、Platform snapshot |
| quota / usage | limit field、usage source、超過時 status/code、write denial | quota matrix、usage snapshot |
| metadata | schema version、field casing、default、legacy migration | old/new fixture、migration transcript |
| error/status | HTTP status、`code`、message 粒度、retry 可否 | error matrix、client action snapshot |
| SDK behavior | TypeScript/Rust/Go libSQL client の接続、CRUD、auth、error observation | SDK transcript、regression command |

**差分分類：**

| Classification | 意味 | 実装可否 |
|----------------|------|----------|
| `follow` | Turso Cloud と同一挙動へ追従する | 可。snapshot と regression を更新 |
| `intentional_diff` | 自己ホストの安全性・永続性・運用制約のため意図的に異なる | 可。ただし理由、SDK 影響、代替仕様、後方互換を明記 |
| `unsupported` | Adlaire DB の対象外または未来 Phase として成功応答しない | 可。§9.4 または該当 Phase 節に 501/404/400 等を固定 |
| `deferred` | 追従対象だが当該 PR では実装しない | 実装 PR では成功応答禁止。期限、Phase、stub error を固定 |
| `unknown` | Turso Cloud 挙動を確認できていない | 実装禁止。snapshot 更新禁止 |

**snapshot 更新許可条件：**

| 条件 | 判定 |
|------|------|
| Turso Cloud 追従により期待値が変わり、compat diff に source / date / endpoint / 差分理由がある | 更新可 |
| 仕様本文が先に更新され、変更対象 API / metadata / error の契約が明記されている | 更新可 |
| dynamic 値だけが異なり、placeholder 正規化規則の不足が原因 | 正規化規則を先に更新してから snapshot 更新可 |
| 実装都合で response wrapper、field casing、status、code が変わっただけ | 更新禁止 |
| Turso Cloud の確認結果がないのに `/v1/*` snapshot を変更する | 更新禁止 |
| secret、JWT、Bearer、absolute path、raw SQL args が snapshot に含まれる | 更新禁止。secret scan failure |

**compat diff 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `surface` | hrana HTTP、hrana WebSocket、Platform API、metadata、auth、quota など |
| `reference` | Turso Cloud / libSQL SDK / hrana spec / legacy Adlaire のどれを参照したか |
| `observed_at` | 確認日。PR 作成日と大きく離れる場合は再確認する |
| `classification` | `follow` / `intentional_diff` / `unsupported` / `deferred` |
| `request` | method、path、query、header subset、body または SDK call |
| `reference_response` | status、header subset、body、error code、wire message |
| `adlaire_response` | 期待する Adlaire の status、header subset、body、error code、wire message |
| `diff_reason` | 差分理由。実装都合だけの理由は禁止 |
| `sdk_impact` | 既存 SDK / CLI / client がどう観測するか |
| `legacy_impact` | 既存 Adlaire metadata、config、API への影響 |
| `migration_or_rollback` | migration、互換維持策、rollback flag、または不要理由 |

**優先順位：**

| Order | 判断軸 | 備考 |
|-------|--------|------|
| 1 | security / data durability / secret redaction | Turso 追従より優先。差分理由を必ず書く |
| 2 | hrana wire / libSQL SDK 互換 | client 接続互換を最優先の互換面とする |
| 3 | Turso Platform `/v1/*` 互換 | wrapper、status、field casing を固定 |
| 4 | 既存 Adlaire 後方互換 | 既存 Phase の API、metadata、config を壊さない |
| 5 | 自己ホスト運用最適化 | 上位互換を壊さない範囲で採用 |
| 6 | 内製化都合 | API / wire / metadata 差分の理由にしてはならない |

上位の判断軸と下位の判断軸が衝突する場合は、上位を優先し、compat diff と仕様本文に衝突内容、採用理由、テスト証跡を残す。

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| Turso snapshot | `/v1/*` の status、header subset、body、wrapper、field casing |
| SDK transcript | libSQL SDK の接続、CRUD、auth、error 観測結果 |
| compat diff | 上記必須項目を満たす差分表 |
| legacy regression | 既存 Adlaire endpoint、metadata、config、DB 名の後方互換 |
| unsupported snapshot | 対象外 / deferred endpoint の 501/404/400 response |
| redaction scan | snapshot / transcript / diff に secret、JWT、Bearer、raw path、SQL args がないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| Turso Cloud 互換 endpoint の snapshot 差分理由が仕様本文にない | Phase 未完了 |
| `/v1/*` の wrapper / field casing / status を実装都合で変更する | merge 不可 |
| `unknown` 分類のまま成功応答を公開する | merge 不可 |
| 自己ホスト都合の差分に SDK 影響と代替仕様がない | review failure |
| snapshot を更新してから仕様本文を合わせる | review failure |
| secret を含む snapshot / transcript を証跡として採用する | merge 不可 |
| 内製化都合で API / wire / metadata / error 差分を作る | merge 不可 |

Turso Cloud 互換に関係する仕様変更は、§1.4、§3.5.3、§6、§7、§9.4、§9.5、§9.8、§9.15、該当 Phase 節、manifest の `Compatibility map`、COMPAT 契約 ID、Turso snapshot、SDK transcript、compat diff、legacy regression を同時更新する。Turso snapshot が更新されていても、差分分類、SDK 影響、後方互換、redaction が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.24 CI / release-check / environment reproducibility 固定契約

各 Phase 実装 PR は、完了判定に使う CI、Docker test environment、local verification、release-check、artifact 生成環境を実装開始前に固定しなければならない。実行環境によって結果が変わる検証、手順が口頭説明だけの検証、artifact と manifest が一致しない検証は Phase 完了根拠として扱わない。

**必須 gate：**

| Gate | 必須内容 | 適用条件 |
|------|----------|----------|
| format / lint | repository 標準 formatter / linter。存在しない場合は `N/A` 理由を manifest に記載 | all |
| unit | 対象 crate/module の unit test | all implementation phases |
| integration | HTTP、WebSocket、CLI、persistence の外部挙動 test | 外部 API または永続化を持つ Phase |
| SDK compatibility | TypeScript/Rust/Go libSQL SDK の該当 regression | hrana / SDK 互換面を変更する Phase |
| Turso compatibility | `/v1/*`、metadata、auth、quota、error の snapshot / compat diff | Turso 互換対象を変更する Phase |
| persistence / restart / rollback | fsync、rename、crash、corruption、migration、rollback | 永続化 schema または破壊的操作を変更する Phase |
| security / secret scan | auth denial、scope denial、artifact secret scan、log redaction | secret、auth、artifact を扱う Phase |
| release-check | repository 標準 release validation または同等 command | release、packaging、CI 完了判定を行う Phase |

**environment 固定項目：**

| 項目 | 必須固定 |
|------|----------|
| Rust toolchain | channel / version / target / components。`rust-toolchain.toml` がある場合はそれを正とする |
| Cargo lock | `Cargo.lock` を正とし、CI で lock drift を許可しない |
| Node.js | SDK compatibility test で使う Node.js version と package lock |
| Docker image | image 名、tag、digest または build context。`latest` のみは禁止 |
| OS / arch | Linux target、CPU architecture、glibc/musl 方針、unsupported OS |
| timezone / locale | `TZ=UTC`、UTF-8 locale を既定。timestamp snapshot は UTC 正規化 |
| env vars | test に必要な env 名、default、secret redaction。未定義時の挙動 |
| ports | 固定 port または ephemeral port の割当方法。衝突時 retry / failure 方針 |
| data dir | test ごとに isolated temporary directory。既存 user data dir 使用禁止 |
| network | 外部 network 依存の可否、mock / fixture、offline 時の扱い |

**local / Docker / CI 差分許容範囲：**

| 差分 | 許可 | 条件 |
|------|------|------|
| absolute path | Yes | artifact では `<normalized-path>` に置換 |
| host / port | Yes | `<normalized-host>` / `<normalized-port>` に置換 |
| execution time | Yes | performance gate 以外は pass/fail に使わない |
| timestamp | Yes | UTC 秒精度または `<normalized-time>` |
| dependency version | No | lock file / image tag / toolchain に一致させる |
| feature flag | No | manifest に明記された flags だけ使用 |
| test order | No | order 依存がある場合は test failure |
| network availability | No | external dependency は fixture 化または Manual exception |

**CI artifact 必須内容：**

| Artifact | 必須内容 |
|----------|----------|
| `environment.txt` | OS、arch、Rust、Cargo、Node、Docker image、timezone、locale、env subset |
| `ci.txt` | 実行 command、開始順序、exit code、pass/fail summary、対象 commit |
| `release-check.txt` | release-check command、exit code、検査項目、失敗時の理由 |
| `secret-scan.txt` | scan command、対象 path、検出 0 件または blocking failure |
| `toolchain.json` | machine-readable な toolchain / dependency / image version |
| `flaky-report.txt` | flaky なし、または検出時の原因と deterministic 化修正 |

artifact の保存先は §9.1.11 に従い、Contract ID を path に含める。CI artifact と manifest の `Regression set`、PR description、test name、snapshot path が 1 つでも不一致の場合は Phase 未完了とする。

**flaky / timeout / network 依存の扱い：**

| 状態 | 判定 |
|------|------|
| retry すれば通るが原因未特定 | Phase 未完了 |
| timeout が環境依存で、上限や待機条件が仕様化されていない | Phase 未完了 |
| 外部 network がないと失敗する test | fixture 化する。不可なら Manual exception と期限が必須 |
| sleep 固定待ちで成立する race test | review failure。状態待ち / event / timeout 契約へ置換 |
| CI では skip、local では実行 | skip 理由と代替 artifact がない限り Phase 未完了 |
| secret scan を通すために artifact を削除する | merge 不可。redaction 修正が必要 |

**release-check 必須検査：**

| Check | 必須判定 |
|-------|----------|
| worktree clean | release-check 開始時と終了時に clean |
| spec version | 仕様本文変更時に `V.{累積番号}` が増加している |
| lock drift | `Cargo.lock` / package lock / image pin に未承認差分がない |
| contract coverage | 対象 Phase の Contract ID に未カバーが 0 件 |
| artifact existence | manifest の Evidence path が存在する |
| snapshot strictness | snapshot 差分に仕様本文または compat diff の理由がある |
| secret scan | JWT、Bearer、admin/platform/replication/HA token、生 SQL args、absolute path がない |
| regression result | 対象 Phase 以前の regression が全件成功 |
| zero-bug gate | 既知不具合、未検証、未生成証跡、production TODO/FIXME が 0 件 |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| local だけ通して CI / Docker 再現性を固定しない | Phase 未完了 |
| `latest` tag、floating toolchain、unlocked dependency を完了 gate に使う | merge 不可 |
| CI artifact に実行 command、exit code、対象 commit がない | Phase 未完了 |
| failed gate を `known issue`、`flaky`、`後で検証` として残す | merge 不可 |
| release-check の対象を PR ごとに実装者判断で減らす | review failure |
| secret scan を未実行のまま artifact を採用する | merge 不可 |
| Docker と CI の差分理由が manifest にない | Phase 未完了 |

CI / release-check に関係する仕様変更は、§3.5.5、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.8、manifest の `Regression set`、該当 Contract ID、`environment.txt`、`ci.txt`、`release-check.txt`、`secret-scan.txt` を同時更新する。検証 command が成功していても、環境、toolchain、artifact、secret scan、release-check の再現性が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.25 Long-running operation / job lifecycle 固定契約

各 Phase 実装 PR は、restore、PITR、backup streaming、branch 作成/削除、replication catchup、extension load/unload、HA promote/demote、internal adapter 切替など、長時間・中断・再試行・部分失敗が起こり得る operation について、同期完了 API として扱うか、streaming operation として扱うか、永続 job として扱うかを実装開始前に固定しなければならない。完了していない operation に対して成功応答を返してはならない。

**operation 分類：**

| Classification | 定義 | 成功応答条件 |
|----------------|------|--------------|
| `sync` | request 処理中に全副作用が完了し、restart 後にも結果が確定している操作 | commit、fsync、metadata 更新、必要な検証が完了 |
| `streaming` | response body を返しながら処理するが、metadata や DB state を変更しない操作 | stream header 送信前に source snapshot が固定済み |
| `async_job` | request は job 作成だけを行い、後続 status endpoint で結果を確認する操作 | job metadata の atomic commit 完了。実処理完了ではない |
| `background_internal` | 外部 API の成功応答に直結しない内部 loop / maintenance | health / metrics / log に状態を出し、API 成功と混同しない |
| `stub` | 未来 Phase または unsupported として失敗応答だけ返す操作 | 501/404/400 等の仕様済み error のみ |

operation 分類を変更する場合は、該当 API 節、§9.5、§9.6、§9.7、manifest の Endpoint / Persistence / Error map、snapshot を同じ PR で更新する。

**job schema 必須 field：**

| Field | 必須仕様 |
|-------|----------|
| `job_id` | `job_` prefix の opaque id。request id や path を含めない |
| `type` | `restore`、`pitr_restore`、`branch_create`、`branch_delete`、`replication_catchup`、`extension_load`、`ha_promote`、`internal_adapter_switch` など |
| `resource` | 対象 DB / branch / node / extension の normalized id。secret や absolute path を含めない |
| `state` | `queued`、`running`、`verifying`、`committed`、`failed`、`rolled_back`、`cancelled`、`expired` のいずれか |
| `progress` | 0〜100 の整数または `null`。正確に出せない場合は `null` 固定 |
| `created_at` / `updated_at` | RFC3339 UTC 秒精度。snapshot では正規化 |
| `expires_at` | job result を保持する期限。期限なしは禁止 |
| `result` | 完了時の resource summary。secret、path、backup body は含めない |
| `error` | 失敗時の `{"error": string, "code": string}`。§9.1.22 に従う |
| `idempotency_key_hash` | idempotency key を受ける場合のみ保存。raw key は保存しない |

**共通 state machine：**

| From | To | 条件 |
|------|----|------|
| `queued` | `running` | worker が lock を取得し、対象 resource が存在する |
| `running` | `verifying` | 副作用の prepare が完了し、commit 前検証に入る |
| `verifying` | `committed` | integrity、checksum、quota、metadata consistency、fsync が完了 |
| `running` / `verifying` | `rolled_back` | 失敗後に旧 state へ戻せた |
| `running` / `verifying` | `failed` | rollback 不要または rollback 不能 marker を残した |
| `queued` / `running` | `cancelled` | cancel 可能な段階で operator が cancel した |
| terminal | `expired` | retention 期限を過ぎ、result summary だけ破棄した |

terminal state は `committed`、`failed`、`rolled_back`、`cancelled`、`expired` とする。terminal state から副作用を再開してはならない。再実行は新しい job または idempotency 契約に従う。

**Phase 別 job 化方針：**

| 対象 | 既定分類 | 例外条件 |
|------|----------|----------|
| DB create/delete | `sync` | 大容量 cleanup を後段に回す場合でも metadata commit は sync |
| backup download | `streaming` | backup body を metadata job として保存する仕様が追加された場合のみ `async_job` |
| restore / PITR | `sync` | Phase 節で status endpoint と job schema を定義した場合のみ `async_job` |
| branch create/delete | `sync` | source snapshot が大きく async 化する場合は branch status API を先に仕様化 |
| replication catchup | `background_internal` | operator-triggered catchup API を追加する場合は `async_job` |
| extension load/unload | `sync` | 外部検証が長時間化する場合は `async_job` とし、未検証 load 禁止 |
| HA promote/demote | `sync` | quorum / external consensus を導入する Phase までは `async_job` 禁止 |
| internal adapter switch | `sync` | shadow comparison を長時間 job にする場合は active 化と分離 |

**timeout / cancel / shutdown：**

| 状態 | 必須挙動 |
|------|----------|
| request timeout before commit | 成功応答禁止。rollback 可能なら rollback、不能なら recovery marker |
| request timeout after commit | commit 済みなら status / retry で完了を確認できるようにする |
| operator cancel before prepare | `cancelled`。副作用なし |
| operator cancel after prepare | rollback 成功後のみ `cancelled`。rollback 不能なら `failed` |
| graceful shutdown while running | lock release 前に rollback または resumable marker を fsync |
| forced shutdown | restart recovery で `running` / `verifying` job を deterministic に解決 |
| expired job cleanup | result summary を消してよいが、resource state と audit log は消さない |

cancel API を提供しない Phase では、cancel 不可を該当 API 節に明記し、shutdown/restart recovery だけを固定する。cancel 不可でも process shutdown による中断は必ず検証対象とする。

**retry / idempotency：**

| 状態 | 必須挙動 |
|------|----------|
| 同一 idempotency key + 同一 request body | 既存 job/result を返す。副作用を二重実行しない |
| 同一 idempotency key + 異なる request body | `409 IDEMPOTENCY_CONFLICT` または既存 error code を仕様化 |
| idempotency key なしの retry | endpoint 固有の重複 error または新規 job。曖昧な再実行禁止 |
| committed 後の retry | committed result を返すか、resource already exists / not found を返すかを固定 |
| failed / rolled_back 後の retry | 新規 job を許可するか、operator action を要求するかを固定 |

idempotency key を導入する場合は、header 名、body hash 範囲、retention、conflict response、secret redaction を §9.5 と API 節に明記する。raw idempotency key を metadata、log、artifact に保存してはならない。

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| job state fixture | queued/running/verifying/terminal の metadata sample |
| shutdown fixture | running/verifying 中 shutdown 後の recovery 結果 |
| retry matrix | idempotency key あり/なし、same/different body、terminal state retry |
| timeout snapshot | request timeout 前後の response、job state、resource state |
| recovery log | interrupted job の検出、rollback、commit 判定、operator action |
| redaction snapshot | job response/log/artifact に secret、absolute path、raw SQL args、backup body がないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| job metadata が committed でないのに 2xx success として扱う | merge 不可 |
| async job を作るが status endpoint / schema / retention がない | Phase 未完了 |
| retry で restore、branch create、quota charge、token revoke が二重適用される | merge 不可 |
| shutdown 中の running job が再起動後に silent success する | merge 不可 |
| cancel 後に副作用が残るのに `cancelled` と表示する | merge 不可 |
| job result に token、raw path、SQL args、backup body を含める | merge 不可 |
| background_internal の失敗を health / metrics / log に出さない | Phase 未完了 |

long-running operation に関係する仕様変更は、§9.1.16、§9.1.21、§9.1.22、§9.5、§9.6、§9.7、該当 Phase 節、manifest の Endpoint / Persistence / Error / Regression map、job state fixture、shutdown fixture、retry matrix、recovery log を同時更新する。正常系だけが通っても、timeout、cancel、shutdown、restart recovery、retry、idempotency が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.26 Resource identity / naming / path normalization 固定契約

各 Phase 実装 PR は、追加・変更する resource の external slug、internal id、filesystem name、display name、metadata key、URL path parameter、JWT scope value、quota key の関係を実装開始前に固定しなければならない。名前の validation、正規化、照合、path 変換が曖昧な resource は API、metadata、filesystem に公開してはならない。

**identity 種別の分離：**

| 種別 | 用途 | 変換可否 |
|------|------|----------|
| external slug | API path、query、Turso Platform 互換 field、SDK が見る名前 | validation 後に metadata lookup。勝手に内部 id へ表示変更しない |
| internal id | metadata 内部参照、stable primary key、audit / job reference | API path として直接受け付ける場合は専用 endpoint に明記 |
| filesystem name | data-dir 配下の directory / file 名 | external slug から直接 join せず、validated mapping を通す |
| display name | 将来 UI / description 用の任意文字列 | identity として使わない。path、scope、metadata key に使わない |
| legacy name | Phase 1〜7 で許可済みの既存 DB 名 | migration で保持し、新規作成規則とは分離 |

external slug、internal id、filesystem name を同一文字列として扱ってよいのは、該当 resource 節で validation、reserved word、collision、migration、path mapping が明記されている場合だけとする。

**resource 別 validation 既定表：**

| Resource | 形式 | 大小文字 | 予約 / 禁止 |
|----------|------|----------|-------------|
| DB slug Phase 8+ | `^[a-z0-9-]{1,64}$` | lowercase only | `admin`、`meta`、`.`、`..`、`___`、`/`、NUL |
| legacy DB name | 既存 metadata に存在する値だけ許可 | 既存値を保持 | 新規作成不可。rename 自動実行禁止 |
| branch name | `^[a-z0-9-]{1,64}$` | lowercase only | source DB と同名、`___`、`admin`、`meta`、`.`、`..` |
| internal branch DB name | `{source}___{branch}` | source / branch の規則に従う | external DB create では常に拒否 |
| organization slug | `^[a-z0-9-]{1,64}$` | lowercase only | `admin`、`default` の扱いは Phase 節で固定 |
| group slug | `^[a-z0-9-]{1,64}$` | lowercase only | organization 内 unique。global unique にしない |
| location code | `^[a-z0-9-]{1,32}$` | lowercase only | 空文字、unknown location |
| token id | `tok_[A-Za-z0-9_-]{16,80}` | case-sensitive | raw token、JWT、secret を id として保存しない |
| job id | `job_[A-Za-z0-9_-]{16,80}` | case-sensitive | request id、path、secret を含めない |
| extension name | `^[a-z0-9][a-z0-9_-]{0,63}$` | lowercase only | path separator、dot-prefix、SQL からの直接指定 |
| HA node id | `^[a-z0-9-]{1,64}$` | lowercase only | `standalone` は default 値専用。multi-node identity と混同しない |

上表と既存 Phase 詳細が衝突する場合は、本節、§9.5、§9.6、該当 Phase 節、migration 契約を同じ PR で更新する。互換維持のため緩和する場合は、緩和対象、期限、legacy fixture、Turso / SDK 影響を明記する。

**URL / Unicode / case normalization：**

| 対象 | 固定仕様 |
|------|----------|
| URL path parameter | percent decode 後に UTF-8 として validation。decode 不能は `INVALID_REQUEST` |
| Unicode | identity field は ASCII subset を原則とする。Unicode normalization に依存する identity は禁止 |
| case | lowercase only resource は入力が大文字を含む場合 `INVALID_*`。自動 lower-case 変換は禁止 |
| trailing slash | endpoint 節に明記がない限り、path canonicalization で同一扱いにしない |
| duplicate separator | `//`、`___`、`..`、`.` は identity 内で禁止 |
| whitespace | 前後空白、tab、newline、zero-width space は禁止。trim して受理しない |
| percent encoded slash | `%2F` は decode 後 `/` として拒否。path segment 分割前後で bypass させない |

**metadata / filesystem mapping：**

| 対象 | 必須仕様 |
|------|----------|
| metadata key | external slug ではなく、該当 resource の stable key を明記。legacy lookup がある場合は別 map を持つ |
| filesystem join | validated filesystem name だけを data-dir 配下へ join し、canonicalize 後に data-dir 配下であることを確認 |
| symlink | data-dir 内の resource directory / file に symlink を使う場合は Phase 節で明記。未定義なら拒否 |
| collision | case-insensitive filesystem でも衝突しないことを validation / fixture で確認 |
| rename | DB/org/group/branch/token/job/extension の rename は仕様化されるまで禁止。表示名変更と identity rename を混同しない |
| delete | metadata 削除と filesystem 削除 / trash / cleanup の順序を §9.1.21 と一致させる |
| backup/restore | backup file 内の DB 名や path を trust しない。restore target identity は request path を正とする |

filesystem path は API response、log、artifact では logical resource 名または `<normalized-path>` に正規化する。absolute path、local username、temp directory、trash directory を外部へ返してはならない。

**lookup precedence：**

| 状態 | 必須挙動 |
|------|----------|
| exact current slug が存在する | current resource を返す |
| exact legacy name が存在する | legacy resource として返し、response に legacy marker が必要か Phase 節で固定 |
| current slug と legacy name が衝突する | 起動失敗または migration failure。先勝ち/後勝ち禁止 |
| internal id と external slug の両方を受け付ける endpoint | 優先順位と ambiguity error を endpoint 節に明記 |
| branch internal DB name と通常 DB slug が衝突する | 通常 DB create を拒否。既存衝突は起動失敗 |
| org/group scoped lookup | organization slug → group slug → DB slug の順で scope を解決し、存在漏洩を §9.1.18 に従い抑制 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| name validation matrix | valid、uppercase、unicode、空、reserved、separator、長さ超過、percent encoded slash |
| path traversal fixture | `..`、`%2F`、symlink、absolute path、case collision、data-dir escape |
| legacy name fixture | Phase 1〜7 DB 名、legacy marker、new create rejection、lookup compatibility |
| metadata/path consistency snapshot | metadata key、filesystem path、canonical path、logical resource の対応 |
| scope lookup fixture | org/group/db、token scope、quota key の lookup precedence |
| redaction snapshot | path、local username、token id/raw token、job id が適切に秘匿 / 正規化されること |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| URL decode 前の文字列だけで validation する | merge 不可 |
| 大文字を自動 lowercase して別 resource として受理する | merge 不可 |
| external slug を未検証のまま filesystem path に join する | merge 不可 |
| legacy name を新規作成でも許可する | review failure。ただし仕様本文に例外がある場合のみ可 |
| internal id、external slug、display name を同じ field として混用する | Phase 未完了 |
| filesystem case collision の test がない | Phase 未完了 |
| resource not found の前に scope 外 resource の存在を漏らす | merge 不可 |

resource identity に関係する仕様変更は、§6、§8、§9.1.18、§9.1.20、§9.1.21、§9.5、§9.6、§9.14、該当 Phase 節、manifest の Endpoint / Persistence / Security / Compatibility map、name validation matrix、path traversal fixture、legacy fixture、metadata/path consistency snapshot を同時更新する。正常系だけが通っても、decode、case、legacy、collision、path traversal、scope lookup が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.27 List / pagination / cursor / filtering 固定契約

一覧 API を追加・変更する Phase は、実装開始前に並び順、limit、cursor、filter、snapshot 正規化、ページ跨ぎの mutation 挙動を固定しなければならない。`limit` と `cursor` の存在だけを endpoint 表に書くことは十分条件ではない。Turso Cloud 互換 endpoint と Adlaire 管理 endpoint が同じ resource を返す場合、wrapper / field casing を除き、対象集合、順序、cursor 境界、filter 結果は一致させる。

**共通 pagination 入力契約：**

| 項目 | 固定仕様 |
|------|----------|
| `limit` 未指定 | 100 |
| `limit` 最小 / 最大 | 1〜500。範囲外、非整数、浮動小数、指数表記、符号付き表記、空文字は `INVALID_REQUEST` |
| `cursor` 未指定 | 先頭ページ |
| `cursor` 空文字 | `INVALID_REQUEST`。空文字を先頭ページ扱いしない |
| duplicate query | 同一 key の複数指定は `INVALID_REQUEST` |
| unknown query | endpoint 固有表に明記されていない key は `INVALID_REQUEST` |
| offset pagination | `offset`、`page`、`per_page` は endpoint 固有表に明記しない限り禁止 |
| body | `GET` 一覧 API は body 禁止。body がある場合は `INVALID_REQUEST` |

**cursor 形式と検証：**

| 項目 | 固定仕様 |
|------|----------|
| wire format | base64url without padding の opaque string。client は内容を解釈してはならない |
| payload | `version`、`endpoint_id`、`resource_kind`、`filters_hash`、`sort_key`、`last_seen_key`、`issued_at` を含む |
| 署名 / 改ざん検知 | HMAC または同等の改ざん検知を必須とする。復号不能、署名不一致、必須 field 欠落は `INVALID_REQUEST` |
| scope binding | endpoint、resource kind、organization/group/database scope、filter、sort が一致しない cursor は `INVALID_REQUEST` |
| version | 実装が解釈できない cursor version は `INVALID_REQUEST`。silent reset 禁止 |
| expiry | endpoint が expiry を明記しない限り無期限。ただし metadata format 変更で解釈不能になった cursor は `INVALID_REQUEST` |
| secret | cursor payload に token、raw path、SQL、local username、quota 値の秘匿情報を入れない |

**共通 list 順序：**

| Resource 種別 | 既定順序 |
|---------------|----------|
| metadata resource | `created_at` 昇順、同値は stable identity 昇順 |
| usage / quota 集計 | scope 種別順 `organization` → `group` → `database`、scope identity 昇順 |
| location | `primary` true を先頭、その後 `created_at` 昇順、同値は location code 昇順 |
| token 一覧 | `created_at` 昇順、同値は token id 昇順。raw token は返さない |
| log / audit / event | `created_at` 昇順、同値は monotonic sequence 昇順。降順を採用する場合は endpoint 表に明記 |

endpoint が上表と異なる順序を必要とする場合は、endpoint 表、snapshot、cursor payload の `sort_key` を同じ PR で更新する。実装言語の map iteration order、filesystem order、JSON object key order、database engine の未指定順序に依存してはならない。

**filter 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| filter key | endpoint 表に明記された key のみ許可 |
| filter value | identity 系 value は §9.1.26 の validation 後に scope 解決する。trim / lowercase 補正は禁止 |
| 複数 filter | AND 条件。OR、部分一致、prefix match は endpoint 表に明記しない限り禁止 |
| 空結果 | 200 と空配列、`next_cursor:null`。対象 scope 自体が存在しない場合は対応する `*_NOT_FOUND` |
| 権限外 scope | §9.1.18 の precedence に従い、存在漏洩しない error を返す |
| cursor 併用 | cursor 発行時と同じ filter set だけ許可。filter 変更時は新規先頭ページとして cursor なしで要求する |

**ページ跨ぎ mutation 挙動：**

| 状態 | 必須挙動 |
|------|----------|
| 前ページ返却後に resource が追加された | 追加 resource が cursor の sort key より後なら後続ページに出てもよい。前なら今回の traversal には出なくてよい |
| 前ページ返却後に未返却 resource が削除された | 削除済み resource は返さない。穴埋めのために既返却 resource を重複返却しない |
| 前ページ返却後に既返却 resource が削除された | 後続ページで再返却しない |
| sort key が変更された | identity rename が禁止されている resource では発生させない。変更可能 resource は endpoint 節で cursor invalidation を明記 |
| metadata 破損 / migration 中 | partial list を返さず、該当 error と recovery evidence を残す |

`next_cursor` は次ページが存在する場合だけ non-null とする。最終ページ、空結果、`limit` より少ない結果しかない場合は `null` とする。`next_cursor` が non-null の場合でも、次 request までに resource が削除されて空ページになることは許容するが、その場合も 200 空配列 `next_cursor:null` とし、cursor を巻き戻してはならない。

**response / snapshot 正規化：**

| 項目 | 固定仕様 |
|------|----------|
| array order | API 契約の list 順序通りに比較する。snapshot 比較前に sort し直さない |
| object key order | snapshot 比較前に辞書順へ正規化する |
| timestamp | format、timezone、precision を endpoint / DTO 節に明記し、fixture では固定値を使う |
| wrapper | `/v1/*` は Turso 互換 wrapper / field casing、`/admin/v1/*` は Adlaire wrapper を維持する |
| empty list | 空配列と `next_cursor:null` を必ず含め、省略しない |
| redaction | cursor、token id、scope は snapshot に含めてよいが、raw token / raw path / local username は含めない |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| pagination matrix | limit 未指定、1、500、0、501、非整数、空、duplicate、unknown query |
| cursor fixture | valid、改ざん、別 endpoint、別 filter、別 scope、unknown version、空 cursor |
| order snapshot | created_at 同値、identity 同値不可、filesystem / map order 非依存 |
| mutation fixture | page 取得間の追加、削除、空ページ、最終ページ |
| filter matrix | scope 存在、scope 不存在、権限外、複数 filter、空結果 |
| Turso compatibility snapshot | `/v1/*` と `/admin/v1/*` の対象集合、順序、cursor 境界の差分分類 |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| cursor を単なる array index / offset として実装する | merge 不可 |
| filter 変更後も古い cursor を受理する | merge 不可 |
| map iteration / filesystem order に依存した list を返す | merge 不可 |
| duplicate query key を最後勝ち / 先勝ちで黙認する | merge 不可 |
| page 間削除で既返却 resource を再返却する | merge 不可 |
| snapshot 比較時に array を test 側で任意 sort する | Phase 未完了 |

list / pagination に関係する仕様変更は、§9.1.18、§9.1.20、§9.1.22、§9.1.23、§9.1.26、§9.5、該当 Phase 節、manifest の Endpoint / Compatibility / Regression map、pagination matrix、cursor fixture、order snapshot、mutation fixture、filter matrix を同時更新する。正常系だけが通っても、cursor scope、filter binding、stable order、page mutation、snapshot order が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.28 SQL execution / transaction / result mapping 固定契約

hrana-http、hrana-ws、ATTACH、replication redirect、backup consistency、internal executor 差し替えに関係する Phase は、SQL 実行単位、transaction 境界、result mapping、error surface、SDK 互換 transcript を実装開始前に固定しなければならない。SQL を libsql crate に渡すだけでは完了条件を満たさない。client が観測する wire format は Turso Cloud / libSQL SDK 互換を優先し、内部 executor の都合で status、result shape、transaction rollback 条件を変えてはならない。

**SQL request 種別別契約：**

| 種別 | 実行単位 | 成功応答 | 失敗応答 |
|------|----------|----------|----------|
| hrana-http `execute` | 単一 `stmt`。`want_rows` に従い `query` または `execute` | `results[i].type="ok"` + `response.type="execute"` | pipeline 全体は HTTP 200、当該 item は `results[i].type="error"` |
| hrana-http `sequence` | SQL 文字列全体を `execute_batch` へ渡す。自前 semicolon split 禁止 | 当該 item `ok`。個別 statement result は返さない | pipeline 全体は HTTP 200、当該 item は `error` |
| hrana-http `close` | 以降の request を処理しない | `close` の ok response | close 後の request は無視し、追加 result を返さない |
| hrana-ws `execute` | open stream 上の単一 `stmt` | `response_ok` + `execute` result | `response_error`。connection は維持 |
| hrana-ws `batch` | `batch` 配列順に `stmt` を逐次実行 | step ごとの result / error を順序維持で返す | request 自体の schema 不正は `response_error` |
| hrana-ws `sequence` | SQL 文字列全体を `execute_batch` へ渡す | `response_ok`。個別 statement result は返さない | `response_error` |
| hrana-ws `describe` | prepare / describe のみ。実行しない | parameter / column metadata | prepare error は `response_error` |

**`stmt` validation / parameter mapping：**

| 項目 | 固定仕様 |
|------|----------|
| `sql` | 非空文字列必須。空、空白のみ、NUL を含む値は `INVALID_REQUEST` |
| `args` | positional args。省略時は空配列。配列以外、変換不能 value は request 単位の SQL error surface に載せる |
| `named_args` | Phase 3〜8 の hrana-http では空配列または省略のみ許可。非空は当該 item を `SQLITE_ERROR` とする。Phase 9 以降で対応する場合は name 重複、positional 併用、SDK transcript を固定する |
| `want_rows` | boolean 必須。省略許可にする場合は endpoint / protocol 節で default を明記する |
| integer | hrana wire 上は文字列で返す。SQLite integer 範囲外または parse 不能は `SQLITE_ERROR` |
| float | IEEE 754 double として扱う。NaN / Infinity は JSON として受理しない |
| text | UTF-8 文字列。変換不能 byte列は blob としてのみ扱う |
| blob | base64 文字列。decode 不能は `SQLITE_ERROR` |
| null | SQLite NULL として bind し、response では hrana の null value として返す |

SQL text、args、row value は log / artifact / error message に生値で出してはならない。証跡では statement kind、arg count、type list、result column count までを許可し、値は `<redacted-arg>` または fixture 固定値だけにする。

**result mapping：**

| 項目 | 固定仕様 |
|------|----------|
| `cols` | `want_rows=true` の query では libsql / SQLite の column order を保持する。`want_rows=false` の execute では空配列 |
| `rows` | row order は SQLite が返した順序を保持する。test 側で sort しない。`want_rows=false` では空配列 |
| `rows_affected` | execute / write statement は libsql が返す affected rows。DDL や query で未定義の場合は 0 |
| `last_insert_rowid` | insert 成功後の値を文字列で返す。値が意味を持たない operation は `null`。数値型で返さない |
| multi result | `sequence` は個別 statement の `cols` / `rows` / `rows_affected` を返さない |
| column type | SQLite dynamic type を hrana value type に変換し、unsupported type は `SQLITE_ERROR` |
| response order | request order と results order は 1:1。parallel 実行で順序を入れ替えない |

**transaction / connection state：**

| 状態 | 固定仕様 |
|------|----------|
| HTTP pipeline | Phase 3〜8 では request 間 session を保持しない。SQLite 明示 transaction SQL は同一 pipeline 内の同一 connection で順序実行する場合のみ有効 |
| WebSocket stream | stream ごとに dedicated connection または同等の transaction isolation を持つ。別 stream の transaction state と混ぜない |
| `BEGIN` | stream state を `tx_active` にする。`BEGIN IMMEDIATE` / `BEGIN EXCLUSIVE` は write operation として認可・quota・block 判定する |
| `COMMIT` | commit が libsql / SQLite で成功し、response 送信前の状態更新が完了した場合だけ success |
| `ROLLBACK` | rollback 成功後に stream state を `open` へ戻す。rollback 失敗は `response_error` |
| close_stream | open transaction がある場合は rollback を試行してから close。rollback 失敗時は log / metric / evidence に残す |
| connection close | open transaction は rollback。COMMIT 成功応答前の切断は success と扱わない |
| shutdown | shutdown timeout 内に running SQL を完了または rollback する。timeout 後の abort は recovery evidence 必須 |

**permission / quota / block precedence：**

| 順位 | 判定 | 失敗時 |
|------|------|--------|
| 1 | request / wire schema validation | `INVALID_REQUEST` または protocol 固有 error |
| 2 | DB existence / resource state | `DB_NOT_FOUND`、`PERMISSION_DENIED` など |
| 3 | auth / scope / ro-rw | `AUTH_*`、`PERMISSION_DENIED` |
| 4 | block policy | `block_reads` / `block_writes` に応じて `PERMISSION_DENIED` |
| 5 | quota | write / import 相当 SQL は `QUOTA_EXCEEDED` |
| 6 | SQL prepare / execute | `SQLITE_ERROR`、`SQLITE_CONSTRAINT`、SQLite 固有 code |

read/write 判定は SQL text の単純な prefix だけに依存してはならない。Phase 10 の ATTACH、Phase 16 の extension、transaction control statement、DDL、PRAGMA を含む分類表を該当 Phase 節で更新する。分類できない statement は write 側に倒す。

**batch / sequence 途中失敗：**

| 種別 | 固定仕様 |
|------|----------|
| hrana-http request array | item ごとに順序実行する。ある item が SQL error でも後続 item は処理する。ただし `close` 後は処理しない |
| hrana-http `sequence` | `execute_batch` に委ねる。途中 statement の commit / rollback は SQLite の transaction semantics に従う |
| hrana-ws `batch` | step ごとの success / error を順序保持で返す。transaction 中の error 後に stream を強制 close しない |
| hrana-ws `sequence` | `execute_batch` に委ね、成功なら全体 success、失敗なら全体 `response_error` |
| explicit transaction | `BEGIN` 後の step error では自動 rollback しない。client の `ROLLBACK` または close / disconnect で rollback |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| SDK transcript | TypeScript / Rust / Go libSQL SDK の execute、batch、transaction、error 観測結果 |
| result mapping snapshot | cols、rows、integer string、blob、null、rows_affected、last_insert_rowid |
| args conversion matrix | integer、float、text、blob、null、invalid base64、integer overflow、named_args 非空 |
| SQL error matrix | syntax error、missing table、constraint、readonly、busy、permission、quota、block_reads/writes |
| transaction fixture | BEGIN/COMMIT/ROLLBACK、disconnect rollback、close_stream rollback、COMMIT 前切断 |
| batch / sequence matrix | 途中失敗、後続継続、sequence の execute_batch 境界、close 後無視 |
| redaction snapshot | SQL text、args、row value、absolute path、token が log / artifact / error に出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| SQL error を hrana-http の HTTP 500 / 400 に変換する | merge 不可 |
| `sequence` を ad hoc semicolon split する | merge 不可 |
| request order と results order を入れ替える | merge 不可 |
| `last_insert_rowid` を数値型で返す | SDK 互換 failure |
| open transaction を close / disconnect 後に残す | merge 不可 |
| SQL text、args、row value を log / artifact に出す | merge 不可 |
| read/write 判定不能 statement を read として許可する | merge 不可 |

SQL execution に関係する仕様変更は、§6.2、§6.3、§7.3、§9.1.16、§9.1.18、§9.1.20、§9.1.21、§9.1.22、§9.1.23、§9.5、§9.14、該当 Phase 節、manifest の Endpoint / Security / Compatibility / Regression map、SDK transcript、result mapping snapshot、args conversion matrix、transaction fixture、batch / sequence matrix を同時更新する。正常系だけが通っても、SQL error surface、result mapping、transaction rollback、permission precedence、SDK transcript が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.29 Specification consistency / cross-reference 固定契約

仕様書を更新する PR は、変更対象の本文だけでなく、関連する Phase 表、API 契約、error、永続化、テスト、evidence、manifest、実装前チェックリストの相互参照を同時に整合させなければならない。仕様本文に正しい内容を書いていても、別表や Phase 詳細が古いまま残る場合、その PR は仕様未確定として扱う。

**識別子整合ルール：**

| 識別子 | 固定仕様 |
|--------|----------|
| Phase 番号 | `Phase N` 表記を正とし、同一機能を別 Phase に移す場合は §9 の Phase 一覧、§9.2、§9.4、Phase 詳細節、§9.8、§9.11 を同時更新する |
| TC ID | テストケース番号は該当 Phase 詳細節、§9 Phase 一覧、§9.8、evidence artifact 名で一致させる。欠番を残す場合は理由を明記する |
| Task ID | 実装タスク番号は Phase 詳細節、§9 Phase 一覧、manifest の `Task map` で一致させる |
| API 契約 ID | endpoint、request/response snapshot、error snapshot、compatibility snapshot、manifest の `Endpoint map` で同一 ID を使う |
| Error code | §7.3、§9.1.22、endpoint 表、Phase 詳細節、error snapshot で同じ code / status / retry 方針を使う |
| Persistence key | §9.6、Phase 詳細節、migration fixture、recovery evidence、manifest の `Persistence map` で同じ file 名 / schema version を使う |
| Evidence 名 | §9.1.11、§9.1.24、Phase 詳細節、manifest の `Evidence map` で保存先と名前を一致させる |

**変更種別ごとの同時更新必須箇所：**

| 変更種別 | 同時更新必須箇所 |
|----------|------------------|
| 新 API / route | §6 または §9.5、§7.3、§9.1.20、§9.17、該当 Phase 詳細節、API 契約 ID、request/response/error snapshot |
| 新 error code | §7.3、§9.1.22、該当 endpoint 表、該当 Phase 詳細節、error matrix、client action snapshot |
| 新 metadata / file | §9.6、§9.1.21、該当 Phase 詳細節、migration plan、rollback / recovery fixture |
| 新 auth / scope | §9.1.18、§9.14、該当 endpoint 表、JWT / token fixture、permission matrix |
| 新 list API | §9.1.27、endpoint 表、pagination matrix、cursor fixture、order snapshot |
| SQL execution 変更 | §9.1.28、§6.2 / §6.3、SDK transcript、result mapping snapshot、transaction fixture |
| Phase スコープ変更 | §9 Phase 一覧、§9.2、§9.4、§9.8、§9.10、§9.11、該当 Phase 詳細節 |
| Turso 互換変更 | §9.1.23、Phase 8 または該当 Phase 詳細節、compatibility diff、Turso snapshot source |

**古い参照の扱い：**

| 状態 | 扱い |
|------|------|
| 存在しない節番号を参照している | merge 不可 |
| 古い Phase 名 / Phase 番号が残っている | 仕様未確定 |
| TC 範囲と実際の TC 定義数が一致しない | Phase 未完了 |
| Task 範囲と実装タスク表が一致しない | 実装開始禁止 |
| endpoint 表と Phase 詳細節の status / body / error が違う | endpoint 表を正とせず、同じ PR で解消するまで実装禁止 |
| error code が §7.3 にない | 新 code を使う実装禁止 |
| snapshot 名だけ存在し、生成条件がない | evidence 不足 |
| manifest だけ更新され本文がない | 仕様として扱わない |

**PR self-check 必須項目：**

| Check | 必須確認 |
|-------|----------|
| section reference scan | 追加・変更した `§` 参照が実在し、見出し番号と一致する |
| Phase table scan | §9 の Phase 一覧、§9.2、§9.4、§9.8、§9.11 の対象 Phase 行が同じスコープを表す |
| endpoint scan | method/path/auth/status/body/error が §6 / §9.5 / Phase 詳細節で一致する |
| evidence scan | manifest、artifact path、snapshot 名、test ID が一致する |
| compatibility scan | Turso / libSQL SDK 影響が §9.1.23 と該当 Phase 節に同じ分類で記録されている |
| stale text scan | 旧バージョン番号、旧 Phase 境界、旧対象外理由、旧 API 名が残っていない |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 本文だけ変更して Phase 表 / test matrix を更新しない | merge 不可 |
| Phase 詳細節だけ変更して §9.2 / §9.8 / §9.11 を更新しない | merge 不可 |
| error code を本文に書くが §7.3 に追加しない | merge 不可 |
| API response を変えるが snapshot / SDK transcript を更新しない | Phase 未完了 |
| 古い参照を「後で直す」として残す | 仕様未確定 |
| PR description だけで整合性を説明し、仕様本文に反映しない | 仕様として扱わない |

cross-reference に関係する仕様変更は、変更した節だけでなく、参照元、参照先、Phase 一覧、Phase 完了ゲート、test matrix、Definition of Ready / Done、manifest、evidence artifact を同時確認する。相互参照の機械確認が未整備の場合でも、PR 内で上記 self-check を完了し、未確認項目を `N/A` にしてはならない。

#### 9.1.30 Backup / restore / PITR / rollback 固定契約

backup、restore、PITR、branch seed、WAL archive replay に関係する Phase は、元 DB を破壊しない commit / rollback 境界、排他 lock、temporary layout、検証順序、失敗 marker、recovery evidence を実装開始前に固定しなければならない。破壊的操作は「成功応答を返した時点で、再起動後も新状態が一貫している」または「失敗応答後に旧状態へ戻っている」のどちらかだけを許可する。

**backup 契約：**

| 項目 | 固定仕様 |
|------|----------|
| snapshot | SQLite Online Backup API 相当の一貫 snapshot を返す。copy 中の filesystem 直読みは禁止 |
| concurrent write | backup 中の通常 write はブロックしない。backup snapshot に含まれるかどうかは snapshot 開始時点で固定する |
| response header | `Content-Type: application/octet-stream`、`Content-Disposition: attachment; filename="{db}.db"` |
| stream failure | client 切断時は backup を中断し、partial response を成功扱いにしない。DB 状態は変更しない |
| block / quota | `block_reads=true` は backup download を `403 PERMISSION_DENIED`。quota 超過中でも backup は許可 |
| redaction | backup body、SQLite page、absolute path は log / artifact に出さない。size、checksum、duration だけ許可 |

**restore / PITR 状態遷移：**

| 状態 | 必須条件 | 次状態 |
|------|----------|--------|
| `prepared` | request validation、auth、scope、quota/block 事前判定、exclusive lock 取得が完了 | `uploaded` または `replaying` |
| `uploaded` | upload body を temp に保存し fsync 済み | `verifying` |
| `replaying` | PITR snapshot と WAL frame を temp DB へ replay 中 | `verifying` |
| `verifying` | `PRAGMA integrity_check`、checksum、schema 互換確認を実行 | `committing` または `rolled_back` |
| `committing` | runtime DB close、old 退避、new rename、directory fsync、DB reopen を順に実行 | `committed` または `rolled_back` |
| `committed` | success response 可能。`204 No Content` 以外を返さない | terminal |
| `rolled_back` | old DB を復元し runtime DB reopen 済み | terminal |
| `failed_unrecoverable` | old DB 復元不能。`restore-failed.json` を fsync 済み | 起動失敗 / operator 対応 |

**temp layout / marker：**

| Path | 内容 |
|------|------|
| `{data-dir}/databases/{db}/restore-{request_id}/upload.db` | restore upload body。backup body を log に出さない |
| `{data-dir}/databases/{db}/restore-{request_id}/verified.db` | integrity_check 済み commit 候補 |
| `{data-dir}/databases/{db}/restore-{request_id}/old/data.db` | rollback 用旧 DB |
| `{data-dir}/databases/{db}/restore-{request_id}/restore-state.json` | state、request_id、target、started_at、phase、checksums |
| `{data-dir}/meta/restore-failed.json` | unrecoverable failure marker。存在する場合は対象 DB を起動時に open しない |

temp directory は data-dir 配下だけに作成する。request_id は §9.1.17 の秘匿規則に従い、path、token、SQL、user input body を含めない。cleanup は `committed` または `rolled_back` が fsync 済みであることを確認してから行い、cleanup 失敗は WARN として次回起動時に再試行する。

**restore / PITR request validation：**

| 項目 | 固定仕様 |
|------|----------|
| restore body | SQLite DB file binary。body size は設定値 `restore_max_bytes` が未定義の間、既存 DB size の 2 倍または 1 GiB の小さい方を上限 |
| PITR selector | `timestamp` または `frame_no` のどちらか 1 つだけ。両方あり、両方なし、型不正は `INVALID_REQUEST` |
| PITR disabled | `wal_retention_days = 0` または archive 未初期化は `503 PITR_NOT_ENABLED` |
| frame lookup | timestamp は manifest の frame timestamp へ決定的に解決する。同値境界は target 以下の最大 frame |
| checksum | snapshot と WAL frame は replay 前に checksum 検証。不一致は `RESTORE_FRAME_CORRUPT` |
| missing frame | target までの連続 frame が欠ける場合は `FRAME_NOT_FOUND` |
| integrity | replay 後または upload 後の DB は `PRAGMA integrity_check` が `ok` の場合だけ commit |

**拒否条件 precedence：**

| 順位 | 条件 | 失敗時 |
|------|------|--------|
| 1 | request / body / selector validation | `400 INVALID_REQUEST` または `413 PAYLOAD_TOO_LARGE` |
| 2 | auth / admin scope | `AUTH_REQUIRED`、`AUTH_INVALID`、`PERMISSION_DENIED` |
| 3 | DB existence / identity validation | `DB_NOT_FOUND`、`INVALID_DB_NAME` |
| 4 | `delete_protection=true` | `403 ORG_SCOPE_DENIED` |
| 5 | `block_reads=true` for backup | `403 PERMISSION_DENIED` |
| 6 | `block_writes=true` for restore / PITR | `403 PERMISSION_DENIED` |
| 7 | quota after restore/PITR candidate size calculation | `402 QUOTA_EXCEEDED` |
| 8 | restore lock / DB busy / shutdown | `503 STORAGE_BUSY` |
| 9 | PITR archive / checksum / integrity | `PITR_NOT_ENABLED`、`FRAME_NOT_FOUND`、`RESTORE_FRAME_CORRUPT`、`RESTORE_INTEGRITY_FAILED` |

**shutdown / restart recovery：**

| 状態 | 起動時挙動 |
|------|------------|
| temp directory exists, no committed marker | old DB がある場合は rollback して起動。old DB がなければ起動失敗 |
| `committing` marker exists | old/new の実ファイル状態を検査し、どちらか一方へ決定的に収束させる。silent success 禁止 |
| `committed` marker exists, cleanup 未完了 | DB を通常 open し、temp cleanup を WARN 付きで再試行 |
| `rolled_back` marker exists, cleanup 未完了 | old DB を通常 open し、temp cleanup を WARN 付きで再試行 |
| `restore-failed.json` exists | 対象 DB を open せず、health / admin detail で degraded を示す。operator が marker を解消するまで write 禁止 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| backup consistency artifact | backup 中 write、snapshot integrity、response header、client disconnect |
| restore rollback fixture | invalid upload、commit 前失敗、commit 中失敗、DB reopen 失敗、old 復元 |
| PITR replay fixture | timestamp selector、frame_no selector、missing frame、checksum corrupt、retention disabled |
| lock matrix | restore 中 read/write、backup 中 write、shutdown 中 restore、concurrent restore |
| precedence matrix | auth、delete_protection、block_reads、block_writes、quota、storage busy の優先順位 |
| recovery log | restart recovery、cleanup retry、unrecoverable marker、operator action |
| redaction snapshot | backup body、uploaded DB、WAL frame bytes、absolute path、token が出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 元 DB を直接上書きしてから検証する | merge 不可 |
| integrity_check 前に runtime DB として公開する | merge 不可 |
| rollback 不能なのに 2xx success を返す | merge 不可 |
| PITR checksum 不一致 frame を skip して続行する | merge 不可 |
| restore 中 write を受け付ける | merge 不可 |
| backup / upload body を log / artifact に残す | merge 不可 |
| `restore-failed.json` を無視して起動する | merge 不可 |

backup / restore / PITR に関係する仕様変更は、§6.4、§7.3、§9.1.16、§9.1.18、§9.1.21、§9.1.22、§9.1.24、§9.1.29、§9.6、§9.14、Phase 13 / 14 / 15 詳細節、manifest の Endpoint / Persistence / Security / Regression map、backup consistency artifact、restore rollback fixture、PITR replay fixture、recovery log を同時更新する。正常系だけが通っても、rollback、restart recovery、checksum、lock、quota/block precedence、redaction が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.31 Branch lifecycle / seed / isolation 固定契約

branch 作成、branch 削除、Turso database seed、source DB 削除、branch routing に関係する Phase は、branch metadata、branch DB directory、runtime map、token scope、quota、source selector、restart recovery の関係を実装開始前に固定しなければならない。branch は通常 DB の別名ではなく、source から作成された独立 DB resource として扱う。

**branch create 状態遷移：**

| 状態 | 必須条件 | 次状態 |
|------|----------|--------|
| `creating` | name validation、source lookup、auth/scope、quota/block 判定、source lock 取得が完了 | `materializing` |
| `materializing` | current backup または PITR replay で temp DB を構築中 | `verifying` |
| `verifying` | integrity_check、source selector 記録、internal DB name 衝突確認 | `activating` または `rolled_back` |
| `activating` | branch DB directory finalize、runtime open、`branches.json` atomic commit | `active` または `rolled_back` |
| `active` | pipeline routing と admin list/detail に公開可能 | terminal |
| `rolled_back` | temp/partial directory を cleanup し、metadata 未公開 | terminal |

`branches.json` に `active` として commit する前に branch pipeline が成功してはならない。runtime map にだけ存在し metadata に存在しない branch は restart 後に消えるため、success response を返してはならない。

**branch delete 状態遷移：**

| 状態 | 必須条件 | 次状態 |
|------|----------|--------|
| `deleting` | branch lookup、auth/scope、branch exclusive lock 取得、new connection 拒否が完了 | `finalizing` |
| `finalizing` | runtime map から除去、active connection close、directory を trash へ rename、`branches.json` atomic commit | `deleted` または `delete_failed` |
| `deleted` | branch route は `DB_NOT_FOUND`。trash cleanup は完了または再試行可能 | terminal |
| `delete_failed` | metadata / directory の片方だけが残る可能性を recovery log に記録 | 起動時 recovery |

DELETE は Phase 15 では冪等 `204` とする。存在しない branch への DELETE は metadata と directory がどちらも存在しない場合だけ `204` とし、metadata 破損や directory だけ残る状態を silent success にしない。

**identity / routing / metadata：**

| 項目 | 固定仕様 |
|------|----------|
| branch name | §9.1.26 の `branch name` validation に従う。自動 lowercase / trim 禁止 |
| internal DB name | `{source_db}___{branch_name}`。external DB create では `___` を含む name を常に `DB_RESERVED_NAME` |
| metadata key | `source_db` + `branch_name` を logical key とし、internal DB name だけを primary key にしない |
| route | `/{source}___{branch}/v2/pipeline` は `branches.json` active entry と DB directory の両方がある場合だけ許可 |
| list order | §9.1.27 に従い、`created_at` 昇順、同値は `source_db`、`branch_name` 昇順 |
| response | branch API は `branch_name`、`source_db`、`db_name`、`from`、`created_at`、`state` を返す。filesystem path は返さない |

**source selector / Turso seed：**

| 入力 | 固定仕様 |
|------|----------|
| `from:"current"` | source DB の Online Backup API 相当で snapshot を作る。source write はブロックしない |
| `from:{timestamp}` | §9.1.30 の PITR timestamp selector と同じ解決規則を使う |
| `from:{frame_no}` | §9.1.30 の PITR frame selector と同じ checksum / missing frame 規則を使う |
| `/v1/* seed.type:"database"` | Phase 15 で有効化する場合、branch 名 field、source database field、group/org scope、Turso snapshot を同じ PR で固定する |
| branch 名なし seed | `INVALID_REQUEST`。source DB と同名の暗黙 branch 作成は禁止 |
| source not found | scope 判定後に `DB_NOT_FOUND`。存在漏洩は §9.1.18 に従う |

**isolation / permission / quota：**

| 項目 | 固定仕様 |
|------|----------|
| write isolation | branch write は source DB に反映しない。source write は既存 branch に反映しない |
| token scope | source DB token は branch DB へ自動拡張しない。branch DB 用 token は別途発行する |
| org/group/location | branch は source の organization、group、location を継承する。変更 API は Phase 15 対象外 |
| quota | branch 作成時に organization/group quota と branch DB candidate size を判定する。database quota は source の値を初期値として copy |
| source delete | active branch がある source DB delete は `403 ORG_SCOPE_DENIED`。cascade delete と orphan 化は禁止 |
| source block_reads | branch create は `403 PERMISSION_DENIED` |
| source block_writes | branch create は読み取り snapshot のため許可。ただし branch DB 作成先 quota / block policy は判定する |
| delete_protection | source DB の delete_protection は branch create を禁止しない。branch 自身の delete_protection は branch delete を禁止する |

**restart recovery：**

| 状態 | 起動時挙動 |
|------|------------|
| metadata active + directory missing | 起動失敗。silent cleanup 禁止 |
| metadata active + integrity_check failed | 起動失敗または branch disabled を Phase 節で明記。未定義なら起動失敗 |
| directory exists + metadata missing | partial create として WARN、接続不可、cleanup 対象 |
| trash exists + metadata deleted | cleanup 再試行。cleanup 失敗は WARN で起動継続 |
| metadata deleting + directory exists | delete recovery を実行し、完了まで branch route は `DB_NOT_FOUND` |
| duplicate branch key | metadata 破損として起動失敗。先勝ち/後勝ち禁止 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| branch lifecycle fixture | creating/materializing/verifying/active/deleting/deleted の metadata sample |
| branch race fixture | 同名 create、create 中 delete、source delete、concurrent route access |
| restart recovery fixture | metadata active + missing dir、dir only、trash cleanup、duplicate key |
| seed compatibility snapshot | `/admin/v1/*` branch API と `/v1/* seed.type:"database"` の成功/失敗差分 |
| isolation matrix | source write、branch write、token scope、quota、block_reads/block_writes |
| PITR branch fixture | timestamp/frame selector、missing frame、checksum corrupt、retention disabled |
| redaction snapshot | branch path、absolute path、token、source selector raw body が log / artifact に出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| metadata commit 前に branch route を成功させる | merge 不可 |
| source DB token を branch DB に自動適用する | merge 不可 |
| source DB delete で active branch を orphan 化する | merge 不可 |
| directory だけ存在する branch を active 扱いする | merge 不可 |
| branch create retry で二重 DB / 二重 quota charge を起こす | merge 不可 |
| branch internal DB name を通常 DB create で受理する | merge 不可 |
| branch path / absolute path を response、log、artifact に出す | merge 不可 |

branch に関係する仕様変更は、§6.4、§7.3、§9.1.16、§9.1.18、§9.1.21、§9.1.22、§9.1.26、§9.1.27、§9.1.29、§9.1.30、§9.5、§9.6、§9.14、Phase 8 / 14 / 15 詳細節、manifest の Endpoint / Persistence / Security / Compatibility / Regression map、branch lifecycle fixture、branch race fixture、restart recovery fixture、seed compatibility snapshot、isolation matrix を同時更新する。正常系だけが通っても、metadata/file commit 順序、restart recovery、source delete denial、token scope、quota、Turso seed 互換が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.32 Phase implementation packet / per-phase execution contract 固定契約

各 Phase の実装 PR は、実装開始前に Phase implementation packet を 1 つ作成し、その Phase で読むべき仕様、変更してよい範囲、変更してはならない範囲、完了証跡を 1 箇所に固定しなければならない。Phase 詳細節、§9.2、§9.4、§9.8、§9.11、§9.17 を実装者が手作業で突き合わせないと判断できない状態は、実装開始不可とする。

**Phase packet 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `phase` | `Phase N`、対象 version、対象 commit |
| `scope` | 実装する機能と、同 Phase で実装しない対象外機能 |
| `entry_sections` | 実装前に読む節番号。最低でも §9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節 |
| `target_surface` | 変更対象 module、API、CLI、config、metadata、runtime state |
| `api_contracts` | method/path/auth/request/success/error/snapshot/SDK transcript |
| `persistence_contracts` | file path、schema、update order、fsync、rollback、recovery |
| `security_contracts` | auth、scope、ro/rw、quota、block policy、redaction |
| `concurrency_contracts` | lock、idempotency、retry、shutdown、long-running operation |
| `compatibility_contracts` | Turso / libSQL SDK / legacy metadata / previous Phase への影響 |
| `evidence_plan` | test、snapshot、fixture、log、artifact、secret scan の保存先 |
| `regression_set` | 当該 Phase と過去 Phase の実行必須コマンド |
| `unsupported_behavior` | 未来 Phase、stub、501/400/404/405 の固定挙動 |
| `dependency_graph` | API、persistence、security、compatibility、oracle、evidence、operational state の prerequisite |
| `invariant_ledger` | §9.1.39 の invariant ID、scope、before/after 条件、violation signal、regression guard |
| `scenario_matrix` | §9.1.40 の normal/error/auth/persistence/rollback/concurrency/compatibility/unsupported/redaction/operational scenario |
| `resource_lifecycle_matrix` | §9.1.41 の resource type、state、allowed/forbidden transition、commit order、recovery behavior |
| `schema_registry` | §9.1.42 の field-level schema、required/null/default/migration/compatibility/redaction 契約 |
| `decision_precedence_matrix` | §9.1.43 の複数条件同時成立時の優先順位、selected behavior、losing behavior、error/status/client action |
| `coverage_closure_matrix` | §9.1.44 の Contract ID / source matrix / test / artifact / oracle / N/A reason の網羅完了表 |
| `change_impact_matrix` | §9.1.45 の実装中変更に対する影響範囲、同時更新対象、承認状態、drift closure |
| `rollout_readiness_matrix` | §9.1.46 の起動、停止、再起動、rollback、health、operator action、release 可否 |
| `compatibility_baseline_matrix` | §9.1.47 の Turso Cloud / libSQL SDK 互換 baseline、refresh trigger、差分分類、証跡 |
| `security_abuse_matrix` | §9.1.48 の attack surface、untrusted input、bypass attempt、denial、redaction、audit、regression |
| `ambiguity_closure_matrix` | §9.1.49 の implementation question、推奨決定、却下案、根拠、影響範囲、証跡 |
| `atomic_task_ledger` | §9.1.50 の task ID、入力契約、変更対象、禁止変更、完了条件、検証、rollback |
| `review_handoff_packet` | §9.1.51 の読む順番、再現 command、期待 artifact、判断基準、失敗分類、レビュー禁止事項 |
| `operator_behavior_delta` | §9.1.52 の外部挙動、運用影響、互換差分、設定移行、rollback、release note |
| `completion_gate` | merge 前に満たす Done 条件と失敗時の扱い |

**Phase group 粒度：**

| Phase group | Packet で特に固定すること |
|-------------|---------------------------|
| Phase 1〜5 | CLI、data-dir、default DB、hrana-http、JWT、ログ、SDK smoke、再起動永続性 |
| Phase 6〜8 | multi DB、admin API、Turso Platform API、metadata migration、organization/group/location/quota、legacy fallback |
| Phase 9〜10 | WebSocket stream/transaction、ATTACH policy、metrics counter 更新点、任意 path 拒否 |
| Phase 11〜13 | replication primary/replica、frame number、checksum、archive manifest、retention、health/redirect |
| Phase 14〜15 | backup/restore/PITR、branch lifecycle、destructive rollback、source snapshot、restart recovery |
| Phase 16〜18 | extension manifest/signature、metrics persistence、HA term/leader/split-brain、operator action |
| Phase 19 | internal adapter shadow/active/rollback、performance baseline、Phase 1〜18 regression、wire/API 差分ゼロ |

**実装開始禁止条件：**

| 状態 | 判定 |
|------|------|
| Phase packet がない | 実装開始禁止 |
| `scope` と §9.2 / §9.4 / Phase 詳細節が一致しない | 仕様修正 PR に戻す |
| `api_contracts` に snapshot / error case がない外部 API を追加する | route 追加禁止 |
| `persistence_contracts` に rollback / recovery がない状態変更を追加する | 書き込み処理禁止 |
| `security_contracts` に auth/scope/redaction がない管理 API を追加する | success response 禁止 |
| `regression_set` が固定されていない | 完了判定不可 |
| unsupported/stub の status と body が未定義 | future route 実装禁止 |

**完了扱い禁止条件：**

| 状態 | 判定 |
|------|------|
| Phase packet と実装差分が一致しない | Phase 未完了 |
| packet の `evidence_plan` にある artifact が生成されていない | Phase 未完了 |
| packet 外の API / config / metadata / dependency を追加している | merge 不可 |
| unsupported 対象が成功応答を返す | merge 不可 |
| regression set の一部を未実行にしている | Phase 未完了 |
| failure / flaky / TODO を後続 PR に持ち越す | Phase 未完了 |
| packet 更新が PR description のみで仕様本文にない | 仕様として扱わない |

**Phase packet と既存節の関係：**

| 既存節 | Packet への反映 |
|--------|-----------------|
| §9.2 | `scope`、`completion_gate`、対象外 |
| §9.4 | `target_surface`、Phase 別 API/永続化/error/test 契約 |
| §9.5 | `api_contracts`、API 契約 ID、snapshot |
| §9.6 | `persistence_contracts`、schema、recovery |
| §9.7 | `error code`、retry、client action |
| §9.8 | `regression_set`、test matrix |
| §9.11 | Definition of Ready / Done |
| §9.17 | 実装前チェックリスト |
| Phase 詳細節 | endpoint 固有仕様、TC、task、対象外 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| phase packet artifact | `docs/phase-evidence/phase-{phase}/packet.md` または PR description の同等表 |
| contract mapping | API / Persistence / Security / Compatibility / Regression の各 map |
| unsupported snapshot | future route / config / body field の拒否結果 |
| regression transcript | fixed command、exit code、環境、artifact path |
| cross-reference scan | §9.1.29 の self-check 結果 |
| redaction scan | packet / artifact / log に secret、token、raw path、SQL args、backup body がないこと |

Phase packet に関係する仕様変更は、§9.1.9、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.29、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。Phase 単位で何を実装し、何を実装しないか、何をもって完了とするかが 1 箇所で読めない場合は、実装精度不足として Phase 未完了扱いにする。

#### 9.1.33 Phase completion gate / Done evidence / bug-zero acceptance 固定契約

各 Phase の完了判定は、コード差分の有無ではなく、Phase packet、受入 manifest、仕様本文、test、artifact、PR description、release check result が同じ契約集合を証明していることをもって行う。実装者、reviewer、後続 Phase 担当者が仕様本文だけで完了可否を再判定できない場合、その Phase は未完了とする。

**Done receipt 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `phase` | `Phase N`、対象仕様 version、対象 commit、対象 PR |
| `packet_ref` | §9.1.32 の Phase packet 参照と commit |
| `manifest_ref` | §9.1.10 の受入 manifest 参照と version |
| `implemented_scope` | §9.2 / §9.4 / Phase 詳細節で完了した項目 |
| `excluded_scope` | 同 Phase で意図的に実装しない対象と、その固定挙動 |
| `contract_map` | API、永続化、error、security、compatibility、concurrency、operation の契約 ID |
| `evidence_index` | artifact 名、保存先、生成コマンド、exit code、正規化内容 |
| `regression_result` | §9.8 の対象 TC、過去 Phase regression、SDK / CLI / HTTP / WS 実行結果 |
| `failure_closure` | 失敗、flaky、未検証、artifact 欠落、secret 混入が 0 件である根拠 |
| `compatibility_result` | Turso Cloud / libSQL SDK / legacy metadata / previous Phase への差分分類 |
| `redaction_result` | log、artifact、PR description、error body に secret / token / raw path / SQL args が残っていない根拠 |
| `release_check_result` | local / Docker / CI の実行コマンド、環境差分、再実行条件 |
| `oracle_result` | §9.1.35 の oracle 名、version、path、比較 command、exit code、差分理由 |
| `defect_classification_result` | §9.1.36 の defect 件数、分類別件数、`open:0`、evidence path |
| `operational_state_result` | §9.1.37 の state matrix、health snapshot、API behavior、recovery runbook |
| `dependency_graph_result` | §9.1.38 の prerequisite がすべて satisfied である証跡 |
| `invariant_result` | §9.1.39 の invariant ID ごとの pass/fail、violation 0 件、regression guard、evidence path |
| `scenario_matrix_result` | §9.1.40 の scenario ID ごとの pass/fail、not_applicable reason、evidence path、manual only 0 件 |
| `resource_lifecycle_result` | §9.1.41 の state / transition ごとの pass/fail、forbidden transition 0 件、recovery evidence path |
| `schema_registry_result` | §9.1.42 の schema ID ごとの field coverage、unknown/null/default/migration/redaction evidence |
| `decision_precedence_result` | §9.1.43 の decision ID ごとの precedence 実行結果、selected behavior、error/status snapshot、losing behavior 非発火証跡 |
| `coverage_closure_result` | §9.1.44 の coverage ID ごとの pass/fail/N/A、gap 0 件、test/artifact/oracle 実在証跡 |
| `change_impact_result` | §9.1.45 の change ID ごとの affected sections/matrices/tests/artifacts 更新完了、drift 0 件、承認証跡 |
| `rollout_readiness_result` | §9.1.46 の rollout ID ごとの startup/shutdown/restart/rollback/health/operator/compat/data safety 証跡 |
| `compatibility_baseline_result` | §9.1.47 の baseline ID ごとの upstream source、snapshot / SDK version、差分分類、refresh 可否、証跡 |
| `security_abuse_result` | §9.1.48 の security case ID ごとの bypass denial、redaction、audit/log、quota/rate/persistence、証跡 |
| `ambiguity_closure_result` | §9.1.49 の ambiguity ID ごとの採用決定、却下案、仕様反映、証跡、open ambiguity 0 件 |
| `atomic_task_result` | §9.1.50 の task ID ごとの完了条件、検証 command、証跡、rollback 可否、open task 0 件 |
| `review_handoff_result` | §9.1.51 の第三者再現 command、artifact、判断結果、失敗分類、口頭補足なし証跡 |
| `operator_behavior_delta_result` | §9.1.52 の external behavior、operator impact、compatibility delta、migration、rollback、release note 証跡 |
| `reviewer_decision` | `Done`、`Not Done`、`Spec correction required` のいずれか |

**Phase group Done minimum：**

| Phase group | 最低証跡 |
|-------------|----------|
| Phase 1〜5 | build/test、CLI help、config precedence、data-dir 再起動、hrana-http transcript、JWT / log redaction、SDK smoke |
| Phase 6〜8 | admin API / Platform API snapshot、metadata migration、auth scope、organization/group/location/quota、legacy fallback、Phase 1〜7 regression |
| Phase 9〜10 | WebSocket transcript、transaction rollback、managed ATTACH 拒否、metrics counter 更新、任意 path 拒否 |
| Phase 11〜13 | replication frame、checksum、snapshot、archive manifest、retention cleanup、corruption detection、primary/replica restart |
| Phase 14〜15 | backup manifest、restore rollback、PITR selector、branch lifecycle、source snapshot、seed isolation、restart recovery |
| Phase 16〜18 | extension signature/load policy、metrics persistence、Prometheus output、HA term、leader transition、split-brain prevention |
| Phase 19 | adapter shadow/active/rollback、wire/API 差分、performance baseline、Phase 1〜18 full regression |

**完了扱い禁止条件：**

| 状態 | 判定 |
|------|------|
| Done receipt がない | Phase 未完了 |
| Done receipt と Phase packet / manifest / 仕様本文が一致しない | Phase 未完了 |
| evidence artifact が再生成不能、または生成コマンドが未記載 | Phase 未完了 |
| regression set の一部が skipped / manual only / not run のまま | Phase 未完了 |
| unsupported 対象が success response、部分成功、暗黙 fallback を返す | merge 不可 |
| failure / flaky / TODO / FIXME / unimplemented を後続 PR に持ち越す | merge 不可 |
| secret / token / raw path / SQL args / backup body の秘匿確認がない | merge 不可 |
| compatibility diff が仕様化されていない | 仕様修正 PR に戻す |
| destructive operation、auth、quota、restore、branch delete、extension load の検証が manual only | merge 不可 |

**bug-zero acceptance：**

| 項目 | 必須条件 |
|------|----------|
| known bug | 0 件。既知不具合、既知 flaky、既知未検証、既知 artifact 欠落を残さない |
| detected bug | 同一 PR 内で code fix + test、または仕様誤りとして spec fix + test/evidence に変換する |
| regression bug | 影響 Phase、契約 ID、TC ID、再発防止 evidence を Done receipt に記録する |
| accepted risk | 使用禁止。risk acceptance ではなく仕様変更、対象外明記、または実装修正で閉じる |
| follow-up | 完了条件の代替に使わない。follow-up は Done 後の改善のみ許可する |

**必須 evidence artifact：**

| Artifact | 必須内容 |
|----------|----------|
| Done receipt | `docs/phase-evidence/phase-{phase}/done.md` または PR description の同等表 |
| contract coverage matrix | Phase packet の全契約 ID と実行結果の対応 |
| regression transcript | command、exit code、環境、artifact path、正規化済み出力 |
| failure closure log | 発生した失敗、原因、修正、再実行結果、残件 0 の宣言 |
| unsupported snapshot | 未来 Phase、対象外 endpoint / field / config / operation の拒否結果 |
| redaction scan | secret、token、raw path、SQL args、backup body が残っていない確認 |
| compatibility diff | Turso Cloud、libSQL SDK、legacy metadata、previous Phase との差分分類 |
| cross-reference scan | §9.1.29 の参照整合 self-check 結果 |

Phase completion gate に関係する仕様変更は、§9.1.9、§9.1.10、§9.1.11、§9.1.14、§9.1.24、§9.1.29、§9.1.32、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。Done receipt を満たさない Phase は、機能が動作していても仕様上は完了扱いにしない。

#### 9.1.34 Phase execution sequence / handoff / interruption 固定契約

各 Phase の実装は、Phase packet と Done receipt の間に固定された execution sequence を持たなければならない。実装者は、どの順番で契約を確定し、どの順番でコードへ反映し、どの時点で API を公開してよいかを Phase 開始前に固定する。実装途中で担当者が変わる、中断する、失敗を検出する、未来 Phase の helper を先に作る場合でも、この節に従い現在位置と次の作業を判定する。

**Execution sequence 必須 step：**

| Step | 名称 | 完了条件 | 次 step へ進む条件 |
|------|------|----------|--------------------|
| 0 | intake | `AGENTS.md`、仕様 version、branch 整合性、対象 Phase を確認する | local / remote が一致し、対象 Phase が明記されている |
| 1 | packet freeze | §9.1.32 の Phase packet と §9.1.10 の manifest を作成する | scope、target surface、unsupported behavior、regression set が固定済み |
| 2 | contract freeze | schema、error、persistence、auth、compatibility、concurrency 契約を確定する | §9.5 / §9.6 / §9.7 / Phase 詳細節の参照が一致済み |
| 3 | persistence / recovery | metadata、migration、atomic update、rollback、recovery を実装する | 破損、再起動、rollback の証跡計画が存在する |
| 4 | core logic | API 非公開の service logic、validation、permission、state transition を実装する | internal tests または unit tests で契約境界が確認済み |
| 5 | external surface | HTTP / WebSocket / CLI / config / admin API を公開する | auth、error、unsupported、redaction、compatibility の snapshot が固定済み |
| 6 | evidence run | normal、error、auth、persistence、restart、rollback、regression、secret scan を実行する | §9.1.11 の artifact が生成済み |
| 7 | Done receipt | §9.1.33 の Done receipt を作成し、failure closure を 0 件にする | reviewer が `Done` と判定できる |

**Handoff state 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `phase` | 対象 Phase、仕様 version、branch、commit |
| `current_step` | 上記 execution sequence の step 番号 |
| `completed_contracts` | 完了済み API / persistence / error / security / compatibility / regression 契約 ID |
| `open_contracts` | 未完了契約 ID と未完了理由 |
| `last_successful_command` | 最後に成功した command、exit code、artifact path |
| `last_failed_command` | 最後に失敗した command、exit code、原因、再実行条件。失敗がない場合は `none` |
| `allowed_next_changes` | 次に変更してよい file / module / spec 節 |
| `forbidden_changes` | 現在 step で変更してはいけない API / metadata / config / behavior |
| `next_command` | 次に実行する verification command |
| `blocking_decision` | ユーザー承認、仕様変更、依存修正、環境復旧など必要な判断。不要な場合は `none` |

**前倒し実装の扱い：**

| 前倒し内容 | 許可条件 | 禁止条件 |
|------------|----------|----------|
| internal helper / type | 外部 API、永続化 schema、config default、wire format を変えない | success response や metadata commit に接続する |
| future route stub | 501 `NOT_IMPLEMENTED` と固定 body のみ返す | 200 / 201 / 204 / 307 / partial success を返す |
| future metadata file | §9.6 に初期値、破損時挙動、読み飛ばし可否が明記済み | migration なしで既存起動 path に必須化する |
| future config key | default、validation、unknown key handling、secret redaction が明記済み | default 挙動を変える、または未定義 key を有効化する |
| future dependency | feature flag off で既存挙動に影響しない | build/test/release-check を遅くするだけの未使用依存 |

**中断・再開ルール：**

| 状態 | 必須対応 |
|------|----------|
| Step 1 前に中断 | Phase 実装未開始として扱い、再開時に packet を作り直す |
| Step 2〜4 で中断 | Handoff state を残し、API success response を公開しない |
| Step 5 で中断 | 公開 surface を 501 / auth reject / feature flag off のいずれかに戻す |
| Step 6 で失敗 | §9.1.14 の failure closure に記録し、同一 PR で修正または仕様修正する |
| Step 7 前に未検証が残る | Done receipt 作成禁止 |
| 再開時に仕様 version が進んでいる | Phase packet、manifest、contract map を新 version に合わせて再確認する |

**実装順序違反の扱い：**

| 違反 | 判定 |
|------|------|
| persistence / rollback 未確定で external API を公開する | merge 不可 |
| error code 未確定で route を追加する | merge 不可 |
| auth / quota / scope 未確定で管理 API success response を返す | merge 不可 |
| migration / recovery 未確定で metadata schema を変更する | Phase 未完了 |
| test / artifact より先に Done receipt を作成する | Phase 未完了 |
| handoff state なしで途中作業を引き継ぐ | Phase 未完了 |

Phase execution sequence に関係する仕様変更は、§9.1.9、§9.1.10、§9.1.11、§9.1.14、§9.1.32、§9.1.33、§9.2、§9.4、§9.8、§9.11、§9.16、§9.17、該当 Phase 詳細節を同時更新する。実装者が現在 step、次 step、禁止変更を仕様本文だけで判断できない場合は、実装開始不可とする。

#### 9.1.35 Phase acceptance oracle / golden fixture 固定契約

各 Phase の完了判定には、実装結果を照合する acceptance oracle を持たなければならない。oracle は「実装が出した結果」ではなく「仕様が要求する正解」であり、snapshot、fixture、transcript、manifest、baseline のいずれかとして保存する。実装者が期待値を実装後に都合よく作る、snapshot 更新で差分を隠す、手元目視だけで一致扱いにすることは禁止する。

**Oracle 必須 artifact：**

| Artifact | 必須内容 | 適用 Phase |
|----------|----------|------------|
| API snapshot | method、path、status、headers subset、body schema、error body、content type | API を公開する全 Phase |
| error snapshot | error code、HTTP status、wire surface、retry、client action、precedence | 全 Phase |
| persistence fixture | metadata JSON、DB/file layout、atomic update 後状態、破損 fixture、recovery 後状態 | 永続化を変更する全 Phase |
| migration fixture | old format、new format、missing field、unknown field、corrupt file、rollback marker | metadata schema を変更する Phase |
| SDK transcript | TypeScript SDK / libSQL SDK / WebSocket client の request/response transcript | Phase 5 以降、WebSocket は Phase 9 以降 |
| unsupported snapshot | 未来 Phase、対象外 API、未対応 field、未対応 config、stub route の拒否結果 | 全 Phase |
| security fixture | auth failure、scope denial、quota denial、redaction、secret scan 結果 | auth / admin / quota / replication / HA / extension Phase |
| compatibility baseline | Turso Cloud、libSQL SDK、legacy metadata、previous Phase との差分分類 | Phase 5 以降 |
| regression baseline | 該当 Phase より前の TC / command / snapshot の再実行結果 | 全 Phase |

**Phase group oracle minimum：**

| Phase group | 最低 oracle |
|-------------|-------------|
| Phase 1〜5 | CLI help snapshot、config precedence fixture、hrana-http snapshot、JWT/auth error snapshot、log redaction fixture、TypeScript SDK transcript |
| Phase 6〜8 | admin API snapshot、Turso Platform API snapshot、metadata migration fixture、org/group/location/quota fixture、scope denial matrix、legacy fallback fixture |
| Phase 9〜10 | hrana-ws transcript、transaction commit/rollback fixture、store_sql cache snapshot、ATTACH allow/deny fixture、metrics counter baseline |
| Phase 11〜13 | replication log/snapshot transcript、frame checksum fixture、replica catchup fixture、archive manifest fixture、retention cleanup fixture |
| Phase 14〜15 | backup snapshot manifest、restore rollback fixture、PITR selector fixture、branch seed fixture、branch isolation/restart fixture |
| Phase 16〜18 | extension manifest/signature fixture、extension load failure snapshot、metrics snapshot/prometheus baseline、HA term/leader fixture、split-brain rejection transcript |
| Phase 19 | adapter shadow/active/rollback baseline、wire/API diff snapshot、performance baseline、Phase 1〜18 regression baseline |

**Snapshot / fixture 更新条件：**

| 状態 | 判定 |
|------|------|
| 仕様変更 commit なしに expected snapshot を更新する | merge 不可 |
| 実装差分だけを理由に oracle を更新する | merge 不可 |
| dynamic 値を placeholder 正規化せずに snapshot 化する | Phase 未完了 |
| secret、token、JWT signature、raw path、SQL args、backup body が oracle に残る | merge 不可 |
| failed / skipped / flaky の artifact を oracle として採用する | Phase 未完了 |
| body schema だけで status、header、error code を比較しない | Phase 未完了 |
| snapshot 差分理由が仕様本文にない | 仕様修正 PR に戻す |
| oracle 更新と実装修正を同じ PR で行う場合に差分理由がない | review failure |

**Oracle 正規化ルール：**

| 値 | 正規化 |
|----|--------|
| timestamp | `<timestamp:rfc3339>`。秒精度より細かい値は比較対象にしない |
| UUID / generated id | `<id:{prefix}>`。prefix と形式だけ比較する |
| request id | `<request_id>` |
| token / JWT / secret | `<redacted>`。生値が残る場合は failure |
| data-dir / absolute path | `<data_dir>` または `<tmp_dir>` |
| SQL args / backup body | `<redacted>`。値の有無と型だけ比較する |
| JSON object key order | 辞書順へ正規化する |
| array order | API 契約で定義した順序を保持する。test 側で sort し直さない |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `oracle_result` | 使用した oracle 名、version、path、生成/比較 command、exit code |
| `oracle_diff` | 差分なし、または仕様変更に基づく差分理由と参照 commit |
| `oracle_update_policy` | snapshot / fixture を更新した場合の仕様変更参照。更新なしなら `not updated` |

**Oracle 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| production code の現在出力をそのまま expected として採用する | merge 不可 |
| regression failure を snapshot 更新で消す | merge 不可 |
| Turso Cloud / libSQL SDK 互換差分を自己ホスト都合だけで許容する | 仕様修正 PR に戻す |
| unsupported API の success response を snapshot として固定する | merge 不可 |
| human-readable log だけを正解 artifact にする | Phase 未完了 |
| oracle なしで Done receipt を作成する | Phase 未完了 |

Phase acceptance oracle に関係する仕様変更は、§9.1.3、§9.1.10、§9.1.11、§9.1.23、§9.1.24、§9.1.27、§9.1.28、§9.1.29、§9.1.33、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。oracle が固定されていない機能は、実装が動作していても Phase 完了扱いにしない。

#### 9.1.36 Phase defect classification / zero-bug triage 固定契約

各 Phase の実装中に見つかった不具合、仕様不足、互換差分、検証不足は、発見時点で defect classification に記録し、同一 PR 内で閉じなければならない。分類されていない失敗、分類済みだが処理方針が未確定の失敗、後続 PR に送られた失敗が 1 件でもある場合、その Phase は未完了とする。

**Defect classification 必須分類：**

| Classification | 定義 | 必須対応 | 完了判定 |
|----------------|------|----------|----------|
| `spec_gap` | 仕様本文に API、error、永続化、auth、test、oracle、対象外の記述がない | 実装を止め、仕様修正 PR として本文を先に更新する | 仕様更新後に packet / manifest / oracle を再固定するまで実装再開不可 |
| `implementation_bug` | 仕様は明確だが実装が満たしていない | 同一 PR 内で code fix、test、evidence を追加する | failure closure が 0 件になれば完了可 |
| `regression_bug` | 過去 Phase の契約、TC、snapshot、SDK 互換が壊れた | 対象 Phase 未完了として同一 PR 内で修正する | regression baseline が再度 pass するまで完了不可 |
| `compatibility_diff` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase と差分が出た | §9.1.23 と §9.1.35 に従い差分理由を仕様化する | 差分理由なしは merge 不可 |
| `oracle_gap` | 正解 snapshot / fixture / transcript / baseline が不足している | oracle を追加し、仕様変更根拠を明記する | oracle 追加前の実装完了は禁止 |
| `environment_gap` | local / Docker / CI / release-check の環境差で結果が揺れる | §9.1.24 の environment artifact と再現条件を更新する | 環境差分が再現可能になるまで完了不可 |
| `security_gap` | auth、scope、quota、redaction、secret scan、任意 path 拒否に不足がある | success response を禁止し、同一 PR 内で仕様または実装を修正する | merge 不可。manual only 禁止 |
| `persistence_gap` | atomic update、fsync、rollback、migration、recovery、破損時挙動が不足している | 書き込み処理を止め、§9.1.21 / §9.6 / Phase 詳細節を修正する | merge 不可。rollback evidence 必須 |

**Defect record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `id` | `DEF-P{phase}-{number}` |
| `classification` | 上記 8 分類のいずれか |
| `detected_by` | test、review、oracle diff、CI、manual observation、Turso diff など |
| `contract_id` | 関連する API / persistence / error / security / compatibility / regression 契約 ID |
| `symptom` | 観測された失敗。推測ではなく artifact / command / snapshot path を含める |
| `root_cause` | 仕様不足、実装誤り、oracle 不足、環境差分などの確定原因 |
| `resolution` | spec fix、code fix、test fix、oracle追加、environment fix のいずれか |
| `evidence` | 修正後の command、exit code、artifact path |
| `status` | `open`、`fixed`、`spec_updated`、`not_a_defect`。Phase 完了時は `open` 禁止 |

**分類別禁止事項：**

| 状態 | 判定 |
|------|------|
| `spec_gap` を実装判断で埋める | merge 不可 |
| `implementation_bug` を既知課題として残す | Phase 未完了 |
| `regression_bug` を対象外扱いにする | merge 不可 |
| `compatibility_diff` を自己ホスト都合だけで許容する | 仕様修正 PR に戻す |
| `oracle_gap` を現行出力 snapshot 更新で隠す | merge 不可 |
| `environment_gap` を local pass だけで完了扱いにする | Phase 未完了 |
| `security_gap` / `persistence_gap` を follow-up に送る | merge 不可 |
| `not_a_defect` にした理由が仕様本文にない | review failure |

**zero-bug triage 処理順：**

| 順序 | 処理 | 失敗時 |
|------|------|--------|
| 1 | failure / diff / skipped / flaky / manual only をすべて列挙する | Phase 未完了 |
| 2 | 各項目に classification と contract_id を付与する | Phase 未完了 |
| 3 | `spec_gap` / `oracle_gap` / `compatibility_diff` は仕様本文を先に更新する | 実装継続禁止 |
| 4 | `implementation_bug` / `regression_bug` は同一 PR 内で修正し test を追加する | merge 不可 |
| 5 | `security_gap` / `persistence_gap` は success response と write path を停止する | merge 不可 |
| 6 | 修正後に oracle、regression、secret scan、release-check を再実行する | Done receipt 作成禁止 |
| 7 | defect record の `open` が 0 件であることを Done receipt に記録する | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `defect_classification_result` | defect 件数、分類別件数、`open:0`、各 defect の evidence path |
| `triage_result` | zero-bug triage の 1〜7 が完了したこと |
| `not_a_defect_decisions` | `not_a_defect` 判定がある場合の仕様本文参照。なければ `none` |

**完了根拠として禁止する表現：**

| 表現 | 扱い |
|------|------|
| `known issue` / `既知課題` | Phase 未完了 |
| `minor` / `軽微` | defect classification がない限り無効 |
| `later` / `follow-up` / `後続 Phase で対応` | 完了条件の代替に使えない |
| `manual only` / `手元確認済み` | §9.1.10 の Manual exception がない限り無効 |
| `accepted risk` | 使用禁止。仕様変更または修正で閉じる |
| `works for me` / `想定通り` | artifact と oracle がない限り無効 |

Phase defect classification に関係する仕様変更は、§9.1.1、§9.1.1a、§9.1.3、§9.1.10、§9.1.11、§9.1.14、§9.1.23、§9.1.24、§9.1.33、§9.1.35、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。defect classification がない失敗を残したまま Phase を完了扱いにしてはならない。

#### 9.1.37 Phase operational state / recovery runbook 固定契約

各 Phase は、実装対象 resource が正常時、劣化時、復旧中、rollback 必須時、operator 判断必須時にどの API 応答、health、log、write 可否、retry 可否を示すかを固定しなければならない。壊れた状態を `ok` と表示すること、operator 判断が必要な状態を自動成功扱いにすること、recovery 中に成功応答を返すことは禁止する。

**Operational state 固定表：**

| State | 定義 | API success | Write | Health | Log | Operator action |
|-------|------|-------------|-------|--------|-----|-----------------|
| `healthy` | 対象 resource が仕様通り利用可能 | 許可 | 許可 | `ok` | `INFO` | 不要 |
| `degraded` | read または一部機能は可能だが lag、retry、再取得、縮退がある | read は Phase 節で許可可。write は Phase 節で明記した場合のみ | 原則禁止。許可する場合は data loss なしを証明 | `degraded` | `WARN` | 状態確認または復旧判断 |
| `unavailable` | 対象 resource を安全に利用できない | 禁止。規定 error を返す | 禁止 | `unavailable` | `ERROR` | 必要 |
| `recovering` | 起動時または job recovery が進行中 | 対象 resource は成功応答禁止 | 禁止 | `recovering` | `INFO` / `WARN` | 原則不要。ただし timeout 時は必要 |
| `rollback_required` | 旧状態へ戻す必要があるが未完了 | 禁止 | 禁止 | `degraded` または `unavailable` | `ERROR` | 自動 rollback 可否を runbook で判定 |
| `operator_required` | 自動復旧が安全でない、または外部判断が必要 | 禁止 | 禁止 | `degraded` または `unavailable` | `ERROR` | 必須 |
| `blocked` | auth、quota、config、scope、policy により意図的に拒否 | 規定 error のみ | 禁止 | 原則 `ok`。system 健全性は壊れていない | `WARN` または audit 相当 | 設定変更または権限変更 |

**Runbook 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `state` | 上記 operational state のいずれか |
| `detection_condition` | state を検出する条件。file marker、checksum、health probe、job status、error code、fixture を含める |
| `affected_resource` | DB、branch、metadata file、replica、archive、extension、HA node、internal adapter など |
| `api_behavior` | 対象 API の status、body、error code、retry header、partial response 可否 |
| `write_policy` | write 許可 / 拒否、拒否 error、既存 transaction の扱い |
| `health_response` | `GET /v2/health` または該当 health/status endpoint の response field |
| `automatic_recovery` | 自動 recovery が許可される条件、禁止される条件、timeout |
| `operator_action` | 必要な手動操作、承認、復旧手順。不要な場合は `none` |
| `retry_condition` | client retry 可否、server retry 可否、backoff、再実行 idempotency |
| `evidence_artifact` | recovery log、health snapshot、error snapshot、restart fixture、operator marker |

**Phase group operational minimum：**

| Phase group | 必須 state / runbook |
|-------------|----------------------|
| Phase 1〜5 | 起動失敗、config invalid、DB open failure、lock held、health `ok` / `unavailable` |
| Phase 6〜8 | metadata migration failure、legacy fallback failure、quota blocked、scope denial、usage unavailable |
| Phase 9〜10 | WebSocket tx rollback、disconnect recovery、ATTACH blocked、metrics read degraded |
| Phase 11〜13 | replica lag degraded、primary unavailable、checksum mismatch、archive manifest corruption、retention cleanup failure |
| Phase 14〜15 | restore rollback、`restore-failed.json`、PITR archive missing、branch delete recovery、source unavailable |
| Phase 16〜18 | extension load failure、metrics snapshot corruption、HA no leader、candidate state、split-brain operator_required |
| Phase 19 | adapter shadow diff、active failure、rollback flag、performance blocked、internal path unavailable |

**自動 recovery 許可 / 禁止：**

| 状態 | 自動 recovery |
|------|---------------|
| temp file のみ残存し target が正常 | 許可。temp cleanup と WARN log |
| commit marker 前の interrupted migration | rollback または起動失敗。仕様にない推測 migration 禁止 |
| checksum mismatch | 対象 artifact を使用禁止にし、再取得または operator_required |
| `restore-failed.json` 存在 | 自動復旧禁止。対象 DB は write 禁止、operator_required |
| HA split-brain | 自動 primary 昇格禁止。operator_required |
| extension signature mismatch | 自動許可禁止。extension unavailable |
| internal adapter active failure | silent fallback 禁止。rollback flag または operator_required |

**API / health 禁止事項：**

| 状態 | 判定 |
|------|------|
| `degraded` / `unavailable` / `recovering` を health `ok` のみで返す | merge 不可 |
| rollback 未完了 resource に 2xx success を返す | merge 不可 |
| operator_required を自動修復して成功扱いにする | merge 不可 |
| blocked と degraded を同じ error code で返す | Phase 未完了 |
| recovery 中に partial list / partial metadata を返す | Phase 未完了 |
| recovery log に secret、token、raw path、SQL args、backup body を出す | merge 不可 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `operational_state_result` | state matrix、health snapshot、API behavior snapshot、write policy evidence |
| `recovery_runbook_result` | runbook fields、automatic recovery 可否、operator action、restart fixture |
| `operator_required_result` | operator_required がある場合の marker、health、manual action、解除条件。なければ `none` |

Phase operational state に関係する仕様変更は、§6.4、§7.3、§9.1.16、§9.1.21、§9.1.22、§9.1.25、§9.1.30、§9.1.31、§9.1.33、§9.1.36、§9.5、§9.6、§9.7、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。operational state と recovery runbook が固定されていない failure path は、実装が正常系で動いていても Phase 完了扱いにしない。

#### 9.1.38 Phase dependency graph / contract prerequisite 固定契約

各 Phase の実装 PR は、実装開始前に Phase 内 contract の dependency graph を固定しなければならない。dependency graph は「どの契約が満たされていなければ、次の契約を実装・公開・完了扱いにできないか」を示す実装順序の正本である。prerequisite が未完了の契約を実装済み、証跡済み、または Done として扱ってはならない。

**Contract prerequisite 固定表：**

| Contract type | prerequisite | prerequisite 未完了時の扱い |
|---------------|--------------|------------------------------|
| API contract | error contract、auth/security contract、persistence contract、oracle、unsupported behavior | route 追加禁止。stub は §9.1.5 / §9.1.34 に従う |
| Persistence contract | schema、atomic update、fsync、rollback、recovery、operational state | 書き込み処理禁止。metadata commit 禁止 |
| Security contract | auth source、scope、quota/block policy、denial snapshot、redaction artifact | success response 禁止 |
| Compatibility contract | Turso / libSQL SDK / legacy metadata 差分、oracle、migration / rollback 方針 | 互換差分を伴う実装禁止 |
| Concurrency contract | resource lock、idempotency、shutdown、retry、transaction boundary | 状態変更処理禁止 |
| Oracle contract | Contract ID、expected artifact、normalization、update policy | snapshot / fixture / expected 更新禁止 |
| Evidence contract | artifact path、generation command、secret scan、manifest linkage | Phase 完了扱い禁止 |
| Operational contract | state matrix、health response、write policy、operator action、recovery runbook | failure path 実装禁止 |
| Defect contract | classification、root cause、resolution、evidence、open:0 | Done receipt 作成禁止 |

**Dependency graph 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `contract_id` | §9.1.7 の Contract ID |
| `contract_type` | API / Persistence / Security / Compatibility / Concurrency / Oracle / Evidence / Operational / Defect |
| `requires` | prerequisite Contract ID の配列。不要な場合は `[]` |
| `blocks` | この契約が未完了の場合に止める契約 ID または operation |
| `status` | `planned`、`satisfied`、`blocked`、`not_applicable` のいずれか |
| `evidence` | satisfied 判定に使う artifact path / command / section |
| `not_applicable_reason` | `not_applicable` の場合のみ、仕様本文の根拠 |

**Phase group dependency minimum：**

| Phase group | 必須 dependency |
|-------------|-----------------|
| Phase 1〜5 | CLI/config prerequisite、DB open 前の data-dir contract、hrana API 前の SQL/error/oracle contract |
| Phase 6〜8 | DB route 前の naming/persistence/security contract、Turso API 前の org/group/location/quota/oracle contract |
| Phase 9〜10 | WebSocket execute 前の auth/stream/tx contract、ATTACH 前の parser/security/persistence contract |
| Phase 11〜13 | replication response 前の frame/checksum/auth contract、archive cleanup 前の manifest/recovery contract |
| Phase 14〜15 | restore/branch success 前の rollback/source/quota/security/operational contract |
| Phase 16〜18 | extension load 前の allowlist/signature/redaction contract、HA promotion 前の term/operator/split-brain contract |
| Phase 19 | adapter active 前の shadow/oracle/performance/rollback/full regression contract |

**未完了 prerequisite の禁止事項：**

| 状態 | 判定 |
|------|------|
| prerequisite が `planned` / `blocked` のまま dependent API を公開する | merge 不可 |
| persistence prerequisite 未完了で metadata を commit する | merge 不可 |
| security prerequisite 未完了で 2xx success を返す | merge 不可 |
| oracle prerequisite 未完了で snapshot を更新する | merge 不可 |
| operational prerequisite 未完了で recovery / degraded path を実装する | Phase 未完了 |
| defect prerequisite 未完了で Done receipt を作成する | Phase 未完了 |
| dependency graph を PR description だけに置き、仕様本文または artifact と対応しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `dependency_graph_result` | Contract ID ごとの status、blocked 0 件、not_applicable reason、evidence path |
| `blocked_contracts` | `blocked` がある場合は Phase 未完了。完了時は `none` |
| `prerequisite_closure` | dependency graph 上の全 dependent contract が prerequisite satisfied 後に検証済みであること |

Phase dependency graph に関係する仕様変更は、§9.1.7、§9.1.8、§9.1.10、§9.1.11、§9.1.32、§9.1.33、§9.1.34、§9.1.35、§9.1.36、§9.1.37、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。dependency graph がない Phase 実装 PR は、実装順序と完了判定が未確定として扱い、実装開始不可とする。

#### 9.1.39 Phase invariant ledger / non-regression invariant 固定契約

各 Phase の実装 PR は、実装開始前に Phase invariant ledger を固定しなければならない。invariant ledger は「その Phase と過去 Phase で、実装後も絶対に壊れてはならない条件」の正本である。単体の API、永続化、security、oracle、dependency graph が満たされていても、invariant が 1 件でも破れている場合、その Phase は完了扱いにしない。

**Invariant ledger 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `invariant_id` | `INV-P{phase}-{domain}-{number}` 形式の一意 ID |
| `scope` | API、persistence、security、compatibility、operation、redaction、dependency の対象範囲 |
| `phase` | invariant を導入する Phase と、継続して守る Phase 範囲 |
| `must_hold_before` | 実装前、migration 前、state transition 前に成立している必要条件 |
| `must_hold_after` | success response、commit、restart、rollback、operator action 後に成立している結果条件 |
| `violation_signal` | 破れた場合に返す HTTP status、error code、health state、log event、defect classification |
| `evidence` | invariant を証明する test、snapshot、fixture、log、manifest、artifact path |
| `regression_guard` | 過去 Phase で再実行する TC / command / oracle |
| `owner_contract` | invariant を所有する API / persistence / security / operational Contract ID |

**Invariant category 固定表：**

| Category | 必ず守る条件 | 違反時の扱い |
|----------|--------------|--------------|
| Durability invariant | success response は metadata / DB file / archive / branch state の fsync または明示された durable boundary 後にのみ返す | merge 不可。success-before-fsync は critical defect |
| Metadata/file consistency invariant | metadata と実ファイル、manifest、archive、branch source、replication frame の参照が相互に存在し、片側だけの commit を残さない | recovery / rollback 必須。未解消なら Phase 未完了 |
| Auth/scope/quota invariant | auth、scope、read-only/write、quota、block policy、organization/group/location boundary を bypass できない | 2xx success 禁止。security gap として open:0 まで完了不可 |
| Compatibility invariant | Turso Cloud API、libSQL SDK、legacy metadata、previous Phase の既存 response/error/snapshot を理由なしに変えない | compatibility diff の仕様化と oracle 更新まで merge 不可 |
| Error surface invariant | 同じ原因は同じ HTTP status、error code、body shape、client action、retry policy で返す | `INTERNAL_ERROR` への逃避禁止。error contract 修正まで未完了 |
| Operational state invariant | healthy、degraded、unavailable、recovering、rollback_required、operator_required、blocked の write policy と health response が矛盾しない | operator action または recovery runbook なしの実装禁止 |
| Redaction invariant | token、secret、raw filesystem path、SQL args、backup body、private metadata を log、artifact、PR description、error body に出さない | secret 混入は merge 不可。artifact 再生成必須 |
| Dependency/prerequisite invariant | prerequisite が satisfied になる前に dependent API、write、snapshot 更新、Done receipt を公開しない | dependent contract は未実装扱い |

**Phase group invariant minimum：**

| Phase group | 最低 invariant |
|-------------|----------------|
| Phase 1〜5 | CLI/config precedence、data-dir boundary、default DB identity、hrana-http error surface、JWT scope、log redaction、restart persistence |
| Phase 6〜8 | DB identity、admin API auth、Turso metadata shape、organization/group/location/quota boundary、legacy metadata migration、fallback 禁止条件 |
| Phase 9〜10 | WebSocket transaction atomicity、stream close semantics、managed ATTACH path isolation、metrics counter consistency、arbitrary path denial |
| Phase 11〜13 | replication frame monotonicity、checksum consistency、snapshot/archive manifest consistency、retention cleanup safety、primary/replica health transition |
| Phase 14〜15 | restore/PITR rollback safety、branch source selector、branch isolation、destructive operation lock、restart recovery marker、quota precedence |
| Phase 16〜18 | extension allowlist/signature boundary、metrics persistence、HA term monotonicity、leader election safety、split-brain prevention、operator-required transition |
| Phase 19 | internal adapter wire/API parity、shadow/active/rollback boundary、full regression non-regression、performance baseline、Turso Cloud compatibility gap closure |

**Invariant violation handling：**

| 状態 | 判定 |
|------|------|
| invariant violation が 1 件でも残る | Phase 未完了 |
| invariant violation 中に 2xx success を返す | merge 不可 |
| violation を `INTERNAL_ERROR`、generic 500、またはログのみで隠す | Phase 未完了。ただし未知の内部 bug は defect record と oracle gap を残した上で open:0 まで修正する |
| invariant の `evidence` がない | Done receipt 作成禁止 |
| `regression_guard` が未実行 | Phase 完了扱いにしない |
| 過去 Phase invariant を破る互換差分が仕様化されていない | merge 不可 |
| invariant ledger を PR description だけに置き、仕様本文または artifact と対応しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `invariant_result` | invariant ID ごとの pass/fail、violation 0 件、evidence path、regression guard command、owner Contract ID |
| `violated_invariants` | 完了時は `none`。1 件でもある場合は Phase 未完了 |
| `non_regression_closure` | 過去 Phase invariant が再実行され、互換差分が 0 件または仕様化済みであること |

Phase invariant ledger に関係する仕様変更は、§9.1.7、§9.1.10、§9.1.11、§9.1.14、§9.1.15、§9.1.23、§9.1.24、§9.1.32、§9.1.33、§9.1.35、§9.1.36、§9.1.37、§9.1.38、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。invariant ledger がない Phase 実装 PR は、バグ修正ゼロ判定に必要な非退行条件が未確定として扱い、実装開始不可とする。

#### 9.1.40 Phase scenario matrix / implementation blueprint 固定契約

各 Phase の実装 PR は、実装開始前に Phase scenario matrix を固定しなければならない。scenario matrix は、その Phase で実装・拒否・検証する全シナリオの実装設計図である。正常系だけを実装してから異常系、権限、永続化、互換、rollback、unsupported を後追いで補う進め方は禁止する。

**Scenario matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `scenario_id` | `SCN-P{phase}-{category}-{number}` 形式の一意 ID |
| `category` | normal / invalid_request / auth_scope_quota / persistence_restart / rollback_recovery / concurrency_idempotency / compatibility / unsupported / redaction / operational |
| `entrypoint` | API path、WebSocket message、CLI command、config key、startup path、background job、internal adapter |
| `precondition` | data-dir、metadata、token、config、DB state、Phase prerequisite、operator state |
| `input` | request body、query、header、SQL、config、fixture、failure injection |
| `expected_behavior` | HTTP status、error code、response body、metadata/file state、health、log、retry 可否 |
| `persistence_effect` | no-write、atomic write、fsync、rollback、migration、recovery marker、not_applicable のいずれか |
| `security_effect` | auth required、scope check、quota check、redaction、not_applicable のいずれか |
| `compatibility_effect` | Turso snapshot、libSQL SDK transcript、legacy fixture、previous Phase regression、not_applicable のいずれか |
| `evidence` | test ID、command、snapshot、fixture、artifact path |
| `owner_contract` | API / persistence / security / compatibility / operational Contract ID |
| `not_applicable_reason` | `not_applicable` の場合のみ、仕様本文の根拠 |

**Scenario category 固定表：**

| Category | 必ず固定すること | 未定義時の扱い |
|----------|------------------|----------------|
| `normal` | 成功条件、response、永続化後状態、idempotency | 実装開始禁止 |
| `invalid_request` | malformed body、unknown field、invalid name、bad type、size limit、unsupported query | validation 実装禁止 |
| `auth_scope_quota` | token なし、token 不正、scope 不一致、quota 超過、block policy | success response 禁止 |
| `persistence_restart` | file path、schema、fsync、再起動後復元、破損検出 | 書き込み処理禁止 |
| `rollback_recovery` | 途中失敗、crash、rollback marker、operator_required、retry | destructive operation 公開禁止 |
| `concurrency_idempotency` | 同時 create/delete/write、retry、duplicate request、lock timeout | 状態変更処理禁止 |
| `compatibility` | Turso Cloud snapshot、libSQL SDK transcript、legacy metadata、previous Phase 差分 | 互換対象 API 公開禁止 |
| `unsupported` | future Phase、対象外 field/config/route/operation の拒否 status/body | stub / route 追加禁止 |
| `redaction` | token、secret、raw path、SQL args、backup body、private metadata の非露出 | artifact / log 完了不可 |
| `operational` | health、degraded、unavailable、recovering、operator_required、write policy | failure path 実装禁止 |

**Phase group scenario minimum：**

| Phase group | 最低 scenario |
|-------------|----------------|
| Phase 1〜5 | CLI help/error、config precedence、data-dir lock/open、default DB restart、hrana normal/error、JWT allow/deny、log redaction、SDK smoke |
| Phase 6〜8 | DB create/delete/list/detail、invalid DB name、admin auth、Turso Platform wrapper、organization/group/location/quota、metadata migration、legacy fallback |
| Phase 9〜10 | WebSocket hello/auth/order、transaction commit/rollback/disconnect、managed ATTACH allow/deny、metrics counter/read、arbitrary path denial |
| Phase 11〜13 | replication token/range/checksum、primary/replica lag、redirect、archive manifest/snapshot/frame、retention cleanup、corrupt frame |
| Phase 14〜15 | backup stream、restore integrity、PITR selector、restore rollback、branch create/delete/routing/source isolation、seed compatibility、restart recovery |
| Phase 16〜18 | extension register/load/delete/signature、metrics persistence/Prometheus、HA leader/follower/failover/split-brain/operator_required |
| Phase 19 | internal adapter disabled/shadow/active/rollback、wire/API diff zero、metadata no-migration、performance baseline、crash recovery、Phase 1〜18 full regression |

**Scenario coverage 禁止事項：**

| 状態 | 判定 |
|------|------|
| scenario matrix がない | 実装開始禁止 |
| category が normal だけで error/auth/persistence/compatibility/unsupported がない | Phase 未完了 |
| scenario が `manual only`、`not run`、`todo`、`later`、空欄のまま | Phase 未完了 |
| `not_applicable` の仕様本文根拠がない | Phase 未完了 |
| unsupported / future feature の scenario が success response を期待する | merge 不可 |
| persistence effect があるのに rollback / restart scenario がない | merge 不可 |
| compatibility effect があるのに Turso / SDK / legacy / previous Phase evidence がない | merge 不可 |
| scenario matrix と Phase packet / manifest / Done receipt の Contract ID が一致しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `scenario_matrix_result` | scenario ID ごとの category、pass/fail、evidence path、not_applicable reason、manual only 0 件 |
| `scenario_coverage` | category 別件数、未実行 0 件、not_applicable の仕様本文参照 |
| `implementation_blueprint_closure` | scenario matrix の全 owner Contract ID が packet、manifest、oracle、invariant、test、artifact と一致すること |

Phase scenario matrix に関係する仕様変更は、§9.1.7、§9.1.10、§9.1.11、§9.1.14、§9.1.24、§9.1.32、§9.1.33、§9.1.35、§9.1.36、§9.1.37、§9.1.38、§9.1.39、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。scenario matrix がない Phase 実装 PR は、ケース漏れによる後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.41 Phase resource lifecycle / state transition matrix 固定契約

各 Phase の実装 PR は、実装開始前に Phase resource lifecycle matrix を固定しなければならない。resource lifecycle matrix は、対象 resource が持つ state、許可される transition、禁止される transition、API 応答、write policy、永続化 commit 順序、recovery behavior を 1 箇所に固定する正本である。metadata と実ファイル、runtime map、health、operator state が別々の state を示す場合、仕様本文の lifecycle matrix を正として復旧または起動失敗を選ぶ。

**Resource lifecycle matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `resource_type` | server / config / data_dir / database / token / organization / group / location / quota / websocket / transaction / replica / archive / restore_job / branch / extension / metrics_snapshot / ha_node / internal_adapter |
| `state` | `uninitialized`、`creating`、`active`、`blocked`、`degraded`、`recovering`、`deleting`、`deleted`、`rollback_required`、`operator_required`、`disabled`、`shadow`、`active_internal` など |
| `allowed_transition` | 許可される `from -> to` と発火条件 |
| `forbidden_transition` | 禁止される `from -> to` と検出時の判定 |
| `entry_condition` | state に入る条件。API、job、startup、failure、operator action、config flag を含める |
| `exit_condition` | state から出る条件。commit marker、fsync、health、oracle、operator action を含める |
| `api_behavior` | state ごとの HTTP/WebSocket/CLI success、error code、body、retry 可否 |
| `write_policy` | write 許可 / 拒否、既存 transaction の扱い、read 可否 |
| `persistence_commit_order` | file、metadata、marker、runtime map、directory fsync、cleanup の順序 |
| `recovery_behavior` | restart 時、crash 時、corruption 時、partial state 時の挙動 |
| `evidence` | state fixture、transition test、forbidden transition test、recovery log、health snapshot |

**Phase group lifecycle minimum：**

| Phase group | 最低 lifecycle |
|-------------|----------------|
| Phase 1〜5 | server startup/shutdown、config valid/invalid、data-dir locked/open、default DB open/error、token active/revoked、log redaction state |
| Phase 6〜8 | DB creating/active/deleting/deleted、admin token active/revoked、organization/group/location active/blocked、quota active/exceeded、usage available/unavailable |
| Phase 9〜10 | WebSocket connecting/hello/active/closing/closed、stream open/closed、transaction open/committed/rolled_back、ATTACH allowed/blocked、metrics active/degraded |
| Phase 11〜13 | primary active/degraded、replica syncing/caught_up/lagged/unavailable、frame available/missing/corrupt、archive active/retention_cleanup/corrupt |
| Phase 14〜15 | backup running/completed/failed、restore preparing/verifying/committed/rolled_back/rollback_required、PITR selected/missing/corrupt、branch creating/active/deleting/deleted |
| Phase 16〜18 | extension registered/loaded/failed/disabled、metrics snapshot active/corrupt/rebuilt、HA node follower/candidate/leader/demoted/operator_required |
| Phase 19 | internal adapter disabled/shadow/active_internal/rollback/operator_required、shadow diff clean/dirty、performance baseline pass/fail |

**State transition 固定表：**

| Transition type | 必須仕様 | 未定義時の扱い |
|-----------------|----------|----------------|
| create | file / runtime 準備後に metadata を commit する。success は durable boundary 後のみ | create API 実装禁止 |
| activate | integrity、auth、quota、compatibility、oracle が pass してから active にする | success response 禁止 |
| block | auth、quota、policy、operator action により意図的に blocked にする | 2xx success 禁止 |
| degrade | read-only / partial / lag など縮退時の health と write policy を固定する | degraded path 実装禁止 |
| recover | startup / job recovery の entry、exit、timeout、operator_required を固定する | recovery 実装禁止 |
| delete | route disable、runtime close、file cleanup、metadata commit の順序を固定する | delete API 実装禁止 |
| rollback | rollback marker、旧状態復元、失敗時 operator_required を固定する | destructive operation 禁止 |
| internal switch | shadow、active_internal、rollback flag、compatibility oracle を固定する | internal adapter active 禁止 |

**Lifecycle 禁止事項：**

| 状態 | 判定 |
|------|------|
| metadata が `active` だが file / directory が存在しない resource に success response を返す | merge 不可 |
| `creating` / `deleting` / `recovering` / `rollback_required` 中の resource に通常 write を許可する | merge 不可 |
| unknown state を silent fallback で `active` または `disabled` として扱う | merge 不可 |
| state transition evidence なしに Done receipt を作成する | Phase 未完了 |
| forbidden transition を検出しても error / health / log に出さない | Phase 未完了 |
| runtime map と metadata state が不一致なのに route を成功させる | merge 不可 |
| rollback_required / operator_required を自動成功扱いにする | merge 不可 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `resource_lifecycle_result` | resource type ごとの state、allowed transition、forbidden transition 0 件、recovery evidence path |
| `state_transition_coverage` | transition type 別 test、startup/restart fixture、forbidden transition test の実行結果 |
| `resource_state_closure` | metadata、file、runtime map、health、operator marker が同じ state 判定に収束していること |

Phase resource lifecycle に関係する仕様変更は、§9.1.16、§9.1.21、§9.1.25、§9.1.30、§9.1.31、§9.1.32、§9.1.33、§9.1.37、§9.1.39、§9.1.40、§9.2、§9.4、§9.5、§9.6、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。resource lifecycle matrix がない Phase 実装 PR は、状態不整合による後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.42 Phase schema registry / field-level contract 固定契約

各 Phase の実装 PR は、追加・変更する request、response、metadata、config、JWT claim、WebSocket message、artifact、fixture、log、metric の field-level schema を schema registry として固定しなければならない。field の必須/任意、null 可否、省略可否、default、validation、serialization、migration、compatibility、redaction が未定義のまま実装してはならない。

**Schema registry 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `schema_id` | `SCHEMA-P{phase}-{surface}-{name}` 形式の一意 ID |
| `owner_phase` | field を導入または変更する Phase |
| `surface` | request_body / response_body / metadata_json / config / jwt_claim / websocket_message / artifact_fixture / log_metric |
| `field_name` | 外部 field 名。内部名と異なる場合は mapping を明記 |
| `type` | string / integer / number / boolean / object / array / enum / null / bytes / timestamp / opaque string |
| `required` | 必須なら `true`。省略可能なら `false` と理由 |
| `nullable` | `null` 可否。`nullable:true` の場合は null の意味 |
| `omittable` | 省略可否。省略と null を同一扱いにする場合は明示 |
| `default` | 省略時、migration 時、旧 metadata 読み込み時の default |
| `validation` | length、regex、range、enum、unknown field、duplicate query、empty string/array の扱い |
| `serialization` | casing、timestamp 精度、sort order、redaction、response wrapper |
| `migration` | old field、new field、rename/delete、backfill、rollback、破損時挙動 |
| `compatibility` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase との差分 |
| `redaction` | secret / token / raw path / SQL args / backup body / private metadata の扱い |
| `evidence` | request fixture、response snapshot、metadata fixture、migration fixture、secret scan、compat diff |

**Schema surface 固定表：**

| Surface | 必ず固定する field |
|---------|--------------------|
| request_body | required、nullable、unknown field、empty string/array、duplicate key、content type |
| response_body | wrapper、field casing、nullable、omittable、timestamp、error body、secret 非露出 |
| metadata_json | schema version、required/default、migration、rollback、破損時挙動、unknown field |
| config | CLI/env/TOML/default、対象 Phase 前挙動、secret file、invalid value |
| jwt_claim | claim 名、scope、expiry、ro/rw、DB/org/group binding、legacy claim |
| websocket_message | message type、request_id、stream_id、unknown field、response_ok/error |
| artifact_fixture | 正規化 field、redaction、Contract ID、再生成 command |
| log_metric | field 名、label、cardinality、secret 非露出、request id / trace id |

**Field-level 禁止事項：**

| 状態 | 判定 |
|------|------|
| `required` / `nullable` / `omittable` が未定義の field を実装する | 実装開始禁止 |
| metadata field の rename / delete / required 化に migration と rollback がない | merge 不可 |
| response field casing、wrapper、null/省略を仕様化なしに変更する | merge 不可 |
| DTO と metadata で同名 field の意味が異なるのに mapping がない | Phase 未完了 |
| secret / token / raw path / SQL args / backup body field の redaction が未定義 | merge 不可 |
| unknown field を黙って無視する。ただし hrana wire 互換など該当節で明記された場合を除く | merge 不可 |
| `null` を省略扱いにする根拠がない | Phase 未完了 |
| schema registry と fixture / snapshot / migration test が一致しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `schema_registry_result` | schema ID ごとの field coverage、unknown/null/default/migration/redaction evidence |
| `field_compatibility_result` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase との差分分類 |
| `schema_migration_closure` | old/new/corrupt fixture、rollback、default backfill、破損時挙動が検証済みであること |

Phase schema registry に関係する仕様変更は、§9.1.10、§9.1.11、§9.1.12、§9.1.15、§9.1.19、§9.1.20、§9.1.23、§9.1.24、§9.1.27、§9.1.28、§9.1.29、§9.1.32、§9.1.33、§9.1.35、§9.1.40、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.13、§9.17、該当 Phase 詳細節を同時更新する。schema registry がない field 変更は、DTO / metadata / fixture の意味ズレによる後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.43 Phase decision precedence / conflict resolution matrix 固定契約

各 Phase の実装 PR は、複数の仕様条件が同時に成立した場合に、どの判定、error、HTTP status、state、client action を優先するかを decision precedence matrix として固定しなければならない。実装者が handler 内の if/else 順、既存 helper の都合、または実装しやすさで優先順位を決めることは禁止する。

**Decision precedence matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `decision_id` | `DEC-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 判定を導入または変更する Phase |
| `surface` | HTTP route / hrana request / WebSocket message / admin API / CLI / metadata migration / background job / health / log_metric |
| `conflicting_conditions` | 同時成立し得る条件。例: invalid body、auth missing、scope denied、resource missing、quota exceeded、block_writes、recovering、unsupported |
| `precedence_order` | 高い順の判定名。最低でも選択判定と退けられる判定を含める |
| `selected_behavior` | 優先される response、state transition、write policy、log、metric |
| `losing_behavior` | 優先されなかった条件をどう扱うか。隠す、audit のみ、内部 reason に残す、再評価する等 |
| `error_code` | §7.3 の error code。該当しない場合は `none` と理由 |
| `status` | HTTP status、WebSocket error、CLI exit code、health state、job state |
| `client_action` | retry、auth refresh、scope change、quota adjustment、operator action、request correction、none |
| `evidence` | conflict fixture、snapshot、SDK transcript、metadata fixture、job log、health snapshot |

**共通 precedence 固定表：**

| 同時成立条件 | 優先する判定 | 退ける判定 / 扱い |
|--------------|--------------|-------------------|
| body が安全に parse できない / content-type 不正 / request size 超過 と auth missing | parse / content validation | auth 判定は行わない。認証情報、resource 存在、scope 情報を露出しない |
| auth missing / invalid と resource missing | auth failure | resource の存在有無を返さない |
| scope denied / permission denied と resource missing | scope / permission denial | 存在漏洩を避ける endpoint では `not_found` を返さない。Turso 互換で `not_found` が必要な場合は decision に明記する |
| read-only token / write scope denied と quota exceeded / block_writes | scope / permission denial | quota / block policy は audit または internal reason に残すが client には優先しない |
| quota exceeded と block_writes | block_writes | operator または policy による明示停止を quota より優先する |
| recovering / rollback_required / operator_required と normal write | operational state refusal | write commit、metadata commit、success response を行わない |
| unsupported future feature と invalid field | route / feature ownership 確定後に unsupported | 未所有 route は 404/405、所有済み future feature は規定 501/400。invalid field で unsupported を隠さない |
| compatibility rule と self-host optimization | compatibility rule | security / durability / data loss を除き、Turso Cloud / libSQL SDK 互換を優先する |
| persistence conflict と response generation | persistence conflict | success response、snapshot 更新、Done receipt 作成を禁止する |
| secret / redaction violation と normal error body | redaction violation | 詳細 error を抑止し、artifact/log を再生成する |

**Phase group 最低 decision：**

| Phase group | 必須 decision |
|-------------|---------------|
| Phase 1〜5 | CLI/env/TOML priority、data-dir invalid vs DB open failure、JWT missing/invalid vs DB missing、hrana parse error vs auth error、log redaction vs detailed error |
| Phase 6〜8 | DB name validation vs auth、admin scope denial vs not found、organization/group/location mismatch、quota exceeded vs block_writes、legacy metadata migration failure vs API success |
| Phase 9〜10 | WebSocket message parse vs auth、stream/transaction rollback vs success frame、ATTACH unsupported vs path denial、metrics read degraded vs unavailable |
| Phase 11〜13 | replica lag vs primary unavailable、checksum mismatch vs retry、archive manifest missing vs retention cleanup、replication auth denial vs frame not found |
| Phase 14〜15 | restore rollback_required vs branch write、PITR archive missing vs invalid selector、branch source missing vs permission denied、destructive lock vs quota |
| Phase 16〜18 | extension signature failure vs allowlist denial、metrics corruption vs read degraded、HA no leader vs split-brain operator_required、promotion conflict vs retry |
| Phase 19 | adapter shadow diff vs active success、internal adapter failure vs compatibility fallback、rollback flag vs performance target、Turso parity diff vs self-host optimization |

**merge 不可条件：**

| 状態 | 判定 |
|------|------|
| conflict が起きる条件に decision ID がない | 実装開始禁止 |
| handler の最初に一致した if/else で判定し、precedence matrix と対応しない | merge 不可 |
| 同じ conflict を endpoint / protocol / CLI ごとに異なる error/status で返す | Phase 未完了。ただし Turso 互換差分として明記されている場合を除く |
| 存在漏洩を避けるべき endpoint で scope denial より `DB_NOT_FOUND` を優先する | merge 不可 |
| quota、block_writes、recovering、rollback_required、operator_required を write commit 後に判定する | merge 不可 |
| unsupported / compatibility conflict を `INTERNAL_ERROR`、generic 500、ログのみで隠す | Phase 未完了 |
| losing behavior の証跡がなく、退けられた判定が副作用を起こしていないことを確認できない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `decision_precedence_result` | decision ID ごとの conflict fixture、selected behavior、error/status snapshot、losing behavior 非発火証跡 |
| `decision_consistency_result` | HTTP / WebSocket / CLI / job / health 間で同じ conflict が同じ優先順位で処理された証跡 |
| `decision_compatibility_result` | Turso Cloud、libSQL SDK、previous Phase と異なる precedence がある場合の差分理由。なければ `none` |

Phase decision precedence に関係する仕様変更は、§7.3、§9.1.10、§9.1.11、§9.1.12、§9.1.14、§9.1.23、§9.1.24、§9.1.32、§9.1.33、§9.1.35、§9.1.36、§9.1.37、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.15、§9.17、該当 Phase 詳細節を同時更新する。decision precedence matrix がない conflict は、実装者ごとの分岐順差、存在漏洩、誤った retry、commit 後拒否による後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.44 Phase coverage closure / implementation completeness matrix 固定契約

各 Phase の実装 PR は、Phase packet に含まれる全契約が test、artifact、oracle、regression、Done receipt のいずれで証明されるかを coverage closure matrix として固定しなければならない。契約を定義しただけ、正常系だけ、PR description の説明だけ、または根拠のない `N/A` で Phase 完了扱いにしてはならない。

**Coverage closure matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `coverage_id` | `COV-P{phase}-{contract-id}` 形式の一意 ID |
| `phase` | coverage を閉じる Phase |
| `source_contract` | API / persistence / error / security / compatibility / concurrency / operation / regression の Contract ID |
| `source_matrix` | dependency_graph / invariant_ledger / scenario_matrix / resource_lifecycle_matrix / schema_registry / decision_precedence_matrix / phase_detail / api_table / persistence_table / error_table |
| `implementation_surface` | code module、route、CLI、config、metadata、job、health、log/metric、test fixture の対象 |
| `required_test` | 必須 test ID、command、または SDK / Turso snapshot compare。自動化不能な場合は manual exception ID |
| `required_evidence` | artifact path。snapshot、fixture、transcript、log、compat diff、secret scan、CI output のいずれか |
| `oracle` | §9.1.35 の oracle 名と version。oracle 不要の場合は理由 |
| `status` | `planned`、`pass`、`fail`、`blocked`、`not_applicable` のいずれか。Done 時は `pass` または根拠付き `not_applicable` のみ |
| `not_applicable_reason` | `not_applicable` の場合のみ、§9.2、§9.4、該当 Phase 詳細節、unsupported 固定表の本文参照 |
| `gap_class` | gap がある場合は `missing_test`、`missing_artifact`、`missing_oracle`、`missing_regression`、`manual_only`、`spec_gap`、`implementation_gap` |
| `blocker` | `status` が `pass` / 根拠付き `not_applicable` 以外の場合に止める operation。例: route publish、metadata commit、Done receipt、merge |

**coverage 対象固定表：**

| Source | 必ず coverage に含めるもの |
|--------|-----------------------------|
| dependency_graph | 全 prerequisite と dependent contract。`blocked`、`planned`、未証明 satisfied を残さない |
| invariant_ledger | invariant ID ごとの before/after、violation signal、regression guard |
| scenario_matrix | normal、invalid、auth/scope/quota、persistence、rollback、concurrency、compatibility、unsupported、redaction、operational |
| resource_lifecycle_matrix | state、allowed transition、forbidden transition、recovery、operator_required |
| schema_registry | field ごとの required/null/default/validation/migration/redaction/compatibility |
| decision_precedence_matrix | conflict ごとの selected behavior、losing behavior、error/status/client action |
| API / persistence / security contract | method/path/body/error、file/schema/fsync/rollback、auth/scope/quota/redaction |
| compatibility / regression contract | Turso Cloud、libSQL SDK、legacy metadata、previous Phase regression、Phase 1 から対象 Phase 直前までの影響 |

**Phase group coverage minimum：**

| Phase group | 最低 coverage |
|-------------|----------------|
| Phase 1〜5 | CLI/config/data-dir/default DB/hrana-http/JWT/log の normal/error/restart/redaction/SDK smoke |
| Phase 6〜8 | multi DB/admin API/Turso Platform/org/group/location/quota/legacy migration の auth/persistence/compat/regression |
| Phase 9〜10 | WebSocket/transaction/ATTACH/metrics の protocol、rollback、path denial、counter consistency、disconnect |
| Phase 11〜13 | replication/archive の frame/checksum/snapshot/retention/corruption/restart/role health |
| Phase 14〜15 | backup/restore/PITR/branch の destructive rollback、source selector、quota、seed isolation、restart recovery |
| Phase 16〜18 | extension/metrics/HA の signature、allowlist、snapshot corruption、term/leader/split-brain/operator_required |
| Phase 19 | internal adapter の shadow/active/rollback、wire/API parity、performance baseline、Phase 1〜18 full regression |

**coverage closure 禁止事項：**

| 状態 | 判定 |
|------|------|
| Contract ID、schema ID、scenario ID、decision ID が coverage matrix に存在しない | 実装開始禁止 |
| normal 系 test だけで error/auth/persistence/rollback/compatibility を pass 扱いにする | Phase 未完了 |
| `manual only` を coverage pass として扱う | merge 不可。ただし §9.1.10 の manual exception がある補助証跡を除く |
| `N/A` に本文参照と理由がない | Phase 未完了 |
| artifact path が存在しない、または Contract ID と対応しない | Phase 未完了 |
| oracle なしで snapshot / fixture / expected を更新する | merge 不可 |
| previous Phase regression を coverage 外にする | Phase 未完了 |
| failed / skipped / flaky / blocked を pass 扱いにする | merge 不可 |
| coverage matrix を PR description だけに置き、仕様本文、manifest、artifact と対応しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `coverage_closure_result` | coverage ID ごとの status、gap 0 件、required_test、required_evidence、oracle、not_applicable reason |
| `coverage_gap_result` | `missing_test`、`missing_artifact`、`missing_oracle`、`missing_regression`、`manual_only`、`spec_gap`、`implementation_gap` が 0 件である証跡 |
| `coverage_regression_result` | previous Phase regression と互換 snapshot が coverage 対象に含まれ、pass していること |

Phase coverage closure に関係する仕様変更は、§9.1.7、§9.1.8、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.35、§9.1.36、§9.1.38、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.1.43、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。coverage closure matrix がない Phase 実装 PR は、契約定義と実装証跡の未対応、検証漏れ、根拠なし N/A、旧 Phase regression 漏れによる後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.45 Phase change impact / drift control matrix 固定契約

各 Phase の実装 PR は、実装中に scope、API、schema、metadata、error、auth、quota、lifecycle、test、oracle、artifact、compatibility のいずれかを変更する必要が生じた場合、変更前に change impact matrix を更新しなければならない。Phase packet freeze 後の暗黙変更、PR description だけの説明、snapshot だけの更新、または実装都合の仕様 drift を禁止する。

**Change impact matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `change_id` | `CHG-P{phase}-{number}` 形式の一意 ID |
| `phase` | 変更が発生した Phase |
| `change_type` | scope / api / request_response / error / persistence_schema / auth_scope_quota / lifecycle_state / compatibility / oracle_snapshot / test_artifact / dependency / operational |
| `changed_contract` | 変更対象の Contract ID、schema ID、scenario ID、decision ID、coverage ID |
| `affected_sections` | 同時更新する仕様節番号。最低でも変更対象節、§9.1.32、§9.1.33、§9.17 を含める |
| `affected_matrices` | dependency、invariant、scenario、resource lifecycle、schema registry、decision precedence、coverage closure のうち影響する表 |
| `affected_tests` | 追加・変更・再実行する TC、unit、integration、SDK、Turso snapshot、regression command |
| `affected_artifacts` | 更新する snapshot、fixture、transcript、compat diff、secret scan、CI output、Done receipt |
| `compatibility_impact` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase への影響。影響なしの場合も根拠を明記 |
| `migration_impact` | metadata / file / config / JWT claim / artifact schema migration の要否、old/new/corrupt fixture |
| `rollback_impact` | rollback 手順、rollback flag、復旧 marker、operator_required への影響 |
| `regression_expansion` | 追加する previous Phase regression、互換 snapshot、failure injection |
| `approval_state` | `not_required`、`approved`、`spec_pr_required`、`blocked`。仕様変更を伴う場合は承認済みでなければならない |
| `closure_evidence` | 更新後に drift 0 件を示す cross-reference scan、coverage result、artifact path |

**変更種別別の同時更新表：**

| Change type | 同時更新必須 |
|-------------|--------------|
| API route / method / path | §9.5、Phase 詳細節、API Contract ID、error snapshot、scenario matrix、coverage closure、SDK/Turso snapshot |
| request / response field | schema registry、API snapshot、validation rule、compatibility impact、redaction、coverage closure |
| error code / status / retry | §7.3、§9.7、decision precedence、error oracle、client action、regression test |
| metadata / persistence schema | §9.6、migration、rollback、resource lifecycle、invariant、old/new/corrupt fixture |
| auth / scope / quota / block policy | security contract、decision precedence、scenario matrix、denial snapshot、redaction、compatibility impact |
| lifecycle / state transition | resource lifecycle、operational state、recovery runbook、forbidden transition test、health snapshot |
| Turso Cloud / SDK compatibility | compatibility contract、mode boundary、snapshot source、diff reason、regression expansion、migration / rollback impact |
| test / oracle / snapshot | acceptance oracle、coverage closure、artifact path、normalization rule、snapshot update reason |
| Phase scope / target surface | Phase packet、manifest、§9.2、§9.4、dependency graph、coverage closure、Done receipt criteria |

**drift control 禁止事項：**

| 状態 | 判定 |
|------|------|
| Phase packet freeze 後に scope / API / schema / error / persistence を変更し、change ID がない | 実装開始禁止または Phase 未完了 |
| 実装都合で仕様外 field、metadata、error、config を追加する | merge 不可 |
| PR description だけで変更理由を説明し、仕様本文を更新しない | 仕様として扱わない |
| snapshot / fixture / expected だけを更新し、oracle と仕様本文を更新しない | merge 不可 |
| compatibility 影響なしと書くだけで SDK / Turso / previous Phase regression を増やさない | Phase 未完了 |
| migration / rollback 影響を未評価のまま metadata / file schema を変更する | merge 不可 |
| affected matrices のうち 1 つでも旧 Contract ID / 旧 field / 旧 error を参照する | Phase 未完了 |
| approval_state が `spec_pr_required` / `blocked` のまま実装を進める | 実装禁止 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `change_impact_result` | change ID ごとの affected sections/matrices/tests/artifacts、更新完了、承認状態、closure evidence |
| `drift_closure_result` | Phase packet、manifest、仕様本文、test、artifact、Done receipt の drift 0 件を示す scan 結果 |
| `compatibility_reassessment_result` | change によって再評価した Turso Cloud、libSQL SDK、previous Phase、migration、rollback の結果 |

Phase change impact / drift control に関係する仕様変更は、§0、§7.3、§9.1.10、§9.1.11、§9.1.12、§9.1.13、§9.1.14、§9.1.15、§9.1.23、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.35、§9.1.38、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.1.43、§9.1.44、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.15、§9.17、該当 Phase 詳細節を同時更新する。change impact matrix がない Phase 実装 PR は、実装中の仕様 drift、古い Contract ID、古い snapshot、互換影響見落とし、migration / rollback 漏れによる後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.46 Phase rollout readiness / operator acceptance matrix 固定契約

各 Phase の実装 PR は、Phase Done と rollout ready を分離して判定しなければならない。Phase Done は仕様・実装・test・artifact の完了判定であり、rollout ready は operator が起動、停止、再起動、rollback、health 監視、互換性確認、data safety 確認を行ったうえで運用投入してよい状態を指す。Done receipt があっても rollout readiness matrix が未完了なら release / deploy / production enable を行ってはならない。

**Rollout readiness matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `rollout_id` | `ROLL-P{phase}-{surface}` 形式の一意 ID |
| `phase` | rollout 判定対象 Phase |
| `release_surface` | binary / CLI / HTTP API / WebSocket / admin API / metadata migration / background job / Docker / CI / docs |
| `startup_condition` | 起動してよい前提条件、config、data-dir、metadata、lock、migration marker、secret の状態 |
| `shutdown_condition` | shutdown 時に flush / fsync / lock release / job cancel / transaction rollback が必要な条件 |
| `restart_condition` | restart 後に同じ state、health、metadata、runtime map、compat behavior に戻る条件 |
| `rollback_condition` | rollback flag、backup、old metadata、restore marker、operator 手順、rollback 不能時の扱い |
| `health_gate` | release 可否を判断する health endpoint / CLI status / log / metric の期待値 |
| `operator_action` | operator が実行する command、確認、承認、manual step。不要なら `none` |
| `observability_evidence` | log、metric、health snapshot、audit 相当記録、request id / trace id、redaction scan |
| `compatibility_gate` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase regression の pass 条件 |
| `data_safety_gate` | fsync、atomic write、backup、rollback、corruption handling、data loss なしの証跡 |
| `blocked_release_reason` | rollout 不可の場合の理由。不可でなければ `none` |

**Phase group rollout minimum：**

| Phase group | 最低 readiness |
|-------------|----------------|
| Phase 1〜5 | CLI help、config resolution、data-dir lock、default DB restart、hrana health、JWT secret、log redaction、SDK smoke |
| Phase 6〜8 | admin API auth、Turso Platform snapshot、metadata migration/restart、organization/group/location/quota、legacy fallback、quota block |
| Phase 9〜10 | WebSocket reconnect、open transaction rollback、ATTACH path denial、metrics counter persistence/visibility、disconnect behavior |
| Phase 11〜13 | primary/replica role health、replication lag、archive manifest integrity、retention cleanup safety、checksum mismatch handling |
| Phase 14〜15 | backup artifact readability、restore/PITR rollback、branch create/delete recovery、source snapshot、destructive operation lock |
| Phase 16〜18 | extension load failure handling、metrics snapshot rebuild、HA candidate/leader health、operator promote、split-brain rejection |
| Phase 19 | internal adapter shadow/active/rollback flag、compat snapshot diff zero、performance baseline、Phase 1〜18 regression、silent fallback 禁止 |

**rollout ready 禁止事項：**

| 状態 | 判定 |
|------|------|
| Phase Done receipt だけで rollout ready と扱う | release 不可 |
| rollback 手順、rollback flag、backup、または recovery marker が未検証 | release 不可 |
| health が `ok` でも `operator_required`、`rollback_required`、`degraded` を隠す | merge 不可 |
| Docker / CI / local の結果差分が未記録 | rollout ready 不可 |
| data loss、partial commit、success-before-fsync の可能性が残る | merge 不可 |
| Turso Cloud / libSQL SDK / previous Phase 互換差分が未分類 | rollout ready 不可 |
| startup / restart 後に metadata、file、runtime map、health が一致しない | release 不可 |
| operator action が必要なのに command、解除条件、確認 artifact がない | Phase 未完了 |
| rollout 不可理由があるのに `blocked_release_reason:none` とする | review failure |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `rollout_readiness_result` | rollout ID ごとの startup、shutdown、restart、rollback、health、operator、compatibility、data safety 証跡 |
| `operator_acceptance_result` | operator action、manual step、解除条件、command、artifact。不要な場合は `none` |
| `release_blocker_result` | blocked release reason が 0 件、または rollout 不可として明示されていること |

Phase rollout readiness に関係する仕様変更は、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.15、§9.1.24、§9.1.32、§9.1.33、§9.1.37、§9.1.39、§9.1.41、§9.1.44、§9.1.45、§9.2、§9.4、§9.6、§9.8、§9.11、§9.15、§9.16、§9.17、該当 Phase 詳細節を同時更新する。rollout readiness matrix がない Phase 実装 PR は、実装完了後の起動・再起動・rollback・health・operator 判断の欠落による後続バグ修正を防げないため、release / deploy / production enable 不可とする。

#### 9.1.47 Phase compatibility baseline / upstream refresh matrix 固定契約

各 Phase の実装 PR は、Turso Cloud、libSQL SDK、hrana wire format、legacy metadata、previous Phase response を互換 baseline として固定しなければならない。互換 baseline は「現在の実装出力」ではなく「Adlaire DB が追従または明示差分化する外部基準」である。実装都合で snapshot、oracle、SDK transcript、expected を更新して互換差分を消すことは禁止する。

**Compatibility baseline matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `baseline_id` | `BASE-P{phase}-{surface}` 形式の一意 ID |
| `phase` | baseline を固定する Phase |
| `upstream_source` | Turso Cloud API、libSQL SDK、hrana spec、legacy Adlaire artifact、previous Phase snapshot のいずれか |
| `observed_behavior` | upstream または previous Phase で観測した path、method、request、response、error、metadata、SDK behavior |
| `adlaire_behavior` | Adlaire DB が採用する挙動。差分がある場合は mode、error mapping、client impact を明記 |
| `compatibility_class` | `match`、`intentional_self_host_diff`、`unsupported_until_phase`、`upstream_changed`、`sdk_regression`、`adlaire_extension_mode_only` |
| `snapshot_version` | snapshot / fixture / transcript の version、生成 commit、正規化 rule |
| `sdk_version` | 対象 SDK 名と version。SDK 対象外なら本文根拠付き `not_applicable` |
| `refresh_trigger` | upstream changelog、SDK update、Turso behavior diff、spec change、regression failure、security/persistence reason |
| `refresh_allowed_by` | refresh を許可する根拠。仕様変更 commit、Turso 追従記録、SDK changelog、承認済み change ID |
| `diff_reason` | 差分理由。差分なしなら `none` |
| `migration_impact` | metadata、config、JWT claim、artifact、client migration の要否 |
| `regression_scope` | refresh 後に再実行する Phase regression、SDK transcript、Turso snapshot、legacy fixture |
| `evidence` | upstream observation、snapshot diff、SDK transcript、compat diff、changelog reference、artifact path |

**compatibility class 固定表：**

| Class | 意味 | 完了条件 |
|-------|------|----------|
| `match` | upstream / SDK / previous Phase と client-visible behavior が一致 | snapshot / transcript 差分ゼロ |
| `intentional_self_host_diff` | security、persistence、operation の理由で自己ホスト差分を採用 | diff reason、client impact、代替仕様、regression が必須 |
| `unsupported_until_phase` | 将来 Phase まで明示的に未対応 | unsupported behavior、error/status、対象 Phase、snapshot が必須 |
| `upstream_changed` | Turso Cloud / SDK 側の変更に追従する必要がある | upstream 証跡、仕様更新、migration/rollback 評価が必須 |
| `sdk_regression` | SDK 互換が壊れた | Phase 未完了。修正または仕様差分化まで Done 禁止 |
| `adlaire_extension_mode_only` | Adlaire 独自 mode のみの挙動 | Turso 互換 mode への混入なし、mode boundary snapshot が必須 |

**baseline refresh 禁止事項：**

| 状態 | 判定 |
|------|------|
| 実装変更だけを理由に互換 snapshot / oracle / expected を更新する | merge 不可 |
| upstream 変更を確認せず `upstream_changed` とする | review failure |
| SDK transcript なしに SDK 互換を `match` とする | Phase 未完了 |
| Turso Cloud 差分を `INTERNAL_ERROR`、generic 500、ログだけで隠す | merge 不可 |
| 自己ホスト差分に diff reason、client impact、代替仕様がない | 実装開始禁止 |
| 古い baseline のまま Phase Done / rollout ready と扱う | Phase 未完了 |
| Adlaire extension mode の field / auth / metadata が Turso 互換 mode に混入する | merge 不可 |
| refresh 後の previous Phase regression を実行しない | rollout ready 不可 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `compatibility_baseline_result` | baseline ID ごとの compatibility class、snapshot version、SDK version、diff reason、refresh trigger、evidence |
| `upstream_refresh_result` | refresh がある場合の upstream observation、許可根拠、migration / rollback / regression 結果。ない場合は `none` |
| `compatibility_mode_boundary_result` | Turso 互換 mode と Adlaire extension mode の差分、混入なし、snapshot 証跡 |

Phase compatibility baseline に関係する仕様変更は、§1.4、§1.5、§3.5.3、§7.3、§9.1.10、§9.1.11、§9.1.15、§9.1.23、§9.1.24、§9.1.32、§9.1.33、§9.1.35、§9.1.39、§9.1.40、§9.1.43、§9.1.44、§9.1.45、§9.1.46、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.15、§9.17、該当 Phase 詳細節を同時更新する。compatibility baseline matrix がない Phase 実装 PR は、互換基準、snapshot 更新条件、SDK transcript、upstream 追従差分、自己ホスト例外の根拠が未確定であるため、実装開始不可とする。

#### 9.1.48 Phase security abuse / bypass resistance matrix 固定契約

各 Phase の実装 PR は、攻撃面、信頼しない入力、必須制御、bypass attempt、拒否応答、redaction、audit/log、quota/rate、永続化副作用を security abuse matrix として固定しなければならない。正常系 auth が通ることだけでは security 完了扱いにしない。悪用された場合に何を拒否し、何を記録し、何を漏らさず、どの副作用を禁止するかを Phase 開始前に固定する。

**Security abuse matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `security_case_id` | `SEC-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | security case を導入または変更する Phase |
| `attack_surface` | CLI / config / HTTP API / WebSocket / admin API / Turso Platform API / replication / backup / extension / HA / internal adapter |
| `untrusted_input` | token、JWT claim、path/query/body、SQL args、backup body、replication frame、extension path、metadata file、env/config、header |
| `required_control` | auth、scope、quota、block policy、path normalization、signature/checksum、idempotency、redaction、rate/size limit |
| `bypass_attempt` | control をすり抜ける試行。例: missing token、wrong scope、path traversal、duplicate request、replay、tampered metadata、oversized body |
| `expected_denial` | HTTP status、error code、WebSocket error、CLI exit code、health state、client action |
| `redaction_rule` | response/log/artifact/metric に出してよい field と禁止 field |
| `audit_log_rule` | 記録する event、level、request id / trace id、secret 非露出。記録不要なら理由 |
| `rate_or_quota_effect` | quota、block_reads、block_writes、rate/size limit への影響。影響なしなら理由 |
| `persistence_effect` | 拒否時に metadata、DB file、archive、token、usage、counter を変更しないこと。変更が必要なら commit order |
| `regression_test` | bypass / denial / redaction / persistence no-op を検証する TC、command、fixture |
| `evidence` | denied response、log sample、secret scan、metadata before/after、replay fixture、path traversal fixture |

**Phase group security minimum：**

| Phase group | 最低 security case |
|-------------|--------------------|
| Phase 1〜5 | CLI secret、config precedence、data-dir permission/lock、JWT missing/invalid/revoked、log redaction、hrana malformed body |
| Phase 6〜8 | admin token、DB scope、organization/group/location boundary、quota exceeded、block_reads/block_writes、Platform token、legacy metadata |
| Phase 9〜10 | WebSocket auth/stream ownership、transaction close rollback、ATTACH path traversal、ro token write denial、metrics label injection |
| Phase 11〜13 | replication token、frame checksum tamper、snapshot access、archive manifest corruption、replica retry without token leak |
| Phase 14〜15 | backup body size/type、restore/PITR destructive lock、branch source scope、delete protection、quota before commit |
| Phase 16〜18 | extension path/symlink/signature、metrics snapshot poisoning、HA promote auth、term rollback、split-brain operator_required |
| Phase 19 | internal adapter shadow isolation、active switch flag、silent fallback denial、compat diff redaction、rollback flag abuse |

**bypass resistance 禁止事項：**

| 状態 | 判定 |
|------|------|
| security case がない attack surface を公開する | 実装開始禁止 |
| auth / scope / quota / block policy を bypass して success response を返す | merge 不可 |
| token、JWT、admin/platform/replication/HA token、SQL args、backup body、raw path を response/log/artifact/metric に出す | merge 不可 |
| path traversal、absolute path、symlink、reserved name を受理する | merge 不可 |
| quota、block_writes、rate/size limit、delete protection を commit 後に評価する | merge 不可 |
| replay / duplicate request で二重作成、二重 token、二重 quota charge、二重 branch を発生させる | merge 不可 |
| security failure を `INTERNAL_ERROR`、generic 500、ログのみで隠す | Phase 未完了 |
| security case を manual only で pass 扱いにする | merge 不可 |
| denial 時に metadata / file / runtime map が変わらない証跡がない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `security_abuse_result` | security case ID ごとの bypass attempt、expected denial、redaction、audit/log、persistence no-op、evidence |
| `security_redaction_result` | secret scan、log/artifact/response/metric redaction、漏洩 0 件 |
| `security_bypass_closure` | bypass、replay、path traversal、scope/quota/block policy 回避が 0 件である証跡 |

Phase security abuse に関係する仕様変更は、§7.3、§9.1.10、§9.1.11、§9.1.14、§9.1.17、§9.1.18、§9.1.21、§9.1.23、§9.1.24、§9.1.26、§9.1.32、§9.1.33、§9.1.36、§9.1.37、§9.1.39、§9.1.40、§9.1.42、§9.1.43、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.14、§9.17、該当 Phase 詳細節を同時更新する。security abuse matrix がない Phase 実装 PR は、攻撃面、bypass、denial、redaction、persistence no-op の根拠が未確定であるため、実装開始不可とする。

#### 9.1.49 Phase ambiguity closure / implementation decision table 固定契約

各 Phase の実装 PR は、実装者が仕様本文を読んだ時点で迷う可能性がある判断を ambiguity closure matrix として実装開始前に閉じなければならない。曖昧なまま実装し、レビュー、テスト失敗、運用投入、後続 Phase で判断を補うことは禁止する。選択肢が複数ある場合は、推奨決定、却下した案、根拠、影響範囲、証跡、再検討条件を仕様本文に固定する。

**Ambiguity closure matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `ambiguity_id` | `AMB-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 判断を閉じる Phase |
| `implementation_question` | 実装者が迷う具体的な問い。例: auth と quota の優先、metadata 欠損時の扱い、retry 可否 |
| `candidate_options` | 検討した選択肢。最低 2 案。選択肢が 1 つしかない場合は理由 |
| `selected_decision` | 採用する挙動、status、error code、schema、commit order、log、client action |
| `rejected_options` | 却下した案と却下理由。Turso Cloud 互換、自己ホスト安全性、後方互換、運用性への影響を含める |
| `decision_basis` | 参照する仕様節、Turso Cloud / libSQL SDK baseline、既存 invariant、security / persistence 根拠 |
| `affected_contracts` | 同時更新が必要な API、schema、error、persistence、security、compatibility、test、artifact |
| `edge_cases` | null、missing、duplicate、race、restart、legacy、unsupported、malformed、large input、permission denied |
| `not_applicable_rule` | N/A を認める条件。根拠なし N/Aは禁止 |
| `reopen_trigger` | upstream change、仕様変更、regression、security finding、migration failure など再検討条件 |
| `evidence` | snapshot、fixture、oracle、decision log、spec diff、review checklist |

**Phase group ambiguity minimum：**

| Phase group | 最低 ambiguity closure |
|-------------|------------------------|
| Phase 1〜5 | CLI flag/env/TOML/default 優先、data-dir lock、default DB open failure、JWT missing/invalid/revoked、hrana error status |
| Phase 6〜8 | DB 名衝突、org/group/location scope、quota/block precedence、legacy metadata migration、Platform API 互換差分 |
| Phase 9〜10 | WebSocket close 時 tx、stream id 再利用、ATTACH 対象外、read-only token write、metrics counter 更新タイミング |
| Phase 11〜13 | replication frame ordering、checksum 不一致、snapshot lag、archive retention、replica retry/backoff |
| Phase 14〜15 | restore/PITR 失敗時 rollback、branch seed source、delete protection、quota before commit、source unavailable |
| Phase 16〜18 | extension allowlist/signature、Prometheus label、metrics snapshot 破損、HA promote/demote、split-brain handling |
| Phase 19 | shadow diff 許容範囲、active switch 条件、internal fallback、performance regression 閾値、rollback flag |

**曖昧表現禁止事項：**

| 表現 / 状態 | 判定 |
|-------------|------|
| `TBD`、`TODO`、`FIXME`、`未定`、`後で決める` が production path に残る | 実装開始禁止 |
| `実装判断`、`よしなに`、`必要に応じて`、`適宜`、`可能なら` を完了条件に使う | merge 不可 |
| 複数の valid behavior があるのに selected decision がない | 実装開始禁止 |
| 却下案と却下理由がない | review failure |
| N/A に仕様本文の根拠がない | Phase 未完了 |
| upstream / SDK / previous Phase と差分があるのに decision basis がない | merge 不可 |
| error code、status、client action、commit order、redaction のいずれかを実装者判断にする | merge 不可 |
| ambiguity closure が PR description のみで仕様本文にない | 仕様として扱わない |
| open ambiguity が 1 件以上ある状態で Done receipt を出す | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `ambiguity_closure_result` | ambiguity ID ごとの selected decision、rejected options、decision basis、affected contracts、evidence |
| `open_ambiguity_count` | `0` 固定。0 以外は Phase 未完了 |
| `decision_reopen_result` | reopen trigger 該当なし、または該当時の仕様更新と regression 結果 |

Phase ambiguity closure に関係する仕様変更は、§0、§7.3、§9.1.10、§9.1.11、§9.1.12、§9.1.14、§9.1.15、§9.1.18、§9.1.19、§9.1.20、§9.1.21、§9.1.22、§9.1.23、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.35、§9.1.38、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.1.43、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.1.48、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.14、§9.17、該当 Phase 詳細節を同時更新する。ambiguity closure matrix がない Phase 実装 PR は、実装判断、N/A、例外、error/status、commit order、互換差分の根拠が未確定であるため、実装開始不可とする。

#### 9.1.50 Phase atomic implementation task ledger 固定契約

各 Phase の実装 PR は、Phase scope を atomic task ledger に分解し、task ID ごとに入力契約、変更対象、禁止変更、完了条件、検証 command、証跡、rollback 条件を固定しなければならない。Phase 全体を大きな一括実装として扱うこと、task ID なしで差分を追加すること、検証がない task を Done 扱いにすることは禁止する。

**Atomic task ledger 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `task_id` | `TASK-P{phase}-{number}` 形式の一意 ID |
| `phase` | task が属する Phase |
| `task_goal` | 1 task で完了させる具体的な成果。API、persistence、security、test、artifact のいずれに効くかを明記 |
| `input_contracts` | 参照する Phase packet、manifest、schema、scenario、decision、security、compatibility、ambiguity ID |
| `change_targets` | 変更してよい module、API、config、metadata、test、artifact、docs。仕様書 PR では対象節番号 |
| `forbidden_changes` | 同 task で変更してはならない surface、future Phase、互換契約、schema、error、auth、secret、dependency |
| `completion_condition` | task が完了したと判定する observable result、snapshot、artifact、Done receipt field |
| `verification_command` | task 完了を確認する command、expected exit code、環境、artifact path。実行不能なら manual exception ID |
| `rollback_condition` | task 失敗時に戻す state、metadata/file、config、runtime map、artifact。rollback 不要なら理由 |
| `dependency` | 先行 task、block する task、並列可否 |
| `evidence` | diff、test log、snapshot、fixture、secret scan、metadata before/after、review checklist |

**Phase group atomic task minimum：**

| Phase group | 最低 task 分解 |
|-------------|----------------|
| Phase 1〜5 | CLI/config、data-dir、default DB、HTTP pipeline、JWT、log/redaction、SDK smoke、restart test を分離 |
| Phase 6〜8 | DB CRUD、admin token、scope、organization/group/location、quota/block、Platform API、legacy migration を分離 |
| Phase 9〜10 | WebSocket handshake、stream、transaction、ATTACH policy、metrics persistence、ro/rw enforcement を分離 |
| Phase 11〜13 | primary frame、replica apply、checksum、snapshot、archive manifest、retention、health/redirect を分離 |
| Phase 14〜15 | backup create、restore rollback、PITR select、branch create/delete、seed isolation、quota/delete protection を分離 |
| Phase 16〜18 | extension registry、signature/load、metrics snapshot、Prometheus、HA term、promote/demote、split-brain を分離 |
| Phase 19 | shadow adapter、diff capture、active switch、fallback denial、performance baseline、rollback flag を分離 |

**atomic task 禁止事項：**

| 状態 | 判定 |
|------|------|
| Phase 実装を 1 つの巨大 task として扱う | 実装開始禁止 |
| `TASK-P{phase}-{number}` がない差分を入れる | merge 不可 |
| task の input contracts が Phase packet / manifest / matrix に接続していない | 実装開始禁止 |
| task に verification command または manual exception ID がない | Phase 未完了 |
| task 外の API、metadata、config、dependency、error code、auth 境界を変更する | merge 不可 |
| rollback condition がない状態変更 task を完了扱いにする | Phase 未完了 |
| dependency 未完了の task を先に merge する | merge 不可 |
| open task が 1 件以上ある状態で Done receipt を出す | Phase 未完了 |
| task 完了を PR description の説明だけで代替する | 仕様として扱わない |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `atomic_task_result` | task ID ごとの input contracts、change targets、completion condition、verification command、evidence |
| `open_task_count` | `0` 固定。0 以外は Phase 未完了 |
| `task_dependency_result` | dependency 順序、並列実行可否、blocked task 0 件、rollback condition の証跡 |

Phase atomic task ledger に関係する仕様変更は、§9.1.7、§9.1.8、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.16、§9.1.21、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.34、§9.1.35、§9.1.38、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.1.43、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.1.48、§9.1.49、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。atomic task ledger がない Phase 実装 PR は、実装粒度、変更境界、検証単位、rollback 単位、完了判定が未確定であるため、実装開始不可とする。

#### 9.1.51 Phase review handoff / independent reproducibility 固定契約

各 Phase の実装 PR は、実装者以外のレビュアーが仕様本文、Phase packet、artifact だけを使って同じ判断と検証を再現できる review handoff packet を固定しなければならない。口頭説明、チャット履歴、PR description だけの補足、実装者のローカル環境だけに依存する再現手順を完了根拠にしてはならない。

**Review handoff packet 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `handoff_id` | `HANDOFF-P{phase}-{number}` 形式の一意 ID |
| `phase` | handoff 対象 Phase |
| `reading_order` | 第三者が読む順番。最低でも §9.2、§9.4、§9.8、§9.11、§9.17、Phase packet、Done receipt、該当 Phase 詳細節 |
| `reproduction_commands` | local / Docker / CI の再現 command、expected exit code、必要 env、timeout、artifact path |
| `expected_artifacts` | snapshot、fixture、log、secret scan、metadata before/after、compat transcript、release-check result |
| `decision_criteria` | Done / Not Done / Spec correction required を判定する条件、error/status 差分の扱い |
| `failure_classification` | 再現失敗時に defect、spec gap、oracle gap、environment gap、security gap、compatibility diff のどれに分類するか |
| `reviewer_scope` | レビューで確認する API、persistence、security、compatibility、rollback、redaction、task ledger |
| `out_of_scope` | レビュー対象外とする項目と仕様根拠。根拠なし N/Aは禁止 |
| `oral_context_free_evidence` | 口頭補足、チャット履歴、実装者記憶なしで判断できる artifact / spec section |
| `handoff_result_location` | review result、再現 log、差分メモ、未完了判定を保存する場所 |

**Phase group handoff minimum：**

| Phase group | 最低 handoff 対象 |
|-------------|-------------------|
| Phase 1〜5 | CLI/config、data-dir、HTTP pipeline、JWT、log redaction、SDK smoke、restart reproducibility |
| Phase 6〜8 | admin API、Platform API、metadata migration、scope/quota、organization/group/location、legacy fixture |
| Phase 9〜10 | WebSocket transcript、transaction rollback、ATTACH denial、metrics persistence、ro/rw enforcement |
| Phase 11〜13 | replication frame、checksum、snapshot、archive manifest、retention、primary/replica recovery |
| Phase 14〜15 | backup/restore/PITR、branch lifecycle、destructive rollback、seed isolation、quota/delete protection |
| Phase 16〜18 | extension signature/load、metrics snapshot、Prometheus output、HA promote/demote、split-brain |
| Phase 19 | shadow/active diff、fallback denial、performance baseline、rollback flag、Phase 1〜18 regression |

**review handoff 禁止事項：**

| 状態 | 判定 |
|------|------|
| レビュアーが PR description だけを読まないと判断できない | 仕様として扱わない |
| 口頭説明、チャット履歴、実装者の記憶を完了根拠にする | merge 不可 |
| 再現 command が local only で環境差分が固定されていない | Phase 未完了 |
| expected artifact の保存先、正規化、secret scan がない | Phase 未完了 |
| reviewer decision criteria が Done / Not Done / Spec correction required に写像されていない | review failure |
| 再現失敗時の分類先がない | Phase 未完了 |
| レビュアー判断任せで N/A、差分許容、snapshot 更新を決める | merge 不可 |
| handoff result が Done receipt と接続していない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `review_handoff_result` | handoff ID ごとの reading order、reproduction command、expected artifact、decision criteria、review result |
| `independent_reproduction_result` | 実装者以外が再実行できる command、exit code、artifact、環境差分、再現可否 |
| `oral_context_free_result` | 口頭補足なしで Done / Not Done / Spec correction required を判定できる証跡 |

Phase review handoff に関係する仕様変更は、§9.1.3、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.34、§9.1.35、§9.1.36、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.1.48、§9.1.49、§9.1.50、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。review handoff packet がない Phase 実装 PR は、第三者再現性、レビュー判断基準、artifact 完備性、失敗分類が未確定であるため、Phase 完了扱いにしない。

#### 9.1.52 Phase release note / operator-facing behavior delta 固定契約

各 Phase の実装 PR は、利用者・運用者から見える挙動差分を operator behavior delta として固定しなければならない。実装内部の完了、テスト成功、レビュアー再現性が満たされていても、外部 API、CLI、config、metadata、auth、logs、health、metrics、backup、replication、HA、rollback、非対応範囲の変化が運用者向けに分類されていない場合、その Phase は完了扱いにしない。

**Operator behavior delta 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `delta_id` | `DELTA-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | delta を導入する Phase |
| `audience` | user、operator、SDK client、admin、SRE、backup operator、HA operator、developer のいずれか |
| `external_surface` | CLI / HTTP API / WebSocket / config / metadata / auth / log / health / metrics / backup / replication / branch / HA / extension |
| `behavior_before` | 前 Phase または未対応時の挙動。新規の場合は `not_available` と明記 |
| `behavior_after` | 当該 Phase で観測される挙動、status、error code、log、metric、artifact、operator action |
| `compatibility_delta` | `none`、`turso_match`、`intentional_self_host_diff`、`breaking_change`、`new_feature`、`deprecated`、`unsupported_until_phase` |
| `operator_action` | 設定変更、migration、restart、backup、token rotation、monitoring 更新、runbook 更新。不要なら理由 |
| `migration_or_config_change` | config key、metadata migration、env、default 変更、旧形式対応、rollback 可否 |
| `rollback_note` | rollback 時の operator 手順、data safety、互換性、戻せない変更の有無 |
| `release_note_text` | そのまま release note に載せられる短文。内部実装名ではなく利用者視点で書く |
| `evidence` | snapshot、SDK transcript、health/log/metric sample、migration fixture、rollback artifact、compat diff |

**Phase group operator delta minimum：**

| Phase group | 最低 delta |
|-------------|------------|
| Phase 1〜5 | CLI 起動、config precedence、data-dir layout、health/pipeline、JWT/token、log redaction、SDK 接続方法 |
| Phase 6〜8 | admin API、Platform API、multi DB、organization/group/location、quota/block policy、metadata migration |
| Phase 9〜10 | WebSocket behavior、transaction rollback、ATTACH 非対応/拒否、metrics counter、ro/rw write denial |
| Phase 11〜13 | replication primary/replica、archive manifest、retention、health/redirect、lag/degraded 表示 |
| Phase 14〜15 | backup/restore/PITR、destructive lock、branch lifecycle、seed/source、delete protection、rollback 手順 |
| Phase 16〜18 | extension allowlist、metrics/Prometheus、HA status/promote/demote、split-brain operator action |
| Phase 19 | internal adapter switch、shadow/active mode、fallback denial、performance baseline、rollback flag、外部挙動差分ゼロ |

**operator delta 禁止事項：**

| 状態 | 判定 |
|------|------|
| 外部挙動が変わるのに release note / behavior delta がない | Phase 未完了 |
| breaking change または intentional self-host diff が互換差分として分類されていない | merge 不可 |
| operator action が必要なのに手順、restart 要否、rollback が未定義 | rollout ready 不可 |
| config / metadata / token / backup / HA の変更に migration_or_config_change がない | merge 不可 |
| logs / health / metrics の変化が sample artifact なしで記載される | Phase 未完了 |
| unsupported / deprecated / future Phase の挙動が release note で不明 | review failure |
| 内部実装名だけで利用者視点の release_note_text がない | Phase 未完了 |
| rollback 不可の変更を rollback 可として扱う | merge 不可 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `operator_behavior_delta_result` | delta ID ごとの audience、external surface、before/after、compatibility delta、operator action、evidence |
| `release_note_result` | release note text、breaking/new/deprecated/unsupported 分類、Turso/self-host 差分、公開可否 |
| `operator_rollout_result` | migration/config/restart/rollback/monitoring 更新の要否と証跡 |

Phase operator behavior delta に関係する仕様変更は、§1.4、§1.5、§3.5.3、§7.3、§9.1.10、§9.1.11、§9.1.13、§9.1.15、§9.1.17、§9.1.19、§9.1.20、§9.1.22、§9.1.23、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.37、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.1.49、§9.1.50、§9.1.51、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.14、§9.17、該当 Phase 詳細節を同時更新する。operator behavior delta がない Phase 実装 PR は、利用者・運用者から見える変更、互換差分、運用手順、release note、rollback が未確定であるため、Phase 完了扱いにしない。

### 9.2 Phase 別完了ゲート

以下は各 Phase の最終判定条件である。ここに書かれた項目は「推奨」ではなく、Phase 完了の必須条件とする。

| Phase | 完了ゲート | 明示的な対象外 |
|-------|------------|----------------|
| Phase 1 | Cargo workspace が成立し、`adlaire-db serve` / `adlaire-db token create` の CLI skeleton が起動する。`--data` 未指定は clap のエラーになる。config.toml は CLI > TOML > default で解決される | DB オープン、HTTP サーバー、JWT 発行 |
| Phase 2 | `--data` 配下に `databases/`、`meta/`、`.lock` が準備され、`default/data.db` を libsql local で開ける。WAL、busy_timeout、synchronous=NORMAL、integrity_check が起動時に適用される | HTTP API、認証、マルチ DB CRUD |
| Phase 3 | `GET /v2/health` と `POST /v2/pipeline` が hrana-http v2 互換で動く。JSON 不正は HTTP 400、SQL エラーは HTTP 200 + hrana error。Web フレームワーク依存がない | JWT 認証、WebSocket、管理 API 実装、マルチ DB 完了判定 |
| Phase 4 | HS256 JWT 検証、`token create`、tokens.json 追記、revoke 照合、ro/rw 権限チェックが動く。認証なし・不正・期限切れ・失効済みの各エラーが §7.3 と一致する | DB スコープ JWT、管理 API token CRUD、自動化された全 revoke フロー |
| Phase 5 | 構造化 JSON ログ、HTTP request ログ、Phase 1〜5 の統合テスト、TypeScript SDK 互換テスト、再起動後の永続化テストが通る | マルチ DB、WebSocket、replication、backup |
| Phase 6 | `/{db-name}/v2/pipeline` が動き、DB 名バリデーション、DbManager の create/list/get/delete 内部機構、databases.json のアトミック更新が動く。`/v2/pipeline` は default fallback のまま維持される | 管理 API route の完全実装、DB スコープ JWT、backup |
| Phase 7 | 管理 API の DB CRUD、token CRUD、DB スコープ JWT、管理 API 認証、revoke 即時反映が動く。全管理 API は §6.4 の status/body に一致する | WebSocket、ATTACH、replication、backup |
| Phase 8 | Turso Cloud 互換管理モデルとして location、organization/group、quota/usage の API、metadata、migration、権限、quota 判定、エラー、`/v1/*` Platform API 互換 response wrapper が Phase 8 詳細節の契約通り実装される。既存 Phase 1〜7 の API と metadata migration は後方互換を維持する | WebSocket、ATTACH、replication、backup、branch、SQLite 拡張、内製化 |
| Phase 9 | hrana-ws v3 の hello/open_stream/execute/sequence/close_stream/store_sql/close_sql が動き、同一 stream 内の interactive transaction が同一接続で保持される | ATTACH、metrics、replication、backup |
| Phase 10 | 管理下 DB のみを対象に ATTACH が動き、任意パス ATTACH を拒否する。metrics API は counters/gauges を返し、HTTP/DB/WebSocket 経路から値が更新される | WAL replication、backup、branch |
| Phase 11 | primary role で replication API（log/snapshot/heartbeat/status）が起動し、WAL frame 番号、CRC32、snapshot header が仕様通り返る。replica 受信・適用はまだ完了条件に含めない | replica 同期完了、書き込みリダイレクト、WAL archive retention |
| Phase 12 | replica が primary から snapshot/WAL を取得して追いつき、replica 書き込みは 307 redirect または primary 到達不能時の規定エラーになる。health に role/lag が出る | WAL archive、PITR、branch |
| Phase 13 | WAL archive と manifest.json がアトミックに更新され、retention cleanup が動く。CRC32 と manifest/files の整合性検査がある | backup/restore API、PITR restore、branch |
| Phase 14 | backup、restore、PITR API が動き、restore 失敗時は元 DB が復元される。PITR 無効、範囲外、CRC 不一致のエラーが §7.3 と一致する | branch、外部ストレージ転送、HA |
| Phase 15 | branch 作成、一覧、削除、再起動後復元が動く。branch DB は `{db}___{branch}` として通常 DB と同じ pipeline でアクセスでき、元 DB と独立して書き込める | branch merge、copy-on-write 最適化、SQLite 拡張 |
| Phase 16 | SQLite 拡張ロードが動き、許可ディレクトリ、拡張 manifest、署名検証、ロード/アンロード API、sandbox 方針が固定される。未承認拡張と任意パスロードは拒否する | HA、自動 failover、libSQL 内製化、未署名拡張 |
| Phase 17 | metrics snapshot を永続化し、Prometheus text endpoint と usage/quota の整合が動く。再起動後も累積 counter が復元される | HA、自動 failover、libSQL 内製化 |
| Phase 18 | primary/replica 構成で leader election、failover、split-brain 防止、昇格/降格、health/redirect が仕様通り動く | libSQL 内製化、multi-primary write |
| Phase 19 | WAL checkpoint 制御、storage 境界、query executor 境界のうち採用対象を内製 crate へ段階移行し、Turso Cloud / libSQL SDK 互換テストが通る | SQL parser 完全内製、互換性を壊す wire/API 変更 |

### 9.3 API 実装決定表

API を実装する場合は、各 endpoint について必ず次を仕様本文または該当 Phase に明記する。

| 項目 | 必須記述 |
|------|----------|
| 認証 | 不要 / JWT 必須 / Admin token 必須 / replication token 必須 |
| HTTP method/path | method、path parameter、query parameter、末尾 slash の扱い |
| request body | JSON schema、必須/任意/null 可、unknown field の扱い |
| success response | status、headers、body schema、空 body かどうか |
| error response | status、`code`、message の粒度、部分成功があるか |
| 永続化 | 変更するファイル、アトミック更新要否、失敗時 rollback |
| ログ | INFO/WARN/ERROR の発火条件、秘匿する値 |
| テスト | 正常系、異常系、権限系、再起動後確認 |

**デフォルト決定：**

- 管理 API、Turso Platform API、destructive API、永続化 API の unknown JSON field は原則 `INVALID_REQUEST` とする。互換のため unknown field を無視できるのは hrana wire protocol など、該当節に明記した場合のみとする
- request body が空であるべき API に body がある場合は、body を無視せず `INVALID_REQUEST` とする
- path parameter は URL decode 後にバリデーションする
- 管理 API の成功レスポンスは作成 `201`、削除 `204`、取得/一覧 `200` を原則とする
- 非同期ジョブを導入する場合は、job id、status endpoint、再起動後の扱いを先に仕様化する

### 9.4 Phase 別実装契約

各 Phase の実装者は、この表の契約を満たすこと。既存の詳細節と矛盾がある場合は、この表を優先し、矛盾箇所を同時に修正する。

各 Phase の実装 PR は、この表の該当 Phase 行を §9.1.32 の Phase implementation packet に転記し、API / 永続化 / error / test / unsupported behavior の完了条件として固定してから実装する。

#### Phase 1〜5：単一 DB・HTTP・認証基盤

| Phase | 変更対象 | API/CLI 契約 | 永続化 | エラー/ログ | テスト契約 |
|-------|----------|--------------|--------|-------------|------------|
| Phase 1 | `Cargo.toml`, `adlaire-server/Cargo.toml`, `cli.rs`, `config.rs`, `main.rs` | `serve` と `token create` を clap subcommand として定義する。`serve --data` は必須。`token create` は Phase 4 までは stub でよいが、引数 validation は行う | なし | CLI parse error は clap の標準エラー。config parse error は起動失敗 | `cargo build`, `cargo test`, `adlaire-db --help`, `adlaire-db serve --help` |
| Phase 2 | `data_dir.rs`, `db/sqld_adapter.rs`, `db/manager.rs`, `db/meta.rs` | 外部 HTTP API はまだ提供しない。`run_serve` 内で `default/data.db` を開けること | `meta/databases.json`, `meta/tokens.json`, `meta/branches.json`, `.lock`, `databases/default/data.db` を初期化する | lock 取得失敗、metadata parse 失敗、integrity_check 失敗は起動失敗。skip_integrity_check は WARN | 初回起動、再起動、二重起動拒否、DB ファイル作成、WAL 設定確認 |
| Phase 3 | `http/mod.rs`, `http/pipeline.rs`, `http/health.rs`, `hrana/*`, `error.rs`, `main.rs` | `GET /v2/health`, `POST /v2/pipeline` のみ Phase 完了対象。`/v2/pipeline` は `default` DB 固定 | Phase 2 の永続化を継続。SQL 成功応答前に SQLite/libsql の commit が完了していること | malformed JSON は HTTP 400。SQL エラーは HTTP 200 + hrana error。HTTP request log は Phase 5 まで必須ではない | TC-1, TC-2, TC-6。`named_args` 非空、invalid JSON、SQL error、close 後無視を含める |
| Phase 4 | `auth/*`, `token/*`, `config.rs`, `main.rs`, `http/pipeline.rs` | `Authorization: Bearer <JWT>` を検証する。`token create` は JWT を stdout に出す。認証無効モードは secret 未指定時のみ | `meta/tokens.json` に token record を追記する。追記は atomic update | `AUTH_REQUIRED`, `AUTH_INVALID`, `AUTH_EXPIRED`, `PERMISSION_DENIED` を §7.3 通り返す。JWT/token secret はログ出力禁止 | TC-3。valid/none/bad/expired/revoked/ro-write を含める |
| Phase 5 | `tests/*`, logging middleware, `metrics.rs` stub | API 追加は禁止。既存 API の互換性と運用ログを固める | 新規永続化なし。既存 DB の再起動後永続性を検証する | JSON Lines logs。method/path/status/duration_ms を記録し、Authorization と SQL args は出さない | TC-1〜TC-6、TypeScript SDK 互換、再起動後 SELECT、ログ形式検証 |

**Phase 1 完全実装精度固定契約：**

Phase 1 は「実行可能な CLI skeleton と config 解決の土台」を完成させる Phase であり、DB、HTTP、JWT、metadata 永続化を開始してはならない。Phase 1 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 1 固定仕様 |
|------|------------------|
| 実装範囲 | Cargo workspace、binary 起動、`serve` / `token create` subcommand、help、CLI parse、config resolution |
| binary name | `adlaire-db` 固定。別名 binary を Phase 1 完了根拠にしてはならない |
| `serve --data` | 必須。未指定は clap 標準 error、非 0 exit。server startup へ進まない |
| `serve --config` | 任意。指定時は TOML parse を行い、parse error は起動失敗。存在しない file は config error |
| config precedence | CLI > env > TOML > default。Phase 1 ではこの順位の fixture を必ず作る |
| env 対象 | Phase 1 で読む env は config 解決に必要な最小項目だけ。secret / token env は読んでも JWT 発行に使わない |
| `token create` | subcommand と引数 validation だけを実装する。Phase 1 では token 文字列を発行せず、stub response または unsupported error を固定する |
| stdout/stderr | help は stdout、parse/config error は stderr。token secret、raw path の不要な展開値は出力しない |
| 永続化 | Phase 1 は永続化なし。`meta/`、`databases/`、`.lock`、`tokens.json`、`data.db` を作成しない |
| process side effect | help / invalid flag / config parse の各 scenario で data-dir 配下に file を作らない |
| log | Phase 1 では構造化 request log は対象外。CLI error は clap/config error のみ |
| 完了条件 | build/test、help snapshot、invalid flag stderr、config precedence fixture、no persistence evidence、review handoff が揃う |

**Phase 1 atomic task ledger：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P1-1` | Cargo workspace と binary skeleton を成立させる | §9.2 Phase 1、§9.4 Phase 1 | `Cargo.toml`、crate manifest、`main.rs` | DB open、HTTP server、metadata file | `cargo build` が成功し `adlaire-db --help` が起動 | `cargo build`; `adlaire-db --help` |
| `TASK-P1-2` | `serve` subcommand と `--data` 必須 validation | §9.1.20、§9.1.49 | `cli.rs` | data-dir 作成、lock 取得、DB 初期化 | `serve --help` に `--data` が表示され、未指定は非 0 exit | `adlaire-db serve --help`; `adlaire-db serve` |
| `TASK-P1-3` | `token create` subcommand skeleton | §9.2 Phase 1、§9.4 Phase 1 | `cli.rs` | JWT 発行、`tokens.json` 書き込み、secret 永続化 | help と引数 validation が動き、発行処理は Phase 4 まで未対応として固定 | `adlaire-db token create --help` |
| `TASK-P1-4` | TOML/env/default config model | §9.1.19、§9.1.42 | `config.rs` | DB/HTTP/JWT 起動 side effect | CLI > env > TOML > default の fixture が pass | config precedence test |
| `TASK-P1-5` | error surface と stdout/stderr snapshot | §7.3、§9.1.22、§9.1.35 | `cli.rs`、`config.rs` | 独自不定形 error、secret 出力 | help/invalid/config error の snapshot が固定 | help / stderr snapshot |
| `TASK-P1-6` | no persistence evidence | §9.1.21、§9.1.41、§9.1.44 | test / artifact | `meta/`、`databases/`、`.lock`、`data.db` 作成 | 全 Phase 1 scenario 後に data-dir が未作成または空である証跡 | no persistence fixture |
| `TASK-P1-7` | Phase 1 Done receipt / review handoff | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が build/help/config/no persistence を再現できる | handoff checklist |

**Phase 1 scenario / oracle 固定表：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P1-1` | `adlaire-db --help` | exit 0、stdout に top-level commands、stderr empty | help snapshot |
| `SCN-P1-2` | `adlaire-db serve --help` | exit 0、stdout に `--data` / `--config`、stderr empty | serve help snapshot |
| `SCN-P1-3` | `adlaire-db token create --help` | exit 0、stdout に token create の引数、stderr empty | token help snapshot |
| `SCN-P1-4` | `adlaire-db serve` | exit non-zero、clap error、data-dir side effect なし | invalid flag stderr + no persistence |
| `SCN-P1-5` | invalid `--config` path | exit non-zero、config error、DB/HTTP/JWT は起動しない | config error snapshot |
| `SCN-P1-6` | CLI/env/TOML/default が同時指定 | CLI 値が勝ち、env、TOML、default は losing behavior として発火しない | precedence fixture |
| `SCN-P1-7` | `token create` 実行 | JWT を発行せず、Phase 1 stub/unsupported として固定された出力または error | token stub snapshot |

**Phase 1 禁止事項：**

| 状態 | 判定 |
|------|------|
| `serve --data` で DB open まで進む | merge 不可。DB open は Phase 2 |
| HTTP listener を bind する | merge 不可。HTTP は Phase 3 |
| JWT を発行する、または `tokens.json` を作る | merge 不可。JWT/token persistence は Phase 4 |
| `{data-dir}/databases/default/data.db`、`meta/`、`.lock` を作る | merge 不可。data-dir 初期化は Phase 2 |
| Phase 2 以降の metadata schema を先に作る | merge 不可 |
| help / error snapshot なしで CLI 契約を完了扱いにする | Phase 未完了 |
| PR description だけで config precedence / no persistence を説明する | 仕様として扱わない |

**Phase 1 Done receipt 最低 fields：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | Cargo workspace、CLI skeleton、config resolution、stub token create |
| `excluded_scope` | DB open、HTTP server、JWT 発行、tokens.json、data.db、metadata 初期化 |
| `atomic_task_result` | `TASK-P1-1`〜`TASK-P1-7` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P1-1`〜`SCN-P1-7` の pass/fail、artifact path |
| `coverage_closure_result` | help、invalid flag、config precedence、no persistence の coverage gap 0 件 |
| `review_handoff_result` | 第三者が build/help/config/no persistence を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 1 は CLI skeleton 追加のみ。DB/HTTP/JWT は利用不可である release note |

**Phase 1〜5 の境界決定：**

- Phase 5 完了まで、外部公開 API は `/v2/health` と `/v2/pipeline` のみとする
- 管理 API route を早期に生やす場合は 501 stub に限定し、Phase 5 完了条件には含めない
- JWT secret が設定されている場合、Phase 4 完了前は起動拒否、Phase 4 完了後は認証有効として扱う

#### Phase 6〜10：マルチ DB・管理 API・Turso Cloud 互換管理モデル・WebSocket・ATTACH

| Phase | 変更対象 | API 契約 | 永続化 | エラー/ログ | テスト契約 |
|-------|----------|----------|--------|-------------|------------|
| Phase 6 | `db/manager.rs`, `db/meta.rs`, `http/mod.rs`, `http/pipeline.rs` | `POST /{db-name}/v2/pipeline` を追加する。`/v2/pipeline` は `default` のまま。管理 API はまだ完了対象外 | `meta/databases.json` に DB 追加/削除を atomic update。各 DB は `databases/{name}/data.db` | invalid DB name は `INVALID_DB_NAME`、予約名は `DB_RESERVED_NAME`、未存在は `DB_NOT_FOUND` | DB 名 validation、複数 DB 分離、default fallback、再起動後 DB 復元 |
| Phase 7 | `http/admin/*`, `auth/*`, `token/*`, `db/manager.rs` | `/admin/v1/databases`, `/admin/v1/tokens` を §6.4 通り実装する。Admin token は Bearer 完全一致。DB scope JWT を有効化 | `databases.json` と `tokens.json` を API 経由で更新する。削除はファイル/ディレクトリと metadata を整合させる | 管理 API 認証失敗は `401 AUTH_REQUIRED`。重複 DB は `409 DB_ALREADY_EXISTS`。revoke は即時反映 | TC-2-1〜TC-2-6。admin auth、DB CRUD、token CRUD、DB scope ro/rw |
| Phase 8 | `http/admin/*`, `http/platform/*`, `db/meta.rs`, `auth/*`, `quota/*`, `location/*`, `org/*` | §9.5 と Phase 8 詳細節に定義した organization/group/location/quota/usage API、`/v1/*` Turso Platform API 互換、既存 DB/token admin API の scope 拡張を実装する | `organizations.json`、`groups.json`、`locations.json`、`quotas.json`、`usage.json` を §9.6 通り更新し、既存 `databases.json` / `tokens.json` の migration と後方互換を保証する | `ORG_NOT_FOUND`、`GROUP_NOT_FOUND`、`LOCATION_NOT_FOUND`、`QUOTA_EXCEEDED`、`USAGE_UNAVAILABLE`、`ORG_SCOPE_DENIED`、`NOT_IMPLEMENTED` を §7.3 通り返す。secret と課金相当情報はログ出力禁止 | Phase 8 API/metadata/auth/quota/migration/Turso snapshot tests。Phase 1〜7 regression と SDK 互換を必須 |
| Phase 9 | `ws/*`, `hrana/*`, `db/sqld_adapter.rs`, `http/mod.rs` | `GET /v3/baton` と `GET /{db-name}/v3/baton` で WebSocket upgrade。hrana-ws v3 messages を実装 | SQL 実行による DB 永続化のみ。WebSocket session state はプロセス内メモリでよく、再起動復元しない | hello 前 request は protocol error。stream 未存在は hrana error。接続 close 時に未完了 transaction は rollback | TC-3-1〜TC-3-4。interactive transaction、store_sql/close_sql、auth failure、multi stream |
| Phase 10 | `attach/*`, `metrics.rs`, `http/admin/metrics`, `db/sqld_adapter.rs` | 管理下 DB の ATTACH のみ許可。`GET /admin/v1/metrics` を実装する | 新規ファイルなし。metrics はプロセス内 counters/gauges でよく再起動リセット | 任意パス ATTACH は `PERMISSION_DENIED` または `INVALID_REQUEST`。metrics 取得は admin auth 対象 | TC-3-5, TC-3-6。ATTACH 成功/拒否、metrics counters 更新 |

**Phase 6/7 の境界決定：**

- Phase 6 は path-based routing と DbManager 内部機構まで。HTTP 管理 API の完成は Phase 7
- Phase 6 で DB 作成用の内部関数を実装してよいが、外部 API として成功応答を返すのは Phase 7
- DB scope JWT は Phase 7。Phase 6 では global `a` claim のみ有効
- Turso Cloud 互換管理モデルは Phase 8。Phase 7 の DB/token CRUD は既存の最小管理 API として完了済み扱いを維持する

**Phase 8〜10 の境界決定：**

- location、organization/group、quota/usage は Phase 8 で完了条件に含める
- WebSocket の transaction/session 維持は Phase 9
- クロス DB クエリ、ATTACH policy、metrics は Phase 10
- Phase 9 の WebSocket 実装中に metrics hook を入れてよいが、metrics API 完了条件には含めない

#### Phase 11〜15：レプリケーション・WAL アーカイブ・バックアップ・ブランチ

| Phase | 変更対象 | API 契約 | 永続化 | エラー/ログ | テスト契約 |
|-------|----------|----------|--------|-------------|------------|
| Phase 11 | `replication/primary.rs`, `http/replication.rs`, `state.rs`, `config.rs` | primary role で `/replication/v1/log`, `/snapshot`, `/heartbeat`, `/status` を提供する。replica loop は未完了でよい | replication state はプロセス内。snapshot response は DB の整合した byte stream を返す | replication token 不正は `AUTH_INVALID`。from_frame 不正は `INVALID_REQUEST`。checksum を必ず出す | API contract tests、snapshot header、heartbeat status、SSE/long-poll 挙動 |
| Phase 12 | `replication/replica.rs`, redirect middleware, `http/health.rs` | replica role で primary から snapshot/WAL を取得する。replica 書き込みは 307 redirect。primary 到達不能時の挙動を固定する | replica 側 DB に WAL 適用済み状態を保存する。再起動後は最後の frame から再開する | checksum mismatch は frame skip + ERROR log + 再取得対象。health は `ok`/`degraded` を返す | TC-4-1〜TC-4-5。停止/復帰、primary down、multi replica |
| Phase 13 | `wal/archive.rs`, `wal/manifest.rs`, cleanup task, `config.rs` | 外部 API 追加は不要。archive/cleanup は内部機能 | `wal-archive/manifest.json`, frame files, snapshot file を atomic update。retention cleanup は manifest と file を同時整合 | manifest 破損は起動失敗。frame missing は ERROR。cleanup の削除件数を INFO log | TC-5-8。retention、manifest/files consistency、CRC mismatch detection |
| Phase 14 | `http/admin/backup.rs`, restore service, PITR service | backup/restore/PITR API を §6.4 通り実装する。restore/PITR は admin auth 必須 | restore 前に元 DB を退避し、失敗時は必ず rollback。PITR は snapshot + WAL replay | `PITR_NOT_ENABLED`, `FRAME_NOT_FOUND`, `RESTORE_INTEGRITY_FAILED`, `RESTORE_FRAME_CORRUPT` を使用 | TC-5-1〜TC-5-7。同時書き込み、不正 file、範囲外、CRC 破壊、rollback |
| Phase 15 | `http/admin/branches.rs`, branch metadata, `db/manager.rs`, PITR helper | branch CRUD API を実装し、branch DB は `{db}___{branch}` として通常 pipeline でアクセスする | `meta/branches.json` と branch DB directory を atomic に整合。削除時は metadata と directory の両方を消す | branch 名不正は `INVALID_DB_NAME`、`___` 衝突は `DB_RESERVED_NAME`。削除済み branch pipeline は `DB_NOT_FOUND` | TC-6-1〜TC-6-7。current/timestamp/frame branch、独立書き込み、再起動復元 |

**Phase 11/12 の境界決定：**

- Phase 11 は primary が WAL/snapshot を提供するところまで
- Phase 12 は replica がそれを消費し、health/redirect を完成させるところまで
- write-mode `sync` の完全な quorum/ack semantics は Phase 12 で定義されている範囲のみ実装する。未定義なら async と同じ挙動にしてはならず、起動時に設定エラーとする

**Phase 13/14 の境界決定：**

- Phase 13 は WAL archive を作るだけで、restore API は実装しない
- Phase 14 は archive を使って backup/restore/PITR を公開 API として完成させる
- restore/PITR は破壊的操作なので、途中失敗時の rollback 成功までが API 成功条件である

**Phase 15 の境界決定：**

- branch は通常 DB と同じ routing/auth/backup policy に従う
- branch merge、copy-on-write 最適化、外部 storage 連携は Phase 15 対象外
- branch 名と DB 名は同じ validation を使い、`___` を含む名前は禁止する

#### Phase 16〜19：拡張・監視・HA・内製化

| Phase | 変更対象 | API/CLI 契約 | 永続化 | エラー/ログ | テスト契約 |
|-------|----------|--------------|--------|-------------|------------|
| Phase 16 | `extension/*`, `http/admin/extensions.rs`, `config.rs` | `GET/POST/DELETE /admin/v1/extensions` を追加し、許可済み SQLite 拡張だけをロードする。任意パス指定は禁止し、拡張名は manifest 登録名のみ許可する | `meta/extensions.json` に拡張名、version、sha256、enabled、loaded_at を atomic update。拡張 binary は `{data-dir}/extensions/{name}/{version}/` 配下のみ | 未登録拡張は `EXTENSION_NOT_ALLOWED`、署名不一致は `EXTENSION_SIGNATURE_INVALID`、ロード失敗は `EXTENSION_LOAD_FAILED`。拡張 path と secret はログに出さない | TC-16-1〜TC-16-6。allowlist、署名、load/unload、restart、任意パス拒否、SDK regression |
| Phase 17 | `metrics.rs`, `http/admin/metrics.rs`, `http/admin/prometheus.rs`, `usage/*` | `GET /admin/v1/metrics` に永続 counter を追加し、`GET /admin/v1/metrics/prometheus` を追加する。Prometheus endpoint は Admin token 必須 | `meta/metrics-snapshot.json` を定期 atomic update。`usage.json` と quota 判定に使う storage usage を同じ計測源に統一する | snapshot 破損は WARN 後に再計測。Prometheus 出力失敗は `INTERNAL_ERROR`。metric label に secret/token/SQL args を含めない | TC-17-1〜TC-17-5。再起動後 counter 復元、Prometheus schema、quota usage 整合、破損復旧、秘匿 |
| Phase 18 | `ha/*`, `replication/*`, `http/health.rs`, `config.rs` | `GET /ha/v1/status`、`POST /ha/v1/promote`、`POST /ha/v1/demote` を追加する。HA 管理 API は Admin token + HA token 必須 | `meta/ha-state.json` に node_id、term、leader_id、last_applied_frame、role を atomic update。split-brain 防止のため term は単調増加のみ | leader 不明は `HA_NO_LEADER`、split-brain 検出は `HA_SPLIT_BRAIN`、昇格不能は `HA_PROMOTION_FAILED`。term/leader 変更は INFO、矛盾は ERROR | TC-18-1〜TC-18-7。leader election、promotion/demotion、primary down、network partition、restart、redirect、split-brain rejection |
| Phase 19 | `adlaire-wal`, `adlaire-storage`, `db/sqld_adapter.rs`, `hrana/*` | 外部 API は変更しない。内製 crate への切り替えは config flag で段階的に行い、既定は直前 Phase と同じ挙動にする | 新規 metadata 追加は禁止。必要な場合は別 Phase として仕様追加する。rollback は config flag を戻すことで可能にする | 互換性差分は `INTERNAL_ERROR` で隠さず、既存 §7.3 code に写像する。性能回帰が閾値を超えた場合は完了不可 | TC-19-1〜TC-19-6。Turso Cloud / libSQL SDK 互換、WAL consistency、crash recovery、rollback flag、performance baseline、Phase 1〜18 regression |

**Phase 16〜19 の境界決定：**

- Phase 16 は SQLite 拡張ロードのみ。HA、内製化、任意 SQL parser 変更は含めない
- Phase 17 は監視と usage 永続化のみ。quota policy 自体の変更は Phase 8 契約を変更しない限り禁止
- Phase 18 は HA と failover のみ。multi-primary write は対象外
- Phase 19 は内部実装差し替えのみ。wire format、admin API、metadata schema、JWT claim を変更してはならない

### 9.4.1 Phase 9〜19 実装精度固定表

Phase 9 以降は状態、再試行、rollback、snapshot の不足がバグ修正 PR を生むため、下表を各 Phase の実装契約に追加する。ここに書かれた項目は設計メモではなく完了条件である。

| Phase | 固定する境界 | 成功条件 | 失敗時の固定挙動 | 必須 snapshot / artifact |
|-------|--------------|----------|------------------|--------------------------|
| 9 WebSocket | upgrade、hello、stream、transaction、store_sql | hello 後のみ request を処理し、stream_id ごとに connection と tx 状態を分離する | hello 前 request は protocol error、接続 close 時の open tx は rollback、unknown message は response_error | `tests/snapshots/phase9_ws/messages.json` |
| 10 ATTACH/metrics | ATTACH SQL 解決、DB alias、counter 更新点 | 管理下 DB だけ attach し、metrics は HTTP/WS/pipeline の実行後に増加する | 任意 path、未登録 DB、不正 alias は成功させない。metrics 取得失敗は `INTERNAL_ERROR` ではなく対象 field を 0 または明示 error | `tests/snapshots/phase10_metrics/metrics.json` |
| 11 replication primary | frame_no、CRC32、SSE、snapshot headers | `from_frame` 以降を順序通り返し、snapshot は整合した DB byte stream と header を返す | frame 不在は `FRAME_NOT_FOUND`、checksum 計算不能は 500 ではなく起動/stream 失敗として記録 | `tests/snapshots/phase11_replication/api.json` |
| 12 replica/redirect | replica state、catchup、write redirect、primary down | 最後に適用した frame から再開し、replica write は規定 redirect または規定 error | checksum mismatch は破棄して再取得。primary 不達時は write を成功扱いにしない | `tests/snapshots/phase12_replica/health_redirect.json` |
| 13 WAL archive | manifest、frame file、retention cleanup | manifest と frame/snapshot file が双方向に一致する | manifest 破損は起動失敗。orphan file は WARN 後 cleanup。missing frame は PITR 対象外ではなく起動失敗 | `tests/snapshots/phase13_archive/manifest.json` |
| 14 backup/restore/PITR | restore transaction、temp layout、integrity_check | committed 前に元 DB を保持し、成功後は integrity_check が `ok` | 失敗時は必ず rollback。rollback 不能なら起動失敗 marker を残し成功応答しない | `tests/snapshots/phase14_restore/results.json` |
| 15 branch | source selector、branch name、metadata/file commit | branch DB directory 準備後に metadata active commit する | partial branch は起動時 cleanup。metadata active で DB directory 不在は起動失敗 | `tests/snapshots/phase15_branch/branches.json` |
| 16 extension | manifest、sha256、load boundary | sha256 一致、allowlist 一致、固定 directory 内だけ load | load 失敗時は metadata 追加なし。delete は metadata のみ削除し binary は残す | `tests/snapshots/phase16_extension/extensions.json` |
| 17 metrics persistence | counter snapshot、Prometheus text、usage source | snapshot は 30 秒ごとと shutdown 時に書く。usage は Phase 8 `usage.json` と同源 | snapshot 破損は WARN 後 0 から再計測。quota 判定には破損 snapshot を使わない | `tests/snapshots/phase17_metrics/prometheus.txt` |
| 18 HA | term、leader、candidate、operator promotion | term は単調増加。candidate から primary は operator promote 必須 | split-brain は `HA_SPLIT_BRAIN`。自動 primary 昇格は禁止 | `tests/snapshots/phase18_ha/status.json` |
| 19 internal adapter | config flag、shadow/active、rollback、baseline | default は libsql。adapter active でも API/metadata/JWT/wire 差分ゼロ | 性能回帰または snapshot 差分があれば未完了。silent fallback 禁止 | `tests/snapshots/phase19_internal/compat.json` |

**実装精度チェックリスト：**

```
IC-1: §9.1.2 の横断 validation を該当 endpoint 全てで実施
IC-2: §9.4.1 の snapshot/artifact を生成し、dynamic 値を正規化
IC-3: state transition の禁止遷移をテストで発火
IC-4: partial write / interrupted request / process kill 後の再起動結果を固定
IC-5: 既存 Phase の compatibility snapshot に差分がない
IC-6: Phase 外 route は success response を返さない
```

### 9.4.2 Phase 9〜19 ゼロバグ実装補完契約

Phase 9〜19 で実装者が独自判断しやすい境界は、以下を優先仕様として固定する。各 Phase 詳細節と矛盾する場合は、本節を優先し、詳細節も同じ PR で修正する。ここに書かれた項目は推奨ではなく完了条件である。

**Protocol / wire 互換固定表：**

| Phase | 対象 | 固定仕様 | テスト artifact |
|-------|------|----------|-----------------|
| 9 | WebSocket subprotocol | `Sec-WebSocket-Protocol` は client 提示順を保持して解釈し、server が Phase 9 で対応する `hrana3`、`hrana2`、`hrana1` のうち最初に一致したものを選択する。`hrana3-protobuf` は Phase 9 では未対応のため選択しない。protobuf 対応を追加する場合は先に依存と schema を本仕様へ追加する | `phase9_ws/subprotocols.json` |
| 9 | unknown protocol field | Hrana wire protocol の JSON object に含まれる unknown field は互換のため無視する。管理 API の unknown field 拒否方針を WebSocket message に適用してはならない | `phase9_ws/forward_compat.json` |
| 9 | message ordering | client は hello 応答前に request を送ってよい。server は hello 認証完了後に受信順で処理し、hello が失敗した場合は未処理 request へ response を返さず close する | `phase9_ws/pipelined_hello.json` |
| 9 | cursor API | `open_cursor` / `fetch_cursor` / `close_cursor` を受信した場合は Phase 9 では response_error `NOT_IMPLEMENTED` とし、connection は維持する。unknown request type は response_error `INVALID_REQUEST` | `phase9_ws/cursor_not_implemented.json` |
| 10 | ATTACH SQL parse | SQL text の簡易文字列分割で ATTACH を判定しない。SQL tokenizer または SQLite prepare 前の限定 parser で、文字列リテラル内の `ATTACH` を無視する | `phase10_attach/parser_cases.json` |
| 11 | replication stream | SSE は `id:{frame_no}` と `event: wal` を必須にし、`Last-Event-ID` があれば `from_frame=max(query.from_frame, last_event_id+1)` として再開する | `phase11_replication/sse_resume.txt` |
| 14 | backup body | backup は SQLite Online Backup API 相当の snapshot を返す。単純な `data.db` file copy は、共有 lock と WAL checkpoint 整合が証明できない限り禁止 | `phase14_restore/backup_headers.json` |
| 17 | Prometheus text | 出力は UTF-8、LF 改行、末尾 LF 必須。各 metric は `HELP`、`TYPE`、samples の順で grouping する。`TYPE` は最初の sample より前に 1 回だけ出す | `phase17_metrics/prometheus.txt` |

**Phase 10 ATTACH / metrics 優先順位固定表：**

| 条件 | 優先順位 | 結果 |
|------|----------|------|
| attach 対象 DB が存在しない | 1 | `404 DB_NOT_FOUND` |
| JWT / org / group / db scope 外 | 2 | `403 ORG_SCOPE_DENIED` |
| `allow_attach=false` | 3 | `403 PERMISSION_DENIED` |
| `block_reads=true` の DB を read source にする | 4 | `403 PERMISSION_DENIED` |
| `block_writes=true` の DB へ write する | 5 | `403 PERMISSION_DENIED` |
| ro token で attach 先へ write | 6 | `403 PERMISSION_DENIED` |
| alias 不正、quote 不正、任意 path | 7 | `400 INVALID_REQUEST` または DB 名 validation 由来の `INVALID_DB_NAME` |

metrics は成功・失敗の両方で `http_requests_total` と `errors_total` を更新する。SQL execution counter は libSQL に渡した step のみ加算し、validation で拒否した SQL は加算しない。WebSocket は connection close 時に `connections_active` を必ず decrement し、二重 decrement は禁止する。

**Phase 11〜15 data lifecycle 固定表：**

| Phase | 操作 | commit 順序 | crash 後の扱い |
|-------|------|-------------|----------------|
| 11 | replication snapshot | temp snapshot 作成、integrity_check、header 生成、stream 開始 | temp snapshot は起動時 cleanup。metadata は変更しない |
| 12 | replica apply | frame checksum 検証、apply、fsync、`replica-state.json` 更新 | state より進んだ frame は再検証してから再適用。重複 frame は idempotent skip |
| 13 | archive append | frame file fsync、manifest tmp fsync、rename、directory fsync | manifest にない frame は orphan として WARN 後 cleanup。manifest にある file 欠損は起動失敗 |
| 14 | restore/PITR | upload temp、verify、runtime close、old rename、new rename、directory fsync、reopen、success response | `restore-failed.json` があれば起動失敗。operator が手動復旧するまで自動上書き禁止 |
| 15 | branch create | branch dir temp、DB 構築、integrity_check、runtime open、`branches.json` commit、dir finalize | metadata active で dir 不在は起動失敗。dir だけ存在し metadata なしは cleanup |

**Phase 14 backup / restore request 固定表：**

| API | Content-Type | body limit | lock | success |
|-----|--------------|------------|------|---------|
| `GET /admin/v1/databases/{name}/backup` | response `application/octet-stream` | response streaming。メモリ全読み込み禁止 | source read lock は Online Backup API の step 中だけ | 200 + SQLite snapshot bytes |
| `POST /admin/v1/databases/{name}/restore` | request `application/octet-stream` | `restore_max_bytes` 未定義時は min(DB size x2, 1GiB)。超過は `413 PAYLOAD_TOO_LARGE` | DB exclusive restore lock。read は旧 DB で継続、write は `503 STORAGE_BUSY` | 204 empty body |
| `POST /admin/v1/databases/{name}/restore/point-in-time` | request `application/json` | JSON body 最大 64KiB | DB exclusive restore lock | 204 empty body |

restore/PITR は `delete_protection=true` の DB では `403 ORG_SCOPE_DENIED` とする。`block_writes=true` の DB では `403 PERMISSION_DENIED` とする。quota は restore 後サイズを事前推定し、超過する場合は commit 前に `QUOTA_EXCEEDED` を返す。

**Phase 15 branch / Turso seed 接続固定表：**

| 入力 | Phase 15 の扱い |
|------|-----------------|
| `/admin/v1/databases/{db}/branches` | 正式 branch API。`from` selector を必須にする |
| `/v1/organizations/{org}/databases` with `seed.type:"database"` | Phase 15 で Turso 互換 branch create に昇格してよい。ただし `branch_name` または Turso 互換の branch field が明記されない request は `INVALID_REQUEST` |
| source DB `delete_protection=true` | branch create は許可、source DB delete は active branch がある限り `403 ORG_SCOPE_DENIED` |
| source DB `block_reads=true` | branch create は `403 PERMISSION_DENIED` |
| source DB quota | branch DB は作成時に source の database quota を継承する。branch 作成で organization/group quota を超える場合は `QUOTA_EXCEEDED` |
| token scope | source DB token は branch DB へ自動拡張しない。branch 用 token は別途発行する |

**Phase 16 extension 状態遷移固定表：**

| 状態 | 意味 | 許可遷移 |
|------|------|----------|
| `registered` | manifest に存在し binary/sha256 検証済み、未ロード | `loading`、`deleted` |
| `loading` | load 試行中。API success response 前の一時状態 | `loaded`、`load_failed` |
| `loaded` | 新規 connection に load 対象 | `disabled`、`deleted` |
| `load_failed` | load 失敗。既存 connection へ影響なし | `loading`、`deleted` |
| `disabled` | manifest に残すが新規 connection に load しない | `loading`、`deleted` |
| `deleted` | manifest から削除済み。binary は残ってよい | 復帰禁止。再登録は新 record として扱う |

`extensions.json` の `loaded` boolean だけで状態を表現してはならない。Phase 16 実装時は `state` field を追加し、migration で既存 `loaded:true` は `state:"loaded"`、`loaded:false` は `state:"registered"` に変換する。

**Phase 17 Prometheus 固定表：**

| 項目 | 固定仕様 |
|------|----------|
| metric order | metric name 昇順。各 metric 内は label set の辞書順 |
| HELP escape | backslash と LF を escape |
| label escape | backslash、double quote、LF を escape |
| sample value | finite number のみ。取得不能値を `NaN` にせず、該当 sample を出さない |
| route label | route pattern のみ。raw path、DB 名入り path、query string は禁止 |
| content negotiation | `Accept` 未指定、`*/*`、`text/plain` は 200。その他は 406 `NOT_ACCEPTABLE` |

**Phase 18 HA 昇格固定表：**

| 状態 | write 可否 | 自動遷移 | operator API |
|------|------------|----------|--------------|
| `primary` | 可 | split-brain 検出時は `candidate` に降格して write 停止 | demote 可 |
| `replica` | 不可。leader 判明時は 307 redirect | heartbeat timeout で `candidate` | promote 可。ただし最新 frame 到達が必須 |
| `candidate` | 不可 | primary へ自動昇格禁止 | promote/demote 可 |
| `standalone` | 可。ただし HA enabled の場合は起動時に `candidate` へ移行 | なし | promote で primary |

promotion は `last_applied_frame >= leader_known_frame`、`term >= stored_term`、`ha-state.json` commit 成功、replication apply queue empty の全条件を満たす場合だけ成功する。どれか 1 つでも満たさない場合は `HA_PROMOTION_FAILED` とし、write を開始しない。

**Phase 19 adapter 切替固定表：**

| Mode | 実行内容 | success condition | failure |
|------|----------|-------------------|---------|
| `libsql` | 既存 production path | Phase 1〜18 regression pass | 失敗時は通常のテスト失敗 |
| `shadow` | libsql を正として adapter を副実行し、結果・error code・rows affected・last_insert_rowid を比較 | 差分ゼロ。latency は記録のみ | 差分が 1 件でもあれば Phase 19 未完了 |
| `active` | adapter 経路を response に使用 | shadow で差分ゼロ、snapshot 差分ゼロ、p95 latency 2 倍以内 | silent fallback 禁止。失敗時は error を返し rollback 手順へ |

Phase 19 では storage write path の active 化は禁止する。`active` にできるのは WAL checkpoint control と executor adapter boundary までとし、metadata migration を伴う変更は Phase 20 以降の仕様改訂なしに実装してはならない。

### 9.5 全 API endpoint 契約表

この表は実装対象 endpoint のインデックスである。詳細 schema は §6 および各 Phase 節を正とするが、認証・status・永続化・冪等性で迷った場合はこの表を優先する。

| Method / Path | Phase | 認証 | Request | Success | 主な Error | 永続化 | 冪等性 |
|---------------|-------|------|---------|---------|------------|--------|--------|
| `GET /v2/health` | 3 / 12 拡張 | 不要 | body なし | 200 JSON。Phase 3 は `{status:"ok"}`、Phase 12 以降は role/lag を追加 | 500 `INTERNAL_ERROR` | なし | Yes |
| `POST /v2/pipeline` | 3 | Phase 4 以降 JWT。Phase 3 は認証無効のみ | hrana-http v2 `PipelineRequest` | 200 `PipelineResponse`。SQL error は results 内 error | 400 `INVALID_REQUEST`, 401 auth 系, 404 `DB_NOT_FOUND`, 503 `STORAGE_BUSY` | SQL 書き込み時のみ DB | No |
| `POST /{db-name}/v2/pipeline` | 6 | JWT | path DB + hrana-http v2 | 200 `PipelineResponse` | 400 `INVALID_DB_NAME`, 404 `DB_NOT_FOUND`, auth 系, storage 系 | SQL 書き込み時のみ対象 DB | No |
| `GET /v3/baton` | 9 | WebSocket hello JWT | WebSocket upgrade | 101 Switching Protocols | 400 upgrade 不正, hello error `AUTH_*` | session 内 SQL 書き込み時のみ DB | 接続単位 |
| `GET /{db-name}/v3/baton` | 9 | WebSocket hello JWT | path DB + WebSocket upgrade | 101 Switching Protocols | 400/404/auth 系 | session 内 SQL 書き込み時のみ対象 DB | 接続単位 |
| `GET /admin/v1/databases` | 7 | Admin token | body なし | 200 `{databases:[...]}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/databases` | 7 | Admin token | `{name}` | 201 `DbInfo` | 400 `INVALID_DB_NAME`/`DB_RESERVED_NAME`, 409 `DB_ALREADY_EXISTS` | `databases.json`, DB directory | No |
| `GET /admin/v1/databases/{name}` | 7 | Admin token | body なし | 200 `DbInfo` | 400 invalid name, 404 `DB_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/databases/{name}` | 7 | Admin token | body なし | 204 empty body | 400 invalid/reserved, 404 `DB_NOT_FOUND` | `databases.json`, DB directory deletion | Yes: missing DB remains 404 |
| `GET /admin/v1/tokens` | 7 | Admin token | body なし | 200 `{tokens:[...]}` token value は返さない | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/tokens` | 7 | Admin token | `{access, expiry?, dbs?}` | 201 `{id, token, ...}`。token は作成時のみ返す | 400 `INVALID_REQUEST` | `tokens.json` | No |
| `GET /admin/v1/tokens/{id}` | 7 | Admin token | body なし | 200 token metadata。JWT 文字列は返さない | 404 `TOKEN_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/tokens/{id}` | 7 | Admin token | body なし | 204 empty body | 404 `TOKEN_NOT_FOUND` | `tokens.json`, in-memory revoke set | Yes: 既に revoked は 204 |
| `GET /admin/v1/organizations` | 8 | Admin token | body なし | 200 `{organizations:[...]}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/organizations` | 8 | Admin token | `{name, slug?}` | 201 `OrganizationInfo` | 400 `INVALID_REQUEST`, 409 `ORG_ALREADY_EXISTS` | `organizations.json` | No |
| `GET /admin/v1/organizations/{org}` | 8 | Admin token | body なし | 200 `OrganizationInfo` | 404 `ORG_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/organizations/{org}` | 8 | Admin token | body なし | 204 empty body | 404 `ORG_NOT_FOUND`, 403 `ORG_SCOPE_DENIED` | `organizations.json`, related group/quota metadata | Yes: missing org remains 404 |
| `GET /admin/v1/groups` | 8 | Admin token | query `organization?` | 200 `{groups:[...]}` | 404 `ORG_NOT_FOUND` | なし | Yes |
| `POST /admin/v1/groups` | 8 | Admin token | `{organization, name, slug?, location?}` | 201 `GroupInfo` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND`/`LOCATION_NOT_FOUND`, 409 `GROUP_ALREADY_EXISTS` | `groups.json` | No |
| `GET /admin/v1/groups/{group}` | 8 | Admin token | body なし | 200 `GroupInfo` | 404 `GROUP_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/groups/{group}` | 8 | Admin token | body なし | 204 empty body | 404 `GROUP_NOT_FOUND`, 403 `ORG_SCOPE_DENIED` | `groups.json`, related quota metadata | Yes: missing group remains 404 |
| `GET /admin/v1/locations` | 8 | Admin token | body なし | 200 `{locations:[...]}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/locations` | 8 | Admin token | `{name, provider?, region?, primary?}` | 201 `LocationInfo` | 400 `INVALID_REQUEST`, 409 `LOCATION_ALREADY_EXISTS` | `locations.json` | No |
| `GET /admin/v1/locations/{location}` | 8 | Admin token | body なし | 200 `LocationInfo` | 404 `LOCATION_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/locations/{location}` | 8 | Admin token | body なし | 204 empty body | 404 `LOCATION_NOT_FOUND`, 403 `ORG_SCOPE_DENIED` | `locations.json` | Yes: missing location remains 404 |
| `GET /admin/v1/quotas` | 8 | Admin token | query `organization?`, `group?`, `database?` | 200 `{quotas:[...]}` | 404 `ORG_NOT_FOUND`/`GROUP_NOT_FOUND`/`DB_NOT_FOUND` | なし | Yes |
| `PUT /admin/v1/quotas/{scope}` | 8 | Admin token | `{storage_bytes, rows?, write_ops_per_minute?}` | 200 `QuotaInfo` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND`/`GROUP_NOT_FOUND`/`DB_NOT_FOUND` | `quotas.json` | Yes |
| `GET /admin/v1/usage` | 8 | Admin token | query `organization?`, `group?`, `database?` | 200 `{usage:[...]}` | 404 `ORG_NOT_FOUND`/`GROUP_NOT_FOUND`/`DB_NOT_FOUND`, 503 `USAGE_UNAVAILABLE` | `usage.json` snapshot | Yes |
| `GET /v1/auth/validate` | 8 | Platform token | body なし | 200 `{"exp":integer}` | 401 auth 系 | なし | Yes |
| `POST /v1/auth/api-tokens/{tokenName}` | 8 | Platform token | body `{organization?}` または body なし | 200 `{"name","id","token"}` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND` | `tokens.json` | No |
| `DELETE /v1/auth/api-tokens/{tokenName}` | 8 | Platform token | body なし | 200 `{"token":"{tokenName}"}` | 404 `TOKEN_NOT_FOUND` | `tokens.json` | Yes: revoked は 200 |
| `GET /v1/locations` | 8 | Platform token | body なし | 200 `{"locations":{code:name}}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `GET /v1/organizations` | 8 | Platform token | body なし | 200 `[TursoOrganizationInfo]` | 401 `AUTH_REQUIRED` | なし | Yes |
| `PATCH /v1/organizations/{organizationSlug}` | 8 | Platform token | `{overages?, require_mfa?}` | 200 `{"organization":TursoOrganizationInfo}` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND` | `organizations.json` | Yes |
| `GET /v1/organizations/{organizationSlug}/usage` | 8 | Platform token | body なし | 200 `{"organization":TursoOrganizationUsage}` | 404 `ORG_NOT_FOUND`, 503 `USAGE_UNAVAILABLE` | なし | Yes |
| `GET /v1/organizations/{organizationSlug}/groups` | 8 | Platform token | body なし | 200 `{"groups":[TursoGroupInfo]}` | 404 `ORG_NOT_FOUND` | なし | Yes |
| `POST /v1/organizations/{organizationSlug}/groups` | 8 | Platform token | `{name, location}` | 200 `{"group":TursoGroupInfo}` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND`/`LOCATION_NOT_FOUND`, 409 `GROUP_ALREADY_EXISTS` | `groups.json` | No |
| `GET /v1/organizations/{organizationSlug}/groups/{groupName}` | 8 | Platform token | body なし | 200 `{"group":TursoGroupInfo}` | 404 `GROUP_NOT_FOUND` | なし | Yes |
| `PATCH /v1/organizations/{organizationSlug}/groups/{groupName}/configuration` | 8 | Platform token | `{delete_protection}` | 200 `{"delete_protection":boolean}` | 400 `INVALID_REQUEST`, 404 `GROUP_NOT_FOUND` | `groups.json` | Yes |
| `POST /v1/organizations/{organizationSlug}/groups/{groupName}/auth/rotate` | 8 | Platform token | body なし | 200 empty body | 404 `GROUP_NOT_FOUND` | `tokens.json` DB/group token revoke | Yes |
| `POST /v1/organizations/{organizationSlug}/groups/{groupName}/transfer` | 8 | Platform token | any | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `GET /v1/organizations/{organizationSlug}/databases` | 8 | Platform token | query `group?`, `schema?`, `parent?` | 200 `{"databases":[TursoDatabaseInfo]}` | 404 `ORG_NOT_FOUND` | なし | Yes |
| `POST /v1/organizations/{organizationSlug}/databases` | 8 | Platform token | `{name, group, size_limit?}` | 200 `{"database":TursoDatabaseInfo}` | 400 `INVALID_DB_NAME`/`INVALID_REQUEST`, 404 `GROUP_NOT_FOUND`, 409 `DB_ALREADY_EXISTS` | `databases.json`, DB directory | No |
| `GET /v1/organizations/{organizationSlug}/databases/{databaseName}` | 8 | Platform token | body なし | 200 `{"database":TursoDatabaseInfo}` | 404 `DB_NOT_FOUND` | なし | Yes |
| `DELETE /v1/organizations/{organizationSlug}/databases/{databaseName}` | 8 | Platform token | body なし | 200 `{"database":"{databaseName}"}` | 404 `DB_NOT_FOUND` | `databases.json`, DB directory, DB token revoke | Yes: missing DB remains 404 |
| `PATCH /v1/organizations/{organizationSlug}/databases/{databaseName}/configuration` | 8 | Platform token | `{size_limit?, delete_protection?, block_reads?, block_writes?, allow_attach?}` | 200 configuration JSON | 400 `INVALID_REQUEST`, 404 `DB_NOT_FOUND` | `databases.json`, `quotas.json` | Yes |
| `POST /v1/organizations/{organizationSlug}/databases/{databaseName}/auth/tokens` | 8 | Platform token | query `expiration?`, `authorization?`; body `{permissions?}` | 200 `{"jwt":string}` | 400 `INVALID_REQUEST`, 404 `DB_NOT_FOUND` | `tokens.json` | No |
| `POST /v1/organizations/{organizationSlug}/databases/{databaseName}/auth/rotate` | 8 | Platform token | body なし | 200 empty body | 404 `DB_NOT_FOUND` | `tokens.json` DB token revoke | Yes |
| `/v1/organizations/{organizationSlug}/members*` | 8 | Platform token | any | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `/v1/organizations/{organizationSlug}/invites*` | 8 | Platform token | any | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `/v1/organizations/{organizationSlug}/plans` | 8 | Platform token | body なし | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `GET /v1/organizations/{organizationSlug}/audit-logs` | 8 | Platform token | query `page?`, `page_size?` | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `/v1/organizations/{organizationSlug}/databases/{databaseName}/stats` | 8 | Platform token | body なし | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `/v1/upload` | 8 | Database token | binary | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `GET /admin/v1/metrics` | 10 | Admin token | body なし | 200 metrics JSON | 401 `AUTH_REQUIRED` | なし | Yes |
| `GET /replication/v1/log?from_frame=N` | 11 | replication token | query `from_frame` | 200 SSE frames | 400 `INVALID_REQUEST`, 401 `AUTH_INVALID`, 404 `FRAME_NOT_FOUND` | なし | 接続単位 |
| `GET /replication/v1/snapshot` | 11 | replication token | query/body なし | 200 octet-stream + replication headers | auth 系, 500 | なし | Yes |
| `POST /replication/v1/heartbeat` | 11 | replication token | `{replica_id, synced_frame}` | 200 `{primary_frame, lag_frames}` | 400 `INVALID_REQUEST`, auth 系 | primary in-memory replica status | Yes |
| `GET /replication/v1/status` | 11 | replication token | body なし | 200 primary replication status | auth 系 | なし | Yes |
| `GET /admin/v1/databases/{name}/backup` | 14 | Admin token | body なし | 200 octet-stream SQLite backup | 404 `DB_NOT_FOUND`, auth 系 | なし | Yes |
| `POST /admin/v1/databases/{name}/restore` | 14 | Admin token | octet-stream SQLite file | 204 empty body | 413 `PAYLOAD_TOO_LARGE`, 409 `RESTORE_INTEGRITY_FAILED`, 404 `DB_NOT_FOUND`, 403 `ORG_SCOPE_DENIED`/`PERMISSION_DENIED`/`QUOTA_EXCEEDED` | target DB replace + rollback temp | No |
| `POST /admin/v1/databases/{name}/restore/point-in-time` | 14 | Admin token | `{timestamp}` または `{frame_no}` | 204 empty body | 503 `PITR_NOT_ENABLED`, 404 `FRAME_NOT_FOUND`, 409 `RESTORE_FRAME_CORRUPT`, 403 `ORG_SCOPE_DENIED`/`PERMISSION_DENIED`/`QUOTA_EXCEEDED` | target DB replace + rollback temp | No |
| `GET /admin/v1/databases/{name}/branches` | 15 | Admin token | body なし | 200 `{branches:[...]}` | 404 `DB_NOT_FOUND` | なし | Yes |
| `POST /admin/v1/databases/{name}/branches` | 15 | Admin token | `{branch_name, from}` | 201 branch metadata | invalid/reserved name, `FRAME_NOT_FOUND` | `branches.json`, branch DB directory | No |
| `DELETE /admin/v1/databases/{name}/branches/{branch}` | 15 | Admin token | body なし | 204 empty body | 404 `DB_NOT_FOUND` | `branches.json`, branch DB directory deletion | Yes: missing branch remains 404 |
| `GET /admin/v1/extensions` | 16 | Admin token | body なし | 200 `{extensions:[...]}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/extensions` | 16 | Admin token | `{name, version, sha256, enabled?}` | 201 `ExtensionInfo` | 400 `INVALID_REQUEST`, 403 `EXTENSION_NOT_ALLOWED`, 409 `EXTENSION_ALREADY_EXISTS` | `extensions.json` | No |
| `DELETE /admin/v1/extensions/{name}` | 16 | Admin token | body なし | 204 empty body | 404 `EXTENSION_NOT_FOUND` | `extensions.json` | Yes: missing extension remains 404 |
| `GET /admin/v1/metrics/prometheus` | 17 | Admin token | body なし | 200 text/plain Prometheus exposition | 401 `AUTH_REQUIRED`, 406 `NOT_ACCEPTABLE`, 500 `INTERNAL_ERROR` | なし | Yes |
| `GET /ha/v1/status` | 18 | Admin token + HA token | body なし | 200 `HaStatus` | 401 auth 系, 503 `HA_NO_LEADER` | なし | Yes |
| `POST /ha/v1/promote` | 18 | Admin token + HA token | `{node_id, term}` | 200 `HaStatus` | 409 `HA_PROMOTION_FAILED`/`HA_SPLIT_BRAIN` | `ha-state.json` | No |
| `POST /ha/v1/demote` | 18 | Admin token + HA token | `{node_id, term}` | 200 `HaStatus` | 409 `HA_PROMOTION_FAILED` | `ha-state.json` | No |

### 9.6 永続化ファイル契約表

| Path | Phase | Owner | 初期値 | 更新方式 | fsync | 破損時挙動 | Backup 対象 |
|------|-------|-------|--------|----------|-------|------------|-------------|
| `{data-dir}/.lock` | 2 | `ProcessLock` | 空ファイル可 | open + flock。内容は意味を持たない | 不要 | flock が取れれば続行。削除不要 | No |
| `{data-dir}/meta/databases.json` | 2 / 6 / 7 / 8 | `DbManager` | `{"databases":[]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/tokens.json` | 2 / 4 / 7 | `AuthState` / token 管理 | `{"tokens":[]}` | tmp write + fsync + rename | 必須 | 起動失敗。空で上書きしない | Yes |
| `{data-dir}/meta/organizations.json` | 8 | organization 管理 | `{"organizations":[{"id":"default","name":"default","slug":"default","created_at":...}]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/groups.json` | 8 | group 管理 | `{"groups":[{"id":"default","organization":"default","name":"default","slug":"default","location":"default","delete_protection":false}]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/locations.json` | 8 | location 管理 | `{"locations":[{"id":"default","name":"default","provider":"self-hosted","region":"local","primary":true}]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/quotas.json` | 8 | quota 管理 | `{"quotas":[]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/usage.json` | 8 | usage snapshot | `{"usage":[],"updated_at":null}` | tmp write + fsync + rename | 任意。fsync 失敗時は WARN + `USAGE_UNAVAILABLE` | 破損時は起動 WARN 後に再計測。空上書きは禁止 | No |
| `{data-dir}/meta/phase8-migration.json` | 8 | migration marker | migration 完了時のみ作成 | tmp write + fsync + rename | 必須 | marker 破損時は起動失敗。再 migration しない | Yes |
| `{data-dir}/meta/migration-backup/phase8-{timestamp}/` | 8 | migration backup | migration 開始前に作成 | copy + fsync | 必須 | 復元不能なら起動失敗 | No |
| `{data-dir}/meta/branches.json` | 2 / 15 | branch 管理 | `{"branches":[]}` | tmp write + fsync + rename | 必須 | Phase 15 以降は起動失敗。Phase 14 以前は初期化のみ | Yes |
| `{data-dir}/databases/{name}/data.db` | 2+ | libsql / DbManager | libsql 作成 | libsql commit | libsql に委譲 | integrity_check NG なら起動失敗 | Yes |
| `{data-dir}/databases/{name}/data.db-wal` | 2+ | SQLite WAL | SQLite 作成 | SQLite WAL | SQLite に委譲 | SQLite recovery に委譲。integrity_check で検出 | Yes |
| `{data-dir}/meta/replica-state.json` | 12 | replica sync | replica 起動時に作成 | tmp write + fsync + rename | 必須 | 起動失敗。last_applied_frame を推測で進めない | Yes |
| `{data-dir}/databases/{name}/wal-archive/manifest.json` | 13 | WAL archive | archive 有効時に作成 | tmp write + fsync + rename | 必須 | 起動失敗。PITR/backup API は使わない | Yes |
| `{data-dir}/databases/{name}/wal-archive/frame-*.bin` | 13 | WAL archive | なし | create + write + fsync | 必須 | manifest と不整合なら ERROR。PITR 対象から除外または起動失敗を Phase 13 で固定 | Yes |
| `{data-dir}/databases/{name}/wal-archive/snapshot-*.db` | 13 | WAL archive | なし | copy + fsync + rename | 必須 | PITR 不可。manifest 整合性検査で検出 | Yes |
| restore temp dir | 14 | restore/PITR | API 実行時のみ | temp write + fsync + rename/swap | 必須 | 中断時は元 DB を復元。残骸は次回起動時に cleanup して WARN | No |
| `{data-dir}/databases/{name}/restore-failed.json` | 14 | restore/PITR | rollback 不能時のみ | tmp write + fsync + rename | 必須 | 存在する場合は起動失敗。operator が手動復旧するまで自動修復しない | Yes |
| `{data-dir}/databases/{db}___{branch}/data.db` | 15 | branch 管理 | branch 作成時 | libsql commit | libsql に委譲 | branch metadata と不整合なら WARN + branch 無効化、または起動失敗を Phase 15 で固定 | Yes |
| `{data-dir}/meta/extensions.json` | 16 | extension 管理 | `{"extensions":[]}` | tmp write + fsync + rename | 必須 | 起動失敗。未登録拡張を自動許可しない | Yes |
| `{data-dir}/extensions/{name}/{version}/` | 16 | extension binary | extension 登録時 | create + write/copy + fsync | 必須 | sha256 不一致ならロード禁止 | Yes |
| `{data-dir}/meta/metrics-snapshot.json` | 17 | metrics 永続化 | `{"counters":{},"gauges":{},"updated_at":null}` | tmp write + fsync + rename | 任意。失敗時 WARN | 破損時 WARN 後に 0 から再計測。quota usage は `usage.json` を正とする | No |
| `{data-dir}/meta/ha-state.json` | 18 | HA state | `{"node_id":null,"term":0,"leader_id":null,"role":"standalone","last_applied_frame":0}` | tmp write + fsync + rename | 必須 | 起動失敗。split-brain 防止のため自動初期化しない | Yes |

**永続化の禁止事項：**

- metadata 更新後に file 更新する順序は禁止。作成時は file を先に準備し、metadata を最後に commit する
- 削除時は runtime map から外し、DB close を確認し、directory 削除し、metadata 更新する。途中失敗時は再起動後に一貫した状態へ復旧できること
- JSON metadata を partial write してはならない。必ず tmp file を使う
- 破損 metadata を空初期値で上書きしてはならない

### 9.7 エラーコード使用契約表

| Code | 使用 Phase | 使用 API | Retry | Client action |
|------|------------|----------|-------|---------------|
| `AUTH_REQUIRED` | 4+ / admin 7+ | JWT/API/Admin/replication 認証必須 endpoint | No | Authorization header を付ける |
| `AUTH_INVALID` | 4+ | JWT/Admin/replication token 検証 | No | token を再発行または設定修正 |
| `AUTH_EXPIRED` | 4+ | JWT | No | token を再発行 |
| `AUTH_DISABLED` | 8+ | 認証無効時に許可されない管理・HA・extension 操作 | No | サーバー設定を変更 |
| `PERMISSION_DENIED` | 4+ / 10 | ro 書き込み、任意パス ATTACH 等 | No | 権限または request を変更 |
| `DB_NOT_FOUND` | 6+ | DB path, 管理 API, branch/backup | No | DB 名を確認または作成 |
| `TOKEN_NOT_FOUND` | 7+ | token get/delete | No | token id を確認 |
| `ORG_NOT_FOUND` | 8+ | organization get/delete, group/quota scope | No | organization id/slug を確認または作成 |
| `GROUP_NOT_FOUND` | 8+ | group get/delete, DB/token/quota scope | No | group id/slug を確認または作成 |
| `LOCATION_NOT_FOUND` | 8+ | location get/delete, DB/group create | No | location id/slug を確認または作成 |
| `ORG_ALREADY_EXISTS` | 8+ | organization create | No | 別の organization name/slug を使う |
| `GROUP_ALREADY_EXISTS` | 8+ | group create | No | 同一 organization 内で別の group name/slug を使う |
| `LOCATION_ALREADY_EXISTS` | 8+ | location create | No | 別の location name を使う |
| `QUOTA_EXCEEDED` | 8+ | write/import/restore/replication apply/branch create。`/v1/*` では HTTP 402、それ以外は HTTP 403 | No | quota を増やすか使用量を削減 |
| `USAGE_UNAVAILABLE` | 8+ | usage API, quota 判定不能時 | Yes | usage 再計測またはサーバーログ確認 |
| `ORG_SCOPE_DENIED` | 8+ | organization/group scope 外の admin/JWT 操作 | No | token scope または対象 scope を修正 |
| `DB_ALREADY_EXISTS` | 7+ | DB create | No | 別名を使う |
| `INVALID_DB_NAME` | 6+ | DB/branch create/path validation | No | name を修正 |
| `DB_RESERVED_NAME` | 6+ / 15 | `meta`, `admin`, `___` 含有名 | No | name を修正 |
| `NOT_IMPLEMENTED` | all | 未来 Phase の stub endpoint | No | 対象 Phase 実装後に再試行 |
| `INVALID_REQUEST` | 3+ | JSON/schema/query/body 不正 | No | request を修正 |
| `NOT_ACCEPTABLE` | 17+ | Prometheus など media type 固定 endpoint の Accept 不一致 | No | `Accept` を修正 |
| `PAYLOAD_TOO_LARGE` | 14+ | restore/upload body size limit 超過 | No | request body を小さくする |
| `SQLITE_ERROR` | 3+ | hrana results 内 | Depends | SQL を修正。busy は `STORAGE_BUSY` を使う |
| `SQLITE_CONSTRAINT` | 3+ | hrana results 内 | No | data/constraint を修正 |
| `STORAGE_BUSY` | 3+ | DB write/read lock timeout | Yes | backoff retry |
| `REPLICATION_TIMEOUT` | 11+ | sync write / replication ACK | Yes | retry または replication 状態確認 |
| `PITR_NOT_ENABLED` | 14+ | PITR/branch from timestamp/frame when archive disabled | No | `wal_retention_days` を有効化 |
| `FRAME_NOT_FOUND` | 11+ / 14+ / 15 | replication log, PITR, branch | Depends | frame range/retention を確認 |
| `RESTORE_INTEGRITY_FAILED` | 14+ | restore | No | backup file を確認 |
| `RESTORE_FRAME_CORRUPT` | 14+ | PITR | No | archive corruption を復旧 |
| `EXTENSION_NOT_ALLOWED` | 16+ | extension create/load | No | allowlist と manifest を確認 |
| `EXTENSION_NOT_FOUND` | 16+ | extension get/delete/load | No | extension 名を確認 |
| `EXTENSION_ALREADY_EXISTS` | 16+ | extension create | No | version または name を変更 |
| `EXTENSION_SIGNATURE_INVALID` | 16+ | extension create/load | No | sha256/署名を確認 |
| `EXTENSION_LOAD_FAILED` | 16+ | extension load | Depends | extension binary と SQLite ABI を確認 |
| `HA_NO_LEADER` | 18+ | HA status/write redirect | Yes | leader election 状態を確認 |
| `HA_SPLIT_BRAIN` | 18+ | HA promote/status | No | partition を解消し、operator 判断 |
| `HA_PROMOTION_FAILED` | 18+ | HA promote/demote | Depends | node health と term を確認 |
| `INTERNAL_ERROR` | all | 未分類内部エラー | Depends | server log を確認 |

### 9.8 Phase 別テストマトリクス

| Phase | 正常系 | Invalid request | Auth/permission | Persistence/restart | Crash/rollback | Regression |
|-------|--------|-----------------|-----------------|---------------------|----------------|------------|
| 1 | CLI help, config merge | unknown flag, missing `--data` | n/a | n/a | n/a | build/test |
| 2 | data dir init, DB open | invalid config, short secret | n/a | restart opens same DB | double lock / crash leaves flock releasable | Phase 1 |
| 3 | health, CREATE/INSERT/SELECT | malformed JSON, unknown type, bad value | auth disabled only | inserted data survives restart | SIGINT releases lock | Phase 1〜2 |
| 4 | valid JWT, token create | malformed JWT, bad expiry | missing/bad/expired/revoked/ro-write | tokens survive restart | token write atomicity | Phase 1〜3 |
| 5 | TS SDK CRUD, logs | n/a | auth cases from Phase 4 | TC-5 restart | graceful shutdown timeout | Phase 1〜4 |
| 6 | multi DB route isolation | invalid/reserved DB name | global JWT applies | databases.json survives restart | DB create/delete partial failure recovery | Phase 1〜5 |
| 7 | admin DB/token CRUD | malformed admin body | admin token, DB scoped JWT | tokens/databases survive restart | revoke/write atomicity | Phase 1〜6 |
| 8 | Turso Cloud 互換 management model と `/v1/*` snapshot | malformed location/org/group/quota/Turso body | admin/platform auth, ownership/scope permission | metadata migration and legacy fallback survive restart | partial metadata migration rollback | Phase 1〜7 |
| 9 | ws hello/open/execute/tx | invalid frame/order/stream | hello auth, ro-write | committed tx survives restart | disconnect rolls back open tx | Phase 1〜8 |
| 10 | ATTACH managed DB, metrics | arbitrary path ATTACH | admin metrics auth | metrics reset acceptable | n/a | Phase 1〜9 |
| 11 | primary replication APIs | bad from_frame/body | replication token | snapshot consistent | log stream disconnect/reconnect | Phase 1〜10 |
| 12 | replica catchup/redirect | bad primary URL | replication token | replica resumes from last frame | primary down / replica restart | Phase 1〜11 |
| 13 | archive manifest/frame cleanup | bad retention config | n/a | manifest survives restart | partial archive write recovery | Phase 1〜12 |
| 14 | backup/restore/PITR | bad restore file/body | admin auth | restored DB survives restart | restore failure rollback | Phase 1〜13 |
| 15 | branch create/list/delete | invalid branch name | admin/JWT on branch DB | branch survives restart | branch create/delete partial failure | Phase 1〜14 |
| 16 | extension register/list/delete/load | arbitrary path, bad sha256 | admin auth | extensions.json survives restart | failed load rollback | Phase 1〜15 |
| 17 | persistent metrics, Prometheus output | invalid metric request | admin metrics auth | metrics snapshot survives restart | corrupt snapshot recovery | Phase 1〜16 |
| 18 | leader election, promote/demote, redirect | stale term, bad node | admin + HA token | ha-state survives restart | partition / split-brain rejection | Phase 1〜17 |
| 19 | internal crate switch via config flag | invalid flag combination | n/a | no metadata migration | rollback flag restores previous path | Phase 1〜18 |

### 9.9 実装禁止事項

- 仕様にない endpoint を成功応答付きで公開しない
- Phase 外機能を「ついで」に実装しない。前倒しする場合は仕様の Phase 境界を先に変更する
- `unwrap()` / `expect()` で request 由来・disk 由来・network 由来の失敗を panic にしない
- invalid config を黙って default に fallback しない。空文字を無効扱いにする場合は仕様に明記する
- DB/branch/token metadata と実ファイルを不整合なまま成功応答しない
- 認証 secret、JWT、admin token、replication token、生 SQL 引数値、backup contents をログに出さない
- SQL 文字列を ad hoc split して複数 statement として処理しない。`sequence` は `execute_batch()` に渡す
- 任意ファイルパスを SQL/API から開かない。DB 名は必ず validation と管理 metadata 照合を通す
- Web フレームワークを導入しない。HTTP ルーティングは hyper ベースの自前実装を維持する
- `INTERNAL_ERROR` で仕様済みエラーを隠さない。対応する code がある場合は必ずそれを使う

### 9.10 Phase 16〜19 固定タスク

Phase 16〜19 は本節の固定タスクを完了条件とする。追加の仕様変更 PR なしに、ここへ未記載の API、metadata、error code、外部依存を追加してはならない。

| Phase | 実装タスク |
|-------|------------|
| Phase 16 | T16-1 extension manifest schema、T16-2 allowlist/sha256 検証、T16-3 load/unload API、T16-4 任意パス拒否、T16-5 restart 復元、T16-6 TC-16-1〜TC-16-6 |
| Phase 17 | T17-1 metrics snapshot writer、T17-2 Prometheus endpoint、T17-3 usage/quota 計測統合、T17-4 snapshot 破損復旧、T17-5 TC-17-1〜TC-17-5 |
| Phase 18 | T18-1 HA state、T18-2 leader election、T18-3 promote/demote API、T18-4 write redirect、T18-5 split-brain rejection、T18-6 restart recovery、T18-7 TC-18-1〜TC-18-7 |
| Phase 19 | T19-1 config flag、T19-2 adlaire-wal adapter、T19-3 storage boundary adapter、T19-4 executor boundary adapter、T19-5 rollback flag、T19-6 TC-19-1〜TC-19-6 |

### 9.11 Definition of Ready / Definition of Done

各 Phase の実装を始める前に Ready を満たし、merge 前に Done を満たすこと。
Done 判定は §9.1.33 の Done receipt を正とし、下表の Done は Phase 固有の最低条件として扱う。

| Phase | Definition of Ready | Definition of Done |
|-------|---------------------|--------------------|
| 1 | workspace 名、binary 名、CLI subcommand 名、必須 flag が決まっている | help 出力、config merge skeleton、build/test が通る |
| 2 | data-dir 構成、metadata 初期値、lock 方式、libsql open 設定が決まっている | 初回起動/再起動/二重起動拒否/integrity_check が通る |
| 3 | hrana-http request/response、HTTP status 境界、auth disabled 条件が決まっている | `/v2/health` と `/v2/pipeline` の正常/異常/永続化テストが通る |
| 4 | JWT claims、token record schema、expiry/revoke/access 仕様が決まっている | token create、JWT verify、revoke、ro/rw permission tests が通る |
| 5 | ログ field、秘匿対象、統合テスト環境、SDK version が決まっている | Phase 1〜5 TC と SDK 互換、永続化、ログ形式が通る |
| 6 | DB 名 validation、path routing、databases.json migration 方針が決まっている | multi DB routing、分離、再起動復元、invalid name tests が通る |
| 7 | admin API schema、admin auth、token CRUD、DB scope claim が決まっている | DB/token CRUD、DB scoped auth、revoke immediate tests が通る |
| 8 | Turso Cloud 互換の location、organization/group、quota/usage、`/v1/*` Platform API、metadata、auth、migration、error が決まっている | 互換管理モデル、metadata migration、quota/usage、権限、Turso snapshot、legacy fallback、Phase 1〜7 regression、SDK 互換 tests が通る |
| 9 | hrana-ws message schema、stream lifecycle、transaction lifecycle が決まっている | WebSocket handshake/execute/tx/store_sql tests が通る |
| 10 | ATTACH rewrite policy、metrics schema、counter 更新点が決まっている | managed ATTACH、path rejection、metrics auth/counter tests が通る |
| 11 | primary role、replication token、frame format、snapshot headers が決まっている | replication API contract、SSE/snapshot/heartbeat/status tests が通る |
| 12 | replica state persistence、redirect policy、primary-down behavior が決まっている | catchup, redirect, restart, primary-down, multi replica tests が通る |
| 13 | manifest schema、frame naming、retention cleanup、consistency check が決まっている | archive write, cleanup, corruption detection, restart tests が通る |
| 14 | restore transaction model、rollback temp layout、PITR selector schema が決まっている | backup/restore/PITR/rollback/corrupt archive tests が通る |
| 15 | branch metadata schema、branch naming、source selector、delete semantics が決まっている | branch create/list/delete/isolation/restart tests が通る |
| 16 | extension allowlist、manifest、署名/sha256、API schema、任意パス拒否が決まっている | extension CRUD/load/unload、署名検証、restart、任意パス拒否、Phase 1〜15 regression が通る |
| 17 | metrics snapshot schema、Prometheus schema、usage 計測源、破損時復旧方針が決まっている | metrics 永続化、Prometheus endpoint、usage/quota 整合、破損復旧、Phase 1〜16 regression が通る |
| 18 | HA token、node_id、term、leader election、promote/demote、split-brain policy が決まっている | leader election、failover、redirect、restart、partition、split-brain rejection、Phase 1〜17 regression が通る |
| 19 | 切り替える内製 crate、config flag、rollback flag、性能基準、互換テスト範囲が決まっている | Turso Cloud / libSQL SDK 互換、crash recovery、rollback、performance baseline、Phase 1〜18 regression が通る |

### 9.12 PR レビュー観点

PR レビューでは以下を必ず確認する。該当しない項目は PR description に `N/A` と理由を書く。

| 観点 | 確認内容 |
|------|----------|
| Phase 境界 | その PR が対象 Phase のスコープ内か。対象外機能を成功応答付きで公開していないか |
| API 契約 | method/path/auth/status/body/error が §9.5 と一致しているか |
| 永続化 | metadata と file の更新順、atomic update、fsync、rollback が §9.6 と一致しているか |
| エラー | 仕様済み error code を使っているか。`INTERNAL_ERROR` で隠していないか |
| 認証/認可 | JWT/Admin/replication token の境界が正しいか。ro/rw/DB scope が正しいか |
| ログ/秘匿 | secret/token/SQL args/backup contents をログに出していないか |
| 再起動互換 | 既存 metadata で起動できるか。migration が必要なら仕様化されているか |
| 並行性 | concurrent request、DB lock、shutdown 中 request の挙動が決まっているか |
| テスト | §9.8 の該当列を満たしているか。regression target が落ちていないか |
| 後方互換 | 既存 endpoint、metadata、config、SDK 互換を壊していないか |

### 9.13 設定値契約表

| 設定 | CLI | Env | TOML | Default | Phase | 不正値時 |
|------|-----|-----|------|---------|-------|----------|
| data dir | `--data` | なし | 書かない | 必須 | 1 | clap error / 起動失敗 |
| API port | `--port` | なし | `[server] port` | `8080` | 1 | 起動失敗 |
| admin port | `--admin-port` | なし | `[server] admin_port` | `8081` | 1 | 起動失敗 |
| config path | `--config` | なし | n/a | `{data}/config.toml` | 1 | 読み込み/parse 失敗で起動失敗 |
| JWT secret | `--auth-jwt-secret` | `ADLAIRE_JWT_SECRET` | `[auth] jwt_secret` | 認証無効 | 4 | 32 bytes 未満は起動失敗 |
| JWT secret file | `--auth-jwt-secret-file` | なし | `[auth] jwt_secret_file` | なし | 4 | 読み込み失敗/短すぎは起動失敗 |
| admin token | `--admin-auth-token` | `ADLAIRE_ADMIN_TOKEN` | `[admin] auth_token` | 管理 API 認証無効 | 7 | 空文字は未指定扱い |
| log level | `--log-level` | `ADLAIRE_LOG_LEVEL` | `[server] log_level` | `info` | 5 | 不正値は起動失敗を原則とする。fallback する場合は WARN 必須 |
| skip integrity | `--skip-integrity-check` | なし | `[storage] skip_integrity_check` | `false` | 2 | boolean parse 失敗で起動失敗 |
| busy timeout | `--busy-timeout` | なし | `[server] busy_timeout_ms` | `5000` | 2 | `0` は許可しない。起動失敗 |
| shutdown timeout | `--shutdown-timeout` | なし | `[server] shutdown_timeout` | `30` | 3 | `0` は即時 abort として明記しない限り起動失敗 |
| replication role | `--role` | なし | なし | `standalone` | 11 | unknown role は起動失敗 |
| primary port | `--primary-port` | なし | なし | `8082` | 11 | bind 失敗で起動失敗 |
| primary URL | `--primary-url` | なし | なし | replica では必須 | 12 | replica で未指定/parse 失敗なら起動失敗 |
| replication token | `--replication-auth-token` | なし | なし | なし | 11 | required mode で未指定なら起動失敗 |
| replication write mode | `--replication-write-mode` | なし | `[replication] write_mode` | `async` | 11 | unknown は起動失敗。未定義 sync semantics は起動失敗 |
| HA node id | `--ha-node-id` | なし | `[ha] node_id` | `standalone` | 18 | validation 失敗なら起動失敗 |
| HA token | `--ha-token` | `ADLAIRE_HA_TOKEN` | `[ha] token` | なし | 18 | HA API 有効時に未指定なら起動失敗 |
| HA failover timeout | `--ha-failover-timeout-ms` | なし | `[ha] failover_timeout_ms` | `10000` | 18 | `1000` 未満は起動失敗 |
| internal WAL | `--internal-wal` | なし | `[internal] wal` | `libsql` | 19 | `libsql` / `adlaire` 以外は起動失敗 |
| internal storage | `--internal-storage` | なし | `[internal] storage` | `libsql` | 19 | `libsql` / `adlaire-readonly` 以外は起動失敗 |
| internal executor | `--internal-executor` | なし | `[internal] executor` | `libsql` | 19 | `libsql` / `adlaire-adapter` 以外は起動失敗 |
| WAL mode | なし | なし | `[storage] wal_mode` | `passive` | 2 | unknown は起動失敗 |
| WAL retention | なし | なし | `[storage] wal_retention_days` | `0` | 13 | parse 失敗で起動失敗 |
| replication sync timeout | なし | なし | `[replication] sync_timeout_ms` | `5000` | 11 | `0` は起動失敗 |

#### 9.13.1 対象 Phase 前の設定値固定契約

設定値契約表に存在するが、現在の実装 Phase より後の Phase に属する設定値は、先取り実装の根拠にしてはならない。対象 Phase 前に指定された場合の挙動は下表を正とする。

| 設定分類 | 対象 Phase 前に指定された場合 | 理由 |
|----------|------------------------------|------|
| security / auth / HA / replication token | 起動失敗 | secret を受理して未使用にすると運用者が保護済みと誤認するため |
| persistence / WAL / restore / branch / archive | 起動失敗 | 値を受理して未適用にすると durability と recovery の保証が曖昧になるため |
| observability / log output / metrics | 明示 WARN を出して無効化。ただし仕様表に WARN 可と書かれている場合だけ | 監視値は data path を変えないが、silent ignore は禁止 |
| internal adapter | 起動失敗 | 互換性差分と rollback flag が未定義の状態で production path を変えないため |
| commented sample key | 指定不可。TOML に実値として書かれた場合は上記分類に従う | sample コメントは実装許可ではないため |

対象 Phase 前に設定値を受け取っても default と同じ挙動で silent ignore してはならない。起動失敗時は `INVALID_CONFIG` 相当の起動エラーとして扱い、HTTP error code には変換しない。WARN 許可の設定でも、log には key 名、対象 Phase、無効化理由だけを出し、secret 値や path の絶対値は出力しない。

対象 Phase 到達時は、§9.13 の default、不正値、優先順位、§9.1.6 の Phase 別証跡を満たすまで完了扱いにしない。設定値を実装した PR は、指定あり/なし、不正値、対象 Phase 前 fixture、config/env/CLI 優先順位のテストを必須とする。

### 9.14 セキュリティ境界表

| 境界 | 信頼しない入力 | 必須対策 | 禁止事項 |
|------|----------------|----------|----------|
| public HTTP | method/path/header/body | size limit、JSON validation、auth、timeout | panic、secret log、silent fallback |
| hrana SQL | SQL text、args、named_args | parameter conversion、write permission、ATTACH interception | SQL split、任意 path open |
| admin API | path name、JSON body、admin token | Bearer 完全一致、DB name validation、atomic update | token value の再表示、auth bypass |
| Turso Platform API | organizationSlug、databaseName、groupName、query、platform token | Bearer 完全一致、Turso name validation、wrapper snapshot、unsupported API 分類 | field casing 変更、success status 変更、body/token log |
| WebSocket | upgrade headers、frames、stream_id | hello auth、message size limit、stream lifecycle validation | hello 前 request 処理、open tx 放置 |
| replication API | token、frame_no、frame bytes | replication token、CRC32、range validation | checksum 無視、unauthenticated stream |
| extension | extension name/version/sha256/filename | allowlist、sha256、固定 directory、manifest validation | 任意 path load、SQL からの直接 load、未署名拡張 |
| HA | HA token、node_id、term、leader_id | HA token、term 単調増加、operator promotion、split-brain rejection | unauthenticated promotion、自動 primary 昇格、古い term の採用 |
| internal switch | internal flags、adapter output | 起動時 validation、rollback flag、互換 snapshot test | silent fallback、metadata migration、wire/API 差分 |
| filesystem | metadata JSON、DB files、archive files | canonical data_dir join、atomic update、integrity check | path traversal、自動上書き修復 |
| backup/restore | uploaded DB、PITR selector | temp restore、integrity_check、rollback | 元 DB の直接上書き、失敗後の不整合 |
| branch | branch name、source frame/time | DB name validation、source existence check | `___` 含有名、metadata 先行 commit |
| config/env | TOML/env/CLI values | priority rule、type validation、secret length check | invalid default fallback、secret logging |

### 9.15 互換性ルール

- 互換性判断で迷う場合は §1.5 の優先順位を正とし、Turso Cloud 互換と libSQL SDK 互換を内製化都合より優先する
- hrana-http v2 と hrana-ws v3 の wire format は後方互換を維持する
- `/v2/pipeline` は Phase 6 以降も常に `default` DB を対象とする
- 新 field を response に追加する場合は、既存 field を削除・rename しない
- metadata JSON に field を追加する場合は、古い field を読み飛ばせるようにし、既存 file の migration path を仕様化する
- config の default 値を変更する場合は、migration note と regression test を追加する
- error `code` を変更してはならない。message は詳細化してよいが、client が code で分岐できる状態を維持する
- TypeScript `@libsql/client` 互換は Phase 5 以降の regression target とする
- backup/restore/PITR/branch のファイル形式を変える場合は、旧形式読み込み可否と不可の場合の明示エラーを仕様化する
- Adlaire 独自拡張を追加する場合は、Turso 互換 mode の response、metadata、error、SDK 挙動に差分を出さない。差分が必要な場合は mode 分離、互換 snapshot、migration、rollback を仕様化してから実装する

**互換優先の merge 不可条件：**

| 状態 | 判定 |
|------|------|
| Turso 互換 mode の API path、method、request、response wrapper、field casing が仕様差分なしに変わる | merge 不可 |
| libSQL SDK regression が失敗し、差分理由と修正方針が Done receipt にない | merge 不可 |
| metadata schema の rename / delete / required field 追加に migration と rollback がない | merge 不可 |
| error code、HTTP status、retry policy が §7.3 と矛盾する | merge 不可 |
| Adlaire 拡張 mode の挙動が既定 mode に混入する | merge 不可 |
| 内製 crate への切り替えに shadow diff、rollback flag、compat snapshot がない | merge 不可 |
| Turso Cloud との差分を `INTERNAL_ERROR`、generic 500、またはログだけで隠す | Phase 未完了 |

### 9.16 実装順序ルール

各 Phase の実装は原則として次の順に行う。
詳細な step、handoff state、中断・再開、前倒し実装の扱いは §9.1.34 を正とする。

1. 仕様内の schema / error / persistence 契約を確定する
2. 永続化 schema と migration / recovery を実装する
3. core service logic を実装する
4. HTTP/WebSocket/CLI API を公開する
5. 正常系と異常系テストを追加する
6. restart / rollback / regression test を追加する
7. README や運用メモを更新する

**順序例外禁止：**

- 永続化 schema 未確定のまま API を先に公開しない
- rollback 方針未確定のまま破壊的 API を実装しない
- auth 方針未確定のまま管理 API / replication API を公開しない
- tests がない状態で Phase 完了扱いにしない

### 9.17 Phase 実装前チェックリスト

各 Phase の実装 PR は、コード変更前にこの表を満たしていることを確認する。1つでも `未定義` がある場合、その PR は実装 PR ではなく仕様修正 PR として扱う。

| 確認項目 | 必須状態 | 未定義時の扱い |
|----------|----------|----------------|
| Phase スコープ | 対象機能と対象外が §9.2 / §9.4 に明記され、resource identity / naming / path boundary が §9.1.26 に従って固定されている | 仕様追記まで実装しない |
| Phase packet | §9.1.32 に従い、scope、target surface、API、永続化、security、concurrency、compatibility、evidence、regression、unsupported behavior、completion gate が 1 セットで固定されている | 実装開始禁止 |
| Phase 受入 manifest | §9.1.10 の必須 fields が実装開始前に固定されている | 実装 PR として扱わない |
| Evidence artifact | §9.1.11 の保存先、命名、正規化、secret scan が固定されている | 証跡生成まで完了扱いにしない |
| Phase Done receipt | §9.1.33 に従い、packet、manifest、contract map、evidence index、regression result、failure closure、compatibility / redaction result が一致している | Phase 完了扱いにしない |
| Phase execution state | §9.1.34 に従い、current step、completed/open contracts、last command、allowed/forbidden changes、next command、blocking decision が明記されている | 中断・引継ぎ・再開を行わない |
| Acceptance oracle | §9.1.35 に従い、API/error/persistence/migration/SDK/unsupported/security/compatibility/regression の正解 artifact、正規化、更新条件が固定されている | snapshot / fixture / expected を更新しない |
| Defect classification | §9.1.36 に従い、spec gap、implementation bug、regression bug、compatibility diff、oracle gap、environment gap、security gap、persistence gap が分類され、`open:0` になっている | Phase 完了扱いにしない |
| Operational state / recovery runbook | §9.1.37 に従い、healthy、degraded、unavailable、recovering、rollback_required、operator_required、blocked の API / write / health / log / operator action が固定されている | failure path を実装しない |
| Dependency graph | §9.1.38 に従い、各 Contract ID の prerequisite、blocks、status、evidence、not_applicable reason が固定され、未完了 prerequisite が 0 件になっている | dependent contract を実装・公開・完了扱いにしない |
| Invariant ledger | §9.1.39 に従い、durability、metadata/file consistency、auth/scope/quota、compatibility、error surface、operational state、redaction、dependency/prerequisite の invariant と regression guard が固定され、violation が 0 件になっている | 実装開始禁止。違反がある場合は Phase 完了扱いにしない |
| Scenario matrix | §9.1.40 に従い、normal、invalid request、auth/scope/quota、persistence/restart、rollback/recovery、concurrency/idempotency、compatibility、unsupported、redaction、operational の scenario と evidence が固定され、manual only / not run / 根拠なし N/A が 0 件になっている | 実装開始禁止。ケース漏れがある場合は Phase 完了扱いにしない |
| Resource lifecycle | §9.1.41 に従い、resource type、state、allowed/forbidden transition、entry/exit condition、API behavior、write policy、commit order、recovery behavior、state evidence が固定されている | 実装開始禁止。状態遷移未定義または forbidden transition 未検証の場合は Phase 完了扱いにしない |
| Schema registry | §9.1.42 に従い、request、response、metadata、config、JWT claim、WebSocket message、artifact、log/metric の field-level schema、required/null/default/migration/compatibility/redaction が固定されている | 実装開始禁止。field 意味ズレ、根拠なし null/省略、migration 未定義の場合は Phase 完了扱いにしない |
| Decision precedence | §9.1.43 に従い、複数条件同時成立時の precedence、selected behavior、losing behavior、error/status/client action、compatibility 差分が固定されている | 実装開始禁止。分岐順の実装依存、存在漏洩、commit 後拒否、protocol 間不一致がある場合は Phase 完了扱いにしない |
| Coverage closure | §9.1.44 に従い、全 Contract ID / schema ID / scenario ID / decision ID が test、artifact、oracle、regression、N/A 理由へ対応している | 実装開始禁止。coverage gap、manual only pass、根拠なし N/A、旧 Phase regression 漏れがある場合は Phase 完了扱いにしない |
| Change impact / drift control | §9.1.45 に従い、実装中の scope、API、schema、error、metadata、auth、test、oracle、compatibility 変更が change ID、同時更新範囲、承認状態、closure evidence で閉じている | 実装開始禁止。packet freeze 後の暗黙変更、snapshot だけ更新、互換影響未評価、migration / rollback 未評価がある場合は Phase 完了扱いにしない |
| Rollout readiness | §9.1.46 に従い、startup、shutdown、restart、rollback、health、operator action、compatibility、data safety、blocked release reason が固定されている | release / deploy / production enable 禁止。Phase Done だけで rollout ready 扱い、rollback 未検証、operator_required 隠蔽、環境差分未記録の場合は運用投入不可 |
| Compatibility baseline | §9.1.47 に従い、Turso Cloud、libSQL SDK、hrana、legacy metadata、previous Phase の baseline、snapshot / SDK version、refresh trigger、差分分類、証跡が固定されている | 実装開始禁止。実装都合の snapshot 更新、SDK transcript 欠落、upstream 未確認、自己ホスト差分理由なし、古い baseline のまま Done / rollout ready は不可 |
| Security abuse / bypass resistance | §9.1.48 に従い、attack surface、untrusted input、required control、bypass attempt、expected denial、redaction、audit/log、quota/rate、persistence no-op が固定されている | 実装開始禁止。auth/scope/quota bypass、secret 漏洩、path traversal、commit 後拒否、replay 二重処理、manual only security case がある場合は Phase 完了扱いにしない |
| Ambiguity closure / implementation decision | §9.1.49 に従い、implementation question、candidate options、selected decision、rejected options、decision basis、affected contracts、edge cases、reopen trigger が固定されている | 実装開始禁止。TBD、実装判断、根拠なし N/A、open ambiguity、error/status/commit order/redaction の未決定がある場合は Phase 完了扱いにしない |
| Atomic implementation task ledger | §9.1.50 に従い、task ID、input contracts、change targets、forbidden changes、completion condition、verification command、rollback condition、dependency が固定されている | 実装開始禁止。巨大 task、task ID なし差分、検証なし task、task 外変更、rollback 未定義、open task がある場合は Phase 完了扱いにしない |
| Review handoff / independent reproducibility | §9.1.51 に従い、reading order、reproduction commands、expected artifacts、decision criteria、failure classification、oral context free evidence が固定されている | Phase 完了扱いにしない。口頭説明、PR description だけの根拠、local only 再現、artifact 欠落、レビュアー判断任せの N/A / snapshot 更新は禁止 |
| Operator-facing behavior delta | §9.1.52 に従い、audience、external surface、before/after、compatibility delta、operator action、migration/config、rollback、release note、evidence が固定されている | Phase 完了扱いにしない。release note なし、operator 影響未分類、互換差分未記載、運用手順なし、rollback 不明、log/health/metric sample 欠落は禁止 |
| 仕様矛盾 | §9.1.12 の優先順位に従い、矛盾箇所が同じ PR で解消されている | 実装 PR として扱わない |
| 仕様内相互参照 | §9.1.29 に従い、Phase 番号、TC ID、Task ID、API 契約 ID、error code、persistence key、evidence 名、manifest 参照が一致している | 仕様修正 PR に戻す |
| Verification command | §9.1.13 / §9.1.24 の command 分類、順序、exit code、artifact、toolchain、Docker/CI/local 差分が固定されている | 検証完了扱いにしない |
| Failure closure | §9.1.14 の失敗、flaky、未検証、artifact 欠落、secret 混入が同一 PR で閉じている | merge 不可 |
| 後方互換 / migration | §9.1.15 の互換影響、migration plan、rollback、旧形式 fixture が固定されている | 既存契約を変更しない |
| 設定解決 / validation | §9.1.19 に従い、CLI/env/TOML/default/secret file の優先順位、不正値、対象 Phase 前挙動、秘匿が固定されている | config 実装を開始しない |
| API 契約 | §9.1.20 / §9.1.27 / §9.1.28 に従い、method/path/auth/request/success/error/schema/validation/serialization/list order/pagination/cursor/filtering/SQL result mapping が §9.5 または各 API 節に明記されている | route を追加しない |
| Error code | §9.1.22 に従い、失敗条件ごとの `code`、status、wire surface、retry、client action、precedence が §7.3 / §9.7 に存在する | 先に error code と retry 契約を追加する |
| 永続化 | §9.1.21 / §9.1.26 に従い、ファイル名、schema、atomic update、fsync、directory sync、rollback、破損時挙動、path normalization、recovery evidence が §9.6 に明記されている | 書き込み処理を実装しない |
| 認証/認可 | §9.1.18 に従い、必要 token、scope、ro/rw、org/group、quota、block policy、拒否条件 precedence が明記されている | success response を返す API を公開しない |
| Backup / restore / PITR | Phase 13〜15 の backup、restore、PITR、branch seed は §9.1.30 に従い、temp layout、lock、commit/rollback、checksum、quota/block precedence、recovery marker、redaction が固定されている | 破壊的 API を公開しない |
| Branch lifecycle | Phase 15 の branch create/delete/seed/routing は §9.1.31 に従い、metadata/file commit 順序、source selector、isolation、token scope、quota、restart recovery が固定されている | branch API を公開しない |
| ログ/秘匿 | §9.1.17、§12、§9.14 に従い、出力 field、request id、audit 相当記録、秘匿対象、redaction evidence が明記されている | request/SQL/token をログに出す実装を入れない |
| 並行性 / job lifecycle | §9.1.16 / §9.1.25 / §9.1.28 に従い、同時 request、resource lock、idempotency、shutdown、transaction、SQL execution、long-running operation の扱いが定義されている | 並行実行で状態を変更する処理を入れない |
| 後方互換 / Turso 追従 | §9.1.23 に従い、既存 endpoint/schema/config への影響、Turso 差分分類、SDK 影響、migration path が明記されている | 既存契約を変更しない |
| テスト | §9.8 と §9.1.24 / §9.1.27 / §9.1.28 に従い、該当 Phase 行に正常/異常/認可/永続化/障害系、list/pagination/cursor/filter evidence、SQL/result/transaction evidence、CI / release-check / secret scan evidence がある | 完了扱いにしない |
| 運用 | config、metrics、health、rollback、job status / recovery 手順が必要な Phase では明記されている | 運用 API を公開しない |

**Phase 間の前倒し実装ルール：**

- 未来 Phase の内部型や helper を先に置くことは許可する。ただし外部 API、永続化 schema、成功応答を公開してはならない
- 未来 Phase の route を先に置く場合は 501 `NOT_IMPLEMENTED` のみ許可する。200/201/204/307 を返してはならない
- 未来 Phase の永続化ファイルを先に初期化する場合は、初期値・破損時挙動・読み飛ばし可否を §9.6 に明記する
- Phase 完了判定は「コードが存在する」ではなく「該当 Phase の完了ゲートとテスト契約を満たす」で行う

**仕様変更 PR と実装 PR の分離基準：**

| 変更内容 | PR 種別 |
|----------|---------|
| Phase 境界、API schema、error code、永続化 schema、認証境界を変える | 仕様変更 PR |
| 既に仕様化済みの契約をコードへ反映する | 実装 PR |
| 仕様と実装の不一致を見つけ、仕様が正しい場合 | 実装修正 PR |
| 仕様と実装の不一致を見つけ、仕様を変える必要がある場合 | 仕様変更 PR を先行 |
| テストだけで契約を固定できる場合 | 実装 PR。ただし期待値は仕様内の契約を参照する |

### 9.18 共通実装詳細

全フェーズで共有される型定義・モジュール構成を以下に示す。

> **§14.x 番号規則**：§14 以下のサブセクション番号は実装順ではなく参照便宜のための固定 ID。欠番（§14.x が存在しない番号）は将来追加のために予約している。各フェーズの実装詳細は対応フェーズ節の直下に配置するが、§14.x 番号で相互参照できる。

#### 14.1 モジュール構成

```
adlaire-server/src/
├── main.rs              ← エントリポイント・tokio ランタイム起動・run_serve / run_token_create
├── cli.rs               ← Cli / CliCommand / ServeArgs / TokenCreateArgs（clap derive）
├── config.rs            ← Config struct・CLI フラグと config.toml のマージ
├── data_dir.rs          ← DataDir::init()・ProcessLock::acquire()（flock）
├── error.rs             ← AppError enum（thiserror）・HTTP レスポンス変換
├── state.rs             ← AppState struct・Arc<> ラッパー定義
├── metrics.rs           ← Metrics struct（Phase 10 スタブ）
├── db/
│   ├── mod.rs           ← DB 名バリデーション・DbInfo 型
│   ├── manager.rs       ← DbManager struct・open/close/create/delete ロジック
│   ├── meta.rs          ← databases.json / tokens.json / branches.json 読み書き
│   └── sqld_adapter.rs  ← SqldAdapter トレイト・RealSqldAdapter 実装（§14.19）
├── auth/
│   ├── mod.rs           ← JWT 検証ロジック・Claims / AuthState struct
│   └── middleware.rs    ← extract_claims() 関数: JWT → Claims
├── token/
│   └── util.rs          ← parse_expiry() / generate_token_id()（Phase 4）
├── http/
│   ├── mod.rs           ← route() / admin_route() 手動ルーティング・hyper サーバー起動
│   ├── pipeline.rs      ← POST /v2/pipeline ハンドラ
│   ├── health.rs        ← GET /v2/health ハンドラ
│   └── admin/
│       └── mod.rs       ← 管理 API Router・admin_auth_middleware・各ハンドラスタブ（Phase 6〜14）
├── hrana/
│   ├── mod.rs           ← hrana-http v2 型の re-export
│   ├── types.rs         ← PipelineRequest / PipelineResponse / Value 等
│   └── convert.rs       ← libsql 行・カラム型 → hrana 型変換
├── ws/
│   ├── mod.rs           ← hrana-ws v3 WebSocket ハンドラ（Phase 9）
│   ├── session.rs       ← WsSession・stream_id ごとの状態管理
│   └── types.rs         ← ClientMsg / ServerMsg 型定義
├── replication/
│   ├── mod.rs           ← WAL レプリケーション共通型（Phase 11）
│   ├── primary.rs       ← SSE /replication/v1/log・snapshot ハンドラ
│   └── replica.rs       ← フレーム受信・CRC32 検証・適用ループ
└── wal/
    ├── mod.rs           ← WAL アーカイブ公開 API（Phase 13〜14）
    ├── archive.rs       ← フレーム書き込み・fsync・manifest 更新
    └── manifest.rs      ← Manifest / FrameMeta struct・アトミック保存
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
    pub metrics:     Arc<Metrics>,                    // Phase 10～
    pub role:        ServerRole,                      // Phase 11～（デフォルト Standalone）
    pub replication: Option<Arc<()>>,  // Phase 11 で ReplicationState に差し替え
}

pub type SharedState = Arc<AppState>;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum ServerRole {
    Standalone,
    Primary { primary_port: u16 },
    // Phase 11: 下記を有効化（url クレート追加が前提）
    // Replica { primary_url: url::Url },
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
    pub storage:           StorageConfig,   // skip_integrity_check は StorageConfig に移管
    pub replication:       ReplicationConfig,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub busy_timeout_ms:              u64,  // デフォルト 5000
    pub wal_checkpoint_pages:         u64,  // デフォルト 1000
    pub wal_checkpoint_mode:          WalCheckpointMode,
    pub wal_retention_days:           u64,  // 0 = PITR 無効
    pub integrity_check_interval_hrs: u64,  // 0 = 無効
    pub skip_integrity_check:         bool, // 起動時整合性チェックをスキップ（デフォルト false）
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalCheckpointMode { Passive, Full, Restart }

#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub write_mode:      ReplicationWriteMode,
    pub sync_timeout_ms: u64,           // デフォルト 5000
    pub auth_token:      Option<String>, // --replication-auth-token（Phase 11）
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicationWriteMode { Async, Sync }
```

#### DbManager

```rust
// db/manager.rs
pub struct DbManager {
    data_dir: PathBuf,
    dbs:      tokio::sync::RwLock<HashMap<String, Arc<dyn SqldAdapter>>>,
    meta:     tokio::sync::RwLock<DatabasesMeta>,
    config:   Arc<StorageConfig>,
}

impl DbManager {
    /// 起動時: databases.json を読み込み、各 DB を RealSqldAdapter::open() でオープン
    /// Phase 3 シングル DB モード: "default" が存在しなければ自動作成する
    pub async fn open_all(data_dir: &Path, config: Arc<StorageConfig>) -> anyhow::Result<Self> {
        let mut meta = DatabasesMeta::load(data_dir)?;

        // "default" DB が存在しなければ作成する（Phase 3 シングル DB モード）
        if !meta.databases.iter().any(|d| d.name == "default") {
            let info = DbInfo {
                id:         uuid::Uuid::new_v4().to_string(),
                name:       "default".to_string(),
                created_at: chrono::Utc::now(),
                size_bytes: 0,
            };
            let db_path = data_dir.join("databases").join("default");
            std::fs::create_dir_all(&db_path)?;
            meta.databases.push(info);
            meta.save(data_dir)?;
        }

        let mut dbs: HashMap<String, Arc<dyn SqldAdapter>> = HashMap::new();
        for db in &meta.databases {
            let db_file = data_dir.join("databases").join(&db.name).join("data.db");
            match RealSqldAdapter::open(&db_file, config.busy_timeout_ms, !config.skip_integrity_check).await {
                Ok(adapter) => {
                    dbs.insert(db.name.clone(), Arc::new(adapter));
                    tracing::info!(db = %db.name, "opened database");
                }
                Err(e) => {
                    tracing::error!(db = %db.name, err = %e, "failed to open database");
                }
            }
        }

        tracing::info!(count = dbs.len(), "DbManager ready");
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            dbs:      tokio::sync::RwLock::new(dbs),
            meta:     tokio::sync::RwLock::new(meta),
            config,
        })
    }

    /// DB 名 → SqldAdapter を返す（存在しない場合 None）
    pub async fn get(&self, name: &str) -> Option<Arc<dyn SqldAdapter>> {
        self.dbs.read().await.get(name).cloned()
    }

    /// DB 作成: 存在チェック（読み取りロック）→ ディレクトリ作成 → アダプタ生成 → meta 更新（書き込みロック）
    pub async fn create(&self, name: &str) -> Result<DbInfo, AppError> {
        {
            let meta = self.meta.read().await;
            if meta.databases.iter().any(|d| d.name == name) {
                return Err(AppError::DbAlreadyExists(name.to_string()));
            }
        }

        let db_path = self.data_dir.join("databases").join(name);
        std::fs::create_dir_all(&db_path).map_err(|e| AppError::Internal(e.into()))?;

        let db_file = db_path.join("data.db");
        let adapter = RealSqldAdapter::open(&db_file, self.config.busy_timeout_ms, !self.config.skip_integrity_check)
            .await
            .map_err(|e| AppError::Internal(e))?;

        let info = DbInfo {
            id:         uuid::Uuid::new_v4().to_string(),
            name:       name.to_string(),
            created_at: chrono::Utc::now(),
            size_bytes: 0,
        };

        {
            let mut meta = self.meta.write().await;
            meta.databases.push(info.clone());
            meta.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        }

        self.dbs.write().await.insert(name.to_string(), Arc::new(adapter));
        tracing::info!(db = name, id = %info.id, "created database");
        Ok(info)
    }

    /// DB 削除: 存在チェック（読み取りロック）→ アダプタ削除 → ディレクトリ削除 → meta 更新（書き込みロック）
    pub async fn delete(&self, name: &str) -> Result<(), AppError> {
        {
            let meta = self.meta.read().await;
            if !meta.databases.iter().any(|d| d.name == name) {
                return Err(AppError::DbNotFound(name.to_string()));
            }
        }

        self.dbs.write().await.remove(name);

        let db_path = self.data_dir.join("databases").join(name);
        if db_path.exists() {
            std::fs::remove_dir_all(&db_path).map_err(|e| AppError::Internal(e.into()))?;
        }

        {
            let mut meta = self.meta.write().await;
            meta.databases.retain(|d| d.name != name);
            meta.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        }

        tracing::info!(db = name, "deleted database");
        Ok(())
    }

    /// DB 一覧（size_bytes は data.db のファイルサイズを動的取得）
    pub async fn list(&self) -> Vec<DbInfo> {
        let meta = self.meta.read().await;
        meta.databases.iter().map(|info| {
            let db_path = self.data_dir.join("databases").join(&info.name).join("data.db");
            let size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
            DbInfo { size_bytes, ..info.clone() }
        }).collect()
    }

    /// DB 名で DbInfo を返す（存在しない場合は DbNotFound、size_bytes はファイルから動的取得）
    pub async fn get_info(&self, name: &str) -> Result<DbInfo, AppError> {
        let meta = self.meta.read().await;
        let info = meta.databases.iter()
            .find(|d| d.name == name)
            .cloned()
            .ok_or_else(|| AppError::DbNotFound(name.to_string()))?;
        let db_path = self.data_dir.join("databases").join(name).join("data.db");
        let size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
        Ok(DbInfo { size_bytes, ..info })
    }

    /// シャットダウン時: ログのみ（Arc<dyn SqldAdapter> のドロップは Drop に委ねる）
    pub async fn close_all(self) {
        let count = self.dbs.read().await.len();
        tracing::info!(count, "closing all databases");
    }
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

    /// DB 名に対する書き込み権限の有無を返す（per-DB アクセスレベルを優先）
    pub fn can_write_db(&self, db_name: &str) -> bool {
        self.resolve_access(db_name) == AccessLevel::Rw
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

/// Phase 3 スタブ。Phase 4 で full 実装（jsonwebtoken / revoke リスト）に差し替える。
pub struct AuthState {
    pub(crate) secret_bytes: Option<Vec<u8>>,
}

impl AuthState {
    /// 設定から生成する（Phase 3 スタブ）
    pub fn new(config: &Config) -> Self {
        Self { secret_bytes: config.jwt_secret_bytes.clone() }
    }

    /// JWT 認証が有効かどうか（secret が設定されていれば true）
    pub fn is_auth_enabled(&self) -> bool { self.secret_bytes.is_some() }

    /// JWT 検証（Phase 4 で実装。Phase 3 では常に AuthInvalid を返す）
    pub async fn verify(&self, _raw_token: &str) -> Result<Claims, AppError> {
        Err(AppError::AuthInvalid)
    }
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
    #[error("not acceptable")]
    NotAcceptable,
    #[error("payload too large")]
    PayloadTooLarge,
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
    Sqld(String),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn into_response(self) -> Response<Full<Bytes>> {
        use http::StatusCode;
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
            Self::NotAcceptable         => (StatusCode::NOT_ACCEPTABLE,          "NOT_ACCEPTABLE"),
            Self::PayloadTooLarge       => (StatusCode::PAYLOAD_TOO_LARGE,       "PAYLOAD_TOO_LARGE"),
            Self::StorageBusy           => (StatusCode::SERVICE_UNAVAILABLE,     "STORAGE_BUSY"),
            Self::ReplicationTimeout    => (StatusCode::SERVICE_UNAVAILABLE,     "REPLICATION_TIMEOUT"),
            Self::PitrNotEnabled        => (StatusCode::SERVICE_UNAVAILABLE,     "PITR_NOT_ENABLED"),
            Self::FrameNotFound         => (StatusCode::NOT_FOUND,               "FRAME_NOT_FOUND"),
            Self::RestoreIntegrityFailed=> (StatusCode::CONFLICT,                "RESTORE_INTEGRITY_FAILED"),
            Self::RestoreFrameCorrupt   => (StatusCode::CONFLICT,                "RESTORE_FRAME_CORRUPT"),
            Self::AuthDisabled          => (StatusCode::UNAUTHORIZED,            "AUTH_DISABLED"),
            Self::ConfigError(_)        => (StatusCode::INTERNAL_SERVER_ERROR,   "INTERNAL_ERROR"),
            Self::Sqld(_)              => (StatusCode::INTERNAL_SERVER_ERROR,   "INTERNAL_ERROR"),
            Self::Internal(_)           => (StatusCode::INTERNAL_SERVER_ERROR,   "INTERNAL_ERROR"),
        };
        let body = serde_json::to_vec(&serde_json::json!({
            "error": self.to_string(),
            "code":  code,
        })).unwrap_or_default();
        Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(Full::from(body))
            .unwrap()
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
    #[serde(default)]
    pub args:       Vec<Value>,
    #[serde(default)]
    pub named_args: Vec<NamedArg>,
    #[serde(default)]
    pub want_rows:  bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
    Execute  { result: StmtResult },
    Sequence,
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

#### WAL アーカイブ manifest 型（Phase 13〜14）

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

#### Metrics（Phase 10 スタブ）

```rust
// metrics.rs
// Phase 10 で AtomicU64 カウンター・DashMap に拡張する
#[derive(Default)]
pub struct Metrics;

impl Metrics {
    pub fn new() -> Self { Self }
}
```

Phase 10 実装時の完全版（参考）：
```rust
// Phase 10 で以下に差し替える
pub struct Metrics {
    pub started_at:      std::time::Instant,
    pub databases:       dashmap::DashMap<String, DbMetrics>,
    pub tokens_total:    std::sync::atomic::AtomicU64,
    pub tokens_revoked:  std::sync::atomic::AtomicU64,
}

#[derive(Default)]
pub struct DbMetrics {
    pub queries_total:      std::sync::atomic::AtomicU64,
    pub rows_read_total:    std::sync::atomic::AtomicU64,
    pub rows_written_total: std::sync::atomic::AtomicU64,
    pub connections_active: std::sync::atomic::AtomicI64,
    pub integrity_errors:   std::sync::atomic::AtomicU64,
    pub wal_size_bytes:     std::sync::atomic::AtomicU64,
}
```


### Phase 1：ビルド基盤・CLI

**目標**：Cargo ワークスペースを確立し、CLI の骨格を動かす

**スコープ：**
- Cargo workspace 初期化（adlaire-server crate + libsql crate）
- clap による `serve` / `token` サブコマンド骨格
- config.toml 3-way マージ（CLI > TOML > デフォルト）
- CI: cargo build / cargo test が通る状態を維持

**実装タスク：**

```
T-1: リポジトリ・ビルド基盤
  [ ] Cargo workspace 初期化（adlaire-server crate）
  [ ] libsql = "0.6" を [workspace.dependencies] に追加
  [ ] cargo build が通ることを確認
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
// cli.rs
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

#[derive(Parser, Default)]
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
    // Phase 11: レプリケーション設定
    #[arg(long)] pub role:                                     Option<String>,
    #[arg(long)] pub primary_port:                             Option<u16>,
    #[arg(long)] pub primary_url:                              Option<String>,
    #[arg(long)] pub replication_auth_token:                   Option<String>,
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
    let config = Config::resolve(&args)?;

    // Step 2: ログ初期化（tracing + tracing-subscriber JSON）
    init_tracing(&config.log_level);

    tracing::info!(
        version  = env!("CARGO_PKG_VERSION"),
        data_dir = %config.data_dir.display(),
        port     = config.port,
        "Adlaire DB starting"
    );

    if config.storage.skip_integrity_check {
        tracing::warn!("--skip-integrity-check is set; startup integrity check disabled");
    }

    // Step 3: データディレクトリ初期化
    DataDir::init(&config.data_dir)?;

    // Step 4: プロセス排他ロック（flock LOCK_EX | LOCK_NB）
    let _lock = ProcessLock::acquire(&config.data_dir)?;

    // Step 5: AuthState 初期化（Phase 3 スタブ。Phase 4 で tokens.json 読み込みを追加）
    let auth = Arc::new(AuthState::new(&config));
    if !auth.is_auth_enabled() {
        tracing::warn!("JWT auth is disabled — all requests are unauthenticated");
    }
    // Phase 4 未実装のため JWT 設定時は起動を拒否する（verify() が常に失敗するため）
    anyhow::ensure!(
        !auth.is_auth_enabled(),
        "JWT auth is configured but not yet implemented (Phase 4). \
         Unset jwt_secret to start in unauthenticated mode."
    );

    // Step 6: DB 全件オープン（起動時整合性チェック込み）
    let db_mgr = Arc::new(
        DbManager::open_all(&config.data_dir, Arc::new(config.storage.clone())).await?
    );

    // Step 7: AppState 構築
    let role = parse_server_role(&args)?;
    let state: SharedState = Arc::new(AppState {
        config:      Arc::clone(&config),
        db_mgr,
        auth,
        metrics:     Arc::new(Metrics::new()),
        role,
        replication: None,
    });

    // Step 8: TCP ソケット bind
    let api_listener   = tokio::net::TcpListener::bind(("0.0.0.0",       config.port)).await?;
    let admin_listener = tokio::net::TcpListener::bind(("127.0.0.1", config.admin_port)).await?;

    tracing::info!(
        addr       = format!("0.0.0.0:{}", config.port),
        admin_addr = format!("127.0.0.1:{}", config.admin_port),
        "Adlaire DB listening"
    );

    // Step 9: グレースフルシャットダウン付きでサーバー起動
    let shutdown_timeout = config.shutdown_timeout;

    let (sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
    let mut sd_rx2 = sd_rx.clone();

    let api_task = tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            loop {
                tokio::select! {
                    Ok((stream, _)) = api_listener.accept() => {
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            let _ = http1::Builder::new()
                                .serve_connection(
                                    TokioIo::new(stream),
                                    service_fn(move |req| {
                                        let state = Arc::clone(&state);
                                        async move { http::route(req, state).await }
                                    }),
                                )
                                .await;
                        });
                    }
                    _ = sd_rx.changed() => break,
                }
            }
        }
    });
    let admin_task = tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            loop {
                tokio::select! {
                    Ok((stream, _)) = admin_listener.accept() => {
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            let _ = http1::Builder::new()
                                .serve_connection(
                                    TokioIo::new(stream),
                                    service_fn(move |req| {
                                        let state = Arc::clone(&state);
                                        async move { http::admin_route(req, state).await }
                                    }),
                                )
                                .await;
                        });
                    }
                    _ = sd_rx2.changed() => break,
                }
            }
        }
    });

    let sig_name = shutdown_signal_named().await;
    tracing::info!(signal = sig_name, "shutdown signal received");
    let _ = sd_tx.send(true);

    tokio::select! {
        _ = async { let _ = tokio::join!(api_task, admin_task); } => {},
        _ = tokio::time::sleep(std::time::Duration::from_secs(shutdown_timeout)) => {
            tracing::warn!(timeout_secs = shutdown_timeout, "graceful shutdown timed out, forcing exit");
        }
    }

    // Step 10: DB クローズ（WAL flush + checkpoint）
    match Arc::try_unwrap(state) {
        Ok(s) => match Arc::try_unwrap(s.db_mgr) {
            Ok(mgr) => mgr.close_all().await,
            Err(arc) => tracing::warn!(
                refs = Arc::strong_count(&arc),
                "db_mgr still referenced at shutdown — skipping close_all"
            ),
        },
        Err(arc) => tracing::warn!(
            refs = Arc::strong_count(&arc),
            "state still referenced at shutdown — skipping close_all"
        ),
    }

    tracing::info!("Adlaire DB stopped");
    Ok(())
}

fn run_token_create(args: TokenCreateArgs) -> anyhow::Result<()> {
    let secret = args.secret.as_bytes().to_vec();
    anyhow::ensure!(secret.len() >= 32, "--secret は 32 バイト以上の文字列を指定してください");

    let _access: AccessLevel = match args.access.as_str() {
        "rw" => AccessLevel::Rw,
        "ro" => AccessLevel::Ro,
        other => anyhow::bail!("unknown access level: {other}. Use 'rw' or 'ro'"),
    };

    // Phase 4 で JWT 発行を実装する
    println!("(Phase 4 stub) token create — secret len={}", secret.len());
    Ok(())
}

async fn shutdown_signal_named() -> &'static str {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint  = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv()  => "SIGINT",
        _ = sigterm.recv() => "SIGTERM",
    }
}

fn init_tracing(log_level: &str) {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_new(log_level)
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .json()
        .with_env_filter(filter)
        .with_current_span(false)
        .init();
}
```


#### 14.13 Config 解決ロジック

> **フィールドマッピング注意**：`busy_timeout_ms` は TOML では `[server]` セクションに書くが（§4.2）、Rust の型システムでは `StorageConfig::busy_timeout_ms` に格納する。これは「ストレージに近い設定」として内部的に分類しているためで、TOML の `[storage]` に書いても無視される。

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
    pub wal_mode:                          Option<String>,
    pub skip_integrity_check:              Option<bool>,
    pub wal_retention_days:                Option<u64>,
    pub integrity_check_interval_hours:    Option<u64>,
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
        let log_level_env = std::env::var("ADLAIRE_LOG_LEVEL").ok();
        let log_level  = args.log_level.as_deref()
            .or(log_level_env.as_deref())    // env（§8.4: CLI > env > TOML）
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
            .or_else(|| std::env::var("ADLAIRE_ADMIN_TOKEN").ok())
            .or(adm.auth_token);

        Ok(Arc::new(Config {
            data_dir:             args.data.clone(),
            port,
            admin_port,
            log_level,
            admin_auth_token,
            jwt_secret_bytes:     raw_secret,
            shutdown_timeout,
            storage: StorageConfig {
                busy_timeout_ms:              busy_timeout_ms,
                wal_checkpoint_pages:         1000,
                wal_checkpoint_mode:          wal_mode,
                wal_retention_days:           0,
                integrity_check_interval_hrs: 0,
                skip_integrity_check,
            },
            replication: ReplicationConfig {
                write_mode,
                sync_timeout_ms: 5000,
                auth_token: args.replication_auth_token.clone(),
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

pub fn parse_server_role(args: &ServeArgs) -> anyhow::Result<ServerRole> {
    match args.role.as_deref().unwrap_or("standalone") {
        "standalone" => Ok(ServerRole::Standalone),
        "primary"    => Ok(ServerRole::Primary {
            primary_port: args.primary_port.unwrap_or(8082),
        }),
        // Phase 11 で Replica variant 解除後に有効化：
        // "replica" => {
        //     let url = args.primary_url.as_deref()
        //         .ok_or_else(|| anyhow::anyhow!("--primary-url は --role replica 時に必須です"))?;
        //     Ok(ServerRole::Replica { primary_url: url.parse()? })
        // }
        other => anyhow::bail!(
            "unknown role '{other}'. Use 'standalone', 'primary', or 'replica'"
        ),
    }
}
```

---


### Phase 2：データディレクトリ・libsql 統合

**目標**：データディレクトリを初期化し、libsql でシングル DB を開ける状態にする

**スコープ：**
- `--data` パスのディレクトリ作成・パーミッション設定
- flock による排他プロセスロック
- libsql::Builder::new_local() による DB オープン・WAL モード設定

**実装タスク：**

```
T-3: データディレクトリ初期化
  [ ] --data パスの作成（mkdir -p）
  [ ] .lock ファイルによる排他ロック（flock）
  [ ] databases/ meta/ サブディレクトリ作成
  [ ] ディレクトリパーミッション警告（700 未満で WARN）
  参照: §3.2, §8.1 Step 3〜4, §10.3

T-4: libsql 統合・DB オープン
  [ ] libsql::Builder::new_local() でシングル DB を開く
  [ ] PRAGMA journal_mode = WAL を起動時に適用
  [ ] busy_timeout を設定
  [ ] PRAGMA synchronous = NORMAL を設定
  [ ] サーバーシャットダウン時に Arc<libsql::Database> を drop（WAL flush + close）
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
            let p = if sub.is_empty() { data_dir.to_path_buf() } else { data_dir.join(sub) };
            std::fs::create_dir_all(&p)?;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700))?;

            let mode = std::fs::metadata(&p)?.permissions().mode() & 0o777;
            if mode > 0o700 {
                tracing::warn!(
                    path = %p.display(),
                    mode = format!("{:04o}", mode),
                    "data directory permissions are broader than 0700"
                );
            }
        }
        // §8.1 Step 5-1: databases.json が存在しなければ空で初期化する
        let databases_path = data_dir.join("meta").join("databases.json");
        if !databases_path.exists() {
            std::fs::write(&databases_path, r#"{"databases":[]}"#)?;
        }

        // §8.1 Step 5-2: tokens.json が存在しなければ空で初期化する
        let tokens_path = data_dir.join("meta").join("tokens.json");
        if !tokens_path.exists() {
            std::fs::write(&tokens_path, r#"{"tokens":[]}"#)?;
        }

        tracing::info!(path = %data_dir.display(), "DataDir initialized");
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

libsql への依存を 1 ファイルに集約し、アップストリーム変更の影響範囲を限定する。

```rust
// db/sqld_adapter.rs

// ── SQL 値型 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

// ── 実行結果 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct SqlResult {
    pub cols:              Vec<(Option<String>, Option<String>)>, // (name, decltype)
    pub rows:              Vec<Vec<SqlValue>>,
    pub rows_affected:     u64,
    pub last_insert_rowid: Option<i64>,
}

// ── SqldAdapter トレイト ──────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait SqldAdapter: Send + Sync {
    /// 新規 Connection を生成して返す。
    /// Phase 9 の WebSocket セッション（WsStream）が永続コネクションとして保持するために使用する。
    fn connect(&self) -> Result<libsql::Connection, AppError>;

    /// 単一ステートメントを実行する。
    /// **注記**: RealSqldAdapter の実装では呼び出しのたびに新規 Connection を生成する。
    /// そのため、execute() を複数回呼び出しても同一トランザクション内に収まる保証はない。
    /// インタラクティブトランザクション（BEGIN/COMMIT を跨ぐ操作）は WsStream の conn フィールド経由で行う。
    async fn execute(
        &self,
        sql:       &str,
        args:      Vec<SqlValue>,
        want_rows: bool,
    ) -> Result<SqlResult, AppError>;

    /// 複数ステートメントを一括実行する（Sequence リクエスト用）
    async fn execute_batch(&self, sql: &str) -> Result<(), AppError>;
}

// ── RealSqldAdapter（libsql embedded） ────────────────────────────────────────

pub struct RealSqldAdapter {
    db: Arc<libsql::Database>,
}

impl RealSqldAdapter {
    pub async fn open(path: &Path, busy_timeout_ms: u64, run_integrity_check: bool) -> anyhow::Result<Self> {
        let db = libsql::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        let _ = conn.query("PRAGMA journal_mode=WAL", ()).await?;
        conn.execute("PRAGMA synchronous=NORMAL", ()).await?;
        let _ = conn.query(&format!("PRAGMA busy_timeout={busy_timeout_ms}"), ()).await?;

        if run_integrity_check {
            let mut rows = conn.query("PRAGMA integrity_check", ()).await?;
            let row = rows.next().await?
                .ok_or_else(|| anyhow::anyhow!("integrity_check returned no rows"))?;
            let val: String = row.get(0)?;
            if val != "ok" {
                anyhow::bail!("PRAGMA integrity_check failed for {:?}: {val}", path);
            }
            tracing::debug!(path = %path.display(), "integrity check passed");
        }

        tracing::debug!(path = %path.display(), "opened SQLite (WAL mode)");
        Ok(Self { db: Arc::new(db) })
    }
}

fn libsql_err(e: libsql::Error) -> AppError {
    let msg = e.to_string();
    if msg.contains("locked") || msg.contains("busy") {
        AppError::StorageBusy
    } else {
        AppError::Sqld(msg)
    }
}

#[async_trait::async_trait]
impl SqldAdapter for RealSqldAdapter {
    fn connect(&self) -> Result<libsql::Connection, AppError> {
        self.db.connect().map_err(|e| AppError::Sqld(e.to_string()))
    }

    async fn execute_batch(&self, sql: &str) -> Result<(), AppError> {
        let conn = self.db.connect().map_err(|e| AppError::Sqld(e.to_string()))?;
        conn.execute_batch(sql).await.map(|_| ()).map_err(libsql_err)
    }

    async fn execute(&self, sql: &str, args: Vec<SqlValue>, want_rows: bool) -> Result<SqlResult, AppError> {
        let conn   = self.db.connect().map_err(|e| AppError::Sqld(e.to_string()))?;
        let params = to_libsql_params(args);

        if want_rows {
            let mut rows = conn.query(sql, params).await.map_err(libsql_err)?;
            let col_count = rows.column_count();
            let cols: Vec<(Option<String>, Option<String>)> = (0..col_count)
                .map(|i| (rows.column_name(i).map(|s| s.to_string()), None))
                .collect();
            let mut result_rows: Vec<Vec<SqlValue>> = vec![];
            while let Some(row) = rows.next().await.map_err(libsql_err)? {
                let cells = (0..col_count)
                    .map(|i| from_libsql_value(row.get_value(i).unwrap_or(libsql::Value::Null)))
                    .collect();
                result_rows.push(cells);
            }
            Ok(SqlResult { cols, rows: result_rows, rows_affected: 0, last_insert_rowid: None })
        } else {
            let rows_affected      = conn.execute(sql, params).await.map_err(libsql_err)?;
            let last_insert_rowid  = conn.last_insert_rowid();
            Ok(SqlResult { cols: vec![], rows: vec![], rows_affected, last_insert_rowid: Some(last_insert_rowid) })
        }
    }
}

pub(crate) fn to_libsql_params(args: Vec<SqlValue>) -> Vec<libsql::Value> {
    args.into_iter().map(|v| match v {
        SqlValue::Null       => libsql::Value::Null,
        SqlValue::Integer(n) => libsql::Value::Integer(n),
        SqlValue::Real(f)    => libsql::Value::Real(f),
        SqlValue::Text(s)    => libsql::Value::Text(s),
        SqlValue::Blob(b)    => libsql::Value::Blob(b),
    }).collect()
}

pub(crate) fn from_libsql_value(v: libsql::Value) -> SqlValue {
    match v {
        libsql::Value::Null       => SqlValue::Null,
        libsql::Value::Integer(n) => SqlValue::Integer(n),
        libsql::Value::Real(f)    => SqlValue::Real(f),
        libsql::Value::Text(s)    => SqlValue::Text(s),
        libsql::Value::Blob(b)    => SqlValue::Blob(b),
    }
}

// ── MockSqldAdapter（テスト用） ───────────────────────────────────────────────

pub struct MockSqldAdapter;

#[async_trait::async_trait]
impl SqldAdapter for MockSqldAdapter {
    fn connect(&self) -> Result<libsql::Connection, AppError> {
        unimplemented!("MockSqldAdapter::connect")
    }
    async fn execute_batch(&self, _sql: &str) -> Result<(), AppError> { Ok(()) }
    async fn execute(&self, _sql: &str, _args: Vec<SqlValue>, _want_rows: bool) -> Result<SqlResult, AppError> {
        Ok(SqlResult::default())
    }
}
```

> **設計メモ**: `db/manager.rs` の `DbManager` は `Arc<dyn SqldAdapter>` を保持する。
> テストでは `MockSqldAdapter` を差し込んで libsql バイナリなしで単体テストが可能になる。

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


### Phase 4：JWT 認証・token コマンド

**目標**：JWT HS256 認証と `adlaire-db token create` が動作する

**スコープ：**
- Authorization: Bearer ヘッダ抽出・6 ステップ検証フロー
- tokens.json 読み込み・revoke リスト照合
- `token create` サブコマンド（JWT 生成・stdout 出力）

**完了条件（テストケース）：**

```
TC-3: JWT 認証ありモード（Phase 4 CLI 手動確認）
  $ SECRET="test-secret-at-least-32bytes-long"
  $ TOKEN=$(./adlaire-db token create --secret "$SECRET")
  $ ./adlaire-db serve --data ./testdb --port 8080 --auth-jwt-secret "$SECRET"
  （a）有効なトークンで SQL 実行 → 200 OK
  （b）Authorization ヘッダなし → 401 AUTH_REQUIRED
  （c）不正なトークン → 401 AUTH_INVALID
```

> **注意**：TC-3 の自動化（tokens.json CRUD + revoke フローを含む完全検証）は Phase 7 の TC-2-4〜TC-2-5 で行う。Phase 4 では上記の CLI ベース手動確認が完了条件。自動テストは `tests/integration/auth.rs` として Phase 7 で追加する。

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

#### 14.3 JWT 認証関数

```rust
// auth/middleware.rs

use hyper::{Request, body::Incoming};

/// Authorization ヘッダから JWT を検証し Claims を返す
/// 認証無効モード（jwt_secret 未設定）は Claims::unauthenticated() を返す
pub async fn extract_claims<B>(
    req: &Request<B>,
    state: &SharedState,
) -> Result<Claims, AppError> {
    if !state.auth.is_auth_enabled() {
        tracing::debug!("auth disabled — passing unauthenticated claims");
        return Ok(Claims::unauthenticated());
    }

    let raw = req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(AppError::AuthRequired)?;

    state.auth.verify(raw).await
}
```

#### 14.3b 管理 API 認証

管理 API の認証は `http/mod.rs` の `admin_route()` 内で行う（§14.4 参照）。固定トークンの Bearer 文字列完全一致。


#### 14.16 AuthState 実装（Phase 3 スタブ）

Phase 4 で JWT 検証・revoke リスト・token 発行を実装する。

```rust
// auth/mod.rs

impl AuthState {
    pub fn new(config: &Config) -> Self {
        Self { secret_bytes: config.jwt_secret_bytes.clone() }
    }

    pub fn is_auth_enabled(&self) -> bool { self.secret_bytes.is_some() }

    /// Phase 4 で実装。Phase 3 では常に AuthInvalid を返す。
    pub async fn verify(&self, _raw_token: &str) -> Result<Claims, AppError> {
        Err(AppError::AuthInvalid)
    }
}

// NOTE: `revoked` フィールドおよび `load_with(secret, revoked_subs)` の完全版は Phase 4 で追加される。
#[cfg(test)]
impl AuthState {
    /// テスト用: 秘密鍵を直接指定して AuthState を構築する（Phase 3 版）
    pub fn load_with(secret: &[u8]) -> Self {
        Self { secret_bytes: Some(secret.to_vec()) }
    }

    /// テスト用: 有効期限なしのテストトークンを発行する
    pub fn issue_test_token(&self, access: AccessLevel) -> String {
        // Phase 4 で実装
        let _ = access;
        unimplemented!("issue_test_token は Phase 4 で実装")
    }

    /// テスト用: 任意の exp を持つテストトークンを発行する
    pub fn issue_test_token_exp(&self, access: AccessLevel, exp: Option<chrono::DateTime<chrono::Utc>>) -> String {
        let _ = (access, exp);
        unimplemented!("issue_test_token_exp は Phase 4 で実装")
    }

    /// テスト用: 指定 sub を同期的に失効リストへ追加する
    pub fn revoke_sync(&self, sub: &str) {
        // Phase 4 で実装（tokio::sync::RwLock は非同期。テスト用に try_write を使用）
        let _ = sub;
        unimplemented!("revoke_sync は Phase 4 で実装")
    }
}

/// Phase 4 スタブ。tokens.json の読み込みは Phase 4 で実装する。
pub fn load_tokens(_config: &Config) -> anyhow::Result<Vec<TokenRecord>> {
    Ok(vec![])
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
    let n = u64::from_le_bytes(buf);
    format!("tok_{:016x}", n)
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
        AuthState::load_with(&secret())
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

    /// Bearer トークン付きで /v2/pipeline を呼ぶ
    pub async fn pipeline_with_token(&self, token: &str, body: serde_json::Value) -> reqwest::Response {
        self.client.post(format!("{}/v2/pipeline", self.base_url))
            .bearer_auth(token)
            .json(&body).send().await.unwrap()
    }
}

fn minimal_select() -> serde_json::Value {
    serde_json::json!({"baton":null,"requests":[
        {"type":"execute","stmt":{"sql":"SELECT 1","args":[],"want_rows":true}},
        {"type":"close"}
    ]})
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
        jwt_secret:  Some(secret.to_string()),
        admin_token: Some("admin-tok".to_string()),
    }).await;

    // 管理 API でトークン発行（Phase 7 以降で有効）
    let valid_token = srv.create_token("rw").await;

    // (a) 有効トークン → 200
    assert_eq!(srv.pipeline_with_token(&valid_token, minimal_select()).await.status(), 200);

    // (b) Authorization ヘッダなし → 401 AUTH_REQUIRED
    // pipeline() は Bearer ヘッダを付けないので認証エラーになる
    let no_auth = srv.pipeline(minimal_select()).await;
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
  [ ] route() を /{db-name}/v2/pipeline にマッチするように拡張
  [ ] パスセグメントから db-name を抽出し、DB 名バリデーションを適用
  [ ] 存在しない db-name → 404 DB_NOT_FOUND
  [ ] Phase 1〜5 の単一 DB ルート（/v2/pipeline）との共存（後方互換）
  参照: §6.1, §3.4

T2-2: マルチ DB マネージャ
  [ ] 起動時に databases.json を読み込み、全 DB を libsql::Builder::new_local() でオープン
  [ ] DB 名 → Arc<dyn SqldAdapter> のマップをメモリ上で管理（RwLock<HashMap>）
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
    if RESERVED_NAMES.contains(&name) || name.contains(BRANCH_SEP) {
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

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabasesMeta { pub databases: Vec<DbInfo> }

impl DatabasesMeta {
    /// meta/databases.json をロードする（なければ空を返す、保存はしない）
    pub fn load(data_dir: &Path) -> anyhow::Result<Self> {
        let path = data_dir.join("meta").join("databases.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(&path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// meta/databases.json にアトミック保存（json.tmp → rename）
    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        let path = data_dir.join("meta").join("databases.json");
        let tmp  = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self)?;
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
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
  [ ] POST /admin/v1/databases — DB 名バリデーション・ディレクトリ作成・libsql オープン
  [ ] GET /admin/v1/databases/{name} → 個別情報（name・created_at・size_bytes）
  [ ] DELETE /admin/v1/databases/{name} — Arc<libsql::Database> drop・ディレクトリ削除
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
// http/admin/mod.rs
// Phase 6〜7 で各ハンドラを実装する。現時点はすべて 501 を返す stub。

use hyper::{Request, Response, body::Incoming};
use http_body_util::Full;
use bytes::Bytes;
use std::convert::Infallible;
use crate::state::SharedState;

fn not_implemented() -> Response<Full<Bytes>> {
    Response::builder()
        .status(http::StatusCode::NOT_IMPLEMENTED)
        .body(Full::default())
        .unwrap()
}

pub mod databases {
    use super::*;
    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn delete(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}

pub mod tokens {
    use super::*;
    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn revoke(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}

pub mod metrics {
    use super::*;
    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}

pub mod backup {
    use super::*;
    pub async fn backup(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn restore(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn pitr(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}

pub mod branches {
    use super::*;
    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn delete(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}
```

---


### Phase 8：Turso Cloud 互換管理モデル

**目標**：Phase 7 の管理 API を Turso Cloud 互換の管理モデルへ拡張し、location、organization/group、quota/usage を DB・token・admin 権限の正式な管理境界として扱う。

Phase 8 は Turso Cloud 互換を優先するための前倒しフェーズである。WebSocket、ATTACH、replication、backup、branch より先に、管理モデルと metadata の互換性を固める。

**Phase 8 の対象：**

- データベースロケーション（location / region）
- 組織・グループ管理（organization / group）
- ストレージクォータと使用量（quota / usage）
- 既存 `/admin/v1/databases`、`/admin/v1/tokens` の互換拡張
- 既存 `databases.json`、`tokens.json` からの metadata migration

**Phase 8 API 契約：**

Phase 8 で公開する API は §9.5 の Phase 8 行を正とする。`/admin/v1/*` は Admin token 必須、`/v1/*` は Platform token 必須とし、unknown field は `INVALID_REQUEST` とする。一覧 API は §9.1.27 に従い、`limit`（既定 100、最大 500）と `cursor` を受け付ける。Phase 8 では `cursor` は base64url without padding の opaque string とし、endpoint、resource kind、scope、filter、sort に binding する。filter query は `organization`、`group`、`database` のみ許可し、不明 query は `INVALID_REQUEST` とする。

`PUT /admin/v1/quotas/{scope}` の `{scope}` は URL encode 済みの `organization:{id}`、`group:{id}`、`database:{name}` のいずれかとする。scope type が不明な場合は `INVALID_REQUEST`、scope が存在しない場合は対応する `ORG_NOT_FOUND`、`GROUP_NOT_FOUND`、`DB_NOT_FOUND` を返す。

Phase 8 の `/v1/*` Turso 互換 API は、公式 Turso Platform API の主要 path、status、response wrapper、field casing を優先する。`/admin/v1/*` との差分は互換差分として扱い、片方の実装都合で他方の契約を変えてはならない。

**Phase 8 `/v1/*` route 解決順固定表：**

`/v1/*` router は下表の順で完全一致または path parameter 一致を評価する。同じ path で method だけが異なる場合は `405 METHOD_NOT_ALLOWED`、下表と unsupported 固定表のどちらにも該当しない path は `404 ENDPOINT_NOT_FOUND` とする。trailing slash は別 path と扱い、自動 redirect・自動補正を行わない。

| 順位 | Route pattern | 対象 method | 一致時の扱い |
|------|---------------|-------------|--------------|
| 1 | `/v1/auth/validate` | `GET` | Platform token 検証 |
| 2 | `/v1/auth/api-tokens/{tokenName}` | `POST`, `DELETE` | Platform API token 作成・失効 |
| 3 | `/v1/locations` | `GET` | location 一覧 |
| 4 | `/v1/organizations` | `GET` | organization 一覧 |
| 5 | `/v1/organizations/{organizationSlug}` | `PATCH` | organization 更新 |
| 6 | `/v1/organizations/{organizationSlug}/usage` | `GET` | organization usage |
| 7 | `/v1/organizations/{organizationSlug}/audit-logs` | `GET` | unsupported audit logs stub |
| 8 | `/v1/organizations/{organizationSlug}/groups/{groupName}/configuration` | `PATCH` | group configuration 更新。group 詳細より先に評価する |
| 9 | `/v1/organizations/{organizationSlug}/groups/{groupName}/auth/rotate` | `POST` | group 配下 DB token 一括失効。group 詳細より先に評価する |
| 10 | `/v1/organizations/{organizationSlug}/groups/{groupName}/transfer` | `POST` | unsupported group transfer stub。group 詳細より先に評価する |
| 11 | `/v1/organizations/{organizationSlug}/groups/{groupName}` | `GET` | group 詳細。`groups` 一覧より先に評価する |
| 12 | `/v1/organizations/{organizationSlug}/groups` | `GET`, `POST` | group 一覧・作成 |
| 13 | `/v1/organizations/{organizationSlug}/databases/{databaseName}/auth/tokens` | `POST` | DB token 作成。DB 詳細より先に評価する |
| 14 | `/v1/organizations/{organizationSlug}/databases/{databaseName}/auth/rotate` | `POST` | DB token 一括失効。DB 詳細より先に評価する |
| 15 | `/v1/organizations/{organizationSlug}/databases/{databaseName}/configuration` | `PATCH` | DB configuration 更新。DB 詳細より先に評価する |
| 16 | `/v1/organizations/{organizationSlug}/databases/{databaseName}/stats` | `GET` | unsupported stats stub |
| 17 | `/v1/organizations/{organizationSlug}/databases/{databaseName}` | `GET`, `DELETE` | DB 詳細・削除 |
| 18 | `/v1/organizations/{organizationSlug}/databases` | `GET`, `POST` | DB 一覧・作成 |
| 19 | `/v1/organizations/{organizationSlug}/members...`、`/invites...`、`/plans...`、`/billing...`、`/overages...`、`/v1/upload` | 固定表参照 | unsupported stub |

Path parameter は percent decode 後に validation する。decode 不能、decode 後の空文字、`/` を含む値、`.`、`..` は `400 INVALID_REQUEST` とする。`organizationSlug`、`groupName`、`databaseName` は URL 内では case-sensitive とし、大小文字補正を行わない。

**Phase 8 `/v1/*` request validation 固定表：**

| 対象 | 規則 | 失敗時 |
|------|------|--------|
| `GET` | request body 禁止。`Content-Length: 0` または body なしのみ許可 | `400 INVALID_REQUEST` |
| `POST` / `PATCH` | body を持つ場合は `Content-Type: application/json` 必須。`; charset=utf-8` は許可。body は JSON object 必須。endpoint 表で body なし可と明記された POST は body なしを許可 | `400 INVALID_REQUEST` |
| unknown query | endpoint 固有表にない query key は拒否。互換のため黙って無視しない | `400 INVALID_REQUEST` |
| duplicate query key | 同一 query key の複数指定は禁止 | `400 INVALID_REQUEST` |
| empty query value | `cursor` 以外の空文字は禁止。`cursor=` も無効 cursor として拒否 | `400 INVALID_REQUEST` |
| unknown body field | endpoint 固有表にない field は拒否 | `400 INVALID_REQUEST` |
| JSON scalar/array body | body が object でない場合は禁止 | `400 INVALID_REQUEST` |
| trailing slash | `/v1/locations/` など末尾 slash は未定義 path | `404 ENDPOINT_NOT_FOUND` |
| unsupported method | 既知 path に未定義 method を送った場合 | `405 METHOD_NOT_ALLOWED` |

**Phase 8 API schema 固定表：**

| API | Request | Success response | Validation |
|-----|---------|------------------|------------|
| `GET /admin/v1/organizations` | query `limit?`, `cursor?` | `{"organizations":[OrganizationInfo],"next_cursor": string|null}` | 不明 query は `INVALID_REQUEST` |
| `POST /admin/v1/organizations` | `{name, slug?}` | 201 `OrganizationInfo` | `name`/`slug` は `^[a-zA-Z0-9_-]{1,63}$`。`slug` 省略時は `name` と同じ。重複は `ORG_ALREADY_EXISTS` |
| `GET /admin/v1/organizations/{org}` | body なし | `OrganizationInfo` | `{org}` は `id` または `slug` |
| `DELETE /admin/v1/organizations/{org}` | body なし | 204 | `default` は削除禁止で `403 ORG_SCOPE_DENIED`。配下 group/DB/token がある場合も `403 ORG_SCOPE_DENIED` |
| `GET /admin/v1/groups` | query `organization?`, `limit?`, `cursor?` | `{"groups":[GroupInfo],"next_cursor": string|null}` | `organization` が存在しなければ `ORG_NOT_FOUND` |
| `POST /admin/v1/groups` | `{organization, name, slug?, location?}` | 201 `GroupInfo` | `location` 省略時は `default`。同一 organization 内の name/slug 重複は `GROUP_ALREADY_EXISTS` |
| `GET /admin/v1/groups/{group}` | body なし | `GroupInfo` | group 名が複数 organization で重複する場合は `organization` query 必須。未指定なら `INVALID_REQUEST` |
| `DELETE /admin/v1/groups/{group}` | body なし | 204 | `default` は削除禁止。配下 DB/token/quota がある場合は `403 ORG_SCOPE_DENIED` |
| `GET /admin/v1/locations` | query `limit?`, `cursor?` | `{"locations":[LocationInfo],"next_cursor": string|null}` | 不明 query は `INVALID_REQUEST` |
| `POST /admin/v1/locations` | `{name, provider?, region?, primary?}` | 201 `LocationInfo` | `provider` 省略時 `"self-hosted"`、`region` 省略時 `"local"`、`primary` 省略時 `false` |
| `GET /admin/v1/locations/{location}` | body なし | `LocationInfo` | `{location}` は `id` または `name` |
| `DELETE /admin/v1/locations/{location}` | body なし | 204 | DB/group が参照中なら `403 ORG_SCOPE_DENIED`。`default` は削除禁止 |
| `GET /admin/v1/quotas` | query `organization?`, `group?`, `database?`, `limit?`, `cursor?` | `{"quotas":[QuotaInfo],"next_cursor": string|null}` | scope query は同時に 1 種類のみ。複数指定は `INVALID_REQUEST` |
| `PUT /admin/v1/quotas/{scope}` | `{storage_bytes, rows?, write_ops_per_minute?}` | 200 `QuotaInfo` | `storage_bytes` は 0 以上の integer。`null` は無制限。負数は `INVALID_REQUEST` |
| `GET /admin/v1/usage` | query `organization?`, `group?`, `database?`, `limit?`, `cursor?` | `{"usage":[UsageInfo],"next_cursor": string|null}` | usage 再計測不能時は `USAGE_UNAVAILABLE` |

**Phase 8 DTO schema：**

```json
{
  "OrganizationInfo": {
    "id": "org_default",
    "name": "default",
    "slug": "default",
    "created_at": "2026-09-13T00:00:00Z"
  },
  "GroupInfo": {
    "id": "grp_default",
    "organization": "default",
    "name": "default",
    "slug": "default",
    "location": "default",
    "created_at": "2026-09-13T00:00:00Z"
  },
  "LocationInfo": {
    "id": "loc_default",
    "name": "default",
    "provider": "self-hosted",
    "region": "local",
    "primary": true,
    "created_at": "2026-09-13T00:00:00Z"
  },
  "QuotaInfo": {
    "scope_type": "organization",
    "scope": "default",
    "storage_bytes": 10737418240,
    "rows": null,
    "write_ops_per_minute": null,
    "updated_at": "2026-09-13T00:00:00Z"
  },
  "UsageInfo": {
    "scope_type": "database",
    "scope": "default",
    "storage_bytes": 0,
    "rows": null,
    "updated_at": "2026-09-13T00:00:00Z"
  }
}
```

上記 DTO の field はすべて必須である。`rows` と `write_ops_per_minute` だけ `null` 可とする。`id` は実装内で `org_`、`grp_`、`loc_` prefix を付けた一意文字列にする。既存 Turso Cloud の slug 互換を優先するため、API path では `id` と `slug/name` の両方を解決できるようにする。

**Phase 8 Turso 互換 DTO schema：**

```json
{
  "TursoDatabaseInfo": {
    "DbId": "550e8400-e29b-41d4-a716-446655440000",
    "Hostname": "my-db-default.adlaire.local",
    "Name": "my-db",
    "block_reads": false,
    "block_writes": false,
    "regions": ["default"],
    "primaryRegion": "default",
    "group": "default",
    "delete_protection": false,
    "parent": null
  },
  "TursoGroupInfo": {
    "name": "default",
    "version": "adlaire-0.64",
    "uuid": "grp_default",
    "locations": ["default"],
    "primary": "default",
    "delete_protection": false
  },
  "TursoOrganizationInfo": {
    "name": "default",
    "slug": "default",
    "type": "personal",
    "overages": false,
    "require_mfa": false,
    "blocked_reads": false,
    "blocked_writes": false,
    "plan_id": "self-hosted",
    "plan_timeline": "none",
    "platform": "adlaire"
  },
  "TursoOrganizationUsage": {
    "uuid": "org_default",
    "usage": {
      "rows_read": 0,
      "rows_written": 0,
      "databases": 1,
      "locations": 1,
      "storage_bytes": 4096,
      "groups": 1,
      "bytes_synced": 0
    },
    "databases": [
      {
        "uuid": "550e8400-e29b-41d4-a716-446655440000",
        "instances": [
          {
            "uuid": "550e8400-e29b-41d4-a716-446655440000",
            "usage": {
              "rows_read": 0,
              "rows_written": 0,
              "storage_bytes": 4096,
              "bytes_synced": 0
            }
          }
        ],
        "total": {
          "rows_read": 0,
          "rows_written": 0,
          "storage_bytes": 4096,
          "bytes_synced": 0
        }
      }
    ]
  }
}
```

**Phase 8 Turso Platform API token DTO schema：**

```json
{
  "TursoApiTokenInfo": {
    "name": "ci-token",
    "id": "tok_550e8400e29b41d4a716446655440000",
    "token": "adlpt_..."
  },
  "TursoApiTokenValidateInfo": {
    "exp": -1
  }
}
```

`token` は `POST /v1/auth/api-tokens/{tokenName}` の成功応答で 1 回だけ返す。`GET /v1/auth/validate` と `DELETE /v1/auth/api-tokens/{tokenName}` は token secret を返さない。Phase 8 の Platform API token は既存 `tokens.json` に `source:"turso-platform-api-token"`、`name:"{tokenName}"`、`organization_scope:null|string`、`platform_token:true` を付与して保存する。

| Turso API | Status | Response wrapper | Field casing rule |
|-----------|--------|------------------|-------------------|
| `GET /v1/auth/validate` | 200 | `{"exp": integer}` | 無期限 token は `-1` |
| `POST /v1/auth/api-tokens/{tokenName}` | 200 | `{"name": string, "id": string, "token": string}` | `token` は作成時のみ返す |
| `DELETE /v1/auth/api-tokens/{tokenName}` | 200 | `{"token": string}` | 値は token 名。secret ではない |
| `GET /v1/locations` | 200 | `{"locations":{...}}` | location code は object key、display name は string value |
| `GET /v1/organizations` | 200 | `[TursoOrganizationInfo]` | wrapper object は返さない |
| `PATCH /v1/organizations/{org}` | 200 | `{"organization":{...}}` | `overages`、`require_mfa` 以外の body field は `INVALID_REQUEST` |
| `GET /v1/organizations/{org}/usage` | 200 | `{"organization":{...}}` | usage field は snake_case |
| `GET /v1/organizations/{org}/groups` | 200 | `{"groups":[...]}` | group fields は Turso casing |
| `GET /v1/organizations/{org}/groups/{group}` | 200 | `{"group":{...}}` | retrieve は list 要素と同じ schema |
| `POST /v1/organizations/{org}/groups` | 200 | `{"group":{...}}` | 201 を返さない |
| `PATCH /v1/organizations/{org}/groups/{group}/configuration` | 200 | `{"delete_protection": boolean}` | wrapper 名を付けず configuration object を返す |
| `POST /v1/organizations/{org}/groups/{group}/auth/rotate` | 200 | body なし | group 配下 DB token を全失効。Platform API token は失効しない |
| `POST /v1/organizations/{org}/groups/{group}/transfer` | 501 | `{"error":"not implemented","code":"NOT_IMPLEMENTED"}` | transfer は Phase 8 では実装しない |
| `GET /v1/organizations/{org}/databases` | 200 | `{"databases":[...]}` | `DbId`、`Hostname`、`Name` は大文字始まりを維持 |
| `GET /v1/organizations/{org}/databases/{db}` | 200 | `{"database":{...}}` | retrieve は list 要素と同じ schema |
| `POST /v1/organizations/{org}/databases` | 200 | `{"database":{...}}` | 201 を返さない |
| `DELETE /v1/organizations/{org}/databases/{db}` | 200 | `{"database": string}` | 削除した DB 名を返す。204 ではない |
| `PATCH /v1/organizations/{org}/databases/{db}/configuration` | 200 | `{"size_limit": string|null, "allow_attach": boolean, "block_reads": boolean, "block_writes": boolean, "delete_protection": boolean}` | wrapper 名を付けず configuration object を返す |
| `POST /v1/organizations/{org}/databases/{db}/auth/tokens` | 200 | `{"jwt":"..."}` | `token` ではなく `jwt` |
| `POST /v1/organizations/{org}/databases/{db}/auth/rotate` | 200 | body なし | 既存 DB token を全失効。新 token は返さない |

`/v1/*` では request body の `name` validation を Turso 互換 DB 名規則 `^[a-z0-9-]{1,64}$` に固定する。`size_limit` は bytes 数値文字列または `kb`/`mb`/`gb` suffix を受け付け、quota の `storage_bytes` に変換する。`seed`、`remote_encryption`、database upload、CSV import、dump import は Phase 8 対象外であり、指定された場合は `400 INVALID_REQUEST` を返す。特に `seed.type:"database"`、`seed.name`、`seed.database`、`parent` body field による branch/clone 作成は Phase 15 まで受け付けず、Phase 8 では `400 INVALID_REQUEST` に固定する。

**Turso database token 互換：**

`POST /v1/organizations/{organizationSlug}/databases/{databaseName}/auth/tokens` は Turso 互換の query を受け付ける。

| Query/body | Allowed | Adlaire mapping |
|------------|---------|-----------------|
| `expiration` | `never` または `<number><s|m|h|d|w>` の連結。例: `2w1d30m` | JWT `exp`。`never` は `exp` なし |
| `authorization` | `full-access` / `read-only` | `full-access` → `a:"rw"`、`read-only` → `a:"ro"` |
| body `permissions` | object 可。ただし Phase 8 では table/action permission は実装しない | 空 object または省略のみ許可。非空は `INVALID_REQUEST` |

response は必ず `{"jwt":"<token>"}` とし、`id`、`access`、`expires_at` は返さない。発行した token metadata は既存 `tokens.json` に保存し、`source:"turso-platform-api"`、`database:"{databaseName}"`、`organization_scope:"{organizationSlug}"` を付与する。

**Turso Platform API token 互換：**

| API | Request | Success | Validation / persistence |
|-----|---------|---------|--------------------------|
| `GET /v1/auth/validate` | body なし | 200 `{"exp": -1}` または `{"exp": unix_seconds}` | 現在の Platform token が `tokens.json` 管理 token ならその expiry、Phase 8 の Admin token 代用なら `-1` |
| `POST /v1/auth/api-tokens/{tokenName}` | body `{organization?}` または body なし | 200 `{"name":"{tokenName}","id":"tok_...","token":"adlpt_..."}` | `tokenName` は `^[a-zA-Z0-9_-]{1,64}$`。`organization` 指定時は slug/id 解決必須。重複 tokenName は `409 DB_ALREADY_EXISTS` ではなく `400 INVALID_REQUEST` |
| `DELETE /v1/auth/api-tokens/{tokenName}` | body なし | 200 `{"token":"{tokenName}"}` | tokenName が存在しない場合は `404 TOKEN_NOT_FOUND`。失効済みなら 200 を返す |

Platform API token の secret は `adlpt_` prefix の不透明文字列とし、JWT ではない。検証は Bearer 完全一致で行う。作成した token は `tokens.json` に保存し、`revoked:false`、`platform_token:true`、`organization_scope` を持つ。`POST /v1/auth/api-tokens/{tokenName}` の応答以外では secret を返さず、log にも出さない。Phase 8 では Platform API token の権限は Admin token と同等だが、`organization_scope` がある場合は対象 organization 外の `/v1/*` 操作を `403 ORG_SCOPE_DENIED` とする。

**Turso database delete / auth rotate 互換：**

| API | Request | Success | Validation / persistence |
|-----|---------|---------|--------------------------|
| `DELETE /v1/organizations/{organizationSlug}/databases/{databaseName}` | body なし | 200 `{"database":"{databaseName}"}` | DB がない場合は `404 DB_NOT_FOUND`。DB directory 削除と `databases.json` 更新を Phase 7 delete と同じ atomic 手順で行う |
| `POST /v1/organizations/{organizationSlug}/databases/{databaseName}/auth/rotate` | body なし | 200 body なし | 対象 DB に紐づく `source:"turso-platform-api"` token をすべて `revoked:true` にする。Platform API token 自体は失効しない |

`auth/rotate` は新しい DB token を発行しない。失効対象が 0 件でも DB が存在すれば 200 とする。DB 削除時は当該 DB の DB token をすべて失効し、Platform API token は削除しない。

**Turso configuration / group token 互換：**

| API | Request | Success | Validation / persistence |
|-----|---------|---------|--------------------------|
| `PATCH /v1/organizations/{organizationSlug}/databases/{databaseName}/configuration` | `{size_limit?, delete_protection?, block_reads?, block_writes?, allow_attach?}` | 200 `{"size_limit": string|null, "allow_attach": boolean, "block_reads": boolean, "block_writes": boolean, "delete_protection": boolean}` | DB がない場合は `404 DB_NOT_FOUND`。unknown field は `INVALID_REQUEST`。少なくとも 1 field 必須。`size_limit` は quota、他 field は `databases.json` に atomic 永続化 |
| `PATCH /v1/organizations/{organizationSlug}/groups/{groupName}/configuration` | `{delete_protection}` | 200 `{"delete_protection": boolean}` | group がない場合は `404 GROUP_NOT_FOUND`。unknown field は `INVALID_REQUEST`。`delete_protection` は boolean 必須で `groups.json` に atomic 永続化 |
| `POST /v1/organizations/{organizationSlug}/groups/{groupName}/auth/rotate` | body なし | 200 body なし | group がない場合は `404 GROUP_NOT_FOUND`。group 配下 DB に紐づく `source:"turso-platform-api"` token と `group_scope` token をすべて `revoked:true` にする。Platform API token 自体は失効しない |

configuration の field 意味は以下に固定する。

| Field | 型 | 既定値 | 実装効果 |
|-------|----|--------|----------|
| `size_limit` | string / null | null | database scope quota の `storage_bytes`。`null` は無制限。現在使用量より小さい値も受け付け、既存データは読めるが以後の write/import/restore/replication apply/branch create は quota 判定で拒否する |
| `delete_protection` | boolean | false | `true` の DB は `/v1/*` と `/admin/v1/*` の削除を `403 ORG_SCOPE_DENIED` で拒否する。`true` の group は group 削除と配下 DB 削除を同じく拒否する |
| `block_reads` | boolean | false | `true` の DB への hrana read SQL、WebSocket read、backup download、export を `403 PERMISSION_DENIED` で拒否する。管理 API の一覧・詳細は拒否しない |
| `block_writes` | boolean | false | `true` の DB への write SQL、restore、replication apply、branch create、import を `403 PERMISSION_DENIED` で拒否する。quota 超過時は `QUOTA_EXCEEDED` を優先する |
| `allow_attach` | boolean | true | Phase 10 ATTACH の DB 単位許可。`false` の DB を source/target に含む ATTACH は `403 PERMISSION_DENIED` |

`block_reads` と `block_writes` は Turso DTO の同名 field にそのまま反映する。ただし `block_writes` は quota 超過の派生状態ではなく、configuration の明示値を返す。quota 超過を示す場合は error code `QUOTA_EXCEEDED` と usage/quota API で表現する。

**Turso 互換差分固定表：**

| Turso Cloud 機能 | Phase 8 Adlaire の扱い | 理由 |
|------------------|-------------------------|------|
| database upload / seed | `INVALID_REQUEST`。`seed.type:"database"` を含む branch/clone seed も Phase 8 では拒否 | Phase 14 restore/PITR と Phase 15 branch に分離するため Phase 8 対象外 |
| encrypted database | `INVALID_REQUEST` | encryption key/cipher 管理は本仕様の security boundary 外 |
| branch parent filter | query は受け付けるが Phase 8 では空配列。Phase 15 以降に branch metadata と接続する | branch は Phase 15 |
| group delete protection | `PATCH /groups/{group}/configuration` の `delete_protection` を保存して DTO に反映する | Turso Cloud 互換 API と削除保護を自己ホストでも対象に含めるため |
| multiple replica regions per group | Phase 8 は 1 primary location のみ | replication/HA は Phase 11〜18 |

**Phase 8 Turso unsupported API 固定表：**

| API family | Method | Phase 8 response | 理由 |
|------------|--------|------------------|------|
| `/v1/organizations/{org}/members...` | `GET`, `POST`, `PATCH`, `DELETE` | 501 `{"error":"not implemented","code":"NOT_IMPLEMENTED"}` | 自己ホスト単一運営者では user directory を持たない |
| `/v1/organizations/{org}/invites...` | `GET`, `POST`, `PATCH`, `DELETE` | 501 `NOT_IMPLEMENTED` | 招待 workflow と user directory を持たない |
| `/v1/organizations/{org}/plans` | `GET` | 501 `NOT_IMPLEMENTED`。ただし organization DTO の plan fields は固定値を返す | 課金連動は対象外。quota は Adlaire metadata で管理 |
| `/v1/organizations/{org}/plans` | `POST`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | Phase 8 では更新 API として公開しない |
| `/v1/organizations/{org}/billing...`、`/overages...` | `GET`, `POST`, `PATCH`, `DELETE` | 501 `NOT_IMPLEMENTED` | 課金連動は対象外 |
| `/v1/organizations/{org}/audit-logs` | `GET` | 501 `NOT_IMPLEMENTED` | audit log 永続化は Phase 8 の metadata 境界外。将来 Phase で専用 schema を定義する |
| `/v1/organizations/{org}/audit-logs` | `POST`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | 読み取り系 stub のみ定義 |
| `/v1/organizations/{org}/databases/{db}/stats` | `GET` | 501 `NOT_IMPLEMENTED` | SQL text を集計・保存しない秘匿方針を優先 |
| `/v1/organizations/{org}/databases/{db}/stats` | `POST`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | 読み取り系 stub のみ定義 |
| `/v1/organizations/{org}/groups/{group}/transfer` | `POST` | 501 `NOT_IMPLEMENTED` | owner/user directory と cross-organization transfer workflow を持たない |
| `/v1/organizations/{org}/groups/{group}/transfer` | `GET`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | transfer 作成以外の method は未定義 |
| `/v1/upload` | `POST` | 501 `NOT_IMPLEMENTED` | Phase 14 restore API として別管理し、database token upload は Phase 8 対象外 |
| `/v1/upload` | `GET`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | upload 作成以外の method は未定義 |
| encrypted database create/upload | body feature flag | 400 `INVALID_REQUEST` | request body feature flag として指定されるため、未対応入力として拒否 |
| seed / CSV / dump import | body feature flag | 400 `INVALID_REQUEST` | request body feature flag として指定されるため、未対応入力として拒否 |
| database branch seed | body `seed.type:"database"`、`seed.name`、`seed.database`、`parent` | 400 `INVALID_REQUEST` | branch/clone は Phase 15 で専用 metadata と atomic 作成手順を定義する |

501 stub は success response ではない。ログは WARN `not implemented endpoint` とし、request body、token、SQL、upload binary はログに出さない。

**既存 API の Phase 8 拡張：**

- `POST /admin/v1/databases` は `{name, organization?, group?, location?, quota?}` を受け付ける。省略時はすべて `"default"` を使う
- `DbInfo` は Phase 8 以降 `{name, created_at, path, organization, group, location, quota?, usage?}` を返す
- `POST /admin/v1/tokens` は `{access, expiry?, dbs?, organization_scope?, group_scope?}` を受け付ける
- token metadata は Phase 8 以降 `organization_scope` と `group_scope` を返す。ただし JWT 文字列は従来通り作成時のみ返す
- Phase 7 クライアントが送る `{name}`、`{access, expiry?, dbs?}` は後方互換として成功しなければならない

**Phase 8 metadata schema：**

```json
{
  "databases": [
    { "name": "default", "organization": "default", "group": "default", "location": "default", "delete_protection": false, "block_reads": false, "block_writes": false, "allow_attach": true }
  ],
  "organizations": [
    { "id": "default", "name": "default", "slug": "default", "created_at": "2026-09-12T00:00:00Z" }
  ],
  "groups": [
    { "id": "default", "organization": "default", "name": "default", "slug": "default", "location": "default", "delete_protection": false, "created_at": "2026-09-12T00:00:00Z" }
  ],
  "locations": [
    { "id": "default", "name": "default", "provider": "self-hosted", "region": "local", "primary": true }
  ],
  "quotas": [
    { "scope_type": "organization|group|database", "scope": "default", "storage_bytes": 10737418240, "rows": null, "write_ops_per_minute": null }
  ],
  "usage": [
    { "scope_type": "organization|group|database", "scope": "default", "storage_bytes": 0, "rows": null, "updated_at": "2026-09-12T00:00:00Z" }
  ]
}
```

実ファイルは §9.6 の通り `databases.json`、`organizations.json`、`groups.json`、`locations.json`、`quotas.json`、`usage.json` に分ける。上記 JSON は論理 schema の説明であり、1 ファイルへ統合してはならない。Phase 8 migration 後の `databases.json` は legacy field に加えて `organization`、`group`、`location`、`delete_protection`、`block_reads`、`block_writes`、`allow_attach` を必須 field とする。

**Phase 8 metadata unique constraint 固定表：**

Metadata store は読み込み時と書き込み前の両方で下表を検証する。違反を検出した metadata は起動失敗とし、API 書き込み時の競合は対応する error code を返して既存 metadata を変更しない。

| Resource | Unique key | 競合時 error |
|----------|------------|--------------|
| organization | `id` | `ORG_ALREADY_EXISTS` |
| organization | `slug` | `ORG_ALREADY_EXISTS` |
| group | `(organization, id)` | `GROUP_ALREADY_EXISTS` |
| group | `(organization, name)` | `GROUP_ALREADY_EXISTS` |
| group | `(organization, slug)` | `GROUP_ALREADY_EXISTS` |
| location | `id` | `LOCATION_ALREADY_EXISTS` |
| location | `name` | `LOCATION_ALREADY_EXISTS` |
| database | `name` | `DB_ALREADY_EXISTS` |
| database | `(organization, name)` | `DB_ALREADY_EXISTS` |
| quota | `(scope_type, scope)` | 既存 quota を `PUT` で更新し、新規重複 record は作らない |
| usage | `(scope_type, scope)` | 計測値を上書き更新し、新規重複 record は作らない |
| token | `id` | token id を再生成する。3 回連続衝突した場合は `500 INTERNAL_ERROR` |
| platform token | `name` | `400 INVALID_REQUEST`。Phase 8 では同名 Platform API token の上書き作成を許可しない |

DB 名は Phase 8 でも実ファイル path と hrana 接続 path の互換性を守るため global unique とする。Turso Platform API の path では organization 配下に見えるが、同名 DB を別 organization に作成することは Phase 8 では禁止し、`DB_ALREADY_EXISTS` を返す。

**Phase 8 migration：**

Phase 8 初回起動時に Phase 7 までの metadata を検出した場合、次を 1 回だけ実行する。

1. `organizations.json`、`groups.json`、`locations.json`、`quotas.json`、`usage.json` がなければ初期値を作る
2. 既存 `databases.json` の全 DB に `organization:"default"`、`group:"default"`、`location:"default"`、`delete_protection:false`、`block_reads:false`、`block_writes:false`、`allow_attach:true` を付与する
3. 既存 `tokens.json` の全 token に `organization_scope:null`、`group_scope:null` を付与し、従来の `dbs` claim は維持する
4. migration 中に失敗した場合は起動失敗とし、途中で更新済みの metadata を成功扱いにしない
5. migration は tmp write、fsync、rename の順で行い、全 metadata が整合した後に起動成功とする

**Phase 8 migration / rollback 固定手順：**

| 手順 | 必須処理 |
|------|----------|
| preflight | 既存 `databases.json` と `tokens.json` を読み、JSON parse、必須 field、DB directory 存在を検証する。失敗時は書き込み前に起動失敗 |
| backup | `{data-dir}/meta/migration-backup/phase8-{timestamp}/` に既存 metadata を copy + fsync する。backup 失敗時は起動失敗 |
| write | 新規 metadata は `.tmp` に書き、file fsync、directory fsync、rename、directory fsync の順で確定する |
| commit marker | 全 metadata 更新後に `{data-dir}/meta/phase8-migration.json` を `{"from_phase":7,"completed_at":...,"backup":"..."}` で書く |
| rollback | migration 中断を検出した場合、commit marker がなければ backup から復元して起動失敗にする。commit marker がある場合は rollback せず通常起動 |
| idempotency | commit marker がある状態で再起動した場合、migration を再実行しない |

Phase 8 migration は data loss を避けるため自動削除をしない。不要になった backup の削除 API は Phase 8 対象外であり、手動削除のみ許可する。

**Phase 8 権限優先順位：**

1. Admin token は全 organization/group/location/quota にアクセスできる
2. JWT に `org` claim がある場合、その organization 外の DB/group/quota 操作は `ORG_SCOPE_DENIED`
3. JWT に `grp` claim がある場合、その group 外の DB 操作は `ORG_SCOPE_DENIED`
4. `dbs` claim は DB 単位の最終制限として維持する。`org` / `grp` で許可されても `dbs` が拒否する DB は操作不可
5. `a:"ro"` は Phase 8 以降も書き込み、restore、replication apply、branch create を禁止する

**Phase 8 quota 判定：**

quota は organization、group、database の順にすべて評価する。1 つでも超過する場合は `QUOTA_EXCEEDED` を返し、DB ファイルや metadata を変更してはならない。usage が取得できず安全に判定できない場合は `USAGE_UNAVAILABLE` を返し、成功扱いにしない。

quota 判定で拒否する操作:

- `POST /v2/pipeline` と `POST /{db-name}/v2/pipeline` の write SQL
- `sequence` に含まれる write SQL
- backup restore / PITR restore
- replication apply
- branch create
- import API は本仕様書では未定義のため実装禁止

backup download、read-only SELECT、DB/token/location/org/group/quota の一覧取得は quota 超過時でも許可する。

**Phase 8 snapshot artifact 固定表：**

Turso 互換 snapshot は `tests/snapshots/phase8_turso/` に保存する。各 snapshot は `status`、`content_type`、`body` を必須 field とし、`Date`、`Server`、request id、JWT、UUID、timestamp は比較前に placeholder へ正規化する。`content_type` は JSON response では `application/json` とし、charset の有無で比較結果を変えてはならない。

**Snapshot 比較共通仕様：**

| 項目 | 固定仕様 |
|------|----------|
| JSON key order | 比較前に object key を辞書順へ正規化する。array order は API 契約通り比較する |
| dynamic placeholder | UUID は `<uuid>`、RFC3339 timestamp は `<timestamp>`、JWT/Platform token は `<secret>`、request id は `<request_id>`、hostname の DB/org 部分以外は `<host>` |
| header 比較 | `content-type`、互換に必要な `location`、replication 系 `x-adlaire-*` だけ比較する。`date`、`server`、`content-length` は比較しない |
| status 比較 | HTTP status は必ず比較する。hrana SQL error の場合は HTTP 200 と body 内 error code を比較する |
| body 比較 | error body は `error` と `code` だけ比較する。success body は schema field の過不足を厳密比較する |
| 更新禁止条件 | 実装変更だけで snapshot を更新してはならない。Turso 追従または本仕様変更 commit が先に存在する場合だけ更新可 |
| CI failure | snapshot 差分、未生成 snapshot、placeholder 未正規化、secret 検出はすべて CI failure |
| secret scan | snapshot directory に `Bearer `、`eyJ`、`adlpt_`、admin token 生値、JWT signature 形式があれば failure |

| Snapshot file | 対象 |
|---------------|------|
| `auth.validate.json` | `GET /v1/auth/validate` |
| `auth.api_tokens.create.json` | `POST /v1/auth/api-tokens/{tokenName}` |
| `auth.api_tokens.revoke.json` | `DELETE /v1/auth/api-tokens/{tokenName}` |
| `locations.list.json` | `GET /v1/locations` |
| `organizations.list.json` | `GET /v1/organizations` |
| `organizations.update.json` | `PATCH /v1/organizations/{org}` |
| `organizations.usage.json` | `GET /v1/organizations/{org}/usage` |
| `groups.list.json` | `GET /v1/organizations/{org}/groups` |
| `groups.create.json` | `POST /v1/organizations/{org}/groups` |
| `groups.retrieve.json` | `GET /v1/organizations/{org}/groups/{group}` |
| `groups.configuration.update.json` | `PATCH /v1/organizations/{org}/groups/{group}/configuration` |
| `groups.rotate_tokens.json` | `POST /v1/organizations/{org}/groups/{group}/auth/rotate` |
| `databases.list.json` | `GET /v1/organizations/{org}/databases` |
| `databases.create.json` | `POST /v1/organizations/{org}/databases` |
| `databases.retrieve.json` | `GET /v1/organizations/{org}/databases/{db}` |
| `databases.delete.json` | `DELETE /v1/organizations/{org}/databases/{db}` |
| `databases.configuration.update.json` | `PATCH /v1/organizations/{org}/databases/{db}/configuration` |
| `databases.create_token.json` | `POST /v1/organizations/{org}/databases/{db}/auth/tokens` |
| `databases.rotate_tokens.json` | `POST /v1/organizations/{org}/databases/{db}/auth/rotate` |
| `databases.branch_seed.invalid_request.json` | `POST /v1/organizations/{org}/databases` with `seed.type:"database"` |
| `errors.quota_exceeded_402.json` | `/v1/*` quota 超過時の 402 response |
| `unsupported.audit_logs.json` | `GET /v1/organizations/{org}/audit-logs` |
| `unsupported.group_transfer.json` | `POST /v1/organizations/{org}/groups/{group}/transfer` |
| `unsupported.not_implemented.json` | 501 stub response |
| `routing.validation_errors.json` | 404/405/400 の route/request validation response |

Phase 8 の実装 PR は、上記 API、metadata、migration、権限、quota、エラー契約、snapshot artifact をすべて満たすことを完了条件とする。

**Phase 8 追加テストケース：**

```
TC-8-1: organization CRUD
  （a）POST /admin/v1/organizations {name:"acme"} → 201
  （b）GET /admin/v1/organizations → acme を含む
  （c）GET /admin/v1/organizations/acme → 200
  （d）DELETE /admin/v1/organizations/acme → 204

TC-8-2: group と location の関連
  （a）POST /admin/v1/locations {name:"local"} → 201
  （b）POST /admin/v1/groups {organization:"default", name:"app", location:"local"} → 201
  （c）存在しない organization/location 指定 → 404 ORG_NOT_FOUND / LOCATION_NOT_FOUND

TC-8-3: DB 作成の scope 拡張
  （a）POST /admin/v1/databases {name:"db1", organization:"default", group:"default", location:"default"} → 201
  （b）GET /admin/v1/databases/db1 → organization/group/location を含む
  （c）Phase 7 互換の {name:"db2"} → 201、default scope が付与される

TC-8-4: token scope 拡張
  （a）POST /admin/v1/tokens {access:"rw", organization_scope:"default"} → 201
  （b）scope 外 DB への write → 403 ORG_SCOPE_DENIED
  （c）dbs claim が拒否する DB は org/group scope が許可しても 403

TC-8-5: quota exceeded
  （a）PUT /admin/v1/quotas/database:db1 {storage_bytes:1} → 200
  （b）db1 へ write SQL → 403 QUOTA_EXCEEDED
  （c）SELECT と backup download は quota 超過中も成功

TC-8-6: usage unavailable
  （a）usage snapshot を取得不能状態にする
  （b）quota 判定が必要な write → 503 USAGE_UNAVAILABLE
  （c）GET /admin/v1/usage → 503 USAGE_UNAVAILABLE

TC-8-7: metadata migration
  （a）Phase 7 の databases.json/tokens.json だけが存在する data-dir で起動
  （b）Phase 8 metadata が作成され、既存 DB/token に default scope が付与される
  （c）再起動後も同じ metadata が復元される

TC-8-8: atomicity / rollback
  （a）organizations/groups/locations/quotas の更新中に失敗を注入
  （b）起動成功扱いにせず、partial metadata を成功応答しない

TC-8-9: Turso Platform API 互換 snapshot
  （a）GET /v1/locations → 200 {"locations": {"default":"Self Hosted Default"}}
  （b）POST /v1/organizations/default/groups {name:"app",location:"default"} → 200 {"group":{name,version,uuid,locations,primary,delete_protection}}
  （c）POST /v1/organizations/default/databases {name:"my-db",group:"app"} → 200 {"database":{DbId,Hostname,Name,...}}
  （d）GET /v1/organizations/default/databases?group=app → 200、Turso field casing を維持
  （e）POST /v1/organizations/default/databases/my-db/auth/tokens?expiration=2w&authorization=read-only → 200 {"jwt":"..."}
  （f）uppercase / underscore DB 名 → 400 INVALID_DB_NAME
  （g）Phase 7 legacy DB 名は接続・削除可能だが、Phase 8 新規作成では拒否

TC-8-10: Turso organization API
  （a）GET /v1/organizations → 200 [TursoOrganizationInfo]
  （b）PATCH /v1/organizations/default {overages:false,require_mfa:false} → 200 {"organization":TursoOrganizationInfo}
  （c）PATCH unknown field → 400 INVALID_REQUEST
  （d）GET /v1/organizations/default/usage → 200 {"organization":TursoOrganizationUsage}
  （e）usage 計測不能 → 503 USAGE_UNAVAILABLE

TC-8-11: Turso retrieve API
  （a）GET /v1/organizations/default/groups/app → 200 {"group":TursoGroupInfo}
  （b）GET /v1/organizations/default/databases/my-db → 200 {"database":TursoDatabaseInfo}
  （c）存在しない group → 404 GROUP_NOT_FOUND
  （d）存在しない database → 404 DB_NOT_FOUND

TC-8-12: Turso unsupported API
  （a）GET /v1/organizations/default/plans → 501 NOT_IMPLEMENTED
  （b）POST /v1/organizations/default/members → 501 NOT_IMPLEMENTED
  （c）GET /v1/organizations/default/databases/my-db/stats → 501 NOT_IMPLEMENTED
  （d）POST /v1/upload → 501 NOT_IMPLEMENTED
  （e）POST /v1/organizations/default/databases {name:"seeded",group:"default",seed:{...}} → 400 INVALID_REQUEST
  （f）unsupported API の log に token/body/upload binary が出ない

TC-8-13: Turso route priority / method handling
  （a）POST /v1/organizations/default/databases/my-db/auth/tokens は DB 詳細 route ではなく token 作成 route に一致
  （b）GET /v1/organizations/default/groups/app は groups 一覧 route ではなく group 詳細 route に一致
  （c）GET /v1/locations/ → 404 ENDPOINT_NOT_FOUND
  （d）DELETE /v1/locations → 405 METHOD_NOT_ALLOWED
  （e）未定義 path → 404 ENDPOINT_NOT_FOUND

TC-8-14: Turso request validation
  （a）GET request body あり → 400 INVALID_REQUEST
  （b）POST/PATCH で Content-Type 欠落または JSON object 以外 → 400 INVALID_REQUEST
  （c）unknown query、duplicate query key、unknown body field → 400 INVALID_REQUEST
  （d）percent decode 不能、空 path parameter、path parameter に / または . または .. → 400 INVALID_REQUEST
  （e）Platform token 欠落 → 401 AUTH_REQUIRED、不一致 → 401 AUTH_INVALID

TC-8-15: Phase 8 metadata unique constraints
  （a）organization slug/id 重複 → 409 ORG_ALREADY_EXISTS
  （b）同一 organization 内 group name/slug/id 重複 → 409 GROUP_ALREADY_EXISTS
  （c）location name/id 重複 → 409 LOCATION_ALREADY_EXISTS
  （d）DB name は organization が異なっても global duplicate として 409 DB_ALREADY_EXISTS
  （e）quota/usage は同一 scope を重複追加せず更新する

TC-8-16: Turso snapshot artifact completeness
  （a）tests/snapshots/phase8_turso/*.json が固定表の全 file を含む
  （b）各 snapshot は status/content_type/body を含む
  （c）UUID/timestamp/JWT 等の動的値は placeholder に正規化される
  （d）routing.validation_errors.json が 400/404/405 response を含む

TC-8-17: Turso Platform API token
  （a）GET /v1/auth/validate（Admin token 代用）→ 200 {"exp":-1}
  （b）POST /v1/auth/api-tokens/ci {organization:"default"} → 200 {"name":"ci","id":"tok_<uuid>","token":"<secret>"}
  （c）作成 token で GET /v1/organizations/default/groups → 200
  （d）DELETE /v1/auth/api-tokens/ci → 200 {"token":"ci"}
  （e）失効後 token で GET /v1/auth/validate → 401 AUTH_INVALID

TC-8-18: Turso database delete / auth rotate
  （a）POST /v1/organizations/default/databases {name:"delete-me",group:"default"} → 200
  （b）DELETE /v1/organizations/default/databases/delete-me → 200 {"database":"delete-me"}
  （c）削除済み DB の GET → 404 DB_NOT_FOUND
  （d）POST /v1/organizations/default/databases/my-db/auth/rotate → 200 body なし
  （e）rotate 前に発行した DB token は 401 AUTH_INVALID

TC-8-19: Turso audit logs unsupported
  （a）GET /v1/organizations/default/audit-logs → 501 NOT_IMPLEMENTED
  （b）page/page_size query は受け付けるが success response は返さない
  （c）POST/PATCH/DELETE /v1/organizations/default/audit-logs → 405 METHOD_NOT_ALLOWED
  （d）stub log に token/body が出ない

TC-8-20: Snapshot comparison strictness
  （a）snapshot 比較前に JSON key order と dynamic placeholder が正規化される
  （b）snapshot 内に Bearer/JWT/adlpt_ が残る場合は failure
  （c）status/body schema 差分は failure
  （d）仕様変更 commit なしの snapshot 更新は review failure

TC-8-21: Turso database configuration
  （a）PATCH /v1/organizations/default/databases/my-db/configuration {size_limit:"1mb",delete_protection:true,block_reads:false,block_writes:false,allow_attach:true} → 200 {"size_limit":"1mb","allow_attach":true,"block_reads":false,"block_writes":false,"delete_protection":true}
  （b）delete_protection=true の DB を DELETE /v1/* と DELETE /admin/v1/* で削除しようとすると 403 ORG_SCOPE_DENIED
  （c）size_limit は database quota に反映され、制限超過 write は /v1/* では 402 QUOTA_EXCEEDED、hrana/admin 経由では 403 QUOTA_EXCEEDED
  （d）block_reads=true の DB への read SQL、backup download、export は 403 PERMISSION_DENIED
  （e）block_writes=true の DB への write SQL、restore、replication apply、branch create、import は 403 PERMISSION_DENIED

TC-8-22: Turso group configuration / group auth rotate
  （a）PATCH /v1/organizations/default/groups/app/configuration {delete_protection:true} → 200 {"delete_protection":true}
  （b）delete_protection=true の group 削除と配下 DB 削除は 403 ORG_SCOPE_DENIED
  （c）POST /v1/organizations/default/groups/app/auth/rotate → 200 body なし
  （d）rotate 前に発行した group 配下 DB token と group_scope token は 401 AUTH_INVALID、Platform API token は引き続き有効

TC-8-23: Turso group transfer unsupported / branch seed rejection
  （a）POST /v1/organizations/default/groups/app/transfer → 501 NOT_IMPLEMENTED
  （b）GET/PATCH/DELETE /v1/organizations/default/groups/app/transfer → 405 METHOD_NOT_ALLOWED
  （c）POST /v1/organizations/default/databases {name:"branch-copy",group:"default",seed:{type:"database",name:"my-db"}} → 400 INVALID_REQUEST
  （d）seed upload、CSV import、dump import、remote_encryption はすべて 400 INVALID_REQUEST
  （e）unsupported/stub log に token、body、SQL、upload binary が出ない

TC-8-24: Strict unknown field default
  （a）/admin/v1/*、/v1/*、destructive API、永続化 API の unknown body field は 400 INVALID_REQUEST
  （b）unknown field を無視できるのは hrana wire protocol など該当節で明記された endpoint のみ
  （c）unknown query、duplicate query key、空 query value の拒否は §Phase 8 request validation 固定表と一致する
  （d）TC-8-1〜TC-8-24 の全 snapshot と regression が同時に通る
```

**Phase 8 実装タスク：**

```
T8-1: organization/location/group/quota/usage 型と metadata store を追加
T8-2: Phase 7 metadata から Phase 8 metadata への migration を実装
T8-3: organization/group/location CRUD API を実装
T8-4: quota/usage API と scope parser（organization:{id} / group:{id} / database:{name}）を実装
T8-5: DB/token API に organization/group/location/quota scope を統合
T8-6: JWT org/grp claim と dbs claim の権限優先順位を実装
T8-7: quota 判定を write/restore/replication apply/branch create に接続する
T8-8: Turso Platform API 互換 `/v1/*` route と response wrapper を実装
T8-9: Turso organization list/update/usage API を実装
T8-10: Turso group/database retrieve API を実装
T8-11: Turso unsupported API の 501/400 分類を実装
T8-12: Turso route priority と request validation を固定表通り実装
T8-13: Phase 8 metadata unique constraint を起動時・書き込み前に検証
T8-14: Turso snapshot artifact 生成と正規化比較を実装
T8-15: METHOD_NOT_ALLOWED / ENDPOINT_NOT_FOUND error mapping を実装
T8-16: Turso Platform API token create/validate/revoke を実装
T8-17: Turso database delete と auth rotate を実装
T8-18: audit logs 501 stub と method 分類を実装
T8-19: snapshot 比較 strictness と secret scan を実装
T8-20: strict unknown field default を §9.3 と Phase 8 request validation 固定表通り実装
T8-21: Turso database configuration 更新、quota 接続、delete/block/attach policy を実装
T8-22: Turso group configuration と group auth rotate を実装
T8-23: group transfer 501 stub と branch seed 400 rejection を実装
T8-24: TC-8-1〜TC-8-24 を通す
```


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


### Phase 10：ATTACH DB・メトリクス

**目標**：クロス DB クエリとインメモリメトリクス API を実装する

#### ATTACH DATABASE（クロス DB クエリ）

Turso Cloud と同様に、Adlaire が管理する DB 間に限り `ATTACH DATABASE` を許可する。

**セキュリティモデル：**
- クライアントが `ATTACH DATABASE 'other-db' AS alias` を送信した場合、Adlaire は `'other-db'` を DB 名として解釈し、`databases/other-db/data.db` のパスを解決する
- 任意のファイルパス（`/etc/passwd` 等）は DB 名バリデーション（`^[a-zA-Z0-9_-]{1,127}$`）で事前に拒否する
- 存在しない DB 名の場合は `404 DB_NOT_FOUND` を返す

**Phase 10 ATTACH 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| 対象 SQL | `ATTACH DATABASE '<db-name>' AS <alias>` と `ATTACH '<db-name>' AS <alias>` のみ対象 |
| DB 名 | Phase 8 以降の新規 DB は Turso 互換名、legacy DB は metadata に存在する場合だけ許可 |
| alias | `^[a-zA-Z_][a-zA-Z0-9_]{0,63}$`。`main`、`temp`、`sqlite_*` は `INVALID_REQUEST` |
| quote | DB 名は single quote のみ許可。double quote、identifier quote、パラメータ化 ATTACH は Phase 10 対象外で `INVALID_REQUEST` |
| path | 実 OS path は response/log に出さない。SQLite へ渡す直前だけ canonical data_dir 配下 path に変換する |
| auth | 接続先 DB と attach 対象 DB の両方に JWT scope が必要。どちらか scope 外なら `ORG_SCOPE_DENIED` または `PERMISSION_DENIED` |
| detach | `DETACH DATABASE <alias>` は許可。ただし `main`/`temp` は拒否 |
| transaction | active transaction 中の ATTACH/DETACH は SQLite の結果に従うが、任意 path validation は必ず先に行う |

**実装方針：**
- hrana-http v2 / hrana-ws v3 の `execute` / `sequence` / `batch` で ATTACH SQL を受け取った際、Adlaire 側でインターセプトして DB 名を解決する
- ATTACH 判定に正規表現だけを使ってはならない。SQL tokenizer または SQLite prepare 前の限定 parser で、文字列リテラル、コメント、quoted identifier 内の `ATTACH` を無視する
- libsql の Connection に対してパス解決済みの ATTACH を発行する
- 対象 DB の接続が未オープンの場合はその場でオープンする
- `allow_attach=false`、JWT/org/group/db scope、`block_reads`、`block_writes`、ro token の優先順位は §9.4.2 の Phase 10 ATTACH / metrics 優先順位固定表に従う

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
  "tokens_total": 5,
  "tokens_revoked": 1,
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
  ]
}
```

カウンター（`queries_total` 等）はプロセス起動からの累積値。再起動でリセットされる（Phase 10 時点では永続化しない）。

**Phase 10 metrics 固定契約：**

| Field | 更新タイミング |
|-------|----------------|
| `queries_total` | execute/batch/sequence の各 SQL step が libSQL に渡された後。SQL error でも 1 加算 |
| `rows_read_total` | response rows の件数を加算。COUNT など集約結果は返却 row 数を加算 |
| `rows_written_total` | libSQL の affected rows を加算。DDL は 0 |
| `connections_active` | HTTP pipeline は処理中だけ、WebSocket は接続中だけ加算 |
| `size_bytes` / `wal_size_bytes` | metrics API 応答時に filesystem から取得。取得失敗は 0 ではなく WARN + field 0 |
| `tokens_total` / `tokens_revoked` | `tokens.json` から算出。runtime counter と不一致なら metadata を正とする |

**追加テストケース：**

```
TC-3-6: メトリクス API
  （a）GET /admin/v1/metrics（管理トークンあり）→ 200、databases 配列に管理下 DB が含まれる
  （b）クエリ実行後に queries_total が増加していること
  （c）GET /admin/v1/metrics（管理トークンなし）→ 401
```

**Phase 9 実装タスク（WebSocket）：**

```
T3-1: WebSocket サーバー追加（hyper の WebSocket upgrade）
T3-2: hrana-ws v3 hello ハンドシェイク + JWT 認証
T3-3: ストリーム多重化レイヤー実装（stream_id ごとの接続状態管理）
T3-4: execute / batch / sequence / describe リクエスト処理（libsql Connection 経由）
T3-5: インタラクティブトランザクション状態管理（BEGIN/COMMIT/ROLLBACK）
```

**Phase 10 実装タスク：**

```
T3-6: ATTACH DATABASE インターセプト・DB 名バリデーション・パス解決
T3-7: メトリクス収集（インメモリカウンター）+ GET /admin/v1/metrics
T3-8: 統合テスト TC-3-1〜TC-3-6
```

#### 実装詳細

```rust
// ATTACH DATABASE インターセプト
// 注意: 下記の正規表現方式は Phase 10 では実装禁止例である。
// 実装では SQL tokenizer または限定 parser を使い、文字列リテラル・コメント内の ATTACH を誤検出しないこと。

/// "ATTACH DATABASE 'foo' AS alias" を検出して解決済みパスに書き換える。
/// foo が Adlaire 管理外（バリデーション失敗 or 未登録）なら Err を返す。
static ATTACH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)ATTACH\s+(?:DATABASE\s+)?'([^']+)'\s+AS\s+(\w+)"#).unwrap()
});

pub async fn resolve_attach(
    sql: &str,
    db_mgr: &DbManager,
    data_dir: &std::path::Path,
) -> Result<String, AppError> {
    if let Some(caps) = ATTACH_RE.captures(sql) {
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
            metrics.rows_read_total.fetch_add(row.rows.len() as u64, Ordering::Relaxed);
            metrics.rows_written_total.fetch_add(row.rows_affected, Ordering::Relaxed);
        }
    }
}

// http/admin/metrics.rs

/// GET /admin/v1/metrics  → プロセス起動からの累積カウンター
pub async fn get(
    req: Request<Incoming>,
    state: SharedState,
) -> Result<Response<Full<Bytes>>, Infallible> {
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
    Ok(json_ok(&serde_json::json!({
        "uptime_seconds":  uptime,
        "tokens_total":    state.metrics.tokens_total.load(Relaxed),
        "tokens_revoked":  state.metrics.tokens_revoked.load(Relaxed),
        "databases":       databases,
    })))
}
```

---


### Phase 11：レプリケーション基盤（WAL ストリーム・スナップショット）

**目標**：プライマリが WAL ストリームとスナップショットを公開できる状態にする

Phase 11 はレプリケーションの**送信側（primary）基盤**を完成させるフェーズである。レプリカが WAL を継続取得・適用して追いつくこと、レプリカ書き込みを redirect すること、health に lag を出すことは Phase 12 の完了条件とする。

#### アーキテクチャ

```
クライアント
  │
  ├─ 書き込み → プライマリ（:8080）─ WAL 同期 ─→ レプリカ 1（:8080）
  │                                             └→ レプリカ 2（:8080）
  └─ 読み取り → レプリカ（ロードバランサー経由）
```

- プライマリとレプリカは同じバイナリ。起動フラグでロールを決定する
- Phase 11 では primary role の replication API を実装する
- Phase 12 では replica role の WAL 取得・適用ループと書き込み redirect を実装する

**Phase 11 replication primary 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| frame_no | DB ごとに 1 から単調増加。欠番は許可しない。再起動後は WAL/manifest から最大値を復元 |
| checksum | frame payload bytes に CRC32 を付与。HTTP response と archive manifest の checksum は同一 |
| `from_frame` | 1 以上の integer 必須。0、負数、非数値は `INVALID_REQUEST` |
| log response | SSE は `event: frame`、`id: <frame_no>`、`data: <json>`。heartbeat は `event: heartbeat` |
| ordering | 1 connection 内では frame_no 昇順のみ。並列接続でも同じ frame_no に異なる bytes を返さない |
| snapshot | snapshot 取得中は整合した DB byte stream を作る。途中で write があっても snapshot 内は一貫 |
| headers | snapshot は `Content-Type: application/octet-stream`、`X-Adlaire-Base-Frame`、`X-Adlaire-Checksum` を返す |
| heartbeat | unknown replica_id は登録し、既存 replica_id は上書き更新。`synced_frame` が primary 最大 frame を超えたら `INVALID_REQUEST` |
| auth | replication token が設定されている場合は Bearer 完全一致。未設定で role primary の場合、replication API は `AUTH_REQUIRED` |
| logging | frame bytes、SQL、token はログ禁止。frame_no、db、replica_id、lag_frames だけ可 |

#### 起動フラグ（Phase 11 追加）

```
# プライマリとして起動
adlaire-db serve --data ./data --role primary --primary-port 8082

# レプリカとして起動（Phase 12 で完了）
adlaire-db serve --data ./data --role replica --primary-url http://primary:8082
```

| フラグ | 説明 |
|--------|------|
| `--role` | `standalone`（デフォルト）/ `primary` / `replica` |
| `--primary-port` | プライマリが WAL ストリームを公開するポート（デフォルト: 8082）|
| `--primary-url` | レプリカが接続するプライマリの URL（Phase 12 で有効化） |
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
- `db`: 対象 DB 名（Phase 11 はマルチ DB 対応）
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

プライマリは Phase 11 では `synced_frame` を受付・記録するだけで WAL フレーム GC を行わない。WAL フレーム削除は Phase 13 の retention cleanup 契約に従う。

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

Phase 12 から `GET /v2/health` のレスポンスにロール情報を追加する：

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

#### 14.10 WAL レプリケーション（Phase 11）

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

**レプリカ側フレーム受信・適用ループ（Phase 12 実装）：**

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


### Phase 12：レプリカ同期・書き込みリダイレクト

**目標**：レプリカが WAL フレームを受信・適用し書き込みをプライマリへ転送する

**Phase 12 replica 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| replica state | `{data-dir}/meta/replica-state.json` に `primary_url`、`last_applied_frame`、`last_seen_primary_frame`、`updated_at` を保存 |
| resume | 起動時は `last_applied_frame + 1` から取得再開。state 破損は起動失敗 |
| apply order | frame_no 昇順のみ適用。欠番検出時は後続 frame を適用せず再取得 |
| checksum mismatch | 対象 frame を破棄し ERROR log、同じ `from_frame` から再取得。3 回連続失敗で health `degraded` |
| redirect | replica への write SQL、restore、branch create、extension load は 307 で primary URL へ redirect |
| primary down | primary 到達不能時の write は 503 `REPLICATION_TIMEOUT`。read は local replica で許可 |
| health | `role`、`primary_url`、`last_applied_frame`、`lag_frames`、`status` を返す。`status` は `ok` / `degraded` |
| sync write | `sync` mode は primary が replica ACK を待つ場合だけ使用。quorum 未定義なら起動失敗 |
| auth | primary 取得時は replication token を送る。token 不一致は retry せず `degraded` |

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

**Phase 11 実装タスク（前フェーズの完了条件）：**

```
T4-1: --role フラグ対応（standalone / primary / replica の起動分岐）
T4-2: WAL フレームストリーム API（GET /replication/v1/log SSE）
T4-3: スナップショット API（GET /replication/v1/snapshot）
```

**Phase 12 実装タスク：**

```
T4-4: レプリカ側 WAL フレーム受信・適用ループ
T4-5: 書き込みリダイレクト（307 → primary-url）
T4-6: GET /v2/health にロール・ lag 情報を追加
T4-7: 統合テスト TC-4-1〜TC-4-5
```

#### 実装詳細

```rust
// middleware/replica_redirect.rs

use hyper::{Request, Response, body::Incoming};
use http_body_util::Full;
use bytes::Bytes;

/// レプリカモードで書き込みリクエストを受けた際に primary-url へ 307 リダイレクト
/// route() の先頭で呼び出し、Some(response) が返った場合はそれを返す
pub fn maybe_redirect_write(
    req: &Request<Incoming>,
    state: &SharedState,
) -> Option<Response<Full<Bytes>>> {
    if let ServerRole::Replica { primary_url } = &state.role {
        if is_mutating_request(req) {
            let target = format!(
                "{}{}",
                primary_url.as_str().trim_end_matches('/'),
                req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("")
            );
            return Some(Response::builder()
                .status(http::StatusCode::TEMPORARY_REDIRECT)
                .header(http::header::LOCATION, target)
                .body(Full::default())
                .unwrap());
        }
    }
    None
}

/// POST / PUT / DELETE は書き込みリクエストとみなす
fn is_mutating_request(req: &Request<Incoming>) -> bool {
    matches!(
        req.method(),
        &http::Method::POST | &http::Method::PUT | &http::Method::DELETE
    )
}

// http/health.rs（Phase 12 拡張）
// Phase 11 前提: ServerRole::Replica 有効化・ReplicationState 型の確定後に実装
// Phase 11 完了後は ServerRole のコメントアウトを解除する（§14.2 AppState 参照）

#[derive(serde::Serialize)]
pub struct HealthResponse {
    pub status:                   &'static str,
    pub role:                     &'static str,   // "standalone" / "primary" / "replica"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_url:              Option<String>,  // replica のみ
    pub replication_lag_frames:   Option<u64>,     // replica のみ
}

pub async fn handle(
    _req: Request<Incoming>,
    state: SharedState,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let lag = state.replication.as_ref()
        .map(|r| r.lag_frames.load(std::sync::atomic::Ordering::Relaxed));
    let status = match lag {
        Some(lag) if lag > 1000 => "degraded",
        _ => "ok",
    };
    let (role_str, primary_url) = match &state.role {
        ServerRole::Standalone        => ("standalone", None),
        ServerRole::Primary { .. }    => ("primary",    None),
        // Phase 11 解除後: ServerRole::Replica { primary_url } =>
        //     ("replica", Some(primary_url.to_string())),
    };
    Ok(json_ok(&HealthResponse {
        status,
        role: role_str,
        primary_url,
        replication_lag_frames: lag,
    }))
}
```

---


### Phase 13：WAL アーカイブ・manifest 管理

**目標**：WAL フレームのアーカイブと manifest.json による管理を実装する

**スコープ：**
- WAL アーカイブ書き込み（チェックポイント前フック）
- manifest.json による WAL フレーム管理
- `wal_retention_days` 設定によるアーカイブ保持期間の管理

**Phase 13 archive 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| frame file | `frame-{frame_no:012}.bin`。frame_no は manifest 内で unique、昇順 |
| snapshot file | `snapshot-{base_frame:012}.db`。最新 1 件を manifest の `snapshot` に記録 |
| manifest commit | frame file と snapshot file を fsync 後、最後に manifest を atomic rename |
| consistency check | 起動時に manifest の全 file 存在、size、checksum を検証。欠損・不一致は起動失敗 |
| retention | 削除対象 frame を manifest から外す前に削除 plan を作り、削除成功後に manifest commit |
| cleanup failure | file 削除失敗時は manifest を更新しない。WARN log 後、次回 cleanup で再試行 |
| disabled mode | `wal_retention_days = 0` では archive file を新規作成しない。既存 archive は削除しない |
| clock | retention 判定は manifest の `created_at` を使う。file mtime は使わない |

**完了条件（テストケース）：**

```
TC-5-8: 保持期間超過フレームのクリーンアップ
  設定: wal_retention_days = 1
  （a）2 日前のタイムスタンプを持つフレームを作成
  （b）クリーンアップ実行（または 24h 経過後）
  （c）該当フレームが削除され、manifest.json から除去されている
```

**Phase 13 実装タスク：**

```
T5-1: WAL フレームアーカイブ書き込み
  [ ] libsql チェックポイント前フックで WAL フレームを wal-archive/ へコピー
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

#### 14.8 WAL アーカイブ処理（Phase 13〜14）

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
```

```rust
// wal/manifest.rs
use tokio::io::AsyncWriteExt;

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
        {
            let mut f = tokio::fs::File::create(&tmp).await?;
            f.write_all(&json).await?;
            f.sync_all().await?;
        }
        tokio::fs::rename(&tmp, path).await?;
        Ok(())
    }
}
```


### Phase 14：バックアップ・リストア・PITR API

**目標**：WAL アーカイブからのオンラインバックアップと任意時点リストア（PITR）が動作する

**スコープ：**
- バックアップ API：`GET /admin/v1/databases/{name}/backup`
- リストア API：`POST /admin/v1/databases/{name}/restore`
- PITR API：`POST /admin/v1/databases/{name}/restore/point-in-time`

**Phase 14 restore 固定契約：**

Phase 14 の backup、restore、PITR、rollback、restart recovery、redaction は §9.1.30 を正とする。下表は Phase 14 固有の endpoint 入口であり、§9.1.30 と衝突する場合は同じ PR で解消してから実装する。

| 項目 | 固定仕様 |
|------|----------|
| backup response | `Content-Type: application/octet-stream`、`Content-Disposition: attachment; filename="{db}.db"` |
| backup consistency | SQLite online backup 相当で整合 snapshot を返す。backup 中 write はブロックしない |
| upload limit | restore body は設定値 `restore_max_bytes` が未定義の間、DB 既存 size の 2 倍または 1 GiB の小さい方を上限 |
| temp layout | `{data-dir}/databases/{name}/restore-{request_id}/` に upload、verified、old を分けて置く |
| restore lock | 対象 DB 単位で exclusive lock。restore 中の write は 503 `STORAGE_BUSY`、read は既存 DB で継続可 |
| protection | `delete_protection=true` は `403 ORG_SCOPE_DENIED`、`block_writes=true` は `403 PERMISSION_DENIED`。restore 後 size が quota 超過なら commit 前に `QUOTA_EXCEEDED` |
| verification | restore/PITR は `PRAGMA integrity_check` が `ok` の場合だけ commit |
| commit | runtime DB close、old へ退避、new を `data.db` へ rename、directory fsync、DB reopen の順 |
| rollback | commit 前後のどの失敗でも old を戻す。戻せない場合は `restore-failed.json` marker を残し起動失敗 |
| PITR selector | request は `{timestamp}` または `{frame_no}` のどちらか 1 つだけ。両方・どちらもなしは `INVALID_REQUEST` |
| PITR replay | snapshot の `base_frame` から target frame まで checksum 検証しながら適用。欠損は `FRAME_NOT_FOUND` |
| success response | restore / PITR 成功は `204 No Content`。成功時に JSON body は返さない |

**完了条件（テストケース）：**

```
TC-5-1: バックアップと同時書き込み
  （a）GET /admin/v1/databases/{name}/backup を開始（大きな DB でストリーミング）
  （b）バックアップ中に POST /{name}/v2/pipeline で INSERT を実行
  （c）バックアップは整合性を保って完了し、書き込みリクエストも 200 で成功

TC-5-2: バックアップからリストア
  （a）GET /admin/v1/databases/{name}/backup でバックアップファイルを取得
  （b）POST /admin/v1/databases/{name}/restore でリストア → 204 body なし
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

**対象外（Phase 15 以降）：**
- ブランチ機能（Phase 15）
- 外部ストレージへのアーカイブ転送

**Phase 14 実装タスク：**

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
  [ ] 成功時: libsql::Builder::new_local() で DB を再オープンしてサービス再開
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


### Phase 15：ブランチ

**目標**：DB の任意時点からブランチを作成し、独立した DB として読み書き可能にする

**スコープ：**
- ブランチ DB 作成 API（`from: "current"` / `from: {timestamp}` / `from: {frame_no}`）
- ブランチ一覧・削除 API
- ブランチ DB 命名規則と予約名バリデーション（`___`）
- 再起動後のブランチ DB 自動復元

**Phase 15 branch 固定契約：**

Phase 15 の branch create、delete、routing、Turso seed、restart recovery、isolation は §9.1.31 を正とする。下表は Phase 15 固有の入口条件であり、§9.1.31 と衝突する場合は同じ PR で解消してから実装する。

| 項目 | 固定仕様 |
|------|----------|
| branch name | `^[a-z0-9-]{1,64}$`。source DB と同じ名前、`___`、`admin`、`meta` は拒否 |
| internal DB name | `{source_db}___{branch_name}`。通常 DB create API から `___` を含む名前は常に拒否 |
| source selector | `"current"`、`{"timestamp":"..."}`、`{"frame_no":N}` のいずれか 1 つだけ |
| create order | branch directory 作成、DB file 構築、integrity_check、DbManager 登録、最後に `branches.json` commit |
| delete order | runtime map から外し、connection close、directory rename to trash、`branches.json` commit、trash 削除 |
| partial create | `branches.json` にない branch directory は起動時 WARN + cleanup。cleanup 失敗でも active 扱いしない |
| partial delete | `branches.json` にない branch directory は接続不可。cleanup 対象 |
| source delete | active branch がある source DB の削除は `403 ORG_SCOPE_DENIED`。cascade delete は Phase 15 対象外 |
| isolation | branch write は source DB に反映しない。source write は既存 branch に反映しない |
| Turso seed | `/v1/organizations/{org}/databases` の `seed.type:"database"` は Phase 15 で branch create に昇格してよい。ただし branch 名に相当する field がない request は `INVALID_REQUEST` |
| protection | source DB `delete_protection=true` でも branch create は許可する。source DB `block_reads=true` の branch create は `403 PERMISSION_DENIED` |
| quota | branch DB は source の database quota を継承する。branch 作成で organization/group quota を超える場合は `QUOTA_EXCEEDED` |
| token scope | source DB token は branch DB へ自動拡張しない。branch 用 token は別途発行する |

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

**対象外（Phase 16 以降）：**
- ブランチのマージ
- ブランチ間 diff

**Phase 15 実装タスク一覧：**

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
  [ ] 新 DB を libsql::Builder::new_local() でオープン・マルチ DB マネージャへ登録
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
        → Arc<libsql::Database> drop・ディレクトリ削除・branches.json 更新
  参照: §6.4（ブランチ）
  検証: TC-6-4, TC-6-5

T6-6: 起動時ブランチ自動復元ロジック
  [ ] branches.json を読み込み、各 db_name の DB ディレクトリが存在すれば libsql::Builder::new_local() でオープン
  [ ] ディレクトリが存在しないエントリは WARN ログを出力してスキップ
  参照: §8.1 Step 5-3
  検証: TC-6-7

T6-7: 統合テスト
  [ ] TC-6-1〜TC-6-7 を全て実行し PASS することを確認
  [ ] Phase 1〜14 の TC がリグレッションしないことを確認
```

---


### Phase 16：SQLite 拡張ロード

**目標**：許可済み SQLite 拡張だけを安全に登録・ロード・無効化できるようにする。

Phase 16 では `.so` 拡張のみを対象とする。Wasm 拡張、任意パスロード、SQL からの `load_extension()` 直接実行は対象外とする。

- API: `GET/POST/DELETE /admin/v1/extensions`（§9.5）
- 永続化: `meta/extensions.json` と `{data-dir}/extensions/{name}/{version}/`（§9.6）
- 完了条件: TC-16-1〜TC-16-6 と T16-1〜T16-6 をすべて満たす

**Phase 16 extension 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| binary source | HTTP upload は受け付けない。事前配置済み file のみ登録対象 |
| path | `{data-dir}/extensions/{name}/{version}/{name}.so` 以外は拒否。symlink は拒否 |
| sha256 | 登録前、load 前、起動時復元前に毎回検証 |
| load scope | load は新規 DB connection 作成時に適用。既存 connection への retroactive load は保証しない |
| failure | 登録時 load 失敗は metadata を追加しない。起動時 load 失敗は該当 extension を `state:"load_failed"` にし ERROR log |
| delete | metadata から削除し、binary directory は残す。削除済み extension は新規 connection に load しない |
| SQL direct load | `load_extension()` SQL は常に `EXTENSION_NOT_ALLOWED` |
| logging | extension name/version/sha256 は可。絶対 path と load error の環境変数展開値は秘匿 |

**Phase 16 extension manifest schema：**

```json
{
  "extensions": [
    {
      "name": "vector",
      "version": "0.1.0",
      "filename": "vector.so",
      "sha256": "64 lowercase hex chars",
      "enabled": true,
      "state": "registered",
      "loaded_at": null,
      "created_at": "2026-09-13T00:00:00Z"
    }
  ]
}
```

| Field | Validation |
|-------|------------|
| `name` | `^[a-zA-Z0-9_-]{1,64}$`。`sqlite`、`libsql`、`adlaire` prefix は予約で `EXTENSION_NOT_ALLOWED` |
| `version` | semver `MAJOR.MINOR.PATCH` のみ。pre-release/build metadata は Phase 16 対象外 |
| `filename` | `{name}.so` のみ。slash、dot-dot、絶対 path は `INVALID_REQUEST` |
| `sha256` | lowercase hex 64 文字のみ |
| `enabled` | boolean 必須 |
| `state` | `registered` / `loading` / `loaded` / `load_failed` / `disabled` / `deleted` のみ |
| `loaded_at` | `state="loaded"` の時だけ RFC3339 UTC 秒精度。未ロードは `null` |

登録時は binary を `{data-dir}/extensions/{name}/{version}/{filename}` に配置済みであることを確認し、sha256 が一致した場合だけ `extensions.json` に追加する。HTTP API から binary upload は受け付けない。load は server 起動時と `POST /admin/v1/extensions` 後に行い、失敗時は metadata を追加せず `EXTENSION_LOAD_FAILED` を返す。`DELETE` は metadata から削除し、binary directory は削除しない。既存 metadata に `loaded` boolean がある場合は migration で `loaded:true` を `state:"loaded"`、`loaded:false` を `state:"registered"` に変換し、以後 `loaded` boolean を正として参照してはならない。

### Phase 17：メトリクス永続化・外部監視連携

**目標**：Phase 10 のインメモリ metrics を永続 counter に拡張し、Prometheus 互換出力を提供する。

Phase 17 では alerting、remote write、外部 SaaS 連携は対象外とする。Prometheus text exposition のみを対象にする。

- API: `GET /admin/v1/metrics/prometheus`（§9.5）
- 永続化: `meta/metrics-snapshot.json`
- quota 判定用 usage: Phase 8 の `usage.json` を正とする
- 完了条件: TC-17-1〜TC-17-5 と T17-1〜T17-5 をすべて満たす

**Phase 17 metrics 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| counter source | runtime atomic counter を正とし、30 秒ごとに snapshot へ保存 |
| startup | snapshot が正常なら counter 初期値へ反映。破損なら WARN 後 0 初期化 |
| quota usage | quota 判定は `usage.json` を正とし、metrics snapshot から推測しない |
| Prometheus escaping | label value は `\`、`"`、newline を Prometheus 仕様通り escape |
| route label | raw path ではなく route pattern を使う。DB 名や token id を label に入れない |
| content type | `text/plain; version=0.0.4; charset=utf-8` 固定 |
| accept | `Accept` 未指定、`*/*`、`text/plain` は 200。その他は 406 `NOT_ACCEPTABLE` |
| line format | UTF-8、LF 改行、末尾 LF 必須。各 metric は `HELP`、`TYPE`、samples の順で出す |
| invalid value | 取得不能値を `NaN` として出さない。該当 sample を省略し WARN log を出す |
| shutdown | graceful shutdown 時に同期 snapshot を 1 回書く。失敗時は ERROR log |

**Phase 17 metrics snapshot schema：**

```json
{
  "counters": {
    "queries_total": 0,
    "rows_read_total": 0,
    "rows_written_total": 0,
    "http_requests_total": 0,
    "errors_total": 0
  },
  "gauges": {
    "databases_total": 0,
    "connections_active": 0,
    "storage_bytes": 0,
    "wal_size_bytes": 0
  },
  "updated_at": "2026-09-13T00:00:00Z"
}
```

Prometheus endpoint は `text/plain; version=0.0.4; charset=utf-8` を返し、metric 名は以下に固定する。

| Metric | Type | Labels |
|--------|------|--------|
| `adlaire_queries_total` | counter | `db`, `kind` (`read`/`write`) |
| `adlaire_rows_read_total` | counter | `db` |
| `adlaire_rows_written_total` | counter | `db` |
| `adlaire_http_requests_total` | counter | `method`, `route`, `status` |
| `adlaire_errors_total` | counter | `code` |
| `adlaire_databases_total` | gauge | なし |
| `adlaire_connections_active` | gauge | `db` |
| `adlaire_storage_bytes` | gauge | `db` |
| `adlaire_wal_size_bytes` | gauge | `db` |

label 値に DB 名以外の user input を直接入れてはならない。SQL text、SQL args、token、raw path は label 禁止。snapshot 書き込み間隔は 30 秒固定とし、graceful shutdown 時は最終 snapshot を同期書き込みする。

### Phase 18：HA・自動フェイルオーバー

**目標**：primary/replica 構成で leader election、failover、split-brain 防止、write redirect を完成させる。

Phase 18 は single-leader 構成のみを対象にする。multi-primary write、distributed transaction、外部 consensus service 依存は対象外とする。

- API: `GET /ha/v1/status`、`POST /ha/v1/promote`、`POST /ha/v1/demote`（§9.5）
- 永続化: `meta/ha-state.json`
- term: 単調増加のみ許可。古い term による昇格は `HA_SPLIT_BRAIN`
- 完了条件: TC-18-1〜TC-18-7 と T18-1〜T18-7 をすべて満たす

**Phase 18 HA 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| auth | HA API は Admin token と HA token の両方必須。片方欠落は `AUTH_REQUIRED`、不一致は `AUTH_INVALID` |
| term update | promote/demote は request term が保存済み term 以上の場合だけ許可。小さい term は `HA_SPLIT_BRAIN` |
| candidate | heartbeat timeout で candidate になっても write は受けない。operator promote まで primary にならない |
| redirect | leader が分かる replica/candidate は write を leader へ 307。leader 不明なら `HA_NO_LEADER` |
| demote | primary demote 後は role `replica` または `standalone` に遷移し、write を即時停止 |
| split-brain | 自 node と異なる leader_id を同一 term で検出したら write 停止、`HA_SPLIT_BRAIN` |
| persistence | role/term/leader 更新は write 停止または開始より前に `ha-state.json` commit |
| recovery | 起動時に `role=primary` でも HA peer 確認前は write を受けず `candidate` として検証する |

**Phase 18 HA state schema：**

```json
{
  "node_id": "node-a",
  "term": 1,
  "leader_id": "node-a",
  "role": "primary",
  "last_applied_frame": 42,
  "last_heartbeat_at": "2026-09-13T00:00:00Z",
  "updated_at": "2026-09-13T00:00:00Z"
}
```

| Field | Validation |
|-------|------------|
| `node_id` | `^[a-zA-Z0-9_-]{1,64}$`。起動 flag `--ha-node-id` と一致必須 |
| `term` | 0 以上の integer。更新時は既存値以上のみ許可 |
| `leader_id` | `null` または node id。`role=primary` の場合は自 node id と一致必須 |
| `role` | `standalone` / `primary` / `replica` / `candidate` のみ |
| `last_applied_frame` | 0 以上の integer |
| `last_heartbeat_at` | `null` または RFC3339 UTC 秒精度 |

Phase 18 の leader election は外部 consensus service を使わない。promotion は `POST /ha/v1/promote` を operator が明示実行した場合のみ行う。自動 failover は primary heartbeat が `--ha-failover-timeout-ms` を超過し、replica が最新 frame に追いついている場合だけ candidate へ遷移する。candidate は operator promote なしに primary へ昇格しない。

### Phase 19：libSQL 内部コンポーネント段階的内製化

**目標**：Turso Cloud / libSQL SDK 互換を維持したまま、内部コンポーネントを `adlaire-*` crate へ段階的に差し替える。

Phase 19 は wire format、admin API、metadata schema、JWT claim を変更してはならない。SQL parser 完全内製は対象外とし、Phase 19 では WAL checkpoint 制御、storage 境界、executor 境界の adapter 化までを対象にする。

- 切り替え方式: config flag で既存 libSQL 経路と内製 crate 経路を切り替える
- 既定値: 直前 Phase と同じ挙動
- rollback: flag を戻すだけで完了できること
- 完了条件: TC-19-1〜TC-19-6 と T19-1〜T19-6 をすべて満たす

**Phase 19 internal adapter 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| default | 全 flag の default は `libsql`。既存利用者の挙動は変えない |
| shadow mode | adapter は libsql 結果と比較する shadow 実行から開始し、差分は ERROR + test failure |
| active mode | active 化は該当 snapshot と Phase 1〜18 regression が通る場合だけ |
| rollback | flag を戻すだけで metadata migration なしに旧経路へ戻る |
| write path | Phase 19 の storage adapter は readonly まで。write path 差し替えは Phase 20 以降 |
| error mapping | adapter 内部 error は既存 §7.3 code へ写像。新 code が必要なら先に仕様改訂 |
| performance | p95 latency、RSS、DB size、WAL size を baseline artifact に保存 |
| compatibility | API response、wire bytes、metadata JSON、JWT claim の snapshot 差分ゼロ |

**Phase 19 config flags：**

| Flag | TOML | Default | Allowed | 完了条件 |
|------|------|---------|---------|----------|
| `--internal-wal` | `[internal] wal` | `libsql` | `libsql` / `adlaire` | `adlaire` で TC-19-1〜TC-19-3 が通る |
| `--internal-storage` | `[internal] storage` | `libsql` | `libsql` / `adlaire-readonly` | Phase 19 では readonly adapter まで。write path 切替は禁止 |
| `--internal-executor` | `[internal] executor` | `libsql` | `libsql` / `adlaire-adapter` | API 互換を維持した adapter 境界のみ |

不正値は起動失敗。`storage=adlaire-readonly` かつ write workload が来た場合は、自動 fallback せず `INTERNAL_ERROR` ではなく起動時 config error にする。rollback は全 flag を `libsql` に戻すだけで metadata migration なしに完了しなければならない。

**Phase 19 性能・互換 baseline：**

- `libsql` 既定経路に対して Phase 1〜18 regression は 100% 通過
- `adlaire` 経路の p95 latency は同一 workload で `libsql` 経路の 2 倍以内
- crash recovery 後に `PRAGMA integrity_check` が `ok`
- TypeScript/Rust/Go libSQL SDK の CRUD smoke test が全て成功
- wire format、error code、metadata schema、JWT claim に差分がないことを snapshot test で確認

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

TLS ネイティブ対応は Phase 1〜19 では対象外とする。Phase 19 完了後に専用フェーズとして仕様化されるまで、server binary 内に TLS termination を実装してはならない。

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
- 静的リンク（musl）によるランタイム依存ゼロは必須条件にしない。配布物は release-check で glibc 依存、動的ライブラリ依存、対象アーキテクチャを明示し、musl 対応は専用リリース仕様が追加されるまで実装対象外とする
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
- `--log-file <PATH>` 指定時：ファイルへ書き出し（ローテーションは外部ツール任せ）（未実装）
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
| busy timeout | 5000 ms | `--busy-timeout` | `[server] busy_timeout_ms` | ロック待機タイムアウト。超過時 503 BUSY |
| WAL checkpoint interval | 1000 pages | — | `[storage] wal_checkpoint_pages`（未実装・Phase 11） | 自動チェックポイントのページ閾値 |
| WAL checkpoint mode | `passive` | — | `[storage] wal_mode` | `passive` / `full` / `restart` |
| synchronous | `NORMAL` | — | `[storage] synchronous`（未実装・Phase 11） | `OFF` は非サポート（I-4 違反） |

### 13.3 チェックポイント挙動

- SQLite のデフォルト自動チェックポイント（1000 pages）をそのまま使用（Phase 1）
- Phase 1 では手動チェックポイントの API は提供しない
- Phase 11（レプリケーション）時に WAL チェックポイント制御を再設計する

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
| sqld | libSQL プロジェクトのサーバーコンポーネント（参考情報）。Adlaire DB は sqld を使用せず、libsql crate（embedded モード）を使用する |
| libsql crate | Rust の libSQL クライアントライブラリ（crates.io）。embedded SQLite モードで使用する |
| hrana | Turso / libSQL のワイヤプロトコル名。hrana-http（HTTP版）と hrana-ws（WebSocket版）がある |
| baton | hrana プロトコルにおけるセッション継続識別子 |

---

## Phase 19 内製化方針

### 基本方針
- 内製化の単位はクレートとする
- 外部クレートを内製クレートに段階的に差し替えることで内製化を進める
- 内製化の目的は、Turso Cloud 互換の自己ホスト DB 管理基盤を維持したまま、内部実装を Adlaire 独自基盤へ育てることである
- 内製化は Turso Cloud 互換を維持するための内部実装差し替えであり、Turso Cloud 追従を止める理由にしてはならない
- API、認証、metadata、エラー、SDK 互換挙動は互換レイヤーとして固定し、その内側の実装から段階的に置き換える
- Turso 互換 mode は常に既定 mode とし、Adlaire 独自拡張 mode を追加する場合も互換 mode の snapshot と SDK regression に差分を出してはならない
- Phase 19 以降の内製化は `external contract freeze / internal replacement only` を固定契約とし、wire/API/metadata/error/auth/SDK 挙動を変えずに内部 crate、adapter、engine、scheduler、storage 境界だけを置換する
- production path を内製 crate へ切り替える場合は、shadow mode、active mode、rollback flag、compatibility oracle、performance baseline、crash recovery evidence を同一 PR で提示する
- 既存の外部クレートで要件を満たせる場合は積極的に採用する
- 既存クレートで不足する機能は、最初から内製クレートとして開発する
- **外部クレートと内製クレートの併用パターンを初期段階から採用する**
- 内製化の順序は実装難易度が低いものから優先する

### 併用パターン

初期段階から外部クレートと内製クレートを併用する。
外部クレートで不足する機能を内製クレートで補い、
成熟次第に外部クレートを内製クレートへ差し替える。
差し替え中も Turso Cloud 互換テストと既存 libSQL SDK 互換テストを必須とし、互換性が落ちる差し替えは完了扱いにしない。

### 内製化順序（難易度低い順）

| 優先度 | 対象 | 現行クレート | 備考 |
|--------|------|------------|------|
| 1 | WAL チェックポイント制御 | libSQL | Phase 11 と直結 |
| 2 | ストレージ層 | libSQL（SQLite ページャー）| WAL 内製後に着手 |
| 3 | SQL パーサ | libSQL（SQLite）| 最難関・最後 |

### 実施時期

内製化の実装開始は Phase 19 とする。Phase 18 以前は、内製 crate の設計メモ、ベンチマーク、互換テスト追加のみ許可し、production path の切り替えは行わない。

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
