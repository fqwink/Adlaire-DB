# Adlaire DB

Turso Cloud 互換のセルフホスト DB サーバー。libSQL フォーク（sqld）を基盤として Rust で実装する。

## 概要

Adlaire DB は [Turso Cloud](https://turso.tech) が提供する機能をセルフホストで再現する。既存の libSQL クライアント SDK（TypeScript・Rust・Go）から接続 URL を差し替えるだけで動作する。

## 特徴

- **libSQL クライアント SDK 互換** — Turso Cloud の URL を Adlaire DB の URL に置き換えるだけで動作
- **シングルバイナリ** — `./adlaire-db serve --data ./data` 一コマンドで起動
- **hrana-http v2 / hrana-ws v3** — Turso Cloud と同じワイヤプロトコル
- **マルチ DB** — URL パスでデータベースを切り替え（`/{db-name}/v2/pipeline`）
- **JWT 認証** — HS256 Bearer トークン、DB スコープ、失効管理
- **WAL レプリケーション** — プライマリ・レプリカ構成（Phase 4）
- **Backup / PITR / ブランチ** — WAL アーカイブからの任意時点復元（Phase 5・6）

## 実装フェーズ

| フェーズ | 内容 | 状態 |
|----------|------|------|
| Phase 1 | HTTP API（hrana-http v2）・JWT 認証・シングル DB | 設計中 |
| Phase 2 | マルチ DB・トークン管理・管理 API | 設計中 |
| Phase 3 | WebSocket API・埋め込みレプリカ・ATTACH DATABASE・メトリクス | 設計中 |
| Phase 4 | プライマリ・レプリカ構成・WAL レプリケーション | 設計中 |
| Phase 5 | オンラインバックアップ・PITR | 設計中 |
| Phase 6 | ブランチ | 設計中 |

## 起動方法（Phase 1 予定）

```sh
adlaire-db serve \
  --data /var/lib/adlaire \
  --port 8080 \
  --admin-port 8081 \
  --auth-jwt-secret <32バイト以上のシークレット>
```

## 接続例

```typescript
import { createClient } from "@libsql/client";

const db = createClient({
  url: "http://localhost:8080",
  authToken: "<JWT>",
});

const result = await db.execute("SELECT 1");
```

## 技術スタック

| 項目 | 内容 |
|------|------|
| 実装言語 | Rust |
| ストレージ基盤 | libSQL フォーク（sqld） |
| HTTP フレームワーク | axum + tokio |
| WebSocket | tokio-tungstenite |
| 対象 OS | Linux (x86_64 / aarch64) |

## 仕様書

詳細仕様は [`adlaire-db-spec.md`](./adlaire-db-spec.md) を参照。
