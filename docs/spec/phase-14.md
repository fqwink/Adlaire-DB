### Phase 14：バックアップ・リストア・PITR API

**目標**：WAL アーカイブからのオンラインバックアップと任意時点リストア（PITR）が動作する

**スコープ：**
- バックアップ API：`GET /admin/v1/databases/{name}/backup`
- リストア API：`POST /admin/v1/databases/{name}/restore`
- PITR API：`POST /admin/v1/databases/{name}/restore/point-in-time`

**Phase 14 restore 固定契約：**

Phase 14 の backup、restore、PITR、rollback、restart recovery、redaction は §9.1.30 を正とする。下表は Phase 14 固有の endpoint 入口であり、§9.1.30 と衝突する場合は同じ PR で解消してから実装する。

| 項目 | 固定仕様 |
|------|----------|
| backup response | `Content-Type: application/octet-stream`、`Content-Disposition: attachment; filename="{db}.db"` |
| backup consistency | SQLite online backup 相当で整合 snapshot を返す。backup 中 write はブロックしない |
| upload limit | restore body は設定値 `restore_max_bytes` が未定義の間、DB 既存 size の 2 倍または 1 GiB の小さい方を上限 |
| temp layout | `{data-dir}/databases/{name}/restore-{request_id}/` に upload、verified、old を分けて置く |
| restore lock | 対象 DB 単位で exclusive lock。restore 中の write は 503 `STORAGE_BUSY`、read は既存 DB で継続可 |
| protection | `delete_protection=true` は `403 ORG_SCOPE_DENIED`、`block_writes=true` は `403 PERMISSION_DENIED`。restore 後 size が quota 超過なら commit 前に `QUOTA_EXCEEDED` |
| verification | restore/PITR は `PRAGMA integrity_check` が `ok` の場合だけ commit |
| commit | runtime DB close、old へ退避、new を `data.db` へ rename、directory fsync、DB reopen の順 |
| rollback | commit 前後のどの失敗でも old を戻す。戻せない場合は `restore-failed.json` marker を残し起動失敗 |
| PITR selector | request は `{timestamp}` または `{frame_no}` のどちらか 1 つだけ。両方・どちらもなしは `INVALID_REQUEST` |
| PITR replay | snapshot の `base_frame` から target frame まで checksum 検証しながら適用。欠損は `FRAME_NOT_FOUND` |
| success response | restore / PITR 成功は `204 No Content`。成功時に JSON body は返さない |

**完了条件（テストケース）：**

```
TC-5-1: バックアップと同時書き込み
  （a）GET /admin/v1/databases/{name}/backup を開始（大きな DB でストリーミング）
  （b）バックアップ中に POST /{name}/v2/pipeline で INSERT を実行
  （c）バックアップは整合性を保って完了し、書き込みリクエストも 200 で成功

TC-5-2: バックアップからリストア
  （a）GET /admin/v1/databases/{name}/backup でバックアップファイルを取得
  （b）POST /admin/v1/databases/{name}/restore でリストア → 204 body なし
  （c）リストア後 SELECT → 元のデータが参照できる

TC-5-3: 不正ファイルでリストア
  （a）POST /admin/v1/databases/{name}/restore（不正な SQLite ファイルをアップロード）
  期待: 409 RESTORE_INTEGRITY_FAILED

TC-5-4: PITR 無効時の試行
  設定: wal_retention_days = 0（または未設定）
  （a）POST /admin/v1/databases/{name}/restore/point-in-time
  期待: 503 PITR_NOT_ENABLED

TC-5-5: タイムスタンプ指定 PITR
  （a）t=T1 に INSERT A、t=T2 に INSERT B
  （b）POST .../restore/point-in-time {"timestamp": "T1+1s"}
  （c）SELECT → A が存在し B が存在しない

TC-5-6: アーカイブ範囲外タイムスタンプ
  （a）POST .../restore/point-in-time（アーカイブに存在しない timestamp）
  期待: 404 FRAME_NOT_FOUND

TC-5-7: CRC32 不一致フレームで PITR
  （a）フレームファイルを手動で破壊
  （b）POST .../restore/point-in-time
  期待: 409 RESTORE_FRAME_CORRUPT、元 DB が復元されている
```

**対象外（Phase 15 以降）：**
- ブランチ機能（Phase 15）
- 外部ストレージへのアーカイブ転送

