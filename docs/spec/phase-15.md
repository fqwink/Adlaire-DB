### Phase 15：ブランチ

**目標**：DB の任意時点からブランチを作成し、独立した DB として読み書き可能にする

**スコープ：**
- ブランチ DB 作成 API（`from: "current"` / `from: {timestamp}` / `from: {frame_no}`）
- ブランチ一覧・削除 API
- ブランチ DB 命名規則と予約名バリデーション（`___`）
- 再起動後のブランチ DB 自動復元

**Phase 15 branch 固定契約：**

Phase 15 の branch create、delete、routing、Turso seed、restart recovery、isolation は §9.1.31 を正とする。下表は Phase 15 固有の入口条件であり、§9.1.31 と衝突する場合は同じ PR で解消してから実装する。

| 項目 | 固定仕様 |
|------|----------|
| branch name | `^[a-z0-9-]{1,64}$`。source DB と同じ名前、`___`、`admin`、`meta` は拒否 |
| internal DB name | `{source_db}___{branch_name}`。通常 DB create API から `___` を含む名前は常に拒否 |
| source selector | `"current"`、`{"timestamp":"..."}`、`{"frame_no":N}` のいずれか 1 つだけ |
| create order | branch directory 作成、DB file 構築、integrity_check、DbManager 登録、最後に `branches.json` commit |
| delete order | runtime map から外し、connection close、directory rename to trash、`branches.json` commit、trash 削除 |
| partial create | `branches.json` にない branch directory は起動時 WARN + cleanup。cleanup 失敗でも active 扱いしない |
| partial delete | `branches.json` にない branch directory は接続不可。cleanup 対象 |
| source delete | active branch がある source DB の削除は `403 ORG_SCOPE_DENIED`。cascade delete は Phase 15 対象外 |
| isolation | branch write は source DB に反映しない。source write は既存 branch に反映しない |
| Turso seed | `/v1/organizations/{org}/databases` の `seed.type:"database"` は Phase 15 で branch create に昇格してよい。ただし branch 名に相当する field がない request は `INVALID_REQUEST` |
| protection | source DB `delete_protection=true` でも branch create は許可する。source DB `block_reads=true` の branch create は `403 PERMISSION_DENIED` |
| quota | branch DB は source の database quota を継承する。branch 作成で organization/group quota を超える場合は `QUOTA_EXCEEDED` |
| token scope | source DB token は branch DB へ自動拡張しない。branch 用 token は別途発行する |

**完了条件（テストケース）：**

```
TC-6-1: current から新規ブランチ作成
  （a）POST /admin/v1/databases/my-db/branches {"branch_name":"feature-x","from":"current"} → 201
  （b）GET /my-db___feature-x/v2/pipeline SELECT → 元 DB と同じデータが返る
  （c）GET /admin/v1/databases/my-db/branches → feature-x が含まれる

TC-6-2: ブランチ DB への独立書き込み
  （a）ブランチ DB に INSERT A
  （b）元 DB を SELECT → A が存在しない
  （c）ブランチ DB を SELECT → A が存在する

TC-6-3: タイムスタンプ指定でブランチ作成
  （a）t=T1 に my-db へ INSERT A、t=T2 に INSERT B
  （b）POST .../branches {"branch_name":"snap","from":"T1+1s"} → 201
  （c）ブランチ DB を SELECT → A が存在し B が存在しない

TC-6-4: ブランチ一覧取得
  （a）feature-x・snap の 2 ブランチを作成
  （b）GET /admin/v1/databases/my-db/branches → 両ブランチが含まれる

TC-6-5: ブランチ削除
  （a）DELETE /admin/v1/databases/my-db/branches/feature-x → 204
  （b）GET /admin/v1/databases/my-db/branches → feature-x が含まれない
  （c）/my-db___feature-x/v2/pipeline → 404 DB_NOT_FOUND
  （d）{data-dir}/databases/my-db___feature-x/ ディレクトリが削除されている

TC-6-6: 予約名バリデーション
  （a）POST /admin/v1/databases {"name":"a___b"} → 400 DB_RESERVED_NAME

TC-6-7: 再起動後のブランチ自動復元
  （a）feature-x ブランチを作成
  （b）サーバーを再起動（同じ --data）
  （c）/my-db___feature-x/v2/pipeline SELECT → データが復元されている
```

**対象外（Phase 16 以降）：**
- ブランチのマージ
- ブランチ間 diff

**Phase 15 実装タスク一覧：**

