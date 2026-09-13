### Phase 17：メトリクス永続化・外部監視連携

**目標**：Phase 10 のインメモリ metrics を永続 counter に拡張し、Prometheus 互換出力を提供する。

Phase 17 では alerting、remote write、外部 SaaS 連携は対象外とする。Prometheus text exposition のみを対象にする。

- API: `GET /admin/v1/metrics/prometheus`（§9.5）
- 永続化: `meta/metrics-snapshot.json`
- quota 判定用 usage: Phase 8 の `usage.json` を正とする
- 完了条件: TC-17-1〜TC-17-5 と T17-1〜T17-5 をすべて満たす

**Phase 17 metrics 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| counter source | runtime atomic counter を正とし、30 秒ごとに snapshot へ保存 |
| startup | snapshot が正常なら counter 初期値へ反映。破損なら WARN 後 0 初期化 |
| quota usage | quota 判定は `usage.json` を正とし、metrics snapshot から推測しない |
| Prometheus escaping | label value は `\`、`"`、newline を Prometheus 仕様通り escape |
| route label | raw path ではなく route pattern を使う。DB 名や token id を label に入れない |
| content type | `text/plain; version=0.0.4; charset=utf-8` 固定 |
| accept | `Accept` 未指定、`*/*`、`text/plain` は 200。その他は 406 `NOT_ACCEPTABLE` |
| line format | UTF-8、LF 改行、末尾 LF 必須。各 metric は `HELP`、`TYPE`、samples の順で出す |
| invalid value | 取得不能値を `NaN` として出さない。該当 sample を省略し WARN log を出す |
| shutdown | graceful shutdown 時に同期 snapshot を 1 回書く。失敗時は ERROR log |

**Phase 17 metrics snapshot schema：**

```json
{
  "counters": {
    "queries_total": 0,
    "rows_read_total": 0,
    "rows_written_total": 0,
    "http_requests_total": 0,
    "errors_total": 0
  },
  "gauges": {
    "databases_total": 0,
    "connections_active": 0,
    "storage_bytes": 0,
    "wal_size_bytes": 0
  },
  "updated_at": "2026-09-13T00:00:00Z"
}
```

Prometheus endpoint は `text/plain; version=0.0.4; charset=utf-8` を返し、metric 名は以下に固定する。

| Metric | Type | Labels |
|--------|------|--------|
| `adlaire_queries_total` | counter | `db`, `kind` (`read`/`write`) |
| `adlaire_rows_read_total` | counter | `db` |
| `adlaire_rows_written_total` | counter | `db` |
| `adlaire_http_requests_total` | counter | `method`, `route`, `status` |
| `adlaire_errors_total` | counter | `code` |
| `adlaire_databases_total` | gauge | なし |
| `adlaire_connections_active` | gauge | `db` |
| `adlaire_storage_bytes` | gauge | `db` |
| `adlaire_wal_size_bytes` | gauge | `db` |

label 値に DB 名以外の user input を直接入れてはならない。SQL text、SQL args、token、raw path は label 禁止。snapshot 書き込み間隔は 30 秒固定とし、graceful shutdown 時は最終 snapshot を同期書き込みする。

**Phase 17 完全実装精度固定契約：**

Phase 17 は、Phase 10 の in-memory metrics を永続 counter と Prometheus text exposition へ拡張する Phase である。Phase 17 の完了判定は、数値を返すことではなく、counter の lost increment 防止、snapshot 復元、Prometheus format、label 秘匿、quota usage との境界、破損 snapshot recovery、Phase 18 以降との境界がすべて証跡化されていることを必須とする。