**Phase 14 実装タスク：**

```
T5-5: バックアップ API
  [ ] GET /admin/v1/databases/{name}/backup → sqlite3_backup_* API でオンラインバックアップ
  [ ] バックアップ中の書き込みをブロックしない（Online Backup API の並行性保証）
  [ ] レスポンス: SQLite ファイルをストリーミング送信（Content-Type: application/octet-stream）
  参照: §6.4（バックアップ / PITR / ブランチ）
  検証: TC-5-1, TC-5-2

T5-6: リストア API
  [ ] POST /admin/v1/databases/{name}/restore → アップロードされた SQLite ファイルを適用
  [ ] PRAGMA integrity_check でファイル整合性検証
  [ ] 検証失敗時: 元 DB を復元し 409 RESTORE_INTEGRITY_FAILED を返す
  [ ] 成功時: libsql::Builder::new_local() で DB を再オープンしてサービス再開
  参照: §6.4, §7.3
  検証: TC-5-2, TC-5-3

T5-7: PITR API
  [ ] POST /admin/v1/databases/{name}/restore/point-in-time（timestamp / frame_no 指定）
  [ ] wal_retention_days = 0 の場合は即座に 503 PITR_NOT_ENABLED
  [ ] manifest.json から対象フレームを特定
  [ ] スナップショット + WAL フレームリプレイ処理を実装
  [ ] CRC32 検証失敗時: 元 DB 復元 + 409 RESTORE_FRAME_CORRUPT
  参照: §6.4, §7.3, §3.6.3
  検証: TC-5-4〜TC-5-7

T5-8: エラーハンドリング・冪等性
  [ ] 各 API エラーコードを §7.3 の定義に沿って実装
  [ ] リストア・PITR の中断時に元 DB を必ず復元すること（ロールバック保証）
  [ ] 管理 API への Admin JWT 検証をバックアップ・リストアエンドポイントにも適用
  参照: §7.3, §10

T5-9: 統合テスト
  [ ] TC-5-1〜TC-5-8 を全て実行し PASS することを確認
  [ ] Phase 1〜11 の TC がリグレッションしないことを確認
```

**Phase 14 完全実装精度固定契約：**

Phase 14 は、Phase 13 の WAL archive / manifest を入力として backup、restore、PITR を公開 API として完成させる Phase である。Phase 14 の完了判定は、正常系 API が動くことではなく、破壊的操作である restore/PITR が途中失敗、timeout、shutdown、retry、quota/block、corrupt archive、restart recovery を含めて元 DB を守れることまで証跡化されていることを必須とする。

