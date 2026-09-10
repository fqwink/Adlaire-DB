# Adlaire DB 仕様書

**バージョン：** 0.2  
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
| Turso Cloud | 提供機能の参照実装。API 互換を目指す |
| libSQL / sqld | フォーク元。Adlaire DB の全体基盤 |
| SQLite | libSQL 経由で互換性を維持（変更なし） |

### 1.3 固定制約

| 項目 | 内容 |
|------|------|
| 実装言語 | Rust + 標準ライブラリ |
| ストレージ・SQL 基盤 | libSQL フォーク（sqld 含む）|
| 目標機能 | Turso Cloud 機能パリティ |
| 将来方針 | libSQL 内部の段階的内製化（詳細は各フェーズで検討）|
| デプロイ形態 | シングルバイナリ起動 |
| 対象 OS | Linux |

---

## 2. Turso Cloud 機能パリティ

Adlaire DB が実現する機能の一覧。各機能の実装フェーズは §5 で定義する。

### 2.1 クライアント接続

| 機能 | 説明 |
|------|------|
| HTTP API | libSQL クライアント SDK が利用する HTTP/JSON API |
| WebSocket API | インタラクティブ・トランザクション用 WebSocket 接続 |
| 埋め込みレプリカ | クライアント側にローカルレプリカを持ち、リモートと同期する |
| libSQL クライアント互換 | 既存の Turso 向け libSQL クライアント（TypeScript・Rust・Go 等）がそのまま接続できる |

### 2.2 データベース管理

| 機能 | 説明 |
|------|------|
| マルチDB | 1インスタンス上に複数のデータベースを作成・管理 |
| DB 作成・削除 | 管理 API 経由での DB ライフサイクル管理 |
| ブランチ | データベースのブランチ作成（読み取り専用コピー等）|
| ポイントインタイムリストア | 任意の時点への DB 復元 |

### 2.3 レプリケーション

| 機能 | 説明 |
|------|------|
| プライマリ・レプリカ構成 | 書き込みはプライマリ、読み取りはレプリカへ分散 |
| レプリカへの自動同期 | WAL ベースのレプリカ同期 |

### 2.4 認証・アクセス制御

| 機能 | 説明 |
|------|------|
| トークン認証 | JWT ベースの認証トークン |
| DB 単位のアクセス制御 | DB ごとに読み書き権限を管理 |

---

## 3. アーキテクチャ

### 3.1 全体構成

```
libSQL クライアント SDK（TypeScript / Rust / Go 等）
  または curl / WebSocket クライアント
        │
        │ HTTP（JSON）/ WebSocket
        ▼
┌─────────────────────────────────────┐
│          Adlaire サーバー層          │
│                                     │
│  ・HTTP API（Turso 互換）           │
│  ・WebSocket API                    │
│  ・認証（JWT トークン）             │
│  ・マルチDB ルーティング            │
│  ・管理 API                         │
│  ・レプリケーション管理（Phase N）  │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│        libSQL フォーク（sqld）       │
│                                     │
│  ・SQL パーサ・クエリ実行           │
│  ・WAL 管理                         │
│  ・ページストレージ（SQLite 互換）  │
│  ・埋め込みレプリカプロトコル       │
└─────────────────────────────────────┘
```

### 3.2 libSQL フォークの利用方針

- libSQL（sqld）を Adlaire DB 専用にフォークし、必要な改変を加える
- フォーク元：`tursodatabase/libsql`（sqld を含む）
- Phase 1 では libSQL の機能をほぼそのまま活用し、サーバー層の構築に集中する
- 内部コンポーネントの内製化は、各フェーズの完了後に次フェーズとして計画・判断する

### 3.3 データディレクトリ構成

```
adlaire_data/
├── .lock                       # プロセス排他ロック
├── databases/
│   ├── {db-name}/              # DB ごとのディレクトリ
│   │   ├── data.db             # SQLite 互換 DB ファイル（libSQL 管理）
│   │   └── data.db-wal         # WAL ファイル（libSQL 管理）
│   └── ...
└── meta/
    ├── tokens.json             # 認証トークン管理
    └── databases.json          # DB メタデータ（名前・作成日時等）
```

