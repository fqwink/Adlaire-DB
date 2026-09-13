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
