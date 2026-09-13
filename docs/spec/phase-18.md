### Phase 18：HA・自動フェイルオーバー

**目標**：primary/replica 構成で leader election、failover、split-brain 防止、write redirect を完成させる。

Phase 18 は single-leader 構成のみを対象にする。multi-primary write、distributed transaction、外部 consensus service 依存は対象外とする。

- API: `GET /ha/v1/status`、`POST /ha/v1/promote`、`POST /ha/v1/demote`（§9.5）
- 永続化: `meta/ha-state.json`
- term: 単調増加のみ許可。古い term による昇格は `HA_SPLIT_BRAIN`
- 完了条件: TC-18-1〜TC-18-7 と T18-1〜T18-7 をすべて満たす

**Phase 18 HA 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| auth | HA API は Admin token と HA token の両方必須。片方欠落は `AUTH_REQUIRED`、不一致は `AUTH_INVALID` |
| term update | promote/demote は request term が保存済み term 以上の場合だけ許可。小さい term は `HA_SPLIT_BRAIN` |
| candidate | heartbeat timeout で candidate になっても write は受けない。operator promote まで primary にならない |
| redirect | leader が分かる replica/candidate は write を leader へ 307。leader 不明なら `HA_NO_LEADER` |
| demote | primary demote 後は role `replica` または `standalone` に遷移し、write を即時停止 |
| split-brain | 自 node と異なる leader_id を同一 term で検出したら write 停止、`HA_SPLIT_BRAIN` |
| persistence | role/term/leader 更新は write 停止または開始より前に `ha-state.json` commit |
| recovery | 起動時に `role=primary` でも HA peer 確認前は write を受けず `candidate` として検証する |

**Phase 18 HA state schema：**

```json
{
  "node_id": "node-a",
  "term": 1,
  "leader_id": "node-a",
  "role": "primary",
  "last_applied_frame": 42,
  "last_heartbeat_at": "2026-09-13T00:00:00Z",
  "updated_at": "2026-09-13T00:00:00Z"
}
```

| Field | Validation |
|-------|------------|
| `node_id` | `^[a-zA-Z0-9_-]{1,64}$`。起動 flag `--ha-node-id` と一致必須 |
| `term` | 0 以上の integer。更新時は既存値以上のみ許可 |
| `leader_id` | `null` または node id。`role=primary` の場合は自 node id と一致必須 |
| `role` | `standalone` / `primary` / `replica` / `candidate` のみ |
| `last_applied_frame` | 0 以上の integer |
| `last_heartbeat_at` | `null` または RFC3339 UTC 秒精度 |

Phase 18 の leader election は外部 consensus service を使わない。promotion は `POST /ha/v1/promote` を operator が明示実行した場合のみ行う。自動 failover は primary heartbeat が `--ha-failover-timeout-ms` を超過し、replica が最新 frame に追いついている場合だけ candidate へ遷移する。candidate は operator promote なしに primary へ昇格しない。

**Phase 18 完全実装精度固定契約：**

Phase 18 は、primary/replica 構成で single-leader HA を完成させる Phase である。Phase 18 の完了判定は、promote/demote API が動くことではなく、term 単調増加、write 停止境界、leader redirect、split-brain rejection、restart recovery、operator action、primary down、network partition がすべて deterministic に証跡化されていることを必須とする。