```
T6-1: branches.json 読み書きロジック
  [ ] {data-dir}/meta/branches.json の読み込み・書き込み（アトミック更新）
  [ ] 起動時に branches.json を読み込み（§8.1 Step 5-3）
  [ ] branches.json がない場合は空で初期化
  参照: §3.2, §8.1 Step 5-3

T6-2: ブランチ DB 命名・バリデーション
  [ ] `___` を含む DB 名を予約名として判定
  [ ] POST /admin/v1/databases で `___` 含む名前 → 400 DB_RESERVED_NAME
  [ ] ブランチ DB 内部名の生成: {source}___{branch-name}
  参照: §7.3
  検証: TC-6-6

T6-3: `from: "current"` ブランチ作成
  [ ] sqlite3_backup_* API で source DB のオンラインスナップショットを取得
  [ ] {data-dir}/databases/{name}___{branch}/ ディレクトリ作成
  [ ] スナップショットを data.db として配置
  [ ] branches.json へメタデータを追加
  [ ] 新 DB を libsql::Builder::new_local() でオープン・マルチ DB マネージャへ登録
  参照: §6.4（ブランチ）
  検証: TC-6-1, TC-6-2

T6-4: `from: {timestamp}/{frame_no}` ブランチ作成
  [ ] wal_retention_days = 0 の場合は 503 PITR_NOT_ENABLED
  [ ] T5-7 の PITR ロジックを再利用してブランチ DB を構築
  [ ] 構築先: {data-dir}/databases/{name}___{branch}/data.db
  [ ] branches.json へメタデータを追加（from_frame を記録）
  参照: §6.4, T5-7
  検証: TC-6-3

T6-5: ブランチ一覧・削除 API
  [ ] GET /admin/v1/databases/{name}/branches → branches.json からフィルタして返す
  [ ] DELETE /admin/v1/databases/{name}/branches/{branch-name}
        → Arc<libsql::Database> drop・ディレクトリ削除・branches.json 更新
  参照: §6.4（ブランチ）
  検証: TC-6-4, TC-6-5

T6-6: 起動時ブランチ自動復元ロジック
  [ ] branches.json を読み込み、各 db_name の DB ディレクトリが存在すれば libsql::Builder::new_local() でオープン
  [ ] ディレクトリが存在しないエントリは WARN ログを出力してスキップ
  参照: §8.1 Step 5-3
  検証: TC-6-7

T6-7: 統合テスト
  [ ] TC-6-1〜TC-6-7 を全て実行し PASS することを確認
  [ ] Phase 1〜14 の TC がリグレッションしないことを確認
```

**Phase 15 完全実装精度固定契約：**

Phase 15 は、Phase 14 の backup/PITR 基盤を使い、source DB から独立した branch DB resource を作成・一覧・削除・再起動復元できる状態にする Phase である。Phase 15 の完了判定は branch API の正常系だけではなく、`branches.json`、branch DB directory、runtime map、routing、token scope、quota、source selector、Turso seed 互換、restart recovery が相互に矛盾しないことを必須とする。

| 項目 | 固定仕様 |
|------|----------|
| endpoint scope | `POST /admin/v1/databases/{source}/branches`、`GET /admin/v1/databases/{source}/branches`、`DELETE /admin/v1/databases/{source}/branches/{branch}` の 3 API のみ |
| auth | Admin token 必須。auth failure は source lookup、selector parse、quota、file IO より先に評価する |
| branch name | `^[a-z0-9-]{1,64}$` 固定。自動 lowercase、trim、normalize はしない |
| reserved name | source DB と同名、`admin`、`meta`、`.`、`..`、`___` を含む name は 400 `DB_RESERVED_NAME` または `INVALID_DB_NAME` |
| internal DB name | `{source_db}___{branch_name}` 固定。通常 DB create API は `___` を含む name を常に 400 `DB_RESERVED_NAME` |
| metadata path | `{data-dir}/meta/branches.json` 固定。Phase 15 以降は parse 不能、future schema、duplicate key で起動失敗 |
| metadata schema | `schema_version`、`branches[]` を必須。各 branch は `source_db`、`branch_name`、`db_name`、`from`、`from_frame`、`created_at`、`state` を持つ |
| metadata key | `source_db + branch_name` を logical key とする。`db_name` だけを primary key にしない |
| list response | `created_at` 昇順、同値は `source_db`、`branch_name` 昇順。filesystem path は返さない |
| create selector | `"current"`、`{"timestamp":"..."}`、`{"frame_no":N}` のいずれか 1 つだけ。両方指定、unknown field、型不一致は 400 `INVALID_REQUEST` |
| current create | Phase 14 backup と同等の online snapshot を temp branch DB に作る。source write はブロックしない |
| timestamp/frame create | Phase 14 PITR replay と同じ selector、manifest、CRC32、integrity_check を使い temp branch DB を作る |
| create order | branch temp dir 作成、DB file 構築、fsync、integrity_check、runtime open、route dry-run、`branches.json` atomic commit、success response の順 |
| create success | `branches.json` に `active` として commit し、runtime map と DB directory が一致した後だけ 201 を返す |
| duplicate create | 同じ `source_db + branch_name` は 409 `DB_ALREADY_EXISTS`。retry で二重 DB、二重 quota charge を起こさない |
| partial create | metadata にない branch dir は起動時 WARN + cleanup。cleanup 失敗でも active 扱いしない |
| active missing dir | `branches.json` active entry があるのに DB directory / data.db がない場合は起動失敗 |
| route contract | `/{source}___{branch}/v2/pipeline` は active metadata、DB directory、runtime map が揃う場合だけ許可する |
| isolation | branch write は source に反映しない。source write は既存 branch に反映しない |
| token scope | source DB token は branch DB へ自動拡張しない。branch DB 用 token は branch db_name に対して別途発行する |
| org/group/location | branch は source の organization、group、location を継承する。Phase 15 では変更 API を提供しない |
| quota | branch 作成時に organization/group quota と candidate branch size を commit 前に判定する。database quota は source の値を初期値として copy |
| source block_reads | branch create は 403 `PERMISSION_DENIED` |
| source block_writes | branch create は source read snapshot のため許可する。ただし branch DB 側の quota/block は判定する |
| delete_protection | source の `delete_protection=true` は branch create を禁止しない。branch 自身の delete_protection は branch delete を 403 `ORG_SCOPE_DENIED` |
| source delete | active branch がある source DB の削除は 403 `ORG_SCOPE_DENIED`。cascade delete と orphan 化は禁止 |
| delete order | branch lock、新規 route 拒否、runtime map から除去、connection close、directory trash rename、`branches.json` commit、trash cleanup の順 |
| delete idempotency | metadata と directory がどちらも存在しない branch delete のみ 204。metadata 破損、directory のみ残存は silent success 禁止 |
| delete recovery | metadata deleting + directory exists は起動時 delete recovery を実行し、完了まで branch route は 404 `DB_NOT_FOUND` |
| Turso seed | `/v1/organizations/{org}/databases` の `seed.type:"database"` は Phase 15 で branch create に接続可。ただし source/branch field 未確定 request は 400 `INVALID_REQUEST` |
| observability | logs/metrics は source_db、branch_name、db_name、operation、state、quota result まで。absolute path、token、SQL args、raw request body は出力禁止 |
| phase boundary | Phase 15 は branch create/list/delete/routing/restart まで。branch merge、branch diff、copy-on-write 最適化、extension、external storage transfer は対象外 |

**Phase 15 原子タスク台帳：**

| Task ID | 目的 | 変更対象 | 完了条件 | 失敗時の扱い | 必須成果物 |
|---------|------|----------|----------|--------------|------------|
| `TASK-P15-1` | branch name / internal DB name を固定する | validation / router | slug、reserved、`___`、internal db_name が仕様通り | 自動補正禁止 | validation transcript |
| `TASK-P15-2` | `branches.json` schema を固定する | metadata | schema_version、logical key、atomic update、corruption failure が再現できる | duplicate key 先勝ち禁止 | metadata fixture |
| `TASK-P15-3` | branch create current を固定する | branch create | online snapshot、integrity_check、runtime open、metadata commit が順序通り | metadata 先行 active 禁止 | current create trace |
| `TASK-P15-4` | branch create PITR を固定する | PITR helper | timestamp/frame selector、manifest range、CRC32、integrity_check が Phase 14 と一致 | corrupt frame 適用禁止 | PITR branch trace |
| `TASK-P15-5` | branch routing を固定する | router / DbManager | active metadata + directory + runtime map が揃う時だけ pipeline 成功 | dir だけ存在を active 扱いしない | routing matrix |
| `TASK-P15-6` | branch isolation を固定する | DB engine | source write、branch write、backup、restore の相互非反映を検証する | alias DB 扱い禁止 | isolation matrix |
| `TASK-P15-7` | branch list を固定する | admin branches | response schema、sort、path redaction が固定される | filesystem path 返却禁止 | list snapshot |
| `TASK-P15-8` | branch delete を固定する | delete flow | lock、route reject、runtime close、trash rename、metadata commit、cleanup が順序通り | metadata だけ削除禁止 | delete trace |
| `TASK-P15-9` | restart recovery を固定する | startup recovery | active/deleting/partial create/delete/corrupt metadata の挙動が決まる | silent success 禁止 | recovery fixture |
| `TASK-P15-10` | policy / quota / token scope を固定する | policy layer | source delete denial、block_reads、delete_protection、quota、token isolation が固定される | source token 自動拡張禁止 | policy matrix |
| `TASK-P15-11` | Turso seed compatibility を固定する | `/v1/*` platform API | seed.type database の許可/拒否、required fields、response 差分が固定される | branch 名なし作成禁止 | seed compatibility snapshot |
| `TASK-P15-12` | Phase 1〜14 regression を閉じる | tests | HTTP/WS、backup/restore/PITR、archive、replication に差分なし | branch 実装で既存 contract を壊さない | regression report |

**Phase 15 シナリオマトリクス：**

| Scenario ID | Given | When | Then | Artifact |
|-------------|-------|------|------|----------|
| `SCN-P15-1` | valid source DB | `from:"current"` branch を作る | 201、branch metadata、pipeline route が成功する | `phase15_current_create.json` |
| `SCN-P15-2` | source に write A | branch 作成後に branch write B | source に B は出ず、branch に A/B がある | `phase15_isolation_branch_write.json` |
| `SCN-P15-3` | branch 作成後 | source に write C | branch に C は出ない | `phase15_isolation_source_write.json` |
| `SCN-P15-4` | valid WAL archive | timestamp selector branch を作る | 指定時刻のデータだけ復元される | `phase15_timestamp_create.json` |
| `SCN-P15-5` | valid WAL archive | frame_no selector branch を作る | 指定 frame まで復元される | `phase15_frame_create.json` |
| `SCN-P15-6` | `wal_retention_days = 0` | timestamp/frame branch を作る | 503 `PITR_NOT_ENABLED` | `phase15_archive_disabled.json` |
| `SCN-P15-7` | archive 範囲外 selector | branch を作る | 404 `FRAME_NOT_FOUND`、branch は作られない | `phase15_frame_not_found.json` |
| `SCN-P15-8` | corrupt archive frame | branch を作る | 409 `RESTORE_FRAME_CORRUPT`、branch は作られない | `phase15_frame_corrupt.json` |
| `SCN-P15-9` | invalid branch name | branch を作る | 400 `INVALID_DB_NAME` または `DB_RESERVED_NAME` | `phase15_invalid_name.json` |
| `SCN-P15-10` | duplicate branch name | branch を作る | 409 `DB_ALREADY_EXISTS`、二重 directory なし | `phase15_duplicate_create.json` |
| `SCN-P15-11` | normal DB create name includes `___` | DB create する | 400 `DB_RESERVED_NAME` | `phase15_reserved_db_name.json` |
| `SCN-P15-12` | branch が複数ある | list を呼ぶ | sort 済み schema、path なしで返る | `phase15_list.json` |
| `SCN-P15-13` | active branch | delete を呼ぶ | 204、route は 404、metadata と directory が消える | `phase15_delete_success.json` |
| `SCN-P15-14` | missing branch | delete を呼ぶ | metadata/directory 不在なら 204 | `phase15_delete_idempotent.json` |
| `SCN-P15-15` | directory only branch | 起動する | WARN + cleanup、active 扱いしない | `phase15_partial_create_cleanup.json` |
| `SCN-P15-16` | active metadata but missing directory | 起動する | 起動失敗 | `phase15_active_missing_dir.json` |
| `SCN-P15-17` | metadata deleting + directory exists | 起動する | delete recovery 後 route は 404 | `phase15_delete_recovery.json` |
| `SCN-P15-18` | active branch がある source | source delete を実行する | 403 `ORG_SCOPE_DENIED` | `phase15_source_delete_denied.json` |
| `SCN-P15-19` | source `block_reads=true` | branch create する | 403 `PERMISSION_DENIED` | `phase15_block_reads.json` |
| `SCN-P15-20` | branch candidate size quota 超過 | branch create する | 403 `QUOTA_EXCEEDED`、branch は作られない | `phase15_quota_exceeded.json` |
| `SCN-P15-21` | source DB token only | branch pipeline に接続する | branch token がなければ権限拒否 | `phase15_token_scope.json` |
| `SCN-P15-22` | branch create commit 前 failure | create を実行する | rollback または cleanup に収束し route 成功しない | `phase15_create_failure_rollback.json` |
| `SCN-P15-23` | branch delete 中 failure | delete を実行する | recovery 可能な deleting state が残る | `phase15_delete_failure_recovery.json` |
| `SCN-P15-24` | 同一 create retry | branch create を再送する | 二重 DB / 二重 quota charge を起こさない | `phase15_create_retry.json` |
| `SCN-P15-25` | `/v1/* seed.type:"database"` | branch field ありで呼ぶ | Turso 互換 branch create として成功または固定 response を返す | `phase15_seed_success.json` |
| `SCN-P15-26` | `/v1/* seed.type:"database"` | branch field なしで呼ぶ | 400 `INVALID_REQUEST` | `phase15_seed_missing_branch.json` |
| `SCN-P15-27` | logs/artifacts がある | secret scan を実行する | token、SQL args、absolute path、raw body が残らない | `phase15_secret_scan.json` |
| `SCN-P15-28` | Phase 14 backup/restore/PITR | Phase 15 実装後に regression する | API、rollback、archive に差分なし | `phase15_phase14_regression.json` |
| `SCN-P15-29` | extension API を呼ぶ | Phase 15 状態で実行する | Phase 16 まで未提供のまま | `phase15_phase16_boundary.json` |

**Phase 15 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| `branches.json` commit 前に branch route を成功させる | merge 不可 |
| branch DB directory だけの存在を active branch として扱う | merge 不可 |
| active metadata があるのに DB directory 不在を WARN だけで起動継続する | merge 不可 |
| source DB token を branch DB に自動適用する | merge 不可 |
| branch write を source DB に反映する、または source write を既存 branch に反映する | merge 不可 |
| source DB delete で active branch を cascade delete または orphan 化する | merge 不可 |
| branch create retry で二重 DB、二重 metadata、二重 quota charge を起こす | merge 不可 |
| delete failure を 204 success として隠す | merge 不可 |
| branch internal DB name を通常 DB create で受理する | merge 不可 |
| branch path、absolute path、token、SQL args、raw request body を response/log/artifact に出す | merge 不可 |
| Phase 15 で branch merge、branch diff、copy-on-write 最適化、SQLite extension、external storage transfer を完成扱いにする | merge 不可 |
| Phase 1〜14 の wire response、backup/restore/PITR、archive manifest を branch 実装の都合で変更する | merge 不可 |

**Phase 15 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | branch create/list/delete、current/timestamp/frame selector、routing、metadata、runtime map、restart recovery、Turso seed compatibility |
| `excluded_scope` | branch merge、branch diff、copy-on-write optimization、SQLite extension、external storage transfer、HA、内製 branch engine |
| `atomic_task_result` | `TASK-P15-1`〜`TASK-P15-12` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P15-1`〜`SCN-P15-29` の pass/fail、artifact path |
| `metadata_result` | branches.json schema、atomic update、duplicate key、future schema、corruption、restart recovery の検証結果 |
| `create_result` | current/timestamp/frame create、integrity_check、runtime open、metadata commit、rollback の trace |
| `delete_result` | route reject、runtime close、trash rename、metadata commit、cleanup、delete recovery の trace |
| `routing_result` | active metadata + directory + runtime map、deleted branch、partial branch、reserved name の matrix |
| `isolation_result` | source write、branch write、backup、restore、token scope、quota inheritance の matrix |
| `policy_result` | auth、source block_reads/block_writes、delete_protection、source delete denial、quota、DB not found の precedence matrix |
| `seed_compatibility_result` | `/admin/v1/*` branch API と `/v1/* seed.type:"database"` の success/failure snapshot |
| `operation_result` | retry、concurrent create、create/delete race、timeout/shutdown、idempotency、exclusive lock の artifact |
| `compatibility_baseline_result` | Phase 1〜14 regression、TypeScript SDK HTTP/WS transcript、Phase 14 backup/restore/PITR transcript 差分なし |
| `secret_redaction_result` | response/log/artifact に token、JWT、SQL args、absolute path、raw branch request body が残らない scan |
| `review_handoff_result` | 第三者が current/PITR branch、delete/recovery、restart、source delete denial、seed compatibility を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 15 で branch DB が独立 resource として利用可能になり、merge/diff/COW/extension は未提供である release note |
| `precision_closure_result` | branch metadata、current/timestamp/frame create、delete recovery、routing isolation、seed compatibility、Phase 1〜14 regression の artifact path、reviewer 再現 command、`P15-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

---
