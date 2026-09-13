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

**Phase 12 完全実装精度固定契約：**

Phase 12 は、Phase 11 の primary replication API を replica が消費し、read replica として再起動・遅延・primary down・checksum mismatch を含む運用境界で破綻しない状態まで固定する Phase である。Phase 12 の完了判定は「replica が追いつく」だけではなく、replica state の永続化、再開位置、redirect の wire surface、health の劣化条件、secret redaction、Phase 13 以降との境界がすべて再現可能であることを必須とする。

| 項目 | 固定仕様 |
|------|----------|
| phase input | Phase 11 の `/replication/v1/log`、`/replication/v1/snapshot`、`/replication/v1/heartbeat`、`/replication/v1/status` が完成済みであること |
| phase output | replica role が起動し、snapshot 初期化、WAL catch-up、write redirect、health lag、restart resume が仕様通り動くこと |
| state file path | `{data-dir}/meta/replica-state.json` 固定。tmp file + fsync + atomic rename で更新する |
| state schema | `schema_version`、`replica_id`、`primary_url`、`last_applied_frame`、`last_seen_primary_frame`、`last_successful_fetch_at`、`last_error_code`、`updated_at` を必須にする |
| state corruption | JSON parse 失敗、必須 field 欠落、future `schema_version`、frame 逆行は起動失敗。空 state のみ初回同期として扱う |
| initial sync | local DB が空で `last_applied_frame = 0` の場合だけ snapshot を取得して初期化し、snapshot base frame から WAL を取得する |
| resume sync | 起動時は `last_applied_frame + 1` を `from_frame` として WAL を取得する。既に適用済み frame の再適用は禁止 |
| apply transaction | 1 frame 適用、local commit、state commit の順で成功した場合だけ `last_applied_frame` を進める |
| gap handling | 期待 frame より大きい frame を受信した場合、後続 frame を破棄し、同じ `from_frame` から再取得する |
| stale frame | `last_applied_frame` 以下の frame は idempotent duplicate として INFO log のみで破棄する |
| checksum mismatch | mismatch frame は local DB に適用せず破棄し、ERROR log、同じ `from_frame` から再取得する。3 回連続で health `degraded` |
| auth failure | replication token 不一致、未指定、期限切れは retry せず health `degraded`、write は primary 到達不能扱いで 503 |
| primary unavailable | fetch timeout、connect error、5xx は read を local replica で継続し、write は 503 `REPLICATION_TIMEOUT` |
| redirect target | replica write は primary が到達可能で leader/primary URL が有効な場合だけ 307。`Location` は primary URL + 元 path/query |
| redirect body | 307 response body は空。error JSON を混ぜない。SDK が redirect follow できる status/header を優先する |
| redirect methods | `POST`、`PUT`、`PATCH`、`DELETE` と restore、branch create、extension load は mutating request として扱う |
| read allowlist | `GET /v2/health`、read-only SQL、replication status 参照は replica local read として許可する |
| write SQL detection | `POST /v2/pipeline` 内の SQL は `SELECT` 以外を write 扱いにする。判定不能 SQL は write として扱う |
| health schema | Phase 12 以降の `GET /v2/health` は `status`、`role`、`primary_url`、`last_applied_frame`、`last_seen_primary_frame`、`lag_frames` を返す |
| degraded condition | primary unreachable、auth failure、checksum mismatch 3 回連続、lag threshold 超過、state write failure は `degraded` |
| lag calculation | `lag_frames = max(last_seen_primary_frame - last_applied_frame, 0)`。unknown primary frame は `lag_frames = null` |
| sync mode | `write_mode = sync` は quorum/ACK 条件が明示設定された場合のみ起動可。未設定なら起動失敗 |
| multi replica | replica ごとに `replica_id` を必須化し、state と heartbeat summary を混線させない |
| observability | logs/metrics は frame_no、replica_id、lag_frames、error_code まで。SQL、token、frame bytes は出力禁止 |
| excluded scope | WAL archive、PITR、backup restore API、branch、HA promotion/demotion、leader election、内製 WAL engine は Phase 12 対象外 |

**Phase 12 原子タスク台帳：**