| 項目 | 固定仕様 |
|------|----------|
| endpoint scope | 既存 `GET /admin/v1/metrics` の永続 counter 拡張と、新規 `GET /admin/v1/metrics/prometheus` のみ |
| auth | Admin token 必須。auth failure は metrics read、snapshot load、Prometheus render より先に評価する |
| counter source | runtime atomic counter を正とする。snapshot は復元 seed と永続証跡であり、稼働中の正にはしない |
| snapshot path | `{data-dir}/meta/metrics-snapshot.json` 固定。tmp write + fsync + rename + directory fsync で更新する |
| snapshot interval | 通常 flush は 30 秒固定。設定で変更する場合は別仕様更新を必要とする |
| shutdown flush | graceful shutdown 時に同期 snapshot を 1 回書く。失敗時は ERROR log だが data path は継続しない |
| startup restore | snapshot が正常なら counter 初期値へ反映。破損、future schema、不正型は WARN 後 0 初期化 |
| corruption handling | 破損 snapshot は上書き修復前に WARN log を出す。silent ignore は禁止 |
| write concurrency | periodic flush と shutdown flush は同一 lock で直列化し、古い snapshot で新しい値を上書きしない |
| lost increment | snapshot 中も atomic counter increment を止めない。snapshot は読み取り時点の monotonic 値を保存する |
| counter monotonicity | counter は restart 復元後も減少しない。破損 snapshot で 0 初期化した場合は WARN と recovery artifact を残す |
| gauge source | gauges は取得時点の runtime / filesystem / usage source から読む。取得不能値は sample 省略または field 0 を Phase 17 表の通り扱う |
| quota usage source | quota 判定は `usage.json` を正とする。metrics snapshot、Prometheus sample、filesystem gauge から quota 判定を推測しない |
| storage usage | `adlaire_storage_bytes` は usage.json または Phase 8 usage 計測源と一致させる。metrics 独自再計測で quota とズレる値を正にしない |
| Prometheus endpoint | 成功は 200。`Content-Type: text/plain; version=0.0.4; charset=utf-8` 固定 |
| Accept | 未指定、`*/*`、`text/plain`、`text/plain; version=0.0.4` は 200。その他は 406 `NOT_ACCEPTABLE` |
| line format | UTF-8、LF 改行、末尾 LF 必須。metric ごとに HELP、TYPE、sample の順で出す |
| metric order | metric name 昇順。各 metric 内は label set の辞書順 |
| label escaping | backslash、double quote、LF を Prometheus text format 通り escape する |
| HELP escaping | backslash と LF を escape する。HELP に secret、raw path、SQL text を含めない |
| sample value | finite number のみ。NaN、Inf、取得不能値は出さず WARN log を出す |
| route label | route pattern のみ。raw path、query string、DB 名入り path、token id、request id は禁止 |
| db label | DB 名は許可。ただし token、JWT、SQL args、branch path、absolute path は label 禁止 |
| error label | error code のみ。message、panic text、raw extension path、SQL error detail は label 禁止 |
| metrics read effect | Prometheus endpoint 自身の HTTP request counter 更新は許可するが、二重加算は禁止。SQL counter は増やさない |
| health relation | metrics snapshot 破損復旧中でも health を落とさない。Prometheus render 不能時は endpoint のみ 500 `INTERNAL_ERROR` |
| phase boundary | Phase 17 は metrics 永続化と Prometheus text exposition のみ。alerting、remote write、external SaaS、HA/failover は対象外 |

**Phase 17 原子タスク台帳：**

| Task ID | 目的 | 変更対象 | 完了条件 | 失敗時の扱い | 必須成果物 |
|---------|------|----------|----------|--------------|------------|
| `TASK-P17-1` | metrics snapshot schema を固定する | `meta/metrics-snapshot.json` | schema_version、counters、gauges、updated_at、atomic update が検証される | 不正 snapshot を正として使わない | snapshot fixture |
| `TASK-P17-2` | startup restore を固定する | startup recovery | valid snapshot 復元、corrupt/future schema WARN + 0 初期化が再現できる | silent ignore 禁止 | recovery fixture |
| `TASK-P17-3` | periodic/shutdown flush を固定する | metrics writer | 30 秒 flush、shutdown flush、lock、monotonic write が証跡化される | 古い値で上書き禁止 | flush trace |
| `TASK-P17-4` | counter update points を固定する | HTTP/WS/pipeline metrics | request、error、query、row counter の加算点と二重加算禁止が固定される | validation 拒否を SQL counter に加算しない | counter matrix |
| `TASK-P17-5` | usage/quota boundary を固定する | usage / quota / metrics | quota は usage.json 正、metrics snapshot 非参照、storage gauge 整合が証明される | metrics 値で quota 判定禁止 | usage matrix |
| `TASK-P17-6` | Prometheus endpoint API を固定する | admin prometheus route | status、Content-Type、Accept、auth、406 が固定される | Accept 無視禁止 | API transcript |
| `TASK-P17-7` | Prometheus text format を固定する | renderer | HELP/TYPE/sample、order、LF、末尾 LF、finite value が一致する | NaN/Inf 出力禁止 | text snapshot |
| `TASK-P17-8` | label redaction を固定する | renderer / labels | route pattern、db label、error code、escape、secret 非含有が検証される | raw path/token/SQL 出力禁止 | label snapshot |
| `TASK-P17-9` | invalid metric value を固定する | metrics collection | 取得不能 gauge 省略/WARN、render error mapping が固定される | bogus sample 出力禁止 | invalid value artifact |
| `TASK-P17-10` | metrics read side effect を固定する | HTTP metrics middleware | Prometheus read は HTTP counter だけ 1 回加算、SQL counter は増えない | 二重加算禁止 | side-effect transcript |
| `TASK-P17-11` | observability / redaction を固定する | logging / artifacts | logs/snapshots に token、SQL args、raw path、absolute path が残らない | secret scan failure は未完了 | redaction scan |
| `TASK-P17-12` | Phase 1〜16 regression を閉じる | tests | HTTP/WS、backup/restore/PITR、branch、extension、replication に差分なし | metrics 実装で既存 contract を壊さない | regression report |

**Phase 17 シナリオマトリクス：**

| Scenario ID | Given | When | Then | Artifact |
|-------------|-------|------|------|----------|
| `SCN-P17-1` | counters が増加済み | periodic flush | `metrics-snapshot.json` に monotonic counters が atomic 保存される | `phase17_snapshot_flush.json` |
| `SCN-P17-2` | valid snapshot | startup | counter 初期値へ復元される | `phase17_startup_restore.json` |
| `SCN-P17-3` | corrupt snapshot | startup | WARN + 0 初期化、起動継続 | `phase17_corrupt_snapshot.json` |
| `SCN-P17-4` | future schema snapshot | startup | WARN + 0 初期化 | `phase17_future_schema.json` |
| `SCN-P17-5` | periodic flush と shutdown flush が同時 | shutdown | lock で直列化し、新しい値を失わない | `phase17_flush_race.json` |
| `SCN-P17-6` | HTTP request 成功/失敗 | metrics を見る | http_requests_total と errors_total が正しく増える | `phase17_http_counter.json` |
| `SCN-P17-7` | SQL read/write | pipeline 実行後 | queries/rows counters が step 実行分だけ増える | `phase17_sql_counter.json` |
| `SCN-P17-8` | validation で SQL 拒否 | pipeline 実行 | SQL execution counter は増えない | `phase17_validation_no_sql_counter.json` |
| `SCN-P17-9` | quota 超過中 | backup/metrics read | metrics read は許可、quota 判定は usage.json のまま | `phase17_quota_boundary.json` |
| `SCN-P17-10` | usage.json と metrics snapshot が異なる | write 判定 | usage.json を正として判定する | `phase17_usage_source.json` |
| `SCN-P17-11` | valid Admin token | GET Prometheus | 200、固定 Content-Type、HELP/TYPE/sample が返る | `phase17_prometheus_success.txt` |
| `SCN-P17-12` | no Admin token | GET Prometheus | 401 auth error | `phase17_prometheus_auth.json` |
| `SCN-P17-13` | Accept unsupported | GET Prometheus | 406 `NOT_ACCEPTABLE` | `phase17_prometheus_accept.json` |
| `SCN-P17-14` | label に quote/backslash/LF が含まれる | render | Prometheus 仕様通り escape される | `phase17_label_escape.txt` |
| `SCN-P17-15` | route with DB/path/query/token | render | route label は pattern のみで raw path/query/token が出ない | `phase17_route_label.txt` |
| `SCN-P17-16` | gauge 取得不能 | render | NaN を出さず sample 省略 + WARN | `phase17_invalid_gauge.txt` |
| `SCN-P17-17` | Prometheus endpoint を 1 回読む | metrics を確認 | HTTP counter は 1 回だけ増え、SQL counter は増えない | `phase17_read_side_effect.json` |
| `SCN-P17-18` | metrics snapshot write 失敗 | flush | ERROR log、old snapshot 維持、runtime counter は維持 | `phase17_snapshot_write_failure.json` |
| `SCN-P17-19` | logs/artifacts がある | secret scan | token、JWT、SQL args、raw path、absolute path が残らない | `phase17_secret_scan.json` |
| `SCN-P17-20` | Phase 16 extension | Phase 17 実装後に regression | extension register/load/delete に差分なし | `phase17_phase16_regression.json` |
| `SCN-P17-21` | HA API を呼ぶ | Phase 17 状態で実行 | Phase 18 まで未提供のまま | `phase17_phase18_boundary.json` |

**Phase 17 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| metrics snapshot を quota 判定の正として使う | merge 不可 |
| periodic flush と shutdown flush が競合して counter を巻き戻す | merge 不可 |
| snapshot 破損を silent ignore する | merge 不可 |
| Prometheus sample に NaN、Inf、取得不能値を出す | merge 不可 |
| raw path、query string、token id、request id を route label に入れる | merge 不可 |
| SQL text、SQL args、JWT、token、absolute path を metric label / HELP / log / artifact に出す | merge 不可 |
| Prometheus endpoint の Content-Type を JSON または任意 text に変える | review failure |
| unsupported Accept を 200 で返す | review failure |
| metrics read で SQL execution counter を増やす | merge 不可 |
| storage gauge を metrics 独自推定にして `usage.json` と quota 判定をズラす | merge 不可 |
| Phase 17 で alerting、remote write、外部 SaaS、HA/failover を完成扱いにする | merge 不可 |
| Phase 1〜16 の wire response、extension、branch、backup/restore/PITR、replication behavior を metrics 実装の都合で変更する | merge 不可 |

**Phase 17 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | metrics snapshot persistence、startup restore、periodic/shutdown flush、Prometheus endpoint、usage/quota boundary |
| `excluded_scope` | alerting、remote write、external SaaS connector、HA/failover、internal metrics engine replacement |
| `atomic_task_result` | `TASK-P17-1`〜`TASK-P17-12` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P17-1`〜`SCN-P17-21` の pass/fail、artifact path |
| `snapshot_result` | schema、atomic update、30 秒 flush、shutdown flush、startup restore、corrupt/future schema の検証結果 |
| `counter_result` | HTTP/WS/pipeline/error/query/row counter の加算点、lost increment、二重加算禁止の matrix |
| `usage_quota_result` | usage.json 正、metrics snapshot 非参照、storage gauge 整合、quota 超過時 metrics read 許可の証跡 |
| `prometheus_api_result` | status、Content-Type、Accept、auth、406、render failure の transcript |
| `prometheus_format_result` | HELP/TYPE/sample、order、escape、finite values、末尾 LF の text snapshot |
| `label_redaction_result` | route pattern、db/error labels、raw path/query/token/SQL/absolute path 非含有の scan |
| `operation_result` | periodic flush、shutdown flush、write failure、restart、concurrent read の artifact |
| `compatibility_baseline_result` | Phase 1〜16 regression、TypeScript SDK HTTP/WS transcript、Phase 16 extension transcript 差分なし |
| `secret_redaction_result` | response/log/artifact に token、JWT、SQL args、raw path、absolute path が残らない scan |
| `review_handoff_result` | 第三者が snapshot restore、Prometheus output、quota boundary、corrupt recovery、redaction を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 17 で永続 metrics と Prometheus text endpoint が利用可能になり、alerting/remote write/HA は未提供である release note |
| `precision_closure_result` | metrics snapshot persistence、Prometheus format、counter/gauge restore、usage/quota boundary、label redaction、Phase 1〜16 regression の artifact path、reviewer 再現 command、`P17-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |
