# Adlaire DB

Turso Cloud 機能互換のセルフホスト NoSQL DB サーバー。libSQL フォーク（sqld）を基盤として Rust で実装する。

## 概要

Adlaire DB は [Turso Cloud](https://turso.tech) が提供する機能をセルフホストで再現する NoSQL ドキュメント DB サーバーである。REST/JSON API と SSE を採用し、SQL エンジンを持たない。

## 特徴

- **REST/JSON API** — コレクション + ドキュメント（JSON）操作
- **シングルバイナリ** — `./adlaire-db serve --data ./data` 一コマンドで起動
- **SSE レプリケーション** — プライマリ → レプリカへの seq ベース変更ストリーミング
- **ACID トランザクション** — begin/execute/commit/rollback
- **マルチ DB** — URL パスでデータベースを切り替え（`/{db}/collections/{col}/...`）
- **JWT 認証** — HS256 Bearer トークン、DB スコープ、失効管理
- **WAL レプリケーション** — プライマリ・レプリカ構成（Phase 4）
- **Backup / PITR / ブランチ** — WAL アーカイブからの任意時点復元（Phase 5・6）

## 実装フェーズ

| フェーズ | 内容 | 状態 |
|----------|------|------|
| Phase 1 | REST/JSON API・JWT 認証・シングル DB | 設計中 |
| Phase 2 | マルチ DB・トークン管理・管理 API | 設計中 |
| Phase 3 | SSE・ATTACH・メトリクス | 設計中 |
| Phase 4 | プライマリ・レプリカ構成・WAL レプリケーション | 設計中 |
| Phase 5 | オンラインバックアップ・PITR | 設計中 |
| Phase 6 | ブランチ | 設計中 |
| Phase 7 | 拡張機能・HA | 計画中 |

## 起動方法（Phase 1 予定）

```sh
adlaire-db serve \
  --data /var/lib/adlaire \
  --port 8080 \
  --admin-port 8081 \
  --auth-jwt-secret <32バイト以上のシークレット>
```

## 接続例

```sh
# ドキュメント挿入
curl -X POST http://localhost:8080/mydb/collections/users/insert \
  -H "Authorization: Bearer <JWT>" \
  -H "Content-Type: application/json" \
  -d '{"documents": [{"_id": "u1", "name": "Alice"}]}'

# ドキュメント検索
curl -X POST http://localhost:8080/mydb/collections/users/find \
  -H "Authorization: Bearer <JWT>" \
  -H "Content-Type: application/json" \
  -d '{"filter": {"name": {"$eq": "Alice"}}}'
```

## 技術スタック

| 項目 | 内容 |
|------|------|
| 実装言語 | Rust |
| ストレージ基盤 | libSQL フォーク（sqld） |
| HTTP フレームワーク | axum + tokio |
| ストリーミング | SSE（Server-Sent Events） |
| 対象 OS | Linux (x86_64 / aarch64) |

## 仕様書

詳細仕様は [`adlaire-db-spec.md`](./adlaire-db-spec.md) を参照。