| 項目 | 固定仕様 |
|------|----------|
| endpoint scope | `GET /ha/v1/status`、`POST /ha/v1/promote`、`POST /ha/v1/demote` の 3 API のみ |
| auth | Admin token と HA token の両方必須。Admin auth を先に評価し、次に HA token を評価する |
| auth failure | Admin token 欠落/不正は既存 auth error。HA token 欠落は 401 `AUTH_REQUIRED`、不一致は 401 `AUTH_INVALID` |
| ha enabled | HA API を有効化するには `--ha-node-id` と HA token が必須。不足時は HA API 起動不可または起動失敗 |
| node id | `^[a-z0-9-]{1,64}$` 固定。`standalone` は予約値で node_id として使わない |
| state path | `{data-dir}/meta/ha-state.json` 固定。tmp write + fsync + rename + directory fsync で更新する |
| state schema | `schema_version`、`node_id`、`term`、`leader_id`、`role`、`last_applied_frame`、`last_leader_frame`、`last_heartbeat_at`、`updated_at` を必須にする |
| startup state | state 欠損は初期 `standalone` として atomic 作成。parse 失敗、future schema、node_id 不一致、term 逆行は起動失敗 |
| term monotonic | term は保存済み値以上にしか更新できない。小さい term の promote/demote/status conflict は 409 `HA_SPLIT_BRAIN` |
| primary recovery | 起動時に保存 state が `primary` でも peer 検証前は `candidate` として write を受けない |
| role write policy | `primary` と HA disabled `standalone` のみ write 可。`replica` / `candidate` は write 不可 |
| candidate | heartbeat timeout で candidate になっても write は受けない。operator promote 成功まで primary にしない |
| promotion precondition | `last_applied_frame >= last_leader_frame`、replication apply queue empty、term valid、state commit success をすべて満たすこと |
| promotion commit | write 開始より前に `ha-state.json` を `primary` として commit する。commit 失敗時は 409 `HA_PROMOTION_FAILED` |
| demotion commit | write 停止、state commit、redirect/read policy 更新の順。state commit 失敗時も write を再開しない |
| split-brain | 同一 term で自 node と異なる leader_id を検出したら write 停止、state `candidate`、409 `HA_SPLIT_BRAIN` |
| leader unknown | leader 不明時の write は 503 `HA_NO_LEADER`。read は local state に従い許可可否を固定する |
| redirect | leader が分かる replica/candidate は mutating request を 307 + `Location` で leader へ redirect。body は空 |
| redirect target | Location は leader URL + 元 path/query。leader URL 不明なら redirect せず `HA_NO_LEADER` |
| health/status | HA status は role、node_id、term、leader_id、last_applied_frame、last_leader_frame、status を返す |
| degraded/operator | split-brain、leader unknown、promotion failed、state corruption は operator action を必要とする状態として出す |
| network partition | partition 検出時は candidate へ降格し write 停止。自動 primary 昇格は禁止 |
| multi-primary | multi-primary write、distributed transaction、external consensus は Phase 18 対象外 |
| observability | logs/metrics は node_id、role、term、leader_id、result、error_code まで。HA token、JWT、SQL args、frame bytes は出力禁止 |
| phase boundary | Phase 18 は HA/failover まで。libSQL 内製化、storage adapter、multi-primary write は Phase 19 以降 |

**Phase 18 原子タスク台帳：**