| 項目 | 固定仕様 |
|------|----------|
| endpoint scope | `GET /admin/v1/databases/{name}/backup`、`POST /admin/v1/databases/{name}/restore`、`POST /admin/v1/databases/{name}/restore/point-in-time` の 3 API のみ |
| auth | Admin token 必須。auth failure は lock、body parse、quota、file IO より先に評価する |
| db scope | 管理下 DB 名のみ対象。存在しない DB は 404 `DB_NOT_FOUND`、path traversal は 400 `INVALID_REQUEST` |
| backup status | 成功は 200。`Content-Type: application/octet-stream`、`Content-Disposition: attachment; filename="{name}.db"` を返す |
| backup body | SQLite database file の byte stream のみ。JSON wrapper、metadata footer、secret は混ぜない |
| backup consistency | SQLite Online Backup API 相当の一貫 snapshot を返す。backup 中 write は許可し、backup artifact には開始時点の整合状態を固定する |
| backup lock | backup は source DB の read snapshot を確保する。restore/PITR 中の backup は 503 `STORAGE_BUSY` |
| backup permission | `block_reads=true` は backup を 403 `PERMISSION_DENIED`。quota 超過中でも backup は許可する |
| restore content type | `Content-Type: application/octet-stream` のみ受理。その他は 400 `INVALID_REQUEST` |
| restore upload limit | `restore_max_bytes` 未定義時は `min(existing_db_size * 2, 1 GiB)`。超過は 413 `PAYLOAD_TOO_LARGE` |
| restore temp layout | `{data-dir}/databases/{name}/restore-{request_id}/upload.db`、`verified.db`、`old/`、`new/`、`commit.json` を使う |
| restore lock | 対象 DB 単位の exclusive lock。restore/PITR 中 write は 503 `STORAGE_BUSY`、read は旧 DB で継続する |
| restore verification | upload DB は `PRAGMA integrity_check` が `ok` の場合だけ commit 候補にする |
| restore commit order | request body fsync、verified DB fsync、old DB 退避、new DB rename、directory fsync、commit marker fsync、DB reopen の順 |
| restore success | commit marker と DB reopen が成功した後だけ 204 No Content。成功 body は空 |
| restore rollback | commit 前後の失敗、timeout、shutdown では old DB を戻す。戻せた場合は failure error を返し、成功扱いにしない |
| restore failed marker | rollback 不能なら `{data-dir}/databases/{name}/restore-failed.json` を atomic 作成し、次回起動を失敗させる |
| startup recovery | 起動時に restore temp / marker を検査し、commit marker なしの temp は rollback または cleanup。`restore-failed.json` は自動修復禁止 |
| PITR request | JSON body は `{timestamp}` または `{frame_no}` のどちらか 1 つのみ。両方、どちらもなし、unknown field は 400 `INVALID_REQUEST` |
| PITR disabled | `wal_retention_days = 0`、manifest 欠損、archive disabled は 503 `PITR_NOT_ENABLED` |
| PITR target | `frame_no` は manifest 内の frame_no に一致必須。`timestamp` は manifest `created_at` で target frame を決める |
| PITR range | target に必要な snapshot または frame が欠損する場合は 404 `FRAME_NOT_FOUND` |
| PITR replay | snapshot を temp DB に copy し、base frame から target frame まで frame_no 昇順、CRC32 検証後に replay する |
| PITR corruption | frame size/CRC32 不一致、snapshot 不一致、replay integrity failure は 409 `RESTORE_FRAME_CORRUPT` |
| PITR commit | restore と同じ lock、temp layout、verification、commit order、rollback、failed marker を使う |
| quota precedence | restore/PITR 後の DB size を commit 前に推定し、quota 超過なら 403 `QUOTA_EXCEEDED`。旧 DB は維持する |
| block precedence | `delete_protection=true` は 403 `ORG_SCOPE_DENIED`、`block_writes=true` は 403 `PERMISSION_DENIED`。quota より先に判定する |
| idempotency | 同一 request retry で二重 restore、二重 quota charge、temp 衝突を起こしてはならない。`request_id` ごとに temp を分離する |
| timeout/shutdown | commit 完了前に timeout/shutdown した場合は成功応答禁止。rollback 成功または `restore-failed.json` のどちらかに収束する |
| observability | logs/metrics は db、request_id、operation、byte length、checksum、commit/rollback result まで。backup body、uploaded DB bytes、token、SQL args は出力禁止 |
| phase boundary | Phase 14 は backup/restore/PITR まで。branch 作成、branch seed、external storage transfer、HA failover は対象外 |

**Phase 14 原子タスク台帳：**

| Task ID | 目的 | 変更対象 | 完了条件 | 失敗時の扱い | 必須成果物 |
|---------|------|----------|----------|--------------|------------|
| `TASK-P14-1` | backup endpoint contract を固定する | admin backup route | 200、headers、octet-stream、body no wrapper が一致する | JSON wrapper 返却禁止 | backup transcript |
| `TASK-P14-2` | backup consistency を固定する | backup engine | concurrent write 中も開始時点の整合 snapshot を返す | inconsistent backup 禁止 | backup consistency artifact |
| `TASK-P14-3` | restore request validation を固定する | restore route | auth、DB 名、content-type、body size、unknown condition の precedence が固定される | validation 順序の曖昧化禁止 | validation matrix |
| `TASK-P14-4` | restore temp layout を固定する | restore storage | upload/verified/old/new/commit marker が request_id 別に作られる | target DB 直接上書き禁止 | temp layout fixture |
| `TASK-P14-5` | restore verification を固定する | restore verifier | `PRAGMA integrity_check = ok` のみ commit 候補 | 不正 DB の部分適用禁止 | integrity artifact |
| `TASK-P14-6` | restore commit/rollback を固定する | restore transaction | commit order、fsync、reopen、rollback、failed marker が証跡化される | rollback 不能を隠さない | rollback trace |
| `TASK-P14-7` | startup recovery を固定する | startup recovery | temp/marker 検出、rollback、cleanup、operator_required が再現できる | 自動上書き修復禁止 | recovery fixture |
| `TASK-P14-8` | PITR selector を固定する | PITR route | timestamp/frame_no 排他、unknown field、invalid type を 400 にする | selector 推測禁止 | selector matrix |
| `TASK-P14-9` | PITR replay を固定する | PITR engine | manifest 範囲、snapshot、frame order、CRC32、integrity_check が通る | corrupt frame を適用しない | PITR replay artifact |
| `TASK-P14-10` | quota/block/protection を固定する | policy layer | delete_protection、block_writes、quota、block_reads backup の precedence が固定される | commit 後 quota 判定禁止 | policy matrix |
| `TASK-P14-11` | timeout/shutdown/idempotency を固定する | operation control | retry、timeout、shutdown が success-before-commit を起こさない | 二重 restore 禁止 | operation artifact |
| `TASK-P14-12` | Phase 1〜13 regression を閉じる | tests | HTTP/WS、replication、archive manifest に差分なし | restore 実装で既存 contract を壊さない | regression report |

