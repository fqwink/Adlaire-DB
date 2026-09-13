### Phase 19：libSQL 内部コンポーネント段階的内製化

**目標**：Turso Cloud / libSQL SDK 互換を維持したまま、内部コンポーネントを `adlaire-*` crate へ段階的に差し替える。

Phase 19 は wire format、admin API、metadata schema、JWT claim を変更してはならない。SQL parser 完全内製は対象外とし、Phase 19 では WAL checkpoint 制御、storage 境界、executor 境界の adapter 化までを対象にする。

- 切り替え方式: config flag で既存 libSQL 経路と内製 crate 経路を切り替える
- 既定値: 直前 Phase と同じ挙動
- rollback: flag を戻すだけで完了できること
- 完了条件: TC-19-1〜TC-19-6 と T19-1〜T19-6 をすべて満たす

**Phase 19 internal adapter 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| default | 全 flag の default は `libsql`。既存利用者の挙動は変えない |
| shadow mode | adapter は libsql 結果と比較する shadow 実行から開始し、差分は ERROR + test failure |
| active mode | active 化は該当 snapshot と Phase 1〜18 regression が通る場合だけ |
| rollback | flag を戻すだけで metadata migration なしに旧経路へ戻る |
| write path | Phase 19 の storage adapter は readonly まで。write path 差し替えは Phase 20 以降 |
| error mapping | adapter 内部 error は既存 §7.3 code へ写像。新 code が必要なら先に仕様改訂 |
| performance | p95 latency、RSS、DB size、WAL size を baseline artifact に保存 |
| compatibility | API response、wire bytes、metadata JSON、JWT claim の snapshot 差分ゼロ |

**Phase 19 config flags：**

| Flag | TOML | Default | Allowed | 完了条件 |
|------|------|---------|---------|----------|
| `--internal-wal` | `[internal] wal` | `libsql` | `libsql` / `adlaire` | `adlaire` で TC-19-1〜TC-19-3 が通る |
| `--internal-storage` | `[internal] storage` | `libsql` | `libsql` / `adlaire-readonly` | Phase 19 では readonly adapter まで。write path 切替は禁止 |
| `--internal-executor` | `[internal] executor` | `libsql` | `libsql` / `adlaire-adapter` | API 互換を維持した adapter 境界のみ |

不正値は起動失敗。`storage=adlaire-readonly` かつ write workload が来た場合は、自動 fallback せず `INTERNAL_ERROR` ではなく起動時 config error にする。rollback は全 flag を `libsql` に戻すだけで metadata migration なしに完了しなければならない。

**Phase 19 性能・互換 baseline：**

- `libsql` 既定経路に対して Phase 1〜18 regression は 100% 通過
- `adlaire` 経路の p95 latency は同一 workload で `libsql` 経路の 2 倍以内
- crash recovery 後に `PRAGMA integrity_check` が `ok`
- TypeScript/Rust/Go libSQL SDK の CRUD smoke test が全て成功
- wire format、error code、metadata schema、JWT claim に差分がないことを snapshot test で確認

**Phase 19 完全実装精度固定契約：**

Phase 19 は「内部差し替えを始める Phase」であり、「外部契約を変える Phase」ではない。実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。下表のいずれかを満たせない場合は、実装ではなく仕様改訂へ戻す。

| 項目 | 固定仕様 | 完了不可条件 |
|------|----------|--------------|
| external contract freeze | `/v2/*`、`/admin/v1/*`、`/ha/v1/*`、metadata JSON、JWT claim、error code、response wrapper、hrana wire bytes は Phase 18 と差分ゼロ | 内製化都合で外部 API / wire / metadata / auth / error を変更した |
| default behavior | `[internal]` flag 未指定時は全経路 `libsql`。既存 config、既存 data-dir、既存 token、既存 DB は Phase 18 と同じ挙動 | default が内製経路へ変わる、既存 config が起動失敗する |
| flag parser | `--internal-wal`、`--internal-storage`、`--internal-executor` と TOML `[internal]` は CLI 優先で解決する。未知値、空文字、大文字小文字ゆれは起動失敗 | silent fallback、警告だけで起動、環境差で解釈が変わる |
| shadow mode | 内製 adapter はまず shadow 実行し、primary path の結果と比較する。差分は `ERROR` log、artifact 保存、test failure とする | 差分を握りつぶす、metrics のみで pass、expected を実装都合で更新 |
| active mode | active 化は該当 adapter の snapshot、oracle、Phase 1〜18 regression、SDK transcript、rollback test が通った場合のみ | 部分 pass で active を許可する |
| rollback | 全 flag を `libsql` に戻すだけで metadata migration なしに旧経路へ戻る。rollback 後の DB は `PRAGMA integrity_check=ok` | 追加 migration、手動ファイル削除、DB rebuild が必要 |
| WAL adapter scope | checkpoint trigger、WAL size observation、checkpoint result mapping、crash recovery check まで。WAL file format は変更しない | WAL bytes 独自形式、replication frame schema 変更 |
| storage adapter scope | Phase 19 は readonly adapter 境界まで。read path の page observation、open/close lifecycle、integrity_check、permission check を固定する | write path 差し替え、独自 page format、metadata 追加 |
| executor adapter scope | SQL 実行前後の adapter 境界、result mapping、error mapping、transaction boundary observation まで。SQL parser / planner / VM の内製は対象外 | SQL parser 差し替え、result shape 変更、transaction semantics 変更 |
| write policy | write workload で `storage=adlaire-readonly` を active にしてはならない。起動時 workload mode が write-capable なら config error | 実行時に fallback、write を readonly adapter に流す |
| error mapping | adapter 内部 error は既存 §7.3 code に写像し、response body は Phase 18 の schema を保つ。新 code は仕様改訂後のみ | `INTERNAL_ERROR` 乱用、stack trace / internal path 露出 |
| metadata | Phase 19 で新規 metadata file、schema version、migration marker を追加しない。必要な観測値は test artifact に保存する | `meta/*.json` の恒久 schema 追加 |
| metrics | adapter mode、shadow diff count、rollback result、latency/RSS/WAL size は test artifact と log で確認する。Prometheus 公開名を増やす場合は先に仕様化 | production metrics 名の無断追加 |
| performance gate | same workload で active mode p95 latency は `libsql` baseline の 2 倍以内、RSS は 1.5 倍以内、DB/WAL size は差分理由つき | baseline なし、測定条件なし、閾値超過を許容 |
| crash recovery | shadow/active/rollback の各 mode で crash 後に reopen、integrity_check、Phase 11 replication regression を確認する | crash test なしで完了 |
| SDK compatibility | TypeScript/Rust/Go libSQL SDK の connect、execute、batch、transaction、auth error、not found、SQL error transcript を保存する | curl のみで互換完了扱い |
| Turso compatibility | Turso Cloud 互換 mode の observable 差分は §9.1.47 に従い分類する。自己ホスト都合の差分は security/durability 以外では不可 | 差分分類なし、内製都合の差分許可 |
| observability | shadow diff、adapter selected、rollback completed、baseline summary は secret を含めず JSONL log に出す | SQL args、JWT、frame bytes、file path secret を log/artifact に残す |
| phase boundary | Phase 19 は adapter 境界の内側のみ。SQL parser 完全内製、storage write path、独自 WAL format、multi-primary、TLS native は Phase 20 以降 | Phase 20 以降の機能を Phase 19 完了扱いにする |

**Phase 19 原子タスク台帳：**

| Task ID | タスク | 対象 | 入力 | 完了条件 | artifact |
|---------|--------|------|------|----------|----------|
| `TASK-P19-1` | internal config resolver を固定 | CLI/TOML/env | 未指定、正値、未知値、競合指定 | CLI 優先、未知値は起動失敗、default は `libsql` | `phase19_config_matrix.json` |
| `TASK-P19-2` | adapter registry を固定 | internal adapter boundary | `libsql`、`adlaire`、`adlaire-readonly` | 選択 adapter が log と artifact に出る | `phase19_adapter_registry.json` |
| `TASK-P19-3` | WAL shadow adapter を固定 | WAL checkpoint | checkpoint workload | libsql と checkpoint result、WAL size、error mapping が一致 | `phase19_wal_shadow.json` |
| `TASK-P19-4` | WAL active adapter を固定 | WAL checkpoint | active flag | Phase 1〜18 regression、crash recovery、rollback が pass | `phase19_wal_active.json` |
| `TASK-P19-5` | storage readonly adapter を固定 | storage boundary | read-only workload | read/open/close/integrity_check が libsql baseline と一致 | `phase19_storage_readonly.json` |
| `TASK-P19-6` | storage write 禁止を固定 | storage boundary | write workload + `adlaire-readonly` | 起動時 config error。runtime fallback なし | `phase19_storage_write_guard.json` |
| `TASK-P19-7` | executor adapter 境界を固定 | executor boundary | execute/batch/transaction/error | result shape、transaction rollback、SQL error が Phase 18 と一致 | `phase19_executor_adapter.json` |
| `TASK-P19-8` | shadow diff handling を固定 | test/oracle | 意図的差分 | ERROR log、artifact、test failure が発生 | `phase19_shadow_diff.json` |
| `TASK-P19-9` | rollback を固定 | all adapters | active -> libsql | metadata migration なし、integrity_check ok、regression pass | `phase19_rollback.json` |
| `TASK-P19-10` | performance baseline を固定 | benchmark | libsql/adlaire same workload | p95/RSS/DB/WAL size を比較し閾値内 | `phase19_performance_baseline.json` |
| `TASK-P19-11` | SDK 互換 transcript を固定 | TypeScript/Rust/Go SDK | connect/CRUD/auth/error | wire/response/error 差分ゼロ | `phase19_sdk_transcript.json` |
| `TASK-P19-12` | Phase 1〜18 regression を閉じる | tests | full regression | 既存 contract 差分ゼロ | `phase19_regression_report.json` |
| `TASK-P19-13` | secret redaction を固定 | logs/artifacts | JWT、SQL args、frame bytes | artifact/log に secret、raw args、frame bytes が残らない | `phase19_secret_scan.json` |

