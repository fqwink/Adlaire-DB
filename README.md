# Adlaire DB

libSQL ワイヤプロトコル（hrana-http v2）互換のセルフホスト DB サーバー。
Rust + axum + libsql（embedded SQLite）で実装する。

## 現在の状態

**Phase 3 実装済み** — HTTP サーバー・hrana-http v2 パイプライン・シングル DB モードが動作する。

| フェーズ | 内容 | 状態 |
|----------|------|------|
| Phase 1 | Cargo ワークスペース・clap CLI 骨格 | ✅ 完了 |
| Phase 2 | データディレクトリ初期化・libsql 統合・プロセスロック | ✅ 完了 |
| Phase 3 | HTTP サーバー・hrana-http v2 パイプライン・シングル DB | ✅ 完了 |
| Phase 4 | JWT 認証・`token create` コマンド | 🔲 未実装 |
| Phase 5 | hrana WebSocket v3 | 🔲 未実装 |
| Phase 6 | マルチ DB URL ルーティング | 🔲 未実装 |
| Phase 7 | 管理 API（DB CRUD・トークン CRUD） | 🔲 未実装 |
| Phase 8 | WebSocket（hrana-ws v3） | 🔲 未実装 |
| Phase 10 | プライマリ・レプリカ構成 | 🔲 未実装 |
| Phase 12-13 | バックアップ・PITR | 🔲 未実装 |
| Phase 14 | ブランチ | 🔲 未実装 |

## 起動方法

```sh
# 認証なし（開発用）
adlaire-db serve --data ./data --port 8080

# JWT 認証は Phase 4 で実装予定。現在は --auth-jwt-secret を指定すると起動を拒否する。
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
| HTTP フレームワーク | `axum` 0.7 + `tokio` |
| シリアライゼーション | `serde_json` |
| ロギング | `tracing` + `tracing-subscriber`（JSON 形式）|
| CLI | `clap` 4（derive API）|
| 対象 OS | Linux |

## 制約（Phase 3 時点）

- JWT 認証は未実装（`--auth-jwt-secret` を指定すると起動拒否）
- 管理 API（`/admin/v1/...`）はすべて 501 を返す
- "default" データベースのみ利用可能（マルチ DB は Phase 6 以降）
- WebSocket（hrana-ws v3）は未対応

## 仕様書

詳細仕様は [`adlaire-db-spec.md`](./adlaire-db-spec.md) を参照。
