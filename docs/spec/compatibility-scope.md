# 互換性・スコープ仕様

本ファイルは、Adlaire DB の互換性、境界、機能スコープを固定する仕様である。実行基盤、運用、実装統制、Phase 契約、実装詳細は分割後の各仕様正本を参照する。

プロジェクト目的、価値、優先順位、将来方針は `docs/PROJECT_CHARTER.md` を正とする。内製化の実装契約は `docs/spec/internalization.md` を正とする。

## 2. 互換性責務

Adlaire DB は Turso Cloud / libSQL SDK / hrana 互換を最優先の外部契約として扱う。互換性に影響する変更は、実装前に差分、理由、SDK 影響、後方互換性、代替仕様を仕様へ固定する。自己ホスト都合の差分は、security または durability 上必要な場合に限り、仕様上の差分として明示する。

## 3. 境界責務

提供する機能、提供しない機能、管理境界、DB境界、セキュリティ境界を固定する責務である。実装者は境界外の機能を成功応答にしてはならない。

### 3.1 機能スコープ責務

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