| Task ID | 目的 | 変更対象 | 完了条件 | 失敗時の扱い | 必須成果物 |
|---------|------|----------|----------|--------------|------------|
| `TASK-P12-1` | replica role 起動契約を固定する | config / startup | `--role replica` と `--primary-url` と replication token が必須。不足時は起動失敗 | default standalone に黙って戻さない | startup config transcript |
| `TASK-P12-2` | replica state schema を実装する | `{data-dir}/meta/replica-state.json` | 必須 field、atomic rename、corruption 起動失敗が再現できる | state 破損を初期同期扱いしない | state fixture |
| `TASK-P12-3` | snapshot 初期同期を固定する | replica sync loop | 空 DB のみ snapshot を取得し、base frame 以降の WAL に進む | 既存 DB を snapshot で上書きしない | initial sync transcript |
| `TASK-P12-4` | WAL catch-up を実装する | replica apply | `last_applied_frame + 1` から frame 昇順適用、gap は再取得 | 欠番後続 frame を適用しない | frame apply log |
| `TASK-P12-5` | checksum mismatch recovery を固定する | frame verify | mismatch frame 未適用、同一 `from_frame` 再取得、3 回で degraded | frame skip 成功扱い禁止 | mismatch artifact |
| `TASK-P12-6` | primary down behavior を固定する | sync loop / router | read は local 許可、write は 503 `REPLICATION_TIMEOUT` | local write に fallback しない | outage transcript |
| `TASK-P12-7` | write redirect を固定する | redirect middleware | mutating request は 307 + Location、body 空 | 302/308/JSON error に変えない | redirect transcript |
| `TASK-P12-8` | SQL write 判定を固定する | hrana-http / pipeline | read-only SQL は local read、write/判定不能は redirect または 503 | 判定不能 SQL を local 実行しない | SQL matrix |
| `TASK-P12-9` | health lag を固定する | `GET /v2/health` | role、primary_url、frame、lag、status が返る | Phase 3 `{status:"ok"}` だけに戻さない | health snapshots |
| `TASK-P12-10` | sync write 設定を固定する | config / primary write | quorum/ACK 未定義なら起動失敗 | async と同じ挙動にしない | invalid config transcript |
| `TASK-P12-11` | multi replica を固定する | replica identity / heartbeat | 2 replica が別 `replica_id` と別 state で追従する | state/heartbeat 混線禁止 | multi replica transcript |
| `TASK-P12-12` | Phase 1〜11 regression を閉じる | tests | 既存 HTTP/WS、auth、quota、Phase 11 replication API に差分なし | replica 実装で既存 API を壊さない | regression report |

**Phase 12 シナリオマトリクス：**

| Scenario ID | Given | When | Then | Artifact |
|-------------|-------|------|------|----------|
| `SCN-P12-1` | primary に既存 DB がある | 空 replica を起動する | snapshot 取得後、base frame から WAL catch-up する | `phase12_initial_snapshot.json` |
| `SCN-P12-2` | replica が `last_applied_frame = 10` | 再起動する | `from_frame = 11` で取得再開する | `phase12_resume.json` |
| `SCN-P12-3` | primary に frame 11〜15 がある | replica が catch-up する | frame_no 昇順で 15 まで適用する | `phase12_apply_order.json` |
| `SCN-P12-4` | frame 13 が欠番 | replica が frame 14 を受ける | frame 14 を適用せず 13 から再取得する | `phase12_gap.json` |
| `SCN-P12-5` | checksum 不一致 frame がある | replica が受信する | 未適用、ERROR log、同一 `from_frame` 再取得 | `phase12_checksum_mismatch.json` |
| `SCN-P12-6` | checksum mismatch が 3 回連続 | health を取得する | `status = degraded`、`last_error_code` が残る | `phase12_degraded_checksum.json` |
| `SCN-P12-7` | primary が停止 | replica に SELECT を送る | local replica の既存データを 200 で返す | `phase12_primary_down_read.json` |
| `SCN-P12-8` | primary が停止 | replica に INSERT を送る | 503 `REPLICATION_TIMEOUT` を返す | `phase12_primary_down_write.json` |
| `SCN-P12-9` | primary が到達可能 | replica に write request を送る | 307、`Location` は primary の同一 path/query、body 空 | `phase12_redirect.json` |
| `SCN-P12-10` | replica に `/v2/pipeline` SELECT | request を送る | local read として成功する | `phase12_pipeline_read.json` |
| `SCN-P12-11` | replica に `/v2/pipeline` INSERT | request を送る | redirect または primary down 時 503 になる | `phase12_pipeline_write.json` |
| `SCN-P12-12` | SQL 判定不能 | request を送る | write 扱いで local 実行しない | `phase12_unknown_sql.json` |
| `SCN-P12-13` | token 不一致 | replica が fetch する | retry せず `degraded`、secret は log に出ない | `phase12_auth_failure.json` |
| `SCN-P12-14` | replica state が破損 | replica を起動する | 起動失敗し、初期同期に戻さない | `phase12_state_corrupt.json` |
| `SCN-P12-15` | replica state 書き込み失敗 | frame apply 後に state commit する | health `degraded`、frame 進行を成功扱いしない | `phase12_state_write_failure.json` |
| `SCN-P12-16` | 2 replica 構成 | primary に write する | 両 replica が同じ data へ追いつき、state は独立 | `phase12_multi_replica.json` |
| `SCN-P12-17` | lag threshold 超過 | health を取得する | `lag_frames` と `degraded` が返る | `phase12_lag_degraded.json` |
| `SCN-P12-18` | `write_mode = sync` かつ quorum 未設定 | 起動する | 起動失敗し、async fallback しない | `phase12_sync_invalid.json` |
| `SCN-P12-19` | Phase 11 primary API | Phase 12 実装後に再実行する | `/log`、`/snapshot`、`/heartbeat`、`/status` の互換差分なし | `phase12_phase11_regression.json` |
| `SCN-P12-20` | logs/artifacts がある | secret scan を実行する | token、SQL args、frame bytes が残らない | `phase12_secret_scan.json` |

