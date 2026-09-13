### Phase 13：WAL アーカイブ・manifest 管理

**目標**：WAL フレームのアーカイブと manifest.json による管理を実装する

**スコープ：**
- WAL アーカイブ書き込み（チェックポイント前フック）
- manifest.json による WAL フレーム管理
- `wal_retention_days` 設定によるアーカイブ保持期間の管理

**Phase 13 archive 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| frame file | `frame-{frame_no:012}.bin`。frame_no は manifest 内で unique、昇順 |
| snapshot file | `snapshot-{base_frame:012}.db`。最新 1 件を manifest の `snapshot` に記録 |
| manifest commit | frame file と snapshot file を fsync 後、最後に manifest を atomic rename |
| consistency check | 起動時に manifest の全 file 存在、size、checksum を検証。欠損・不一致は起動失敗 |
| retention | 削除対象 frame を manifest から外す前に削除 plan を作り、削除成功後に manifest commit |
| cleanup failure | file 削除失敗時は manifest を更新しない。WARN log 後、次回 cleanup で再試行 |
| disabled mode | `wal_retention_days = 0` では archive file を新規作成しない。既存 archive は削除しない |
| clock | retention 判定は manifest の `created_at` を使う。file mtime は使わない |

**完了条件（テストケース）：**

```
TC-5-8: 保持期間超過フレームのクリーンアップ
  設定: wal_retention_days = 1
  （a）2 日前のタイムスタンプを持つフレームを作成
  （b）クリーンアップ実行（または 24h 経過後）
  （c）該当フレームが削除され、manifest.json から除去されている
```

**Phase 13 実装タスク：**

```
T5-1: WAL フレームアーカイブ書き込み
  [ ] libsql チェックポイント前フックで WAL フレームを wal-archive/ へコピー
  [ ] フレームごとに CRC32 チェックサムを計算・付与
  [ ] manifest.json へフレームメタデータを追記
  参照: §3.2, §3.6.3, §6.4（PITR）

T5-2: スナップショット保存
  [ ] チェックポイント完了後に data.db を snapshot-{frame_no}.db へコピー
  [ ] スナップショットは最新 1 件のみ保持（古い snapshot ファイルを削除）
  [ ] manifest.json の base_frame / snapshot フィールドを更新
  参照: §3.6.3

T5-3: manifest.json 管理
  [ ] manifest.json の読み込み・書き込みロジック（アトミック更新）
  [ ] 整合性確認: frames[] と実ファイルの突合
  [ ] manifest.json 破損時の起動エラー処理
  参照: §3.2, §3.6.3, §3.6.6

T5-4: クリーンアップスレッド
  [ ] wal_retention_days 設定を config.toml から読み込み
  [ ] 24h ごとに manifest.json をスキャンし期限超過フレームを削除
  [ ] 削除後に manifest.json を更新
  参照: §4.2, §3.6.6
  検証: TC-5-8
```

**Phase 13 完全実装精度固定契約：**

Phase 13 は、Phase 11〜12 で生成・消費される WAL frame を、PITR / branch / restore の将来入力として安全に保存する Phase である。Phase 13 の完了判定は「archive file が作られる」だけではなく、manifest/files の双方向整合、fsync 境界、partial write recovery、retention cleanup、restart consistency、secret redaction、Phase 14 以降との境界がすべて証跡化されていることを必須とする。

| 項目 | 固定仕様 |
|------|----------|
| archive root | `{data-dir}/databases/{db}/wal-archive/` 固定。DB 名は管理下 DB 名のみ許可し path traversal を拒否する |
| manifest path | `{archive root}/manifest.json` 固定。archive 有効時は manifest が存在しなければ初期 manifest を atomic 作成する |
| manifest schema | `schema_version`、`db_name`、`created_at`、`updated_at`、`base_frame`、`snapshot`、`frames`、`retention_days` を必須にする |
| frame metadata | 各 frame は `frame_no`、`file`、`size`、`checksum_crc32`、`created_at`、`source` を必須にする |
| snapshot metadata | snapshot は `base_frame`、`file`、`size`、`checksum_crc32`、`created_at` を必須にする |
| frame filename | `frame-{frame_no:012}.bin` 固定。manifest 内の frame_no は unique かつ昇順でなければならない |
| snapshot filename | `snapshot-{base_frame:012}.db` 固定。manifest の `snapshot` は最新 1 件のみ参照する |
| write order | frame/snapshot temp 書き込み、file fsync、directory fsync、manifest tmp 書き込み、manifest fsync、rename、directory fsync の順で成功扱いにする |
| atomic manifest | manifest 更新は tmp file + fsync + atomic rename。直接上書きは禁止 |
| partial frame | manifest に未登録の frame file は orphan として扱う。起動時に WARN を出し、manifest には自動登録しない |
| missing frame | manifest にある frame file が存在しない場合は起動失敗。PITR 対象から黙って除外しない |
| corrupt frame | size または CRC32 不一致は起動失敗。checksum 再計算で上書き修復しない |
| duplicate frame | 同一 frame_no の複数 file、manifest 重複、番号逆行は起動失敗 |
| snapshot consistency | snapshot file 欠損、size/CRC32 不一致、`base_frame` 逆行は起動失敗 |
| retention clock | retention 判定は manifest の `created_at` を使う。file mtime は使わない |
| retention plan | cleanup 前に削除対象 frame/snapshot の plan を作る。削除成功後だけ manifest を commit する |
| retention safety | 現在 snapshot の `base_frame` より新しい frame、Phase 14 が必要とする範囲、処理中の frame は削除しない |
| cleanup failure | file 削除失敗時は manifest を更新せず WARN。次回 cleanup で同じ plan を再評価する |
| disabled mode | `wal_retention_days = 0` は新規 archive を作らない。既存 archive は削除・変更しない |
| config validation | `wal_retention_days` は 0 以上の整数のみ許可。parse 失敗、負数、範囲外は起動失敗 |
| startup check | archive 有効時は起動時に manifest parse、schema、files、size、CRC32、frame order を検査する |
| concurrency | archive write と cleanup は同一 DB 単位の archive lock で直列化する |
| observability | logs/metrics は db、frame_no、file count、checksum result、cleanup count まで。frame bytes、SQL args、token は出力禁止 |
| phase boundary | Phase 13 は archive 作成と retention まで。backup/restore/PITR/branch API は Phase 14〜15 対象 |

**Phase 13 原子タスク台帳：**

| Task ID | 目的 | 変更対象 | 完了条件 | 失敗時の扱い | 必須成果物 |
|---------|------|----------|----------|--------------|------------|
| `TASK-P13-1` | archive 設定を固定する | config / startup | `wal_retention_days` の default、parse、invalid、disabled mode が固定される | 不正値を default 扱いしない | config transcript |
| `TASK-P13-2` | archive directory を固定する | storage path | 管理下 DB の archive root だけを使い path traversal を拒否する | 任意 path 作成禁止 | path fixture |
| `TASK-P13-3` | manifest schema を固定する | `manifest.json` | 必須 field、schema_version、frame/snapshot metadata が検証される | unknown future schema を黙認しない | manifest fixture |
| `TASK-P13-4` | frame archive write を実装する | WAL archive | frame file、CRC32、size、fsync、manifest 追記が順序通り完了する | file だけ作って成功扱いしない | frame write trace |
| `TASK-P13-5` | snapshot archive を実装する | snapshot copy | 最新 snapshot 1 件を manifest に記録し、古い snapshot は cleanup 対象にする | snapshot 不整合を無視しない | snapshot trace |
| `TASK-P13-6` | manifest atomic commit を固定する | manifest writer | tmp write、fsync、rename、directory fsync が artifact で追える | direct overwrite 禁止 | atomic trace |
| `TASK-P13-7` | startup consistency check を固定する | startup validation | missing/corrupt/duplicate/order violation で起動失敗する | 自動修復禁止 | corrupt fixtures |
| `TASK-P13-8` | orphan file handling を固定する | startup cleanup | manifest 未登録 file は WARN のみ、自動登録しない | unknown file を PITR 対象にしない | orphan fixture |
| `TASK-P13-9` | retention cleanup を実装する | cleanup worker | created_at 基準で plan 作成、削除成功後 manifest commit | 削除失敗時 manifest 更新禁止 | cleanup transcript |
| `TASK-P13-10` | cleanup concurrency を固定する | archive lock | archive write と cleanup が同一 DB で直列化される | 同時更新による manifest 競合禁止 | concurrency artifact |
| `TASK-P13-11` | disabled mode を固定する | archive worker | `wal_retention_days = 0` で新規 archive なし、既存 archive 非破壊 | 既存 archive 削除禁止 | disabled transcript |
| `TASK-P13-12` | Phase 1〜12 regression を閉じる | tests | HTTP/WS、replication、health/redirect に差分なし | archive 実装で既存 wire を壊さない | regression report |

**Phase 13 シナリオマトリクス：**

| Scenario ID | Given | When | Then | Artifact |
|-------------|-------|------|------|----------|
| `SCN-P13-1` | `wal_retention_days = 0` | DB に write する | 新規 `wal-archive/` を作らない | `phase13_disabled_no_archive.json` |
| `SCN-P13-2` | `wal_retention_days = 7` | 初回 archive が走る | archive root と初期 manifest が atomic 作成される | `phase13_manifest_create.json` |
| `SCN-P13-3` | WAL frame が生成される | archive write する | `frame-{frame_no:012}.bin` と manifest entry が一致する | `phase13_frame_write.json` |
| `SCN-P13-4` | snapshot 対象 DB がある | snapshot 保存する | `snapshot-{base_frame:012}.db` が manifest の最新 snapshot になる | `phase13_snapshot.json` |
| `SCN-P13-5` | manifest tmp 書き込み後に中断 | 再起動する | 旧 manifest が有効で、tmp は成功扱いされない | `phase13_manifest_crash_before_rename.json` |
| `SCN-P13-6` | rename 後 directory sync 前に中断 | 再起動する | manifest/files 整合を検査し、判断不能なら起動失敗 | `phase13_manifest_crash_after_rename.json` |
| `SCN-P13-7` | manifest にある frame file が欠損 | 起動する | 起動失敗し、欠損を黙って除外しない | `phase13_missing_frame.json` |
| `SCN-P13-8` | frame file の CRC32 が不一致 | 起動する | 起動失敗し、checksum を上書き修復しない | `phase13_corrupt_frame.json` |
| `SCN-P13-9` | 同一 frame_no が重複 | 起動する | 起動失敗する | `phase13_duplicate_frame.json` |
| `SCN-P13-10` | manifest 未登録 file がある | 起動する | WARN し、manifest 自動登録はしない | `phase13_orphan_file.json` |
| `SCN-P13-11` | 2 日前の frame と retention 1 日 | cleanup する | 対象 file 削除後に manifest から除去される | `phase13_retention_cleanup.json` |
| `SCN-P13-12` | cleanup file 削除が失敗 | cleanup する | manifest は更新せず、次回 retry する | `phase13_cleanup_failure.json` |
| `SCN-P13-13` | 現 snapshot に必要な frame | cleanup する | 必要範囲は削除されない | `phase13_retention_safety.json` |
| `SCN-P13-14` | archive write 中 | cleanup が起動する | archive lock により manifest 更新が直列化される | `phase13_archive_lock.json` |
| `SCN-P13-15` | invalid `wal_retention_days` | 起動する | 起動失敗し default に戻さない | `phase13_invalid_config.json` |
| `SCN-P13-16` | path traversal DB 名 | archive path を作る | 管理下 DB 以外へ書かない | `phase13_path_traversal.json` |
| `SCN-P13-17` | logs/artifacts がある | secret scan を実行する | token、SQL args、frame bytes が残らない | `phase13_secret_scan.json` |
| `SCN-P13-18` | Phase 12 replica 構成 | Phase 13 実装後に regression する | catch-up、redirect、health に差分なし | `phase13_phase12_regression.json` |
| `SCN-P13-19` | backup/restore/PITR API を呼ぶ | Phase 13 状態で実行する | Phase 14 まで未提供のまま、archive だけでは成功しない | `phase13_phase14_boundary.json` |
| `SCN-P13-20` | restart 後 | consistency check を実行する | manifest/files が一致する場合のみ起動成功 | `phase13_restart_consistency.json` |

**Phase 13 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| manifest を直接上書きする | merge 不可 |
| frame/snapshot file の fsync 前に manifest を成功 commit する | merge 不可 |
| manifest にない orphan file を自動で正式 archive に昇格する | merge 不可 |
| manifest にある missing/corrupt frame を黙って除外する | merge 不可 |
| checksum 不一致を再計算して manifest を上書き修復する | merge 不可 |
| file mtime を retention 判定に使う | review failure |
| cleanup file 削除失敗後に manifest だけ更新する | merge 不可 |
| `wal_retention_days = 0` で既存 archive を削除する | merge 不可 |
| archive write と cleanup を lock なしで同時実行する | merge 不可 |
| token、SQL args、frame bytes、backup body を log/artifact に出す | merge 不可 |
| Phase 13 で backup/restore/PITR/branch API を完成扱いにする | merge 不可 |
| Phase 1〜12 の wire response を archive 実装の都合で変更する | merge 不可 |