| Task ID | 目的 | 変更対象 | 完了条件 | 失敗時の扱い | 必須成果物 |
|---------|------|----------|----------|--------------|------------|
| `TASK-P18-1` | HA config / auth を固定する | config / HA routes | node_id、HA token、Admin+HA auth、invalid config が固定される | token 混同禁止 | auth/config transcript |
| `TASK-P18-2` | ha-state schema を固定する | `meta/ha-state.json` | schema、atomic update、node mismatch、future schema、term validation が検証される | 自動修復禁止 | state fixture |
| `TASK-P18-3` | HA status API を固定する | `/ha/v1/status` | role、term、leader、status、operator action が返る | secret 出力禁止 | status snapshot |
| `TASK-P18-4` | promote API を固定する | `/ha/v1/promote` | precondition、term、state commit、write start 境界が固定される | success-before-commit 禁止 | promote trace |
| `TASK-P18-5` | demote API を固定する | `/ha/v1/demote` | write stop、state commit、redirect/read policy が固定される | demote 失敗後 write 再開禁止 | demote trace |
| `TASK-P18-6` | candidate / failover を固定する | HA coordinator | heartbeat timeout、candidate 遷移、operator promote 必須が再現できる | 自動 primary 昇格禁止 | failover artifact |
| `TASK-P18-7` | redirect / no leader を固定する | router / middleware | 307 Location、body 空、HA_NO_LEADER、read/write policy が固定される | local write fallback 禁止 | redirect transcript |
| `TASK-P18-8` | split-brain rejection を固定する | HA safety | same term different leader、stale term、partition を拒否する | multi leader write 禁止 | split-brain artifact |
| `TASK-P18-9` | restart recovery を固定する | startup recovery | saved primary starts as candidate、corrupt state failure、state recovery が deterministic | stale primary write 禁止 | recovery fixture |
| `TASK-P18-10` | replication readiness を固定する | replication / HA bridge | last_applied_frame、leader frame、apply queue empty の判定が固定される | lagging replica promote 禁止 | readiness matrix |
| `TASK-P18-11` | observability / redaction を固定する | logs / metrics / artifacts | token、JWT、SQL args、frame bytes が残らない | secret scan failure は未完了 | redaction scan |
| `TASK-P18-12` | Phase 1〜17 regression を閉じる | tests | HTTP/WS、replication、backup/restore/PITR、branch、extension、metrics に差分なし | HA 実装で既存 contract を壊さない | regression report |

**Phase 18 シナリオマトリクス：**

| Scenario ID | Given | When | Then | Artifact |
|-------------|-------|------|------|----------|
| `SCN-P18-1` | HA disabled | HA API を呼ぶ | 起動不可または 404/501 の既定 unsupported。成功応答しない | `phase18_ha_disabled.json` |
| `SCN-P18-2` | Admin token 欠落 | HA API を呼ぶ | 401 auth error | `phase18_admin_auth_required.json` |
| `SCN-P18-3` | HA token 欠落/不一致 | HA API を呼ぶ | 401 `AUTH_REQUIRED` / `AUTH_INVALID` | `phase18_ha_auth.json` |
| `SCN-P18-4` | valid state | GET status | role、node_id、term、leader_id、frames、status が返る | `phase18_status.json` |
| `SCN-P18-5` | stale request term | promote/demote | 409 `HA_SPLIT_BRAIN` | `phase18_stale_term.json` |
| `SCN-P18-6` | replica is caught up | operator promote | state commit 後 primary、write 可 | `phase18_promote_success.json` |
| `SCN-P18-7` | replica lagging | operator promote | 409 `HA_PROMOTION_FAILED`、write 不可 | `phase18_promote_lagging.json` |
| `SCN-P18-8` | apply queue non-empty | operator promote | 409 `HA_PROMOTION_FAILED` | `phase18_promote_queue.json` |
| `SCN-P18-9` | state commit failure | promote | 409 `HA_PROMOTION_FAILED`、write 不可 | `phase18_promote_commit_failure.json` |
| `SCN-P18-10` | primary | demote | write 即時停止、state commit、role replica/standalone | `phase18_demote_success.json` |
| `SCN-P18-11` | demote state commit failure | demote | write は再開しない、operator action required | `phase18_demote_commit_failure.json` |
| `SCN-P18-12` | heartbeat timeout | coordinator tick | candidate へ遷移、write 不可、自動 primary 昇格なし | `phase18_candidate_timeout.json` |
| `SCN-P18-13` | leader unknown | write request | 503 `HA_NO_LEADER` | `phase18_no_leader_write.json` |
| `SCN-P18-14` | leader known replica | write request | 307 Location、body 空 | `phase18_redirect.json` |
| `SCN-P18-15` | same term different leader | status/promote/write | write 停止、409 `HA_SPLIT_BRAIN` | `phase18_split_brain.json` |
| `SCN-P18-16` | network partition | heartbeat/status | candidate/operator_required、write 不可 | `phase18_partition.json` |
| `SCN-P18-17` | saved state primary | restart | candidate として起動し peer 検証前 write 不可 | `phase18_restart_primary_candidate.json` |
| `SCN-P18-18` | corrupt ha-state | startup | 起動失敗 | `phase18_corrupt_state.json` |
| `SCN-P18-19` | node_id mismatch | startup | 起動失敗 | `phase18_node_mismatch.json` |
| `SCN-P18-20` | future schema | startup | 起動失敗 | `phase18_future_schema.json` |
| `SCN-P18-21` | logs/artifacts がある | secret scan | HA token、JWT、SQL args、frame bytes が残らない | `phase18_secret_scan.json` |
| `SCN-P18-22` | Phase 17 metrics | Phase 18 実装後に regression | metrics snapshot / Prometheus に差分なし | `phase18_phase17_regression.json` |
| `SCN-P18-23` | internal adapter flag | Phase 18 状態で使用 | Phase 19 まで production path 切替不可 | `phase18_phase19_boundary.json` |