**Phase 14 シナリオマトリクス：**

| Scenario ID | Given | When | Then | Artifact |
|-------------|-------|------|------|----------|
| `SCN-P14-1` | valid Admin token と DB | backup を実行する | 200 octet-stream、Content-Disposition、SQLite backup を返す | `phase14_backup_success.json` |
| `SCN-P14-2` | backup streaming 中 | 同じ DB に write する | write は成功し、backup は整合 snapshot のまま完了する | `phase14_backup_concurrent_write.json` |
| `SCN-P14-3` | `block_reads=true` | backup を実行する | 403 `PERMISSION_DENIED` | `phase14_backup_block_reads.json` |
| `SCN-P14-4` | restore/PITR lock 中 | backup を実行する | 503 `STORAGE_BUSY` | `phase14_backup_storage_busy.json` |
| `SCN-P14-5` | valid SQLite backup | restore を実行する | 204 body empty、DB reopen 後 SELECT 可能 | `phase14_restore_success.json` |
| `SCN-P14-6` | non-octet-stream body | restore を実行する | 400 `INVALID_REQUEST` | `phase14_restore_content_type.json` |
| `SCN-P14-7` | upload limit 超過 | restore を実行する | 413 `PAYLOAD_TOO_LARGE`、旧 DB 維持 | `phase14_restore_too_large.json` |
| `SCN-P14-8` | corrupt SQLite file | restore を実行する | 409 `RESTORE_INTEGRITY_FAILED`、旧 DB 維持 | `phase14_restore_integrity_failed.json` |
| `SCN-P14-9` | commit 前に failure injection | restore を実行する | rollback し、旧 DB で起動できる | `phase14_restore_rollback_before_commit.json` |
| `SCN-P14-10` | commit 後 reopen 前に failure injection | restore を実行する | rollback または `restore-failed.json` に収束する | `phase14_restore_rollback_after_commit.json` |
| `SCN-P14-11` | rollback 不能状態 | restore を実行する | `restore-failed.json` を作り、次回起動失敗 | `phase14_restore_failed_marker.json` |
| `SCN-P14-12` | restore temp が残っている | 再起動する | commit marker の有無で recovery/cleanup が deterministic に動く | `phase14_startup_recovery.json` |
| `SCN-P14-13` | `delete_protection=true` | restore/PITR を実行する | 403 `ORG_SCOPE_DENIED`、旧 DB 維持 | `phase14_delete_protection.json` |
| `SCN-P14-14` | `block_writes=true` | restore/PITR を実行する | 403 `PERMISSION_DENIED`、旧 DB 維持 | `phase14_block_writes.json` |
| `SCN-P14-15` | restore 後 size が quota 超過 | restore/PITR を実行する | 403 `QUOTA_EXCEEDED`、commit しない | `phase14_quota_exceeded.json` |
| `SCN-P14-16` | `wal_retention_days = 0` | PITR を実行する | 503 `PITR_NOT_ENABLED` | `phase14_pitr_disabled.json` |
| `SCN-P14-17` | `{timestamp, frame_no}` 両方指定 | PITR を実行する | 400 `INVALID_REQUEST` | `phase14_pitr_selector_invalid.json` |
| `SCN-P14-18` | archive 範囲外 target | PITR を実行する | 404 `FRAME_NOT_FOUND`、旧 DB 維持 | `phase14_pitr_frame_not_found.json` |
| `SCN-P14-19` | frame CRC32 不一致 | PITR を実行する | 409 `RESTORE_FRAME_CORRUPT`、旧 DB 維持 | `phase14_pitr_frame_corrupt.json` |
| `SCN-P14-20` | timestamp target T1 | PITR を実行する | T1 時点のデータだけ復元される | `phase14_pitr_timestamp_success.json` |
| `SCN-P14-21` | frame_no target | PITR を実行する | 指定 frame まで復元される | `phase14_pitr_frame_success.json` |
| `SCN-P14-22` | restore request timeout | 操作中に timeout する | 204 を返さず、rollback または marker に収束する | `phase14_restore_timeout.json` |
| `SCN-P14-23` | 同一 request retry | restore を再送する | 二重適用せず deterministic な結果になる | `phase14_restore_retry.json` |
| `SCN-P14-24` | logs/artifacts がある | secret scan を実行する | token、SQL args、backup/uploaded DB bytes が残らない | `phase14_secret_scan.json` |
| `SCN-P14-25` | Phase 13 archive | Phase 14 実装後に regression する | manifest/files/retention に差分なし | `phase14_phase13_regression.json` |
| `SCN-P14-26` | branch API を呼ぶ | Phase 14 状態で実行する | Phase 15 まで未提供のまま | `phase14_phase15_boundary.json` |