**Phase 19 シナリオマトリクス：**

| Scenario ID | 観点 | 入力 | 期待結果 | artifact |
|-------------|------|------|----------|----------|
| `SCN-P19-1` | default flags | `[internal]` なし | 全経路 `libsql`、Phase 18 と差分ゼロ | `phase19_default_flags.json` |
| `SCN-P19-2` | CLI precedence | TOML と CLI が競合 | CLI が勝つ | `phase19_cli_precedence.json` |
| `SCN-P19-3` | invalid wal flag | `--internal-wal invalid` | 起動失敗、config error | `phase19_invalid_wal.json` |
| `SCN-P19-4` | invalid storage flag | `--internal-storage adlaire` | 起動失敗、allowed list 表示 | `phase19_invalid_storage.json` |
| `SCN-P19-5` | invalid executor flag | `--internal-executor native` | 起動失敗、allowed list 表示 | `phase19_invalid_executor.json` |
| `SCN-P19-6` | wal shadow match | checkpoint workload | libsql と shadow 差分ゼロ | `phase19_wal_shadow_match.json` |
| `SCN-P19-7` | wal shadow diff | 差分 injected | ERROR log、test failure | `phase19_wal_shadow_diff.json` |
| `SCN-P19-8` | wal active | active + checkpoint | regression、crash recovery pass | `phase19_wal_active.json` |
| `SCN-P19-9` | storage readonly read | readonly workload | read result 差分ゼロ | `phase19_storage_read.json` |
| `SCN-P19-10` | storage readonly write guard | write-capable workload | 起動時 config error | `phase19_storage_write_guard.json` |
| `SCN-P19-11` | executor execute | single execute | result shape 差分ゼロ | `phase19_executor_execute.json` |
| `SCN-P19-12` | executor batch | batch / sequence | result order、error position 差分ゼロ | `phase19_executor_batch.json` |
| `SCN-P19-13` | executor transaction | commit / rollback | transaction semantics 差分ゼロ | `phase19_executor_transaction.json` |
| `SCN-P19-14` | SQL error mapping | syntax / constraint / busy | §7.3 code と body 差分ゼロ | `phase19_sql_error_mapping.json` |
| `SCN-P19-15` | auth compatibility | valid/invalid JWT | Phase 18 response 差分ゼロ | `phase19_auth_compat.json` |
| `SCN-P19-16` | admin compatibility | Admin API smoke | response wrapper 差分ゼロ | `phase19_admin_compat.json` |
| `SCN-P19-17` | HA compatibility | Phase 18 HA smoke | HA status/promote/demote 差分ゼロ | `phase19_ha_compat.json` |
| `SCN-P19-18` | rollback | active -> libsql | metadata migration なしで復旧 | `phase19_rollback.json` |
| `SCN-P19-19` | crash recovery | crash during checkpoint | reopen、integrity_check ok | `phase19_crash_recovery.json` |
| `SCN-P19-20` | performance | same workload | p95/RSS/WAL/DB size 閾値内 | `phase19_perf.json` |
| `SCN-P19-21` | SDK TypeScript | libSQL TS SDK | connect/CRUD/error transcript 差分ゼロ | `phase19_ts_sdk.json` |
| `SCN-P19-22` | SDK Rust | libSQL Rust SDK | connect/CRUD/error transcript 差分ゼロ | `phase19_rust_sdk.json` |
| `SCN-P19-23` | SDK Go | libSQL Go SDK | connect/CRUD/error transcript 差分ゼロ | `phase19_go_sdk.json` |
| `SCN-P19-24` | metadata invariant | before/after active/rollback | metadata JSON 差分ゼロ | `phase19_metadata_invariant.json` |
| `SCN-P19-25` | secret redaction | token / SQL args / frame bytes | log/artifact に機密なし | `phase19_secret_redaction.json` |

