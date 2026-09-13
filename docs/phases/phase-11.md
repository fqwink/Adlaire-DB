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
