# Adlaire DB

libSQL ワイヤプロトコル（hrana-http v2 / hrana-ws v3）互換のセルフホスト DB サーバー。
Rust + libsql（embedded SQLite）で実装する。外部 Web フレームワークは使用しない。

## 現在の状態

**Phase 7 実装済み** — HTTP サーバー・hrana-http v2 パイプライン・JWT 認証・ログ・マルチ DB ルーター・管理 API が動作する。

| フェーズ | 内容 | 状態 |
|----------|------|------|
| Phase 1 | Cargo ワークスペース・clap CLI 骨格 | ✅ 完了 |
| Phase 2 | データディレクトリ初期化・libsql 統合・プロセスロック | ✅ 完了 |
| Phase 3 | HTTP サーバー・hrana-http v2 パイプライン・シングル DB | ✅ 完了 |
| Phase 4 | JWT 認証・`token create` コマンド | ✅ 完了 |
| Phase 5 | ログ・統合テスト | ✅ 完了 |
| Phase 6 | マルチ DB ルーター | ✅ 完了 |
| Phase 7 | 管理 API・トークン CRUD・DB スコープ JWT | ✅ 完了 |
| Phase 8 | Turso Cloud 互換管理モデル | 🔲 未実装 |
| Phase 9 | WebSocket（hrana-ws v3） | 🔲 未実装 |
| Phase 10 | ATTACH DB・メトリクス | 🔲 未実装 |
| Phase 11 | レプリケーション基盤（WAL ストリーム・スナップショット） | 🔲 未実装 |
| Phase 12 | レプリカ同期・書き込みリダイレクト | 🔲 未実装 |
| Phase 13 | WAL アーカイブ・manifest 管理 | 🔲 未実装 |
| Phase 14 | バックアップ・リストア・PITR | 🔲 未実装 |
| Phase 15 | ブランチ | 🔲 未実装 |
| Phase 16 | SQLite 拡張・内製化・HA | 🔲 未実装 |

## 起動方法

```sh
# 認証なし（開発用）
adlaire-db serve --data ./data --port 8080

# JWT 認証あり
adlaire-db serve --data ./data --port 8080 --auth-jwt-secret "change-me"
```

## 動作確認

```sh
# ヘルスチェック
curl http://localhost:8080/v2/health
# → {"status":"ok"}

# SQL 実行
curl -s -X POST http://localhost:8080/v2/pipeline \
  -H "Content-Type: application/json" \
  -d '{"requests":[{"type":"execute","stmt":{"sql":"SELECT 1","want_rows":true}}]}' | jq .
```

## Docker 検証

ローカルに Rust toolchain がない環境では、Docker でテストを実行する。

```sh
make test-docker
```

直接実行する場合:

```sh
./scripts/test-docker.sh
```

実行内容:

```sh
cargo test --workspace --locked
```

フォーマット確認も Docker で実行できる。

```sh
make fmt-docker
```

Cargo registry / git / target は Docker volume に保存されるため、2回目以降の実行は初回より速くなる。

## libSQL クライアント SDK からの接続

```typescript
import { createClient } from "@libsql/client";
const db = createClient({ url: "http://localhost:8080" });
const result = await db.execute("SELECT 1");
```

## 技術スタック

| 項目 | 内容 |
|------|------|
| 実装言語 | Rust 2021 edition |
| ストレージ基盤 | `libsql` crate（embedded SQLite / WAL モード）|
| HTTP サーバー | `hyper` v1（直接使用 / フレームワーク不使用）+ `tokio` |
| シリアライゼーション | `serde_json` |
| ロギング | `tracing` + `tracing-subscriber`（JSON 形式）|
| CLI | `clap` 4（derive API）|
| 対象 OS | Linux |

## 管理 API

Phase 7 時点では、以下の管理 API が利用できる。

- `GET /admin/v1/databases`
- `POST /admin/v1/databases`
- `GET /admin/v1/databases/{name}`
- `DELETE /admin/v1/databases/{name}`
- `GET /admin/v1/tokens`
- `POST /admin/v1/tokens`
- `GET /admin/v1/tokens/{id}`
- `DELETE /admin/v1/tokens/{id}`

## 制約（Phase 7 時点）

- Turso Cloud 互換管理モデル（location / organization / group / quota）は Phase 8 以降
- WebSocket（hrana-ws v3）は未対応（`GET /v3/baton` は Phase 9 で実装、現在 501 を返す）
- ATTACH DB・メトリクスは Phase 10 以降
- レプリケーション、バックアップ、ブランチ、HA は Phase 11 以降

## 仕様書

詳細仕様は [`adlaire-db-spec.md`](./adlaire-db-spec.md) を参照。