---

## 4. API 仕様

### 4.1 HTTP API（Turso 互換）

libSQL クライアント SDK が期待するエンドポイントを実装する。

```
POST /v2/pipeline
  Body: { "requests": [{ "type": "execute", "stmt": { "sql": "...", "args": [...] } }] }
  Response: { "results": [...] }

GET  /v2/databases          # マルチDB 用（Phase 2）
POST /v2/databases          # DB 作成（Phase 2）
DELETE /v2/databases/{name} # DB 削除（Phase 2）
```

### 4.2 WebSocket API（Turso 互換）

`wss://{host}/v3/baton`

libSQL クライアントの hrana プロトコル（Turso の WebSocket プロトコル）に準拠する。

### 4.3 管理 API

[TBD] — DB 作成・削除・トークン発行・レプリカ管理などの運用 API。

---

## 5. 実装フェーズ

フェーズ単位で機能を積み上げる。各フェーズの詳細・内製化計画はフェーズ着手時に策定する。

### Phase 1：シングル DB・HTTP API（最小動作）

**目標**：libSQL クライアント SDK が Adlaire DB に接続して SQL を実行できる状態

- libSQL フォーク（sqld）のセットアップ・ビルド確認
- HTTP API（`/v2/pipeline`）の実装（Turso 互換）
- JWT トークン認証（最小限）
- シングルバイナリ起動（`./adlaire-db --data ./mydb --port 8080`）
- 既存 Turso 向けコード（TypeScript/Rust クライアント使用）が変更なしで動作することの確認

**完了条件**：libSQL クライアント SDK を Turso Cloud の代わりに Adlaire DB へ向けて、CREATE TABLE / INSERT / SELECT が動作する

### Phase 2：マルチDB・管理 API

**目標**：1インスタンスで複数 DB を管理できる

- DB 作成・削除・一覧 API
- DB ごとのルーティング
- DB ごとのトークン管理
- マルチテナント動作確認

**完了条件**：複数クライアントがそれぞれ独立した DB に接続して競合なく動作する

### Phase 3：WebSocket API・埋め込みレプリカ

**目標**：Turso のインタラクティブトランザクション・埋め込みレプリカが動作する

- hrana WebSocket プロトコル実装
- 埋め込みレプリカ同期プロトコルの対応
- バッチ・インタラクティブトランザクション

**完了条件**：libSQL TypeScript SDK の embedded replica 機能が Adlaire DB と動作する

### Phase 4：レプリケーション

**目標**：プライマリ・レプリカ構成での運用

- WAL ベースのレプリカ同期
- プライマリ障害時のフェイルオーバー（詳細 TBD）

**完了条件**：プライマリ + レプリカ構成でデータが同期される

### Phase 5 以降：内製化・高度機能

フェーズ 4 完了後に計画する。候補：

- ブランチ・ポイントインタイムリストア
- libSQL 内部コンポーネントの内製化（WAL・ページストレージ・SQL エンジン等）
- 高可用性・水平スケール

---

## 6. 配布・デプロイ

- シングルバイナリ（`adlaire-db`）として配布
- Linux x86_64 / aarch64（musl 静的リンクを検討）
- 配布チャネル：GitHub Releases（詳細 TBD）

---

## 付録：用語定義

| 用語 | 定義 |
|------|------|
| Turso Cloud | libSQL のマネージドホスティングサービス。Adlaire DB の機能パリティ参照先 |
| libSQL | SQLite フォーク。HTTP API・WAL レプリケーション等を追加した OSS DB ライブラリ |
| sqld | libSQL のサーバーコンポーネント。HTTP API・WebSocket API を提供する |
| libSQL フォーク | Adlaire DB 専用に改変した libSQL（sqld 含む）。本プロジェクトの全体基盤 |
| hrana | Turso / libSQL の WebSocket ワイヤプロトコル名 |
| 埋め込みレプリカ | クライアント側ローカルに SQLite DB を持ち、リモート libSQL DB と同期する仕組み |
