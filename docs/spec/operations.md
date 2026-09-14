# 運用・データ保全仕様

WAL、整合性、リカバリ、ファイル保護、セキュリティ、ログ、運用用語を固定する仕様である。

## 7. データ保全責務

WAL、整合性、リカバリ、ファイル保護、データ破壊防止を固定する責務である。データ保全に関する挙動は互換性よりも強い安全側判断を許容するが、差分は仕様へ明示する。

### 7.1 WAL 設定責務

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

## 8. 運用責務

運用時のセキュリティ、デプロイ、ログ、TLS、管理ポート、証跡を固定する責務である。運用者が再現できない手順、保存されない証跡、秘匿値が残るログは完了条件として扱わない。

### 8.1 セキュリティ責務

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

### 8.2 配布・デプロイ責務

- シングルバイナリ（`adlaire-db`）として配布
- ターゲット：Linux x86_64 / aarch64
- 静的リンク（musl）によるランタイム依存ゼロは必須条件にしない。配布物は release-check で glibc 依存、動的ライブラリ依存、対象アーキテクチャを明示し、musl 対応は専用リリース仕様が追加されるまで実装対象外とする
- 配布チャネル：GitHub Releases
- リリース成果物には SHA-256 チェックサムを添付する

---

### 8.3 ログ責務

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

### 8.4 用語責務

| 用語 | 定義 |
|------|------|
| Turso Cloud | libSQL のマネージドホスティングサービス。Adlaire DB の hrana プロトコル互換の参照実装 |
| libSQL | SQLite フォーク。HTTP API・WAL レプリケーション等を追加した OSS DB ライブラリ |
| sqld | libSQL プロジェクトのサーバーコンポーネント（参考情報）。Adlaire DB は sqld を使用せず、libsql crate（embedded モード）を使用する |
| libsql crate | Rust の libSQL クライアントライブラリ（crates.io）。embedded SQLite モードで使用する |
| hrana | Turso / libSQL のワイヤプロトコル名。hrana-http（HTTP版）と hrana-ws（WebSocket版）がある |
| baton | hrana プロトコルにおけるセッション継続識別子 |

---

## 9. 実装統制責務

Phase 実装判断、共通完了条件、DoR / DoD、テスト、PRレビュー、証跡、バグ修正ゼロ化を固定する責務である。実装者は「動く」ことではなく、仕様契約を証跡で満たすことを完了根拠にする。