**Phase 13 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | WAL archive write、snapshot archive、manifest schema、atomic commit、startup check、retention cleanup、disabled mode |
| `excluded_scope` | backup API、restore API、PITR replay、branch、external storage transfer、HA、内製 WAL engine |
| `atomic_task_result` | `TASK-P13-1`〜`TASK-P13-12` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P13-1`〜`SCN-P13-20` の pass/fail、artifact path |
| `manifest_result` | schema、frame/snapshot metadata、atomic write、directory fsync、restart consistency の検証結果 |
| `archive_write_result` | frame write、snapshot write、CRC32、size、ordering、partial write recovery の trace |
| `retention_result` | cleanup plan、created_at 判定、delete success、delete failure retry、safety range の検証結果 |
| `corruption_result` | missing file、corrupt file、duplicate frame、future schema、orphan file の fixture 結果 |
| `config_result` | default 0、valid days、invalid parse、negative、disabled mode、secret redaction の matrix |
| `concurrency_result` | archive write と cleanup の DB 単位 lock、同時更新拒否、manifest conflict なしの artifact |
| `compatibility_baseline_result` | Phase 1〜12 regression、TypeScript SDK HTTP/WS transcript、Phase 12 replication transcript 差分なし |
| `secret_redaction_result` | response/log/artifact に token、JWT、SQL args、frame bytes、backup body が残らない scan |
| `review_handoff_result` | 第三者が archive write、restart check、corruption、retention cleanup、disabled mode を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 13 で WAL archive と retention が有効化されるが backup/restore/PITR/branch API は未提供である release note |
| `precision_closure_result` | manifest schema、archive write、snapshot archive、retention cleanup、corruption detection、disabled mode、Phase 1〜12 regression の artifact path、reviewer 再現 command、`P13-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

---


#### 実装詳細

#### 14.8 WAL アーカイブ処理（Phase 13〜14）

チェックポイント前フックで WAL フレームを `wal-archive/` へコピーし、`manifest.json` をアトミックに更新する。

```rust
// wal/archive.rs
use crc32fast::Hasher as Crc32Hasher;
use tokio::io::AsyncWriteExt;

pub async fn archive_frames(
    db_name:    &str,
    data_dir:   &std::path::Path,
    new_frames: &[WalFrame],
) -> anyhow::Result<()> {
    let archive_dir   = data_dir.join("databases").join(db_name).join("wal-archive");
    let manifest_path = archive_dir.join("manifest.json");

    tokio::fs::create_dir_all(&archive_dir).await?;
    let mut manifest = Manifest::load(&manifest_path).await.unwrap_or_default();

    for frame in new_frames {
        // CRC32 計算
        let mut h = Crc32Hasher::new();
        h.update(&frame.data);
        let checksum = h.finalize();

        let filename  = format!("frame-{:012}.bin", frame.frame_no);
        let frame_path = archive_dir.join(&filename);

        // 書き込み + fsync（I-4 保証）
        let mut f = tokio::fs::File::create(&frame_path).await?;
        f.write_all(&frame.data).await?;
        f.sync_all().await?;

        manifest.frames.push(FrameMeta {
            frame_no:   frame.frame_no,
            file:       filename,
            size:       frame.data.len() as u64,
            checksum,
            created_at: chrono::Utc::now(),
        });
    }

    // manifest をアトミック更新（tmp → rename）
    manifest.save_atomic(&manifest_path).await?;
    Ok(())
}
```

```rust
// wal/manifest.rs
use tokio::io::AsyncWriteExt;

impl Manifest {
    /// manifest.json を読み込む。ファイルが存在しない場合は Err を返す（呼び出し側で unwrap_or_default）
    pub async fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let data = tokio::fs::read(path).await?;
        Ok(serde_json::from_slice(&data)?)
    }

    /// manifest.json をアトミック更新（tmp → fsync → rename、I-4 保証）
    pub async fn save_atomic(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self)?;
        {
            let mut f = tokio::fs::File::create(&tmp).await?;
            f.write_all(&json).await?;
            f.sync_all().await?;
        }
        tokio::fs::rename(&tmp, path).await?;
        Ok(())
    }
}
```