**Phase 19 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| 内製化都合で API path、method、request、response、wire bytes、JWT claim、metadata schema、error code を変える | merge 不可 |
| Phase 19 で SQL parser、planner、VM、storage write path、WAL file format を置き換える | merge 不可 |
| shadow diff を expected 更新で消す | merge 不可 |
| `INTERNAL_ERROR` で adapter error を一律に隠す | merge 不可 |
| rollback に metadata migration、手動 data-dir 編集、DB rebuild を要求する | merge 不可 |
| SDK transcript なしに curl smoke だけで互換完了扱いにする | merge 不可 |
| performance baseline なしに active mode を完了扱いにする | merge 不可 |
| JWT、SQL args、frame bytes、内部ファイルパス secret を log/artifact に残す | merge 不可 |
| Phase 20 以降の内製化範囲を Phase 19 Done に含める | merge 不可 |

**Phase 19 Done receipt 必須項目：**

| 項目 | 内容 |
|------|------|
| `version_result` | 仕様書 `V.206` 準拠、Phase 19 contract ID、commit SHA |
| `config_result` | `TASK-P19-1`、`SCN-P19-1`〜`SCN-P19-5` の pass/fail と artifact path |
| `adapter_result` | WAL、storage readonly、executor の selected mode、shadow/active 状態、artifact path |
| `shadow_diff_result` | 差分ゼロまたは差分理由、ERROR log、test failure の証跡 |
| `rollback_result` | active -> `libsql` rollback、metadata migration なし、integrity_check ok |
| `compatibility_result` | Turso Cloud、libSQL SDK、hrana、metadata、JWT、error、Admin/HA API の差分分類 |
| `sdk_transcript_result` | TypeScript/Rust/Go libSQL SDK transcript と snapshot 差分 |
| `performance_result` | p95 latency、RSS、DB size、WAL size、baseline 比率 |
| `crash_recovery_result` | checkpoint 中 crash、reopen、integrity_check、regression 結果 |
| `regression_result` | Phase 1〜18 regression 全 pass、open regression 0 件 |
| `secret_redaction_result` | response/log/artifact に JWT、SQL args、frame bytes、secret path が残らない scan |
| `phase_boundary_result` | Phase 20 以降へ送った項目、Phase 19 で未実装の理由、production path 差分なし |
| `review_handoff_result` | 第三者が config、shadow、active、rollback、SDK transcript を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 19 で内製 adapter 境界が利用可能になり、外部 API と既定挙動は変わらない release note |
| `precision_closure_result` | config flags、shadow/active/rollback、adapter boundary、SDK transcript、performance/crash recovery、Phase 1〜18 regression の artifact path、reviewer 再現 command、`P19-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 12〜19 完全実装精度正規化契約：**

Phase 12〜19 は data durability、recovery、operator action、replication/archive/restore/branch/extension/metrics/HA/internal adapter の境界を扱うため、Done 判定は artifact と再現 command を必須にする。実装者は「主要機能が動く」ことを完了根拠にしてはならない。各 Phase の Done receipt は、原子タスク、シナリオ、禁止事項、compatibility baseline、durability / recovery、rollback、secret redaction、review handoff、operator behavior delta、precision closure をすべて埋める。

| 項目 | 固定仕様 | 完了不可条件 |
|------|----------|--------------|
| artifact path | Done receipt の pass/fail は artifact path または再現 command を必ず持つ | 口頭説明、PR description、スクリーンショットのみ |
| regression chain | Phase 12 は Phase 1〜11、Phase 13 は Phase 1〜12、以後同様に直前 Phase までの regression をすべて通す | 直前 Phase regression skip、影響なし根拠なし |
| compatibility baseline | Turso Cloud snapshot、libSQL SDK HTTP/WS transcript、hrana wire snapshot、Admin/Platform API snapshot のうち該当面を Done receipt に含める | curl smoke のみ、snapshot なし |
| durability evidence | replication state、manifest、backup body、branch DB、extension manifest、metrics snapshot、HA state、adapter rollback は crash/restart/recovery artifact を持つ | restart 未検証、整合性証跡なし |
| rollback and recovery | restore/branch/delete/promote/demote/internal active mode は失敗注入、rollback、startup recovery、operator action を固定する | 失敗時挙動未定義 |
| secret redaction | response/stdout/stderr/log/artifact に JWT、Admin/Platform/replication/HA token、SQL args、frame bytes、backup body、absolute path secret が残らない scan を必須にする | scan なし、秘匿値混入 |
| future boundary | Phase 12〜19 で未対象の archive/PITR/branch/extension/HA/内製化/multi-primary/SQL parser write path を完成扱いにしない | 未来 Phase の成功応答を Done に含める |
| operator reproducibility | 第三者が clean checkout から command で primary/replica、archive、backup/PITR、branch、extension、metrics、HA、adapter rollback を再現できる handoff を必須にする | ローカル状態依存、手順欠落 |
| bug-zero readiness | Done receipt に coverage gap、未解決判断、仕様未確定、既知 flaky が 0 件であることを明記する | 未解決判断を実装者判断へ先送り |

---