**Phase 12 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| replica への write を local DB に直接適用する | merge 不可 |
| primary down 時に write を成功扱いにする | merge 不可 |
| checksum mismatch frame を skip して catch-up 成功扱いにする | merge 不可 |
| state 破損を空 state とみなして snapshot で上書きする | merge 不可 |
| gap 検出後の後続 frame を適用する | merge 不可 |
| redirect status を 302、303、308、200 JSON に変更する | review failure |
| 307 redirect body に error JSON を混ぜる | review failure |
| replication token、SQL args、frame bytes を log/artifact に出す | merge 不可 |
| `write_mode = sync` の quorum 未定義を async fallback で起動する | merge 不可 |
| Phase 12 に WAL archive、PITR、branch、HA promotion、leader election を混ぜる | merge 不可 |
| Phase 1〜11 の wire response を replica 実装の都合で変更する | merge 不可 |

**Phase 12 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | replica role、state persistence、initial snapshot、WAL catch-up、redirect、primary down、health lag、multi replica |
| `excluded_scope` | WAL archive、PITR、backup restore API、branch、HA、leader election、promotion/demotion、内製化 |
| `atomic_task_result` | `TASK-P12-1`〜`TASK-P12-12` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P12-1`〜`SCN-P12-20` の pass/fail、artifact path |
| `state_result` | state schema、atomic update、resume、corruption failure、state write failure の検証結果 |
| `apply_result` | snapshot 初期化、WAL apply order、gap、duplicate、checksum mismatch、re-fetch の検証結果 |
| `redirect_result` | mutating endpoint matrix、307 status、Location、body 空、primary down 503 の transcript |
| `health_result` | ok/degraded、role、primary_url、last_applied_frame、last_seen_primary_frame、lag_frames の snapshot |
| `auth_result` | valid token、不一致、未指定、期限切れ、secret redaction の matrix |
| `multi_replica_result` | replica_id 独立、state 独立、両 replica catch-up、heartbeat summary の検証結果 |
| `sync_mode_result` | quorum 設定あり/なし、ACK timeout、起動失敗、async fallback 禁止の検証結果 |
| `compatibility_baseline_result` | Phase 1〜11 regression、TypeScript SDK HTTP/WS transcript、Phase 11 replication transcript 差分なし |
| `secret_redaction_result` | response/log/artifact に replication token、JWT、SQL args、frame bytes が残らない scan |
| `review_handoff_result` | 第三者が primary + 2 replica、primary down、restart、redirect、checksum mismatch を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 12 で replica read/catch-up/redirect が利用可能になるが archive/PITR/branch/HA は未提供である release note |
| `precision_closure_result` | replica state、snapshot bootstrap、WAL catch-up、redirect、primary down、checksum mismatch、multi replica、Phase 1〜11 regression の artifact path、reviewer 再現 command、`P12-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

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