**Phase 14 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| restore/PITR で target DB を直接上書きする | merge 不可 |
| commit marker / fsync / directory sync 前に 204 を返す | merge 不可 |
| rollback 不能を通常エラーとして隠し、次回起動を許可する | merge 不可 |
| `restore-failed.json` を自動削除または自動修復する | merge 不可 |
| corrupt backup / corrupt frame を部分適用する | merge 不可 |
| PITR selector を推測して timestamp/frame_no の片方へ自動補正する | review failure |
| quota/block/delete_protection を commit 後に判定する | merge 不可 |
| restore/PITR 中の write を成功させる | merge 不可 |
| backup body、uploaded DB bytes、token、SQL args を log/artifact に出す | merge 不可 |
| success response に JSON body を返す | review failure |
| Phase 14 で branch 作成、branch seed、外部 storage 転送、HA failover を完成扱いにする | merge 不可 |
| Phase 1〜13 の wire response、archive manifest、replication behavior を restore 実装の都合で変更する | merge 不可 |

**Phase 14 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | backup API、restore API、PITR API、restore lock、temp layout、commit/rollback、startup recovery、quota/block/auth precedence |
| `excluded_scope` | branch、branch seed、external storage transfer、HA failover、restore job async API、内製 WAL engine |
| `atomic_task_result` | `TASK-P14-1`〜`TASK-P14-12` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P14-1`〜`SCN-P14-26` の pass/fail、artifact path |
| `api_result` | backup/restore/PITR の status、headers、body、error code、unknown field、content-type の transcript |
| `backup_result` | online backup consistency、concurrent write、block_reads、storage busy、streaming redaction の検証結果 |
| `restore_result` | temp layout、integrity_check、commit order、fsync、DB reopen、rollback、failed marker の trace |
| `pitr_result` | selector、manifest range、snapshot、frame replay、CRC32、FRAME_NOT_FOUND、RESTORE_FRAME_CORRUPT の検証結果 |
| `policy_result` | auth、delete_protection、block_reads、block_writes、quota、DB not found、path traversal の precedence matrix |
| `operation_result` | timeout、shutdown、retry、idempotency、exclusive lock、read/write behavior の artifact |
| `startup_recovery_result` | temp cleanup、commit marker、rollback marker、restore-failed 起動失敗、operator action の fixture |
| `compatibility_baseline_result` | Phase 1〜13 regression、TypeScript SDK HTTP/WS transcript、Phase 13 archive transcript 差分なし |
| `secret_redaction_result` | response/log/artifact に token、JWT、SQL args、backup body、uploaded DB bytes が残らない scan |
| `review_handoff_result` | 第三者が backup、restore rollback、PITR success/corrupt/range outside、startup recovery を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 14 で backup/restore/PITR が公開され、失敗時は rollback または restore-failed marker に収束する release note |
| `precision_closure_result` | backup consistency、restore rollback、PITR replay、startup recovery、quota/block/auth precedence、Phase 1〜13 regression の artifact path、reviewer 再現 command、`P14-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

---