**Phase 18 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| candidate から operator promote なしに primary へ自動昇格する | merge 不可 |
| `ha-state.json` commit 前に write を開始する | merge 不可 |
| demote 失敗後に write を再開する | merge 不可 |
| 古い term を採用する | merge 不可 |
| same term different leader を WARN だけで継続する | merge 不可 |
| split-brain 検出後も write を受ける | merge 不可 |
| leader 不明時に local write fallback する | merge 不可 |
| replica/candidate write redirect を 200 JSON や 302 に変える | review failure |
| HA token、JWT、SQL args、frame bytes を response/log/artifact に出す | merge 不可 |
| HA token と Admin token を同一 secret として扱う | merge 不可 |
| Phase 18 で multi-primary write、distributed transaction、external consensus、libSQL 内製化を完成扱いにする | merge 不可 |
| Phase 1〜17 の wire response、metrics、extension、branch、backup/restore/PITR、replication behavior を HA 実装の都合で変更する | merge 不可 |

**Phase 18 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | HA status、promote、demote、ha-state persistence、candidate、redirect、split-brain rejection、restart recovery |
| `excluded_scope` | multi-primary write、distributed transaction、external consensus service、libSQL 内製化、storage adapter |
| `atomic_task_result` | `TASK-P18-1`〜`TASK-P18-12` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P18-1`〜`SCN-P18-23` の pass/fail、artifact path |
| `auth_config_result` | Admin token、HA token、missing/bad token、node_id、failover timeout、invalid config の matrix |
| `state_result` | schema、atomic update、term monotonic、node mismatch、future/corrupt state、restart recovery の検証結果 |
| `promotion_result` | caught-up、lagging、queue non-empty、stale term、commit failure、write start boundary の trace |
| `demotion_result` | write stop、state commit、commit failure、post-demote routing/read/write policy の trace |
| `redirect_result` | leader known 307 Location/body 空、leader unknown `HA_NO_LEADER`、read/write behavior の transcript |
| `split_brain_result` | same term different leader、network partition、stale term、write stop、operator_required の artifact |
| `replication_readiness_result` | last_applied_frame、last_leader_frame、apply queue empty、primary down の matrix |
| `compatibility_baseline_result` | Phase 1〜17 regression、TypeScript SDK HTTP/WS transcript、Phase 17 metrics transcript 差分なし |
| `secret_redaction_result` | response/log/artifact に HA token、JWT、SQL args、frame bytes が残らない scan |
| `review_handoff_result` | 第三者が promote/demote/redirect/split-brain/restart/partition を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 18 で HA status/promote/demote と single-leader failover が利用可能になり、multi-primary/内製化は未提供である release note |
| `precision_closure_result` | HA state、term monotonic、promote/demote、redirect/no leader、split-brain rejection、restart recovery、Phase 1〜17 regression の artifact path、reviewer 再現 command、`P18-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |
