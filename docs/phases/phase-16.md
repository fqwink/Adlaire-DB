### Phase 16：SQLite 拡張ロード

**目標**：許可済み SQLite 拡張だけを安全に登録・ロード・無効化できるようにする。

Phase 16 では `.so` 拡張のみを対象とする。Wasm 拡張、任意パスロード、SQL からの `load_extension()` 直接実行は対象外とする。

- API: `GET/POST/DELETE /admin/v1/extensions`（§9.5）
- 永続化: `meta/extensions.json` と `{data-dir}/extensions/{name}/{version}/`（§9.6）
- 完了条件: TC-16-1〜TC-16-6 と T16-1〜T16-6 をすべて満たす

**Phase 16 extension 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| binary source | HTTP upload は受け付けない。事前配置済み file のみ登録対象 |
| path | `{data-dir}/extensions/{name}/{version}/{name}.so` 以外は拒否。symlink は拒否 |
| sha256 | 登録前、load 前、起動時復元前に毎回検証 |
| load scope | load は新規 DB connection 作成時に適用。既存 connection への retroactive load は保証しない |
| failure | 登録時 load 失敗は metadata を追加しない。起動時 load 失敗は該当 extension を `state:"load_failed"` にし ERROR log |
| delete | metadata から削除し、binary directory は残す。削除済み extension は新規 connection に load しない |
| SQL direct load | `load_extension()` SQL は常に `EXTENSION_NOT_ALLOWED` |
| logging | extension name/version/sha256 は可。絶対 path と load error の環境変数展開値は秘匿 |

**Phase 16 extension manifest schema：**

```json
{
  "extensions": [
    {
      "name": "vector",
      "version": "0.1.0",
      "filename": "vector.so",
      "sha256": "64 lowercase hex chars",
      "enabled": true,
      "state": "registered",
      "loaded_at": null,
      "created_at": "2026-09-13T00:00:00Z"
    }
  ]
}
```

| Field | Validation |
|-------|------------|
| `name` | `^[a-zA-Z0-9_-]{1,64}$`。`sqlite`、`libsql`、`adlaire` prefix は予約で `EXTENSION_NOT_ALLOWED` |
| `version` | semver `MAJOR.MINOR.PATCH` のみ。pre-release/build metadata は Phase 16 対象外 |
| `filename` | `{name}.so` のみ。slash、dot-dot、絶対 path は `INVALID_REQUEST` |
| `sha256` | lowercase hex 64 文字のみ |
| `enabled` | boolean 必須 |
| `state` | `registered` / `loading` / `loaded` / `load_failed` / `disabled` / `deleted` のみ |
| `loaded_at` | `state="loaded"` の時だけ RFC3339 UTC 秒精度。未ロードは `null` |

登録時は binary を `{data-dir}/extensions/{name}/{version}/{filename}` に配置済みであることを確認し、sha256 が一致した場合だけ `extensions.json` に追加する。HTTP API から binary upload は受け付けない。load は server 起動時と `POST /admin/v1/extensions` 後に行い、失敗時は metadata を追加せず `EXTENSION_LOAD_FAILED` を返す。`DELETE` は metadata から削除し、binary directory は削除しない。既存 metadata に `loaded` boolean がある場合は migration で `loaded:true` を `state:"loaded"`、`loaded:false` を `state:"registered"` に変換し、以後 `loaded` boolean を正として参照してはならない。

**Phase 16 完全実装精度固定契約：**

Phase 16 は、事前配置済み SQLite `.so` extension を Admin API で登録・一覧・削除し、新規 DB connection 作成時にだけ検証済み extension を load する Phase である。Phase 16 の完了判定は、extension が読み込めることではなく、任意 path、symlink、未検証 binary、SQL 直接 load、既定 Turso 互換 mode への混入、metadata 破損、restart 復元、secret / path 漏洩をすべて拒否・証跡化できることを必須とする。

| 項目 | 固定仕様 |
|------|----------|
| endpoint scope | `GET /admin/v1/extensions`、`POST /admin/v1/extensions`、`DELETE /admin/v1/extensions/{name}` の 3 API のみ |
| auth | Admin token 必須。auth failure は body parse、file stat、sha256、load より先に評価する |
| compatibility mode | 既定 mode は Turso 互換。extension load は明示的に登録・enabled な extension だけ対象。Adlaire 独自 mode を暗黙追加しない |
| binary source | HTTP upload、remote URL、base64 body、multipart upload はすべて拒否。事前配置済み local file のみ登録対象 |
| extension root | `{data-dir}/extensions/{name}/{version}/` 固定。canonicalize 後も data-dir 配下から出てはならない |
| binary filename | `{name}.so` のみ。slash、dot-dot、dot prefix、絶対 path、別拡張子は 400 `INVALID_REQUEST` |
| symlink policy | extension root、version dir、binary file のいずれかが symlink なら 403 `EXTENSION_NOT_ALLOWED` |
| name validation | `^[a-z0-9][a-z0-9_-]{0,63}$` 固定。`sqlite`、`libsql`、`adlaire` prefix は 403 `EXTENSION_NOT_ALLOWED` |
| version validation | `MAJOR.MINOR.PATCH` の semver のみ。pre-release、build metadata、leading zero は 400 `INVALID_REQUEST` |
| sha256 validation | lowercase hex 64 文字のみ。登録前、load 前、restart 復元前に binary bytes から再計算する |
| signature policy | Phase 16 は sha256 検証を必須署名境界とする。外部署名形式を追加する場合は別仕様更新を必要とする |
| allowlist | allowlist にない name/version は 403 `EXTENSION_NOT_ALLOWED`。allowlist 未設定時の default は deny all |
| manifest path | `{data-dir}/meta/extensions.json` 固定。tmp write + fsync + rename + directory fsync で更新する |
| manifest schema | `schema_version`、`extensions[]` 必須。各 entry は name、version、filename、sha256、enabled、state、loaded_at、created_at、updated_at を持つ |
| legacy migration | 旧 `loaded` boolean がある場合だけ migration し、以後 production path では参照しない |
| state machine | `registered`、`loading`、`loaded`、`load_failed`、`disabled`、`deleted` のみ。未定義 state は起動失敗 |
| register success | binary 存在、path/symlink、allowlist、sha256、load dry-run が成功した後だけ manifest に追加し 201 を返す |
| register failure | load dry-run 失敗、sha256 不一致、allowlist 拒否では manifest を追加しない |
| duplicate register | 同一 name の active entry は 409 `EXTENSION_ALREADY_EXISTS`。version 違いを同名 coexist させない |
| load boundary | load は新規 DB connection 作成時に適用。既存 connection への retroactive load は保証しない |
| load failure | 起動時または connection 作成時の load 失敗は該当 extension を `load_failed` にし、新規 connection では使用不可にする |
| SQL direct load | SQL text / PRAGMA / function 経由の `load_extension()` は常に 403 `EXTENSION_NOT_ALLOWED` |
| delete behavior | DELETE は manifest entry を `deleted` または除去済みとして commit し、binary directory は削除しない |
| delete idempotency | 存在しない extension の DELETE は 404 `EXTENSION_NOT_FOUND`。silent 204 にはしない |
| restart restore | 起動時は enabled + loaded/registered の entry だけ検証し、sha256 不一致や missing binary は起動失敗または `load_failed` を Phase 16 で明示する。本契約では `load_failed` にして起動継続、health degraded とする |
| observability | logs/metrics は name、version、sha256、state、result まで。absolute path、env 展開値、load error 内の raw path、token、SQL args は出力禁止 |
| phase boundary | Phase 16 は SQLite `.so` extension 管理のみ。metrics 永続化、Prometheus、HA、Wasm extension、extension upload、内製化は対象外 |

**Phase 16 原子タスク台帳：**

| Task ID | 目的 | 変更対象 | 完了条件 | 失敗時の扱い | 必須成果物 |
|---------|------|----------|----------|--------------|------------|
| `TASK-P16-1` | extension API surface を固定する | admin extensions route | GET/POST/DELETE の status、body、auth、unknown field が固定される | endpoint 追加禁止 | API transcript |
| `TASK-P16-2` | manifest schema を固定する | `meta/extensions.json` | schema_version、state、legacy migration、atomic update が検証される | undefined state 黙認禁止 | manifest fixture |
| `TASK-P16-3` | path / symlink 拒否を固定する | path validator | canonical path、root confinement、filename、symlink 拒否が通る | 任意 path load 禁止 | path fixture |
| `TASK-P16-4` | allowlist / sha256 を固定する | verifier | allowlist deny、sha256 mismatch、missing binary が正しい error になる | 未検証 load 禁止 | verifier matrix |
| `TASK-P16-5` | register / load 境界を固定する | extension loader | load dry-run 成功後だけ 201、既存 connection 非 retroactive が証跡化される | metadata 先行 commit 禁止 | load trace |
| `TASK-P16-6` | SQL direct load 拒否を固定する | SQL classifier / executor | `load_extension()` 直接実行を常に拒否する | SQL から bypass 禁止 | SQL rejection snapshot |
| `TASK-P16-7` | delete / disable 挙動を固定する | delete flow | metadata 更新、new connection 非 load、binary 残置、404 missing が固定される | binary 自動削除禁止 | delete trace |
| `TASK-P16-8` | restart restore を固定する | startup recovery | enabled extension の再検証、load_failed、health degraded が再現できる | 自動許可禁止 | restart fixture |
| `TASK-P16-9` | connection load scope を固定する | DB connection lifecycle | new connection load、existing connection 非 retroactive、load order が固定される | 既存 connection 破壊禁止 | connection matrix |
| `TASK-P16-10` | redaction / observability を固定する | logging / metrics | absolute path、env、token、SQL args が残らない | raw load error 出力禁止 | redaction scan |
| `TASK-P16-11` | compatibility boundary を固定する | config / mode | Turso 互換既定、Adlaire extension mode 混入なし、SDK regression が通る | default mode 差分禁止 | compatibility snapshot |
| `TASK-P16-12` | Phase 1〜15 regression を閉じる | tests | HTTP/WS、backup/restore/PITR、branch、replication に差分なし | extension 実装で既存 contract を壊さない | regression report |

**Phase 16 シナリオマトリクス：**

| Scenario ID | Given | When | Then | Artifact |
|-------------|-------|------|------|----------|
| `SCN-P16-1` | valid Admin token | GET extensions | 200 `{extensions:[...]}`、path なし | `phase16_list.json` |
| `SCN-P16-2` | valid preplaced `.so` + sha256 | POST extensions | 201、manifest registered/loaded、new connection で load 対象 | `phase16_register_success.json` |
| `SCN-P16-3` | HTTP upload body | POST extensions | 400 `INVALID_REQUEST` | `phase16_upload_rejected.json` |
| `SCN-P16-4` | remote URL / base64 field | POST extensions | 400 `INVALID_REQUEST` | `phase16_remote_rejected.json` |
| `SCN-P16-5` | name with reserved prefix | POST extensions | 403 `EXTENSION_NOT_ALLOWED` | `phase16_reserved_name.json` |
| `SCN-P16-6` | invalid version | POST extensions | 400 `INVALID_REQUEST` | `phase16_invalid_version.json` |
| `SCN-P16-7` | filename contains slash or dot-dot | POST extensions | 400 `INVALID_REQUEST` | `phase16_invalid_filename.json` |
| `SCN-P16-8` | binary path symlink | POST extensions | 403 `EXTENSION_NOT_ALLOWED` | `phase16_symlink_rejected.json` |
| `SCN-P16-9` | sha256 mismatch | POST extensions | 403 `EXTENSION_SIGNATURE_INVALID`、manifest 追加なし | `phase16_sha256_mismatch.json` |
| `SCN-P16-10` | allowlist missing | POST extensions | 403 `EXTENSION_NOT_ALLOWED` | `phase16_allowlist_denied.json` |
| `SCN-P16-11` | SQLite load failure | POST extensions | 500 `EXTENSION_LOAD_FAILED`、manifest 追加なし | `phase16_load_failed_register.json` |
| `SCN-P16-12` | duplicate extension name | POST extensions | 409 `EXTENSION_ALREADY_EXISTS` | `phase16_duplicate.json` |
| `SCN-P16-13` | active extension | DELETE extension | 204、new connection で load されない、binary は残る | `phase16_delete_success.json` |
| `SCN-P16-14` | missing extension | DELETE extension | 404 `EXTENSION_NOT_FOUND` | `phase16_delete_missing.json` |
| `SCN-P16-15` | query includes `load_extension()` | SQL execute | 403 `EXTENSION_NOT_ALLOWED` | `phase16_sql_direct_load_rejected.json` |
| `SCN-P16-16` | extension registered after existing connection | existing connection query | retroactive load は発生しない | `phase16_existing_connection_scope.json` |
| `SCN-P16-17` | extension registered before new connection | new connection opens | sha256 再検証後に load 対象になる | `phase16_new_connection_load.json` |
| `SCN-P16-18` | legacy metadata with `loaded:true` | startup | `state:"loaded"` へ migration される | `phase16_legacy_loaded_migration.json` |
| `SCN-P16-19` | manifest unknown state | startup | 起動失敗 | `phase16_unknown_state.json` |
| `SCN-P16-20` | restart with missing binary | startup | `load_failed`、health degraded、raw path leak なし | `phase16_restart_missing_binary.json` |
| `SCN-P16-21` | restart with sha256 mismatch | startup | `load_failed`、new connection load 禁止 | `phase16_restart_sha_mismatch.json` |
| `SCN-P16-22` | logs/artifacts がある | secret scan | token、SQL args、absolute path、env 展開値が残らない | `phase16_secret_scan.json` |
| `SCN-P16-23` | default Turso compatibility mode | SDK regression | extension metadata が wire/API 互換を壊さない | `phase16_compat_mode.json` |
| `SCN-P16-24` | Phase 15 branch | Phase 16 実装後に regression | branch routing/isolation/recovery に差分なし | `phase16_phase15_regression.json` |
| `SCN-P16-25` | metrics/Prometheus API | Phase 16 状態で呼ぶ | Phase 17 まで未提供のまま | `phase16_phase17_boundary.json` |

**Phase 16 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| HTTP API で extension binary upload を受け付ける | merge 不可 |
| data-dir 外、symlink、任意 filename、absolute path から load する | merge 不可 |
| sha256 / allowlist 検証前に load する | merge 不可 |
| load dry-run 失敗後に manifest へ active entry を追加する | merge 不可 |
| SQL の `load_extension()` 直接実行を許可する | merge 不可 |
| `extensions.json` の `loaded` boolean を production path の正として使い続ける | merge 不可 |
| restart 時の missing/corrupt binary を自動許可または自動修復する | merge 不可 |
| DELETE で binary directory を自動削除する | review failure |
| source absolute path、env 展開値、token、SQL args を response/log/artifact に出す | merge 不可 |
| 既定 Turso 互換 mode に Adlaire 独自 extension mode の差分を混入する | merge 不可 |
| Phase 16 で metrics 永続化、Prometheus、HA、Wasm extension、upload、内製化を完成扱いにする | merge 不可 |
| Phase 1〜15 の wire response、branch、backup/restore/PITR、replication behavior を extension 実装の都合で変更する | merge 不可 |

**Phase 16 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | extension list/register/delete、manifest schema、allowlist、sha256、path/symlink rejection、new connection load、restart restore |
| `excluded_scope` | Wasm extension、binary upload、remote extension source、metrics persistence、Prometheus、HA、内製化 |
| `atomic_task_result` | `TASK-P16-1`〜`TASK-P16-12` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P16-1`〜`SCN-P16-25` の pass/fail、artifact path |
| `api_result` | GET/POST/DELETE の status、body、auth、unknown field、duplicate、missing の transcript |
| `manifest_result` | schema、legacy migration、atomic update、future schema、unknown state、corruption の検証結果 |
| `path_security_result` | canonical path、data-dir confinement、symlink、filename、reserved prefix、absolute path rejection の matrix |
| `verification_result` | allowlist、sha256、load dry-run、restart revalidation、signature error の matrix |
| `load_scope_result` | new connection load、existing connection 非 retroactive、delete 後 new connection 非 load の artifact |
| `sql_rejection_result` | SQL direct `load_extension()`、PRAGMA/function bypass、read/write classifier の拒否証跡 |
| `restart_recovery_result` | missing binary、sha mismatch、load_failed、health degraded、operator action の fixture |
| `compatibility_baseline_result` | Phase 1〜15 regression、TypeScript SDK HTTP/WS transcript、Phase 15 branch transcript 差分なし |
| `secret_redaction_result` | response/log/artifact に token、JWT、SQL args、absolute path、env 展開値が残らない scan |
| `review_handoff_result` | 第三者が register/load/delete/restart/path rejection/SQL rejection を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 16 で事前配置済み `.so` extension の登録/削除が可能になり、upload/Wasm/metrics/HA は未提供である release note |
| `precision_closure_result` | extension manifest、allowlist、sha256、canonical path/symlink rejection、new connection load、SQL bypass rejection、Phase 1〜15 regression の artifact path、reviewer 再現 command、`P16-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |
