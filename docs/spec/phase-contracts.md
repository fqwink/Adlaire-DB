# Phase 契約仕様

Phase 別完了ゲート、API 実装決定表、endpoint / 永続化 / error / test / security 契約、Phase 詳細仕様入口を固定する仕様である。

### 9.2 Phase 別完了ゲート

以下は各 Phase の最終判定条件である。ここに書かれた項目は「推奨」ではなく、Phase 完了の必須条件とする。

| Phase | 完了ゲート | 明示的な対象外 |
|-------|------------|----------------|
| Phase 1 | Cargo workspace が成立し、`adlaire-db serve` / `adlaire-db token create` の CLI skeleton が起動する。`--data` 未指定は clap のエラーになる。config.toml は CLI > TOML > default で解決される | DB オープン、HTTP サーバー、JWT 発行 |
| Phase 2 | `--data` 配下に `databases/`、`meta/`、`.lock` が準備され、`default/data.db` を libsql local で開ける。WAL、busy_timeout、synchronous=NORMAL、integrity_check が起動時に適用される | HTTP API、認証、マルチ DB CRUD |
| Phase 3 | `GET /v2/health` と `POST /v2/pipeline` が hrana-http v2 互換で動く。JSON 不正は HTTP 400、SQL エラーは HTTP 200 + hrana error。Web フレームワーク依存がない | JWT 認証、WebSocket、管理 API 実装、マルチ DB 完了判定 |
| Phase 4 | HS256 JWT 検証、`token create`、tokens.json 追記、revoke 照合、ro/rw 権限チェックが動く。認証なし・不正・期限切れ・失効済みの各エラーが §7.3 と一致する | DB スコープ JWT、管理 API token CRUD、自動化された全 revoke フロー |
| Phase 5 | 構造化 JSON ログ、HTTP request ログ、Phase 1〜5 の統合テスト、TypeScript SDK 互換テスト、再起動後の永続化テストが通る | マルチ DB、WebSocket、replication、backup |
| Phase 6 | `/{db-name}/v2/pipeline` が動き、DB 名バリデーション、DbManager の create/list/get/delete 内部機構、databases.json のアトミック更新が動く。`/v2/pipeline` は default fallback のまま維持される | 管理 API route の完全実装、DB スコープ JWT、backup |
| Phase 7 | 管理 API の DB CRUD、token CRUD、DB スコープ JWT、管理 API 認証、revoke 即時反映が動く。全管理 API は §6.4 の status/body に一致する | WebSocket、ATTACH、replication、backup |
| Phase 8 | Turso Cloud 互換管理モデルとして location、organization/group、quota/usage の API、metadata、migration、権限、quota 判定、エラー、`/v1/*` Platform API 互換 response wrapper が Phase 8 詳細節の契約通り実装される。既存 Phase 1〜7 の API と metadata migration は後方互換を維持する | WebSocket、ATTACH、replication、backup、branch、SQLite 拡張、内製化 |
| Phase 9 | hrana-ws v3 の hello/open_stream/execute/sequence/close_stream/store_sql/close_sql が動き、同一 stream 内の interactive transaction が同一接続で保持される | ATTACH、metrics、replication、backup |
| Phase 10 | 管理下 DB のみを対象に ATTACH が動き、任意パス ATTACH を拒否する。metrics API は counters/gauges を返し、HTTP/DB/WebSocket 経路から値が更新される | WAL replication、backup、branch |
| Phase 11 | primary role で replication API（log/snapshot/heartbeat/status）が起動し、WAL frame 番号、CRC32、snapshot header が仕様通り返る。replica 受信・適用はまだ完了条件に含めない | replica 同期完了、書き込みリダイレクト、WAL archive retention |
| Phase 12 | replica が primary から snapshot/WAL を取得して追いつき、replica 書き込みは 307 redirect または primary 到達不能時の規定エラーになる。health に role/lag が出る | WAL archive、PITR、branch |
| Phase 13 | WAL archive と manifest.json がアトミックに更新され、retention cleanup が動く。CRC32 と manifest/files の整合性検査がある | backup/restore API、PITR restore、branch |
| Phase 14 | backup、restore、PITR API が動き、restore 失敗時は元 DB が復元される。PITR 無効、範囲外、CRC 不一致のエラーが §7.3 と一致する | branch、外部ストレージ転送、HA |
| Phase 15 | branch 作成、一覧、削除、再起動後復元が動く。branch DB は `{db}___{branch}` として通常 DB と同じ pipeline でアクセスでき、元 DB と独立して書き込める | branch merge、copy-on-write 最適化、SQLite 拡張 |
| Phase 16 | SQLite 拡張ロードが動き、許可ディレクトリ、拡張 manifest、署名検証、ロード/アンロード API、sandbox 方針が固定される。未承認拡張と任意パスロードは拒否する | HA、自動 failover、libSQL 内製化、未署名拡張 |
| Phase 17 | metrics snapshot を永続化し、Prometheus text endpoint と usage/quota の整合が動く。再起動後も累積 counter が復元される | HA、自動 failover、libSQL 内製化 |
| Phase 18 | primary/replica 構成で leader election、failover、split-brain 防止、昇格/降格、health/redirect が仕様通り動く | libSQL 内製化、multi-primary write |
| Phase 19 | WAL checkpoint 制御、storage 境界、query executor 境界のうち採用対象を内製 crate へ段階移行し、Turso Cloud / libSQL SDK 互換テストが通る | SQL parser 完全内製、互換性を壊す wire/API 変更 |

### 9.3 API 実装決定表

API を実装する場合は、各 endpoint について必ず次を仕様本文または該当 Phase に明記する。

| 項目 | 必須記述 |
|------|----------|
| 認証 | 不要 / JWT 必須 / Admin token 必須 / replication token 必須 |
| HTTP method/path | method、path parameter、query parameter、末尾 slash の扱い |
| request body | JSON schema、必須/任意/null 可、unknown field の扱い |
| success response | status、headers、body schema、空 body かどうか |
| error response | status、`code`、message の粒度、部分成功があるか |
| 永続化 | 変更するファイル、アトミック更新要否、失敗時 rollback |
| ログ | INFO/WARN/ERROR の発火条件、秘匿する値 |
| テスト | 正常系、異常系、権限系、再起動後確認 |

**デフォルト決定：**

- 管理 API、Turso Platform API、destructive API、永続化 API の unknown JSON field は原則 `INVALID_REQUEST` とする。互換のため unknown field を無視できるのは hrana wire protocol など、該当節に明記した場合のみとする
- request body が空であるべき API に body がある場合は、body を無視せず `INVALID_REQUEST` とする
- path parameter は URL decode 後にバリデーションする
- 管理 API の成功レスポンスは作成 `201`、削除 `204`、取得/一覧 `200` を原則とする
- 非同期ジョブを導入する場合は、job id、status endpoint、再起動後の扱いを先に仕様化する

### 9.4 Phase 別実装契約

各 Phase の実装者は、この表の契約を満たすこと。既存の詳細節と矛盾がある場合は、この表を優先し、矛盾箇所を同時に修正する。

各 Phase の実装 PR は、この表の該当 Phase 行を §9.1.32 の Phase implementation packet に転記し、API / 永続化 / error / test / unsupported behavior の完了条件として固定してから実装する。

#### Phase 1〜5：単一 DB・HTTP・認証基盤

| Phase | 変更対象 | API/CLI 契約 | 永続化 | エラー/ログ | テスト契約 |
|-------|----------|--------------|--------|-------------|------------|
| Phase 1 | `Cargo.toml`, `adlaire-server/Cargo.toml`, `cli.rs`, `config.rs`, `main.rs` | `serve` と `token create` を clap subcommand として定義する。`serve --data` は必須。`token create` は Phase 4 までは stub でよいが、引数 validation は行う | なし | CLI parse error は clap の標準エラー。config parse error は起動失敗 | `cargo build`, `cargo test`, `adlaire-db --help`, `adlaire-db serve --help` |
| Phase 2 | `data_dir.rs`, `db/sqld_adapter.rs`, `db/manager.rs`, `db/meta.rs` | 外部 HTTP API はまだ提供しない。`run_serve` 内で `default/data.db` を開けること | `meta/databases.json`, `meta/tokens.json`, `meta/branches.json`, `.lock`, `databases/default/data.db` を初期化する | lock 取得失敗、metadata parse 失敗、integrity_check 失敗は起動失敗。skip_integrity_check は WARN | 初回起動、再起動、二重起動拒否、DB ファイル作成、WAL 設定確認 |
| Phase 3 | `http/mod.rs`, `http/pipeline.rs`, `http/health.rs`, `hrana/*`, `error.rs`, `main.rs` | `GET /v2/health`, `POST /v2/pipeline` のみ Phase 完了対象。`/v2/pipeline` は `default` DB 固定 | Phase 2 の永続化を継続。SQL 成功応答前に SQLite/libsql の commit が完了していること | malformed JSON は HTTP 400。SQL エラーは HTTP 200 + hrana error。HTTP request log は Phase 5 まで必須ではない | TC-1, TC-2, TC-6。`named_args` 非空、invalid JSON、SQL error、close 後無視を含める |
| Phase 4 | `auth/*`, `token/*`, `config.rs`, `main.rs`, `http/pipeline.rs` | `Authorization: Bearer <JWT>` を検証する。`token create` は JWT を stdout に出す。認証無効モードは secret 未指定時のみ | `meta/tokens.json` に token record を追記する。追記は atomic update | `AUTH_REQUIRED`, `AUTH_INVALID`, `AUTH_EXPIRED`, `PERMISSION_DENIED` を §7.3 通り返す。JWT/token secret はログ出力禁止 | TC-3。valid/none/bad/expired/revoked/ro-write を含める |
| Phase 5 | `tests/*`, logging middleware, `metrics.rs` stub | API 追加は禁止。既存 API の互換性と運用ログを固める | 新規永続化なし。既存 DB の再起動後永続性を検証する | JSON Lines logs。method/path/status/duration_ms を記録し、Authorization と SQL args は出さない | TC-1〜TC-6、TypeScript SDK 互換、再起動後 SELECT、ログ形式検証 |

**Phase 1 完全実装精度固定契約：**

Phase 1 は「実行可能な CLI skeleton と config 解決の土台」を完成させる Phase であり、DB、HTTP、JWT、metadata 永続化を開始してはならない。Phase 1 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 1 固定仕様 |
|------|------------------|
| 実装範囲 | Cargo workspace、binary 起動、`serve` / `token create` subcommand、help、CLI parse、config resolution |
| binary name | `adlaire-db` 固定。別名 binary を Phase 1 完了根拠にしてはならない |
| `serve --data` | 必須。未指定は clap 標準 error、非 0 exit。server startup へ進まない |
| `serve --config` | 任意。指定時は TOML parse を行い、parse error は起動失敗。存在しない file は config error |
| config precedence | CLI > env > TOML > default。Phase 1 ではこの順位の fixture を必ず作る |
| env 対象 | Phase 1 で読む env は config 解決に必要な最小項目だけ。secret / token env は読んでも JWT 発行に使わない |
| `token create` | subcommand と引数 validation だけを実装する。Phase 1 では token 文字列を発行せず、stub response または unsupported error を固定する |
| stdout/stderr | help は stdout、parse/config error は stderr。token secret、raw path の不要な展開値は出力しない |
| 永続化 | Phase 1 は永続化なし。`meta/`、`databases/`、`.lock`、`tokens.json`、`data.db` を作成しない |
| process side effect | help / invalid flag / config parse の各 scenario で data-dir 配下に file を作らない |
| log | Phase 1 では構造化 request log は対象外。CLI error は clap/config error のみ |
| 完了条件 | build/test、help snapshot、invalid flag stderr、config precedence fixture、no persistence evidence、review handoff が揃う |

**Phase 1 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P1-1` | Cargo workspace と binary skeleton を成立させる | §9.2 Phase 1、§9.4 Phase 1 | `Cargo.toml`、crate manifest、`main.rs` | DB open、HTTP server、metadata file | `cargo build` が成功し `adlaire-db --help` が起動 | `cargo build`; `adlaire-db --help` |
| `TASK-P1-2` | `serve` subcommand と `--data` 必須 validation | §9.1.20、§9.1.49 | `cli.rs` | data-dir 作成、lock 取得、DB 初期化 | `serve --help` に `--data` が表示され、未指定は非 0 exit | `adlaire-db serve --help`; `adlaire-db serve` |
| `TASK-P1-3` | `token create` subcommand skeleton | §9.2 Phase 1、§9.4 Phase 1 | `cli.rs` | JWT 発行、`tokens.json` 書き込み、secret 永続化 | help と引数 validation が動き、発行処理は Phase 4 まで未対応として固定 | `adlaire-db token create --help` |
| `TASK-P1-4` | TOML/env/default config model | §9.1.19、§9.1.42 | `config.rs` | DB/HTTP/JWT 起動 side effect | CLI > env > TOML > default の fixture が pass | config precedence test |
| `TASK-P1-5` | error surface と stdout/stderr snapshot | §7.3、§9.1.22、§9.1.35 | `cli.rs`、`config.rs` | 独自不定形 error、secret 出力 | help/invalid/config error の snapshot が固定 | help / stderr snapshot |
| `TASK-P1-6` | no persistence evidence | §9.1.21、§9.1.41、§9.1.44 | test / artifact | `meta/`、`databases/`、`.lock`、`data.db` 作成 | 全 Phase 1 scenario 後に data-dir が未作成または空である証跡 | no persistence fixture |
| `TASK-P1-7` | Phase 1 Done receipt / review handoff | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が build/help/config/no persistence を再現できる | handoff checklist |

**Phase 1 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P1-1` | `adlaire-db --help` | exit 0、stdout に top-level commands、stderr empty | help snapshot |
| `SCN-P1-2` | `adlaire-db serve --help` | exit 0、stdout に `--data` / `--config`、stderr empty | serve help snapshot |
| `SCN-P1-3` | `adlaire-db token create --help` | exit 0、stdout に token create の引数、stderr empty | token help snapshot |
| `SCN-P1-4` | `adlaire-db serve` | exit non-zero、clap error、data-dir side effect なし | invalid flag stderr + no persistence |
| `SCN-P1-5` | invalid `--config` path | exit non-zero、config error、DB/HTTP/JWT は起動しない | config error snapshot |
| `SCN-P1-6` | CLI/env/TOML/default が同時指定 | CLI 値が勝ち、env、TOML、default は losing behavior として発火しない | precedence fixture |
| `SCN-P1-7` | `token create` 実行 | JWT を発行せず、Phase 1 stub/unsupported として固定された出力または error | token stub snapshot |

**Phase 1 禁止事項：**

| 状態 | 判定 |
|------|------|
| `serve --data` で DB open まで進む | merge 不可。DB open は Phase 2 |
| HTTP listener を bind する | merge 不可。HTTP は Phase 3 |
| JWT を発行する、または `tokens.json` を作る | merge 不可。JWT/token persistence は Phase 4 |
| `{data-dir}/databases/default/data.db`、`meta/`、`.lock` を作る | merge 不可。data-dir 初期化は Phase 2 |
| Phase 2 以降の metadata schema を先に作る | merge 不可 |
| help / error snapshot なしで CLI 契約を完了扱いにする | Phase 未完了 |
| PR description だけで config precedence / no persistence を説明する | 仕様として扱わない |

**Phase 1 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | Cargo workspace、CLI skeleton、config resolution、stub token create |
| `excluded_scope` | DB open、HTTP server、JWT 発行、tokens.json、data.db、metadata 初期化 |
| `atomic_task_result` | `TASK-P1-1`〜`TASK-P1-7` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P1-1`〜`SCN-P1-7` の pass/fail、artifact path |
| `coverage_closure_result` | help、invalid flag、config precedence、no persistence の coverage gap 0 件 |
| `review_handoff_result` | 第三者が build/help/config/no persistence を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 1 は CLI skeleton 追加のみ。DB/HTTP/JWT は利用不可である release note |
| `precision_closure_result` | help/config/no persistence/stub token の artifact path、reviewer 再現 command、`P1-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 2 完全実装精度固定契約：**

Phase 2 は data-dir、単一 default DB、metadata 初期値、process lock、libSQL open を完成させる Phase である。外部 HTTP API、JWT、管理 API、マルチ DB routing はまだ公開してはならない。Phase 2 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 2 固定仕様 |
|------|------------------|
| 実装範囲 | `{data-dir}` tree 初期化、`.lock` 排他、metadata 初期化、`default/data.db` open、WAL / busy_timeout / synchronous / integrity_check |
| directory layout | `{data-dir}/meta/`、`{data-dir}/databases/default/` を作成する。permission は §10 の方針に従い、危険な permission は WARN |
| process lock | `{data-dir}/.lock` を open + flock `LOCK_EX | LOCK_NB`。取得失敗は起動失敗。`.lock` の内容に意味を持たせない |
| metadata initial schema | `meta/databases.json={"databases":[]}`、`meta/tokens.json={"tokens":[]}`、`meta/branches.json={"branches":[]}` を初期値とする |
| metadata parse failure | 既存 metadata が parse 不可なら起動失敗。空初期値で上書きしてはならない |
| default DB path | `{data-dir}/databases/default/data.db` 固定。Phase 2 では DB 名変更、複数 DB、path-based routing を実装しない |
| SQLite/libSQL settings | open 後に WAL、busy_timeout、synchronous=NORMAL、integrity_check を適用する |
| integrity check | `--skip-integrity-check=false` なら `PRAGMA integrity_check` が `ok` の場合だけ起動成功。skip 時は WARN を出す |
| HTTP/API exposure | Phase 2 は HTTP listener を bind しない。`/v2/health`、`/v2/pipeline`、管理 API は成功応答を返さない |
| restart behavior | 同じ data-dir で再起動して同じ default DB を open し、metadata を保持する |
| rollback / recovery | 初期化途中失敗時は成功扱いにしない。partial metadata は次回起動で parse できる形式か、起動失敗にする |

**Phase 2 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P2-1` | data-dir tree を初期化する | §8.1、§9.1.21、§9.6 | `data_dir.rs` | HTTP bind、DB routing | `meta/` と `databases/default/` が作成される | tree fixture |
| `TASK-P2-2` | process lock を取得し二重起動を拒否する | §9.1.16、§9.1.41 | `data_dir.rs` | stale lock 削除、lock 無視 | 2 process 目が非 0 exit | lock conflict test |
| `TASK-P2-3` | metadata 初期値を atomic に作る | §9.1.21、§9.1.42、§9.6 | `db/meta.rs` | parse 失敗時の空上書き | 3 metadata file が schema 通り存在 | metadata fixture |
| `TASK-P2-4` | default DB を libSQL local で open する | §3.2、§8.1、§9.1.37 | `db/sqld_adapter.rs` | multi DB、HTTP API | `databases/default/data.db` が open される | DB open test |
| `TASK-P2-5` | WAL / busy_timeout / synchronous を適用する | §3.6、§9.1.35 | `db/sqld_adapter.rs` | unsupported PRAGMA 先送り | PRAGMA 結果が snapshot と一致 | PRAGMA snapshot |
| `TASK-P2-6` | integrity_check と skip warning を固定する | §7.3、§9.1.22、§9.1.37 | `db/sqld_adapter.rs`、`config.rs` | integrity 失敗を成功扱い | ok なら起動成功、NG なら起動失敗、skip は WARN | integrity fixture |
| `TASK-P2-7` | restart persistence を確認する | §9.1.39、§9.1.44 | test / artifact | DB 再作成、metadata 初期化し直し | 再起動後も同じ default DB と metadata | restart fixture |
| `TASK-P2-8` | Phase 2 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が init/lock/open/restart を再現できる | handoff checklist |

**Phase 2 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P2-1` | empty data-dir で `serve --data` | tree、metadata、default DB、`.lock` が作成され起動成功 | tree snapshot |
| `SCN-P2-2` | 同じ data-dir で二重起動 | 2 process 目は起動失敗、既存 process は継続 | lock conflict log |
| `SCN-P2-3` | 再起動 | metadata を保持し default DB を再 open | restart transcript |
| `SCN-P2-4` | 壊れた `databases.json` | 起動失敗。空 metadata で上書きしない | metadata parse error fixture |
| `SCN-P2-5` | default DB open failure | 起動失敗。HTTP/API は公開されない | DB open error snapshot |
| `SCN-P2-6` | integrity_check `ok` | 起動成功 | integrity ok artifact |
| `SCN-P2-7` | integrity_check failure | 起動失敗。read/write 成功応答なし | integrity failure artifact |
| `SCN-P2-8` | `--skip-integrity-check` | 起動成功可、WARN log 必須 | skip warning log |
| `SCN-P2-9` | Phase 2 起動中に HTTP endpoint へ接続試行 | HTTP listener なし、成功応答なし | no HTTP exposure evidence |

**Phase 2 禁止事項：**

| 状態 | 判定 |
|------|------|
| HTTP listener を bind する、または `/v2/health` / `/v2/pipeline` が成功応答を返す | merge 不可。HTTP は Phase 3 |
| JWT 検証、JWT 発行、`token create` の実発行を行う | merge 不可。JWT は Phase 4 |
| 管理 API route を追加する | merge 不可。管理 API は Phase 7 以降 |
| default 以外の DB routing、DB create/list/delete を公開する | merge 不可。マルチ DB は Phase 6 |
| metadata parse 失敗時に空 JSON で上書きする | merge 不可 |
| lock 取得失敗、DB open 失敗、integrity_check 失敗を起動成功扱いにする | merge 不可 |
| `.lock` file の削除を lock 解放手段として要求する | review failure。flock を正とする |
| PR description だけで WAL / integrity / restart を説明する | 仕様として扱わない |

**Phase 2 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | data-dir 初期化、process lock、metadata 初期値、default DB open、WAL/busy_timeout/synchronous/integrity_check |
| `excluded_scope` | HTTP API、JWT、管理 API、multi DB routing、replication、backup |
| `atomic_task_result` | `TASK-P2-1`〜`TASK-P2-8` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P2-1`〜`SCN-P2-9` の pass/fail、artifact path |
| `resource_lifecycle_result` | data-dir、lock、metadata、default DB の state / transition / recovery 証跡 |
| `coverage_closure_result` | tree、metadata、lock、DB open、PRAGMA、integrity、restart、no HTTP exposure の coverage gap 0 件 |
| `review_handoff_result` | 第三者が init/lock/open/restart/no HTTP を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 2 で data-dir と default DB が作成されるが外部 HTTP API はまだ使えない release note |
| `precision_closure_result` | data-dir/lock/metadata/default DB/WAL/integrity/restart/no HTTP の artifact path、reviewer 再現 command、`P2-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 3 完全実装精度固定契約：**

Phase 3 は libSQL client SDK が HTTP 経由で最小 SQL 実行できる hrana-http v2 surface を完成させる Phase である。Phase 3 で公開してよい外部 API は `GET /v2/health` と `POST /v2/pipeline` のみであり、JWT、WebSocket、管理 API、path-based DB routing は実装してはならない。Phase 3 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 3 固定仕様 |
|------|------------------|
| 実装範囲 | hyper HTTP server、route、`GET /v2/health`、`POST /v2/pipeline`、hrana-http v2 JSON、default DB SQL 実行 |
| HTTP framework | hyper ベースの自前 routing。axum、actix-web、rocket 等の web framework 導入は禁止 |
| health response | `GET /v2/health` は 200 JSON `{"status":"ok"}` 固定。Phase 12 までは role/lag を追加しない |
| pipeline route | `POST /v2/pipeline` のみ。`/{db-name}/v2/pipeline` は Phase 6 まで成功応答禁止 |
| auth | Phase 3 は認証無効 mode のみ。JWT secret が設定されている場合は Phase 4 完了前のため起動拒否または auth unavailable とする |
| default DB | すべての SQL は Phase 2 の `default/data.db` へ実行する |
| request body | top-level JSON object 必須。malformed JSON、array、scalar、null、必須 field 欠落は HTTP 400 `INVALID_REQUEST` |
| baton/base_url | request の `baton` は受け取るが session は保持しない。response の `baton` / `base_url` は常に `null` |
| request types | Phase 3 は `execute`、`sequence`、`close` を対象。unknown type は HTTP 400 または当該 item error とし、成功扱いしない |
| `execute` | positional `args` を bind し、`want_rows` に従って rows / rows_affected / last_insert_rowid を返す |
| `named_args` | 空配列または省略のみ許可。非空は当該 item を `results[].type="error"` とする |
| SQL error | SQL 実行 error / constraint error は HTTP 200 のまま `results[i].type="error"` として返す |
| close behavior | `close` 後の request は処理せず、追加 result を返さない |
| persistence | write SQL の commit が完了してから success response を返す。再起動後に書き込みが残ることを証跡化する |

**Phase 3 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P3-1` | hyper HTTP server と route を追加する | §9.2 Phase 3、§9.4 Phase 3 | `http/mod.rs`、`main.rs` | 管理 API、WebSocket、JWT | `/v2/health` と `/v2/pipeline` だけ route される | route snapshot |
| `TASK-P3-2` | health endpoint を固定する | §9.5、§9.1.42 | `http/health.rs` | role/lag 追加 | 200 `{"status":"ok"}` | health snapshot |
| `TASK-P3-3` | hrana request schema を定義する | §6.2、§9.1.20、§9.1.42 | `hrana/*` | unknown success、SDK 非互換 shape | malformed / missing field が規定 error | schema tests |
| `TASK-P3-4` | execute request を実装する | §9.1.28、§9.1.35 | `http/pipeline.rs`、`hrana/*` | named_args 成功、HTTP 500 SQL error | SELECT/INSERT が hrana result になる | pipeline success snapshot |
| `TASK-P3-5` | sequence request を実装する | §9.1.28 | `http/pipeline.rs` | 自前 SQL split | batch 実行は libSQL に委譲し result 順序を固定 | sequence snapshot |
| `TASK-P3-6` | close behavior を固定する | §9.1.28、§9.1.43 | `http/pipeline.rs` | close 後 request 実行 | close 後の item が処理されない | close fixture |
| `TASK-P3-7` | SQL / JSON error surface を固定する | §7.3、§9.1.22 | `error.rs`、`http/pipeline.rs` | SQL error を HTTP 400/500 化 | malformed JSON は 400、SQL error は 200 + item error | error snapshots |
| `TASK-P3-8` | restart persistence / SDK smoke を固定する | §9.1.35、§9.1.47 | tests / artifact | in-memory only 成功 | INSERT 後再起動 SELECT、SDK smoke pass | SDK transcript |
| `TASK-P3-9` | Phase 3 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が health/pipeline/error/restart を再現できる | handoff checklist |

**Phase 3 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P3-1` | `GET /v2/health` | 200 JSON `{"status":"ok"}` | health snapshot |
| `SCN-P3-2` | `POST /v2/pipeline` execute `SELECT 1` | HTTP 200、`results[0].type="ok"`、hrana value 形式 | select snapshot |
| `SCN-P3-3` | CREATE / INSERT / SELECT pipeline | HTTP 200、write 後 read 可能 | CRUD transcript |
| `SCN-P3-4` | malformed JSON body | HTTP 400 `INVALID_REQUEST` | malformed snapshot |
| `SCN-P3-5` | top-level array/scalar/null | HTTP 400 `INVALID_REQUEST` | body shape snapshot |
| `SCN-P3-6` | SQL syntax error | HTTP 200、当該 item が `type="error"` | SQL error snapshot |
| `SCN-P3-7` | constraint error | HTTP 200、当該 item が `type="error"` | constraint snapshot |
| `SCN-P3-8` | `named_args` 非空 | HTTP 200、当該 item が `type="error"` | named args snapshot |
| `SCN-P3-9` | `sequence` request | HTTP 200、sequence 全体の success/error が固定 | sequence snapshot |
| `SCN-P3-10` | `close` 後に追加 request | close 後 request は処理されず追加 result なし | close snapshot |
| `SCN-P3-11` | `/{db-name}/v2/pipeline` | Phase 3 では成功応答なし | no multi DB route evidence |
| `SCN-P3-12` | INSERT 後 restart して SELECT | 書き込みが残る | restart persistence transcript |

**Phase 3 禁止事項：**

| 状態 | 判定 |
|------|------|
| JWT 認証、JWT 検証、token 発行を実装する | merge 不可。JWT は Phase 4 |
| WebSocket / `/v3/baton` を実装する | merge 不可。WebSocket は Phase 9 |
| `/{db-name}/v2/pipeline` を成功応答にする | merge 不可。multi DB routing は Phase 6 |
| 管理 API route を追加する | merge 不可。管理 API は Phase 7 |
| SQL error を HTTP 400 / 500 に変換する | merge 不可 |
| malformed JSON を HTTP 200 の hrana error にする | merge 不可 |
| `named_args` 非空を成功扱いにする | merge 不可 |
| unknown request type を成功扱いにする | merge 不可 |
| `baton` session を保持する | Phase 未完了。session は Phase 9 |
| SDK transcript / wire snapshot なしで完了扱いにする | Phase 未完了 |

**Phase 3 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | hyper HTTP server、`GET /v2/health`、`POST /v2/pipeline`、hrana execute/sequence/close、default DB SQL |
| `excluded_scope` | JWT、WebSocket、管理 API、multi DB route、ATTACH、replication、backup |
| `atomic_task_result` | `TASK-P3-1`〜`TASK-P3-9` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P3-1`〜`SCN-P3-12` の pass/fail、artifact path |
| `compatibility_baseline_result` | hrana-http v2 / libSQL SDK transcript、Turso 差分分類 |
| `coverage_closure_result` | health、pipeline success/error、malformed JSON、SQL error、close、restart、SDK smoke の gap 0 件 |
| `review_handoff_result` | 第三者が health/pipeline/error/restart/SDK smoke を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 3 で HTTP API `/v2/health` と `/v2/pipeline` が初公開され、auth はまだ無効である release note |
| `precision_closure_result` | hrana-http v2 schema、wire snapshot、SDK transcript、restart persistence、unsupported surface の artifact path、reviewer 再現 command、`P3-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 4 完全実装精度固定契約：**

Phase 4 は Phase 3 の hrana-http v2 surface に JWT HS256 認証と `token create` CLI を追加し、認証有効 mode の最小互換境界を完成させる Phase である。Phase 4 で公開してよい外部 API は Phase 3 と同じ `GET /v2/health` と `POST /v2/pipeline` のみであり、管理 API、DB scope JWT、Platform API、WebSocket、multi DB routing は実装してはならない。Phase 4 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 4 固定仕様 |
|------|------------------|
| 実装範囲 | JWT HS256 検証、Authorization Bearer 抽出、`a` claim による ro/rw 判定、`tokens.json` revoke 照合、`token create` CLI、secret redaction |
| API surface | Phase 3 と同じ `GET /v2/health` と `POST /v2/pipeline` のみ。新規 HTTP route は追加しない |
| health auth | `GET /v2/health` は認証不要のまま。JWT secret 設定後も 200 `{"status":"ok"}` 固定 |
| pipeline auth | JWT secret 設定時の `POST /v2/pipeline` は `Authorization: Bearer <JWT>` 必須 |
| auth disabled | JWT secret 未設定時は Phase 3 互換として unauthenticated `rw` claims を使い、WARN log を 1 回以上出す |
| secret source | `--auth-jwt-secret-file` > `--auth-jwt-secret` > `ADLAIRE_JWT_SECRET` > TOML `[auth] jwt_secret_file` > TOML `[auth] jwt_secret` > auth disabled |
| secret length | 解決後 secret は 32 bytes 以上必須。32 bytes 未満は起動拒否し、secret 生値を response/log に出さない |
| header format | `Authorization` は `Bearer ` + token の完全一致。prefix の大小文字補正、token trim、空白補正は禁止 |
| JWT alg | HS256 のみ許可。`alg:none`、HS384/HS512、`kid` による key lookup は拒否 |
| claims | Phase 4 の必須 claim は `sub` と `a`。`iss`、`iat`、`exp` は受け付ける。`dbs`、`org`、`grp` は Phase 7/8 まで権限判定に使わない |
| access value | `a` は `ro` または `rw` のみ。欠落、不正値、大文字値は `AUTH_INVALID` |
| expiry | `exp` が存在し現在時刻より過去なら `AUTH_EXPIRED`。`exp` 省略は無期限 token |
| revoke | `sub` が `tokens.json` に存在し `revoked=true` なら `AUTH_INVALID`。未登録 token の扱いは Phase 4 では拒否し `AUTH_INVALID` とする |
| ro/rw | `ro` token は read-only SQL のみ許可。write SQL、DDL、PRAGMA 書き込み相当は `PERMISSION_DENIED` |
| token create | `adlaire-db token create --secret <VALUE> [--expiry <DURATION>] [--access ro|rw]` は JWT を stdout に 1 回だけ出し、`tokens.json` に token metadata を atomic 追記する |
| token id | `sub` は `tok_` prefix + 128 bit 以上のランダム識別子。重複時は再生成する |
| tokens.json | `{data-dir}/meta/tokens.json` を `0600` 相当で作成し、atomic rename で更新する。JWT 文字列と secret 生値は保存しない |
| error precedence | malformed JSON / method / content-type 判定後に auth 判定を行う。auth header なしは `AUTH_REQUIRED`、形式不正/署名不正/revoked/未登録は `AUTH_INVALID`、期限切れは `AUTH_EXPIRED` |
| logging | Authorization header、JWT、secret、raw claim、SQL args は log に出さず `<redacted-secret>` または `<redacted>` に正規化する |

**Phase 4 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P4-1` | JWT secret 解決と起動 validation を固定する | §4.1、§5.1、§10.1 | `config.rs`、`main.rs` | 32 bytes 未満 secret 起動、secret log | precedence と起動拒否が固定 | config/auth startup tests |
| `TASK-P4-2` | Authorization Bearer 抽出を固定する | §5.1、§9.1.18 | `auth/middleware.rs` | trim / 大小文字補正 | header なし/不正形式が規定 error | auth header matrix |
| `TASK-P4-3` | JWT HS256 検証を実装する | §5.2、§5.6、§7.3 | `auth/mod.rs` | alg none、HS256 以外、kid key lookup | valid/invalid/expired が固定 | JWT unit tests |
| `TASK-P4-4` | `tokens.json` revoke 照合を実装する | §5.6、§9.6、§10.2 | `auth/mod.rs`、`token/*` | 未登録 token 許可、JWT 保存 | revoked/unknown token が拒否される | tokens fixture tests |
| `TASK-P4-5` | `a` claim ro/rw 判定を pipeline に適用する | §5.3、§9.1.18 | `http/pipeline.rs`、`sql/*` | ro write 成功、SQL args log | read は許可、write/DDL は拒否 | permission matrix |
| `TASK-P4-6` | `token create` CLI を実装する | §4.1、§5.5 | `cli.rs`、`token/*` | stdout 以外へ JWT 再表示、弱い token id | JWT 発行と metadata atomic 追記 | CLI transcript |
| `TASK-P4-7` | auth error response を固定する | §7.3、§9.1.18 | `error.rs`、`http/*` | SQL error と auth error 混同 | 401/403 code/body が snapshot 一致 | error snapshots |
| `TASK-P4-8` | secret redaction と artifact scan を固定する | §9.1.17、§9.1.18、§10.1 | logging / tests | JWT/secret/claim raw 出力 | log と artifact に secret が残らない | secret scan artifact |
| `TASK-P4-9` | Phase 4 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | 手動確認のみで完了 | 第三者が auth matrix を再現できる | handoff checklist |

**Phase 4 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P4-1` | JWT secret 未設定で `POST /v2/pipeline` SELECT | HTTP 200、Phase 3 と同じ成功、auth disabled WARN | disabled auth transcript |
| `SCN-P4-2` | JWT secret 設定、Authorization なし | HTTP 401 `AUTH_REQUIRED` | no auth snapshot |
| `SCN-P4-3` | `Authorization: Bearer <valid rw JWT>` で SELECT/INSERT | HTTP 200、read/write 成功 | rw token transcript |
| `SCN-P4-4` | `Authorization: Bearer <valid ro JWT>` で SELECT | HTTP 200、read 成功 | ro read snapshot |
| `SCN-P4-5` | `Authorization: Bearer <valid ro JWT>` で INSERT/CREATE/UPDATE/DELETE | HTTP 403 `PERMISSION_DENIED` | ro write denial matrix |
| `SCN-P4-6` | Bearer prefix 欠落、小文字 bearer、余分な空白 | HTTP 401 `AUTH_INVALID` または `AUTH_REQUIRED`。補正しない | header strictness snapshot |
| `SCN-P4-7` | 署名不正 JWT | HTTP 401 `AUTH_INVALID` | bad signature snapshot |
| `SCN-P4-8` | `exp` 過去 JWT | HTTP 401 `AUTH_EXPIRED` | expired snapshot |
| `SCN-P4-9` | `a` 欠落 / `a:"admin"` / `a:"RW"` | HTTP 401 `AUTH_INVALID` | claim validation snapshot |
| `SCN-P4-10` | `tokens.json` で `revoked=true` の `sub` | HTTP 401 `AUTH_INVALID` | revoked snapshot |
| `SCN-P4-11` | `tokens.json` に存在しない `sub` | HTTP 401 `AUTH_INVALID` | unknown token snapshot |
| `SCN-P4-12` | `token create --secret ... --expiry 30d --access ro` | stdout に JWT 1 回、`tokens.json` に metadata、JWT 文字列は保存しない | CLI + file transcript |
| `SCN-P4-13` | 32 bytes 未満 secret で起動 | 起動失敗、secret 生値なし | startup failure log |
| `SCN-P4-14` | auth failure と malformed JSON が同時に成立 | request parsing precedence に従い `INVALID_REQUEST` | precedence snapshot |
| `SCN-P4-15` | auth failure log / test artifact scan | JWT、Bearer、secret 生値、raw claim が残らない | secret scan result |

**Phase 4 禁止事項：**

| 状態 | 判定 |
|------|------|
| 管理 API、Platform API、WebSocket、multi DB route を追加する | merge 不可。Phase 4 の外部 API は Phase 3 と同じ |
| JWT secret 設定時に認証なし pipeline を通す | merge 不可 |
| Bearer prefix の大小文字・空白・token 値を補正する | merge 不可 |
| `alg:none` または HS256 以外を受け付ける | merge 不可 |
| 32 bytes 未満 secret で起動する | merge 不可 |
| `ro` token の write/DDL を成功させる | merge 不可 |
| JWT 文字列、secret 生値、Authorization header、raw claim を log / artifact / `tokens.json` に保存する | merge 不可 |
| revoked token または未登録 `sub` を成功扱いにする | merge 不可 |
| auth matrix / permission matrix / secret scan なしで完了扱いにする | Phase 未完了 |
| CLI 手動確認だけで Phase 4 完了扱いにする | Phase 未完了。自動検証と再現可能 artifact が必須 |

**Phase 4 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | JWT HS256、Bearer 抽出、`a` claim ro/rw、`tokens.json` revoke/unknown token 拒否、`token create` CLI、secret redaction |
| `excluded_scope` | DB scope JWT、管理 API、Platform API、WebSocket、multi DB route、organization/group/quota |
| `atomic_task_result` | `TASK-P4-1`〜`TASK-P4-9` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P4-1`〜`SCN-P4-15` の pass/fail、artifact path |
| `auth_matrix_result` | no auth、bad format、bad signature、expired、revoked、unknown、valid ro、valid rw の status/code |
| `permission_matrix_result` | ro read allow、ro write deny、rw read/write allow、SQL 分類根拠 |
| `persistence_result` | `tokens.json` 作成・atomic update・restart 後 revoke 反映・JWT 非保存 |
| `secret_redaction_result` | response/log/artifact に secret、JWT、Bearer、raw claim、SQL args が残らない scan 結果 |
| `compatibility_baseline_result` | libSQL SDK auth header 接続 transcript、Turso 差分分類 |
| `coverage_closure_result` | startup、auth、permission、token CLI、revoke、redaction、Phase 1〜3 regression の gap 0 件 |
| `review_handoff_result` | 第三者が auth matrix、permission matrix、token create、restart、secret scan を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 4 で JWT secret 設定時に `/v2/pipeline` が認証必須になり、未設定時は開発用 auth disabled として動く release note |
| `precision_closure_result` | auth matrix、permission matrix、token persistence、secret redaction、SDK auth transcript、Phase 1〜3 regression の artifact path、reviewer 再現 command、`P4-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 5 完全実装精度固定契約：**

Phase 5 は Phase 1〜4 の実装を、構造化ログ、統合テスト、SDK 互換、再起動永続化、secret redaction によって完了判定可能な品質ゲートへ固定する Phase である。Phase 5 では新規外部 API、metadata schema、JWT claim、DB routing、管理 API を追加してはならない。Phase 5 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 5 固定仕様 |
|------|------------------|
| 実装範囲 | JSON Lines logging、HTTP request log、startup/shutdown log、Phase 1〜5 regression、TypeScript SDK CRUD、restart persistence、secret scan |
| API surface | Phase 3/4 と同じ `GET /v2/health` と `POST /v2/pipeline` のみ。新規 route、stub route、管理 API 成功応答は禁止 |
| persistence | 新規永続化 file は追加しない。既存 `default/data.db` と `meta/tokens.json` の再起動後復元を検証する |
| log format | 1 行 1 JSON object の JSON Lines。複数行 JSON、plain text mixed log、非 JSON log は不可 |
| required log fields | `timestamp`、`level`、`event`、`request_id`、`method`、`path`、`status`、`duration_ms`。HTTP request 以外は該当しない field を省略可 |
| request_id | request ごとに生成し、response header または log correlation で追跡できる。Authorization、JWT、SQL、path 絶対値を request_id に含めない |
| duration_ms | 非負整数または小数。clock 逆行で負値を出してはならない |
| forbidden log fields | Authorization header、JWT、secret、raw claim、SQL args、生 SQL の bind 値、admin/platform/replication/HA token |
| log redaction | 秘匿値は `<redacted-secret>`、SQL args は `<redacted>` に正規化する。hash 化しても秘匿値の代替出力として扱い不可 |
| auth disabled log | JWT secret 未設定時は auth disabled WARN を出すが、secret 欠落以外の機密情報は出さない |
| SDK compatibility | TypeScript `@libsql/client` で createClient、execute、CREATE、INSERT、SELECT、authToken あり/なしの transcript を保存する |
| regression | Phase 1〜4 の CLI/config/data-dir/HTTP/hrana/JWT/token/revoke/permission/error tests をすべて再実行する |
| restart persistence | INSERT 後に graceful stop と process abort 相当の両方を行い、同じ `--data` で SELECT 結果が残ることを証跡化する |
| artifact policy | snapshot / transcript / log / test output は secret scan を通過したものだけを Done receipt に添付する |

**Phase 5 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P5-1` | JSON Lines logging を固定する | §9.1.17、§12 | logging middleware | plain text 混在、複数行 JSON | すべての log line が JSON object | log parser test |
| `TASK-P5-2` | HTTP request log を固定する | §9.1.17、§9.4 Phase 5 | logging middleware | Authorization / SQL args 出力 | method/path/status/duration_ms/request_id が出る | request log snapshot |
| `TASK-P5-3` | startup/shutdown log を固定する | §8.1、§9.1.25 | runtime / logging | secret/path 過剰出力 | 起動・停止・lock 解放が追跡可能 | lifecycle log snapshot |
| `TASK-P5-4` | Phase 1〜4 regression gate を固定する | §9.1.1、§9.8 | tests / CI | test skip、手動確認のみ | TC-1〜TC-6 と Phase 1〜4 matrix が pass | regression transcript |
| `TASK-P5-5` | TypeScript SDK CRUD transcript を固定する | §9.1.1、§9.1.35 | tests / docker / artifact | curl だけで代替 | `@libsql/client` CRUD が pass | SDK transcript |
| `TASK-P5-6` | restart persistence を固定する | §9.1.23、§9.1.35 | tests / artifact | in-memory 成功のみ | graceful/abort 後に SELECT できる | restart transcript |
| `TASK-P5-7` | secret scan を固定する | §9.1.17、§9.1.18 | tests / artifact | JWT/secret 混入 | log/artifact に secret が残らない | secret scan result |
| `TASK-P5-8` | unsupported API regression を固定する | §9.1.10、§9.5 | tests / snapshot | 未来 API 成功応答 | 管理 API / multi DB / WS が成功しない | unsupported snapshot |
| `TASK-P5-9` | Phase 5 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が logs/SDK/restart を再現できる | handoff checklist |

**Phase 5 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P5-1` | `GET /v2/health` | HTTP 200、request log が JSON Lines で出る | health log snapshot |
| `SCN-P5-2` | `POST /v2/pipeline` SELECT | HTTP 200、method/path/status/duration_ms/request_id が log に出る | pipeline log snapshot |
| `SCN-P5-3` | JWT secret 設定 + valid token | HTTP 200、JWT/Authorization/raw claim が log に出ない | auth redaction snapshot |
| `SCN-P5-4` | JWT secret 設定 + invalid token | HTTP 401、token 値なしの denied log | auth failure log snapshot |
| `SCN-P5-5` | SQL args あり INSERT | HTTP 200、SQL args と bind 値が log に出ない | SQL redaction snapshot |
| `SCN-P5-6` | TypeScript `@libsql/client` auth なし CRUD | auth disabled mode で CREATE/INSERT/SELECT 成功 | SDK disabled transcript |
| `SCN-P5-7` | TypeScript `@libsql/client` authToken あり CRUD | JWT auth mode で CREATE/INSERT/SELECT 成功 | SDK auth transcript |
| `SCN-P5-8` | INSERT 後 graceful stop / restart | SELECT でデータが残る | graceful restart transcript |
| `SCN-P5-9` | INSERT 後 process abort 相当 / restart | SQLite/WAL recovery 後 SELECT でデータが残る | abort restart transcript |
| `SCN-P5-10` | Phase 1 CLI help/config tests | Phase 1 regression pass | regression output |
| `SCN-P5-11` | Phase 2 data-dir/lock/WAL tests | Phase 2 regression pass | regression output |
| `SCN-P5-12` | Phase 3 hrana/error tests | Phase 3 regression pass | regression output |
| `SCN-P5-13` | Phase 4 auth/token/permission tests | Phase 4 regression pass | regression output |
| `SCN-P5-14` | `/admin/v1/databases`、`/{db}/v2/pipeline`、`/v3/baton` | Phase 5 では成功応答なし | unsupported snapshot |
| `SCN-P5-15` | log / transcript / artifact secret scan | JWT、Bearer、secret、SQL args、生 bind 値が残らない | secret scan result |

**Phase 5 禁止事項：**

| 状態 | 判定 |
|------|------|
| 新規 HTTP route、管理 API、multi DB route、WebSocket を成功応答にする | merge 不可 |
| metadata schema、JWT claim、response wrapper、hrana wire shape を変更する | merge 不可 |
| plain text log、複数行 JSON log、JSON Lines でない log を混在させる | merge 不可 |
| Authorization header、JWT、secret、raw claim、SQL args、生 bind 値を log / artifact に出す | merge 不可 |
| TypeScript SDK regression を curl / Rust test だけで代替する | Phase 未完了 |
| restart persistence を graceful stop だけで完了扱いにする | Phase 未完了。abort 相当 recovery も必要 |
| Phase 1〜4 regression の一部を skip して完了扱いにする | Phase 未完了 |
| secret scan なしで snapshot / transcript を Done receipt に添付する | Phase 未完了 |
| flaky test を retry だけで隠して完了扱いにする | Phase 未完了 |

**Phase 5 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | JSON Lines log、HTTP request log、startup/shutdown log、Phase 1〜5 regression、TypeScript SDK CRUD、restart persistence、secret scan |
| `excluded_scope` | 新規 API、管理 API、multi DB routing、WebSocket、metadata schema 変更、JWT claim 変更 |
| `atomic_task_result` | `TASK-P5-1`〜`TASK-P5-9` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P5-1`〜`SCN-P5-15` の pass/fail、artifact path |
| `log_contract_result` | JSON Lines parse 結果、必須 field、duration_ms、request_id、forbidden field 不在 |
| `sdk_compatibility_result` | TypeScript `@libsql/client` auth disabled / authToken CRUD transcript |
| `restart_persistence_result` | graceful stop と abort 相当の両方で再起動後 SELECT が pass |
| `regression_result` | Phase 1〜4 の CLI/config/data-dir/hrana/JWT/token/error/permission regression pass |
| `unsupported_surface_result` | 管理 API、multi DB route、WebSocket、未来 Phase API が成功応答しない snapshot |
| `secret_redaction_result` | stdout/stderr/log/snapshot/transcript/artifact の secret scan pass |
| `coverage_closure_result` | log、SDK、restart、secret、unsupported、Phase 1〜4 regression の gap 0 件 |
| `review_handoff_result` | 第三者が log parse、SDK CRUD、restart、secret scan、regression を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 5 では API 機能追加はなく、運用ログと互換 regression が完了条件になる release note |
| `precision_closure_result` | JSONL parser、request log、SDK CRUD、restart persistence、secret scan、unsupported surface、Phase 1〜4 regression の artifact path、reviewer 再現 command、`P5-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 1〜5 完全実装精度正規化契約：**

Phase 1〜5 は後続 Phase と同じ Done 判定規則を適用する。実装者は「動いた」「手元確認済み」「PR description に記載済み」を完了根拠にしてはならない。各 Phase の Done receipt は、原子タスク、シナリオ、禁止事項、互換 baseline、secret redaction、review handoff、operator behavior delta、precision closure をすべて埋める。

| 項目 | 固定仕様 | 完了不可条件 |
|------|----------|--------------|
| heading normalization | Phase 1〜5 の仕様見出しは `原子タスク台帳`、`シナリオマトリクス`、`Done receipt 必須項目` を正とする | 旧見出しだけを参照して Done 判定する |
| artifact path | Done receipt の pass/fail は artifact path または再現 command を必ず持つ | 口頭説明、PR description、スクリーンショットのみ |
| atomic closure | `TASK-P{phase}-*` はすべて pass、open task 0 件でなければならない | skip、manual-only、未実装 task あり |
| scenario closure | `SCN-P{phase}-*` はすべて pass/fail と理由を記録する。fail は仕様化された対象外または blocker でなければならない | fail 理由なし、期待値更新だけで pass |
| compatibility baseline | Phase 3〜5 は libSQL SDK transcript と hrana / API snapshot を Done receipt に含める | curl smoke のみ、SDK transcript なし |
| no future surface | Phase 1〜5 で未来 Phase の API、metadata、JWT claim、WebSocket、Admin API、multi DB route を成功応答にしない | 未来 surface が成功応答する |
| persistence evidence | Phase 1 は no persistence、Phase 2〜5 は対象 persistence / restart recovery を artifact 化する | 永続化有無の証跡なし |
| secret redaction | Phase 4〜5 は response/stdout/stderr/log/artifact に secret、JWT、Authorization、raw claim、SQL args が残らない scan を必須にする | scan なし、秘匿値混入 |
| reviewer reproducibility | 第三者が clean checkout から command で再現できる handoff を必須にする | ローカル状態依存、手順欠落 |
| bug-zero readiness | Done receipt に coverage gap、未解決判断、仕様未確定、既知 flaky が 0 件であることを明記する | 未解決判断を実装者判断へ先送り |

**Phase 1〜5 の境界決定：**

- Phase 5 完了まで、外部公開 API は `/v2/health` と `/v2/pipeline` のみとする
- 管理 API route を早期に生やす場合は 501 stub に限定し、Phase 5 完了条件には含めない
- JWT secret が設定されている場合、Phase 4 完了前は起動拒否、Phase 4 完了後は認証有効として扱う

#### Phase 6〜10：マルチ DB・管理 API・Turso Cloud 互換管理モデル・WebSocket・ATTACH

| Phase | 変更対象 | API 契約 | 永続化 | エラー/ログ | テスト契約 |
|-------|----------|----------|--------|-------------|------------|
| Phase 6 | `db/manager.rs`, `db/meta.rs`, `http/mod.rs`, `http/pipeline.rs` | `POST /{db-name}/v2/pipeline` を追加する。`/v2/pipeline` は `default` のまま。管理 API はまだ完了対象外 | `meta/databases.json` に DB 追加/削除を atomic update。各 DB は `databases/{name}/data.db` | invalid DB name は `INVALID_DB_NAME`、予約名は `DB_RESERVED_NAME`、未存在は `DB_NOT_FOUND` | DB 名 validation、複数 DB 分離、default fallback、再起動後 DB 復元 |
| Phase 7 | `http/admin/*`, `auth/*`, `token/*`, `db/manager.rs` | `/admin/v1/databases`, `/admin/v1/tokens` を §6.4 通り実装する。Admin token は Bearer 完全一致。DB scope JWT を有効化 | `databases.json` と `tokens.json` を API 経由で更新する。削除はファイル/ディレクトリと metadata を整合させる | 管理 API 認証失敗は `401 AUTH_REQUIRED`。重複 DB は `409 DB_ALREADY_EXISTS`。revoke は即時反映 | TC-2-1〜TC-2-6。admin auth、DB CRUD、token CRUD、DB scope ro/rw |
| Phase 8 | `http/admin/*`, `http/platform/*`, `db/meta.rs`, `auth/*`, `quota/*`, `location/*`, `org/*` | §9.5 と Phase 8 詳細節に定義した organization/group/location/quota/usage API、`/v1/*` Turso Platform API 互換、既存 DB/token admin API の scope 拡張を実装する | `organizations.json`、`groups.json`、`locations.json`、`quotas.json`、`usage.json` を §9.6 通り更新し、既存 `databases.json` / `tokens.json` の migration と後方互換を保証する | `ORG_NOT_FOUND`、`GROUP_NOT_FOUND`、`LOCATION_NOT_FOUND`、`QUOTA_EXCEEDED`、`USAGE_UNAVAILABLE`、`ORG_SCOPE_DENIED`、`NOT_IMPLEMENTED` を §7.3 通り返す。secret と課金相当情報はログ出力禁止 | Phase 8 API/metadata/auth/quota/migration/Turso snapshot tests。Phase 1〜7 regression と SDK 互換を必須 |
| Phase 9 | `ws/*`, `hrana/*`, `db/sqld_adapter.rs`, `http/mod.rs` | `GET /v3/baton` と `GET /{db-name}/v3/baton` で WebSocket upgrade。hrana-ws v3 messages を実装 | SQL 実行による DB 永続化のみ。WebSocket session state はプロセス内メモリでよく、再起動復元しない | hello 前 request は protocol error。stream 未存在は hrana error。接続 close 時に未完了 transaction は rollback | TC-3-1〜TC-3-4。interactive transaction、store_sql/close_sql、auth failure、multi stream |
| Phase 10 | `attach/*`, `metrics.rs`, `http/admin/metrics`, `db/sqld_adapter.rs` | 管理下 DB の ATTACH のみ許可。`GET /admin/v1/metrics` を実装する | 新規ファイルなし。metrics はプロセス内 counters/gauges でよく再起動リセット | 任意パス ATTACH は `PERMISSION_DENIED` または `INVALID_REQUEST`。metrics 取得は admin auth 対象 | TC-3-5, TC-3-6。ATTACH 成功/拒否、metrics counters 更新 |

**Phase 6/7 の境界決定：**

- Phase 6 は path-based routing と DbManager 内部機構まで。HTTP 管理 API の完成は Phase 7
- Phase 6 で DB 作成用の内部関数を実装してよいが、外部 API として成功応答を返すのは Phase 7
- DB scope JWT は Phase 7。Phase 6 では global `a` claim のみ有効
- Turso Cloud 互換管理モデルは Phase 8。Phase 7 の DB/token CRUD は既存の最小管理 API として完了済み扱いを維持する

**Phase 6 完全実装精度固定契約：**

Phase 6 は Phase 1〜5 の単一 DB HTTP/JWT/SDK 互換を維持したまま、path-based DB routing と DbManager 内部機構を追加する Phase である。Phase 6 で成功応答してよい新規外部 API は `POST /{db-name}/v2/pipeline` のみであり、管理 API、DB scope JWT、Turso Platform API、WebSocket、organization/group/location/quota は実装してはならない。Phase 6 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 6 固定仕様 |
|------|------------------|
| 実装範囲 | path-based DB routing、DbManager load/open/create/delete internal functions、DB name validation、`databases.json` atomic update、multi DB isolation |
| API surface | 既存 `GET /v2/health`、`POST /v2/pipeline` に加え、`POST /{db-name}/v2/pipeline` のみ成功応答可 |
| default route | `POST /v2/pipeline` は Phase 6 以降も常に `default` DB を対象とする。path DB へ暗黙移行してはならない |
| path route | `POST /{db-name}/v2/pipeline` は URL decode 後の `{db-name}` を DB identity とし、該当 DB にだけ SQL を実行する |
| route precedence | `/v2/pipeline` は default route として最優先。`/admin/*`、`/v1/*`、`/v3/*` は Phase 6 では成功応答禁止 |
| auth | Phase 4 の global JWT `a` claim のみ適用する。`dbs`、`org`、`grp` claim は Phase 7/8 まで権限判定に使わない |
| DB name validation | `^[a-zA-Z0-9_-]{1,127}$`。空、slash、dot path、URL decode 後 slash、space、percent decode 不正は `INVALID_DB_NAME` |
| reserved name | `meta`、`admin`、`___` を含む name は `DB_RESERVED_NAME`。branch 内部名は Phase 15 まで外部作成不可 |
| DB existence | validation 通過後、metadata / manager に存在しない DB は `DB_NOT_FOUND`。存在しない DB directory を暗黙作成しない |
| DbManager lifecycle | 起動時に `databases.json` を読み、default を含む全 DB を open する。open/integrity_check 失敗は起動失敗 |
| create/delete scope | Phase 6 では create/delete/list/detail を外部 API として成功応答しない。内部関数と test helper だけで検証する |
| metadata update | `databases.json` は tmp write + fsync + atomic rename。partial write、空上書き、DB directory との不整合を成功扱いにしない |
| directory layout | 各 DB は `{data-dir}/databases/{name}/data.db` 固定。default は `{data-dir}/databases/default/data.db` |
| isolation | DB A への write は DB B に見えてはならない。default route と path route の DB identity を混ぜてはならない |
| compatibility | Phase 1〜5 regression と TypeScript SDK CRUD は default route で継続 pass。path route は libSQL SDK 互換の HTTP surface として snapshot を取る |

**Phase 6 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P6-1` | route precedence を固定する | §6.1、§9.5 | `http/mod.rs` | `/v2/pipeline` の default 破壊、管理 API 成功 | default/path route が分離 | route snapshot |
| `TASK-P6-2` | DB name validation を固定する | §3.4、§7.3、§9.5 | `db/mod.rs` | path traversal、補正、trim | invalid/reserved が規定 error | validation matrix |
| `TASK-P6-3` | DbManager open/load を固定する | §8.1、§9.6 | `db/manager.rs`、`db/meta.rs` | open 失敗無視、空 metadata 上書き | 起動時に全 DB open、失敗時起動失敗 | startup fixture |
| `TASK-P6-4` | internal create/delete lifecycle を固定する | §3.4、§9.1.16 | `db/manager.rs` | 外部管理 API 成功、partial create/delete | internal function が atomic に DB state を変更 | lifecycle tests |
| `TASK-P6-5` | `databases.json` atomic update を固定する | §9.6、§9.1.23 | `db/meta.rs` | partial write、空上書き | tmp/fsync/rename と失敗注入が pass | metadata fixture |
| `TASK-P6-6` | path DB pipeline を実装する | §6.2、§9.1.28、§9.5 | `http/pipeline.rs` | default への誤ルーティング | path DB で SELECT/INSERT 成功 | path pipeline snapshot |
| `TASK-P6-7` | multi DB isolation を固定する | §3.4、§9.1.35 | tests / artifact | DB 間混線、共有 file | DB A/B/default が独立 | isolation transcript |
| `TASK-P6-8` | unsupported surface を固定する | §9.1.10、§9.5 | tests / snapshot | admin/v1/v3 成功応答 | Phase 7+ surface が成功しない | unsupported snapshot |
| `TASK-P6-9` | Phase 6 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が route/isolation/restart を再現できる | handoff checklist |

**Phase 6 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P6-1` | `POST /v2/pipeline` INSERT/SELECT | 常に `default` DB に対して成功 | default route transcript |
| `SCN-P6-2` | `POST /app/v2/pipeline` INSERT/SELECT | `app` DB に対して成功、default には見えない | path route transcript |
| `SCN-P6-3` | DB A に INSERT、DB B で SELECT | DB B には DB A の table/data が存在しない | isolation snapshot |
| `SCN-P6-4` | URL decode 後 `/` を含む DB name | HTTP 400 `INVALID_DB_NAME` | validation snapshot |
| `SCN-P6-5` | 空、space、`.`、`..`、128 文字 name | HTTP 400 `INVALID_DB_NAME` | validation matrix |
| `SCN-P6-6` | `meta`、`admin`、`a___b` | HTTP 400 `DB_RESERVED_NAME` | reserved snapshot |
| `SCN-P6-7` | 存在しない valid DB name | HTTP 404 `DB_NOT_FOUND` | not found snapshot |
| `SCN-P6-8` | malformed pipeline JSON on path route | HTTP 400 `INVALID_REQUEST`。DB は暗黙作成されない | malformed snapshot |
| `SCN-P6-9` | valid JWT global `ro` で path route write | HTTP 403 `PERMISSION_DENIED` | permission snapshot |
| `SCN-P6-10` | valid JWT global `rw` で path route write/read | HTTP 200、対象 DB にだけ反映 | auth path transcript |
| `SCN-P6-11` | restart 後に DB A/B/default を SELECT | 各 DB の data が維持される | restart transcript |
| `SCN-P6-12` | 壊れた `databases.json` で起動 | 起動失敗。空 metadata で上書きしない | corrupt metadata fixture |
| `SCN-P6-13` | DB directory 欠落 / data.db open 失敗 | 起動失敗または明示 recovery failure。暗黙 success なし | recovery fixture |
| `SCN-P6-14` | `/admin/v1/databases`、`/v1/*`、`/v3/baton` | Phase 6 では成功応答なし | unsupported snapshot |
| `SCN-P6-15` | Phase 1〜5 regression + TypeScript SDK default CRUD | すべて pass | regression transcript |

**Phase 6 禁止事項：**

| 状態 | 判定 |
|------|------|
| 管理 API、Turso Platform API、WebSocket を成功応答にする | merge 不可 |
| `POST /v2/pipeline` を default 以外へ向ける | merge 不可 |
| path DB が存在しない場合に暗黙作成する | merge 不可 |
| DB name を trim / lowercase / normalize して受け付ける | merge 不可 |
| URL decode 後の slash、dot path、`___` を許可する | merge 不可 |
| `databases.json` parse 失敗時に空 metadata で上書きする | merge 不可 |
| DB directory / metadata の不整合を成功起動扱いにする | Phase 未完了 |
| DB A/B/default の isolation transcript なしで完了扱いにする | Phase 未完了 |
| Phase 1〜5 regression と TypeScript SDK default CRUD なしで完了扱いにする | Phase 未完了 |
| DB scope JWT、organization/group/location/quota 判定を Phase 6 完了条件に混ぜる | merge 不可。Phase 7/8 対象 |

**Phase 6 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | path-based DB routing、DbManager load/open/internal create/delete、DB name validation、`databases.json` atomic update、multi DB isolation |
| `excluded_scope` | 管理 API 成功応答、DB scope JWT、Turso Platform API、WebSocket、organization/group/location/quota |
| `atomic_task_result` | `TASK-P6-1`〜`TASK-P6-9` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P6-1`〜`SCN-P6-15` の pass/fail、artifact path |
| `route_contract_result` | `/v2/pipeline` default、`/{db-name}/v2/pipeline` path DB、unsupported future route の snapshot |
| `validation_result` | valid / invalid / reserved / URL decode / path traversal / length boundary の matrix |
| `metadata_atomicity_result` | `databases.json` tmp/fsync/rename、失敗注入、破損時起動失敗、空上書きなし |
| `isolation_result` | default / DB A / DB B の write/read 分離 transcript |
| `restart_recovery_result` | 複数 DB の再起動復元、directory 欠落、metadata 破損、open 失敗の結果 |
| `compatibility_baseline_result` | Phase 1〜5 regression、TypeScript SDK default route CRUD、path route wire snapshot |
| `coverage_closure_result` | route、validation、auth、metadata、isolation、restart、unsupported、regression の gap 0 件 |
| `review_handoff_result` | 第三者が route、validation、metadata、isolation、restart、unsupported を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 6 で `/{db-name}/v2/pipeline` が追加されるが、DB 作成管理 API はまだ成功応答しない release note |
| `precision_closure_result` | DB name validation、path routing、default route 後方互換、DB isolation、metadata atomic update、restart restore、Phase 1〜5 regression の artifact path、reviewer 再現 command、`P6-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 7 完全実装精度固定契約：**

Phase 7 は Phase 6 の multi DB routing に、Adlaire 管理 API、token CRUD、DB scope JWT を追加する Phase である。Phase 7 で成功応答してよい新規外部 API は `/admin/v1/databases`、`/admin/v1/databases/{name}`、`/admin/v1/tokens`、`/admin/v1/tokens/{id}` のみであり、Turso Platform API `/v1/*`、organization/group/location/quota、WebSocket、backup/restore、branch は実装してはならない。Phase 7 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 7 固定仕様 |
|------|------------------|
| 実装範囲 | Admin token 認証、DB CRUD 管理 API、token CRUD 管理 API、DB scope JWT `dbs`、revoke 即時反映、metadata atomic update |
| API surface | `GET/POST /admin/v1/databases`、`GET/DELETE /admin/v1/databases/{name}`、`GET/POST /admin/v1/tokens`、`GET/DELETE /admin/v1/tokens/{id}` のみ追加 |
| admin auth | `[admin] auth_token` / `--admin-auth-token` / `ADLAIRE_ADMIN_TOKEN` が設定されている場合は `Authorization: Bearer <token>` 完全一致必須 |
| admin auth disabled | admin token 未設定時は開発用 auth disabled。WARN log を出すが、本番推奨ではないことを operator delta に明記する |
| Bearer strictness | prefix、大小文字、前後空白、token 値を補正しない。Authorization なしは `AUTH_REQUIRED`、形式不正/不一致は `AUTH_INVALID` |
| DB create | `POST /admin/v1/databases {"name":string}` は 201 `DbInfo`。DB directory 作成、libsql open、integrity_check、manager 登録、`databases.json` commit の順に成功させる |
| DB list/detail | list は `{"databases":[DbInfo...]}`、detail は `DbInfo`。`size_bytes` は `data.db` 実ファイルサイズ。JWT/secret を返さない |
| DB delete | `DELETE /admin/v1/databases/{name}` は 204 body なし。削除後の detail/path pipeline は `DB_NOT_FOUND` |
| DB create conflict | 同名 active DB は `409 DB_ALREADY_EXISTS`。delete 中/作成中の race は `409 STORAGE_BUSY` または `DB_ALREADY_EXISTS` を固定 snapshot に残す |
| token create | `POST /admin/v1/tokens` は 201 で JWT を 1 回だけ `token` field に返す。以降の GET/list では JWT 文字列を返さない |
| token metadata | `tokens.json` には `id`、`access`、`dbs`、`created_at`、`expires_at`、`revoked`、`revoked_at` を保存し、JWT 文字列と secret 生値は保存しない |
| token list/detail | list/detail は token metadata のみ返す。`revoked` と `revoked_at` を必ず含める |
| token revoke | `DELETE /admin/v1/tokens/{id}` は 204 body なし。存在する token は即時に memory state と `tokens.json` の両方へ反映する。既に revoked は 204 |
| unknown token revoke | 存在しない token id は `404 TOKEN_NOT_FOUND`。別 token の revoke と混同しない |
| DB scope JWT | `dbs` がある場合は対象 DB 名の値を優先し、存在しない場合は global `a` を使う。`dbs` 値は `ro` / `rw` のみ |
| scope application | `dbs` 判定は `/v2/pipeline` の `default` と `/{db-name}/v2/pipeline` の path DB の両方へ適用する |
| persistence | `databases.json` と `tokens.json` は tmp write + fsync + atomic rename。partial write、JWT 保存、metadata/file 不整合を成功扱いにしない |
| response time | `created_at`、`expires_at`、`revoked_at` は RFC3339 UTC 秒精度。`null` 可 field 以外は null 禁止 |

**Phase 7 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P7-1` | Admin token 認証境界を固定する | §6.4、§9.1.18 | `http/admin/*`、`config.rs` | Bearer 補正、token log | auth matrix が規定 status/code | admin auth matrix |
| `TASK-P7-2` | DB CRUD API contract を固定する | §6.4、§9.5 | `http/admin/databases.rs` | wrapper 変更、body あり DELETE | status/body/error が snapshot 一致 | DB API snapshots |
| `TASK-P7-3` | DB create/delete atomicity を固定する | §3.4、§9.1.16、§9.6 | `db/manager.rs`、`db/meta.rs` | partial create/delete 成功 | metadata/directory/manager が整合 | atomicity fixture |
| `TASK-P7-4` | token CRUD API contract を固定する | §5.5、§5.6、§6.4 | `http/admin/tokens.rs`、`token/*` | GET で JWT 返却、JWT 保存 | create/list/detail/revoke が固定 | token API snapshots |
| `TASK-P7-5` | revoke 即時反映を固定する | §5.6、§9.1.18 | `auth/*`、`token/*` | 再起動まで revoke 未反映 | revoke 後同一 token が即 401 | revoke race test |
| `TASK-P7-6` | DB scope JWT `dbs` を固定する | §5.4、§9.1.18 | `auth/*`、`http/pipeline.rs` | dbs 無視、scope 漏れ | default/path DB で ro/rw が規定通り | scope matrix |
| `TASK-P7-7` | admin request validation を固定する | §6.4、§9.1.2、§9.5 | `http/admin/*` | unknown/null/body 黙認 | invalid request が `INVALID_REQUEST` | validation snapshots |
| `TASK-P7-8` | concurrent create/delete/token を固定する | §9.1.16、§9.1.23 | `db/manager.rs`、`token/*` | lost update、二重作成 | race 後 metadata が整合 | concurrency fixture |
| `TASK-P7-9` | Phase 7 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が admin/scope/revoke を再現できる | handoff checklist |

**Phase 7 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P7-1` | admin token 設定、Authorization なし | HTTP 401 `AUTH_REQUIRED` | admin no auth snapshot |
| `SCN-P7-2` | admin token 設定、不一致 / Bearer 形式不正 | HTTP 401 `AUTH_INVALID` | admin invalid auth snapshot |
| `SCN-P7-3` | `GET /admin/v1/databases` 初期状態 | HTTP 200 `{"databases":[]}` または default 以外空の固定結果 | DB list snapshot |
| `SCN-P7-4` | `POST /admin/v1/databases {"name":"db_a"}` | HTTP 201 `DbInfo`、directory と metadata 作成 | DB create transcript |
| `SCN-P7-5` | 同名 DB 再作成 | HTTP 409 `DB_ALREADY_EXISTS` | duplicate snapshot |
| `SCN-P7-6` | invalid / reserved DB name | `INVALID_DB_NAME` / `DB_RESERVED_NAME` | validation matrix |
| `SCN-P7-7` | DB detail/list/delete lifecycle | detail 200、delete 204、削除後 detail/path route 404 | DB lifecycle transcript |
| `SCN-P7-8` | DB create 中失敗注入 | partial metadata/directory が成功扱いにならない | DB atomicity fixture |
| `SCN-P7-9` | `POST /admin/v1/tokens {"access":"rw","expiry":"30d"}` | HTTP 201、JWT は create response だけ、metadata 保存 | token create transcript |
| `SCN-P7-10` | token list/detail | HTTP 200、JWT 文字列なし、revoked fields あり | token list/detail snapshot |
| `SCN-P7-11` | token revoke 後に同 token で pipeline | revoke 直後から HTTP 401 `AUTH_INVALID` | revoke immediate transcript |
| `SCN-P7-12` | 存在しない token revoke | HTTP 404 `TOKEN_NOT_FOUND` | token not found snapshot |
| `SCN-P7-13` | `dbs {"db_a":"rw"}` + global `ro` で db_a write | HTTP 200 | db scope allow snapshot |
| `SCN-P7-14` | 同 token で db_b write / read | write は 403 `PERMISSION_DENIED`、read は 200 | db scope deny/read snapshot |
| `SCN-P7-15` | `dbs` 不正値、unknown field、null、空 body | HTTP 400 `INVALID_REQUEST` | admin validation snapshots |
| `SCN-P7-16` | DB/token 作成後 restart | DB と token metadata、revoke state、scope が復元 | restart transcript |
| `SCN-P7-17` | concurrent DB create/delete、token create/revoke | metadata lost update なし、規定 conflict error | concurrency transcript |
| `SCN-P7-18` | `/v1/*`、WebSocket、backup/restore/branch API | Phase 7 では成功応答なし | unsupported snapshot |

**Phase 7 禁止事項：**

| 状態 | 判定 |
|------|------|
| `/v1/*` Turso Platform API、organization/group/location/quota を成功応答にする | merge 不可。Phase 8 対象 |
| WebSocket、backup/restore、branch、metrics API を成功応答にする | merge 不可。未来 Phase 対象 |
| Admin token の Bearer 値を trim / lowercase / 補正して受け付ける | merge 不可 |
| 管理 API の unknown field、body あり DELETE、null 不正を黙って無視する | merge 不可 |
| GET/list token response に JWT 文字列を返す | merge 不可 |
| `tokens.json` に JWT 文字列または secret 生値を保存する | merge 不可 |
| token revoke が再起動まで反映されない | merge 不可 |
| `dbs` claim を無視する、または path DB と default DB で異なる規則にする | merge 不可 |
| DB create/delete の partial metadata/directory 不整合を成功扱いにする | Phase 未完了 |
| admin auth matrix、DB CRUD、token CRUD、scope matrix、revoke immediate、concurrency の証跡なしで完了扱いにする | Phase 未完了 |

**Phase 7 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | Admin token 認証、DB CRUD API、token CRUD API、DB scope JWT、revoke 即時反映、metadata atomic update |
| `excluded_scope` | `/v1/*` Turso Platform API、organization/group/location/quota、WebSocket、backup/restore、branch、metrics |
| `atomic_task_result` | `TASK-P7-1`〜`TASK-P7-9` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P7-1`〜`SCN-P7-18` の pass/fail、artifact path |
| `admin_auth_result` | no auth、bad format、不一致、valid、auth disabled、secret redaction の matrix |
| `db_crud_result` | create/list/detail/delete、duplicate、invalid/reserved、deleted route、size_bytes、restart の snapshot |
| `token_crud_result` | create/list/detail/revoke、JWT 一回限り返却、JWT 非保存、TOKEN_NOT_FOUND、restart の snapshot |
| `scope_result` | global `a`、`dbs` override、default route、path route、ro/rw read/write matrix |
| `atomicity_concurrency_result` | DB/token metadata atomic update、失敗注入、concurrent create/delete/revoke の整合性 |
| `secret_redaction_result` | response/log/artifact に admin token、JWT、secret、raw claim、SQL args が残らない scan 結果 |
| `unsupported_surface_result` | `/v1/*`、WebSocket、backup/restore、branch、future admin API が成功応答しない snapshot |
| `compatibility_baseline_result` | Phase 1〜6 regression、TypeScript SDK default/path CRUD、hrana auth/scope transcript |
| `coverage_closure_result` | admin auth、DB CRUD、token CRUD、scope、revoke、atomicity、concurrency、unsupported、regression の gap 0 件 |
| `review_handoff_result` | 第三者が admin API、token API、scope、revoke、restart、concurrency を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 7 で `/admin/v1/databases` と `/admin/v1/tokens` が成功応答になり、DB scope JWT が有効になる release note |
| `precision_closure_result` | Admin auth、DB CRUD、token CRUD、DB scope JWT、revoke immediate effect、metadata/delete consistency、Phase 1〜6 regression の artifact path、reviewer 再現 command、`P7-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 8 完全実装精度固定契約：**

Phase 8 は Turso Cloud 互換の管理モデルを自己ホスト環境へ導入する Phase である。Phase 8 は organization、group、location、quota、usage、Turso Platform API `/v1/*`、既存 `/admin/v1/*` の scope 拡張、Phase 7 metadata migration を同時に完成させる。Phase 8 で WebSocket、ATTACH、metrics API、replication、backup/restore、branch、extension、HA を成功応答にしてはならない。Phase 8 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 8 固定仕様 |
|------|------------------|
| 実装範囲 | organization / group / location / quota / usage metadata、Admin API 拡張、Turso Platform API `/v1/*`、Platform token、metadata migration、scope/quota 判定 |
| default scope | 初回 Phase 8 起動時に `default` organization、`default` group、`default` location、無制限 quota を作成する |
| migration | Phase 7 `databases.json` / `tokens.json` を preflight 検証し、全 DB/token に default scope を付与する。preflight 失敗時は書き込み前に起動失敗 |
| rollback | migration 中断後に partial metadata を成功扱いしない。rollback marker または未完了 marker がある場合は起動失敗し、operator 手動復旧を要求する |
| `/admin/v1/*` | Adlaire 管理 API schema を維持し、organization/group/location/quota/usage を追加する。unknown field は `INVALID_REQUEST` |
| `/v1/*` | Turso Platform API 互換 surface。path、method、query、主要 wrapper、field casing、status を snapshot で固定する |
| wrapper boundary | `/admin/v1/*` と `/v1/*` の response wrapper を混同しない。片方の実装都合で他方の body shape を変更しない |
| Platform auth | `/v1/*` は Platform token または Phase 8 の Admin token 代用 Bearer 完全一致。欠落は `AUTH_REQUIRED`、不一致/形式不正は `AUTH_INVALID` |
| Platform token | `POST /v1/auth/api-tokens/{tokenName}` は `adlpt_` prefix の不透明 token を作成し、secret は作成時のみ返す |
| DB token | `/v1/organizations/{org}/databases/{db}/auth/tokens` は Turso 互換 query `expiration` / `authorization` を受け、response は `{"jwt":string}` 固定 |
| auth rotate | DB/group auth rotate は該当 scope の DB token / group scope token を失効する。Platform API token 自体は失効しない |
| scope precedence | `org` claim、`grp` claim、`dbs` claim、`a` claim、block/quota を §9.1.18 の順序で評価する。`dbs` は DB 単位の最終制限として維持する |
| quota | organization、group、database の順に評価し、usage 取得不能時は `USAGE_UNAVAILABLE`。quota 超過 write は `/v1/*` では 402、admin/hrana では 403 `QUOTA_EXCEEDED` |
| block policy | `block_reads`、`block_writes`、`delete_protection`、`allow_attach` を metadata として保存し、対象 operation の前に判定する |
| metadata files | `organizations.json`、`groups.json`、`locations.json`、`quotas.json`、`usage.json` を §9.6 の atomic update 契約で管理する |
| legacy compatibility | Phase 1〜7 の DB/token は rename せず読み取り・接続・削除可能。Phase 8 新規 DB は Turso 互換名 validation を適用する |
| snapshot | Turso 互換 snapshot は `tests/snapshots/phase8_turso/` に保存し、UUID/timestamp/JWT/request id/host を placeholder 正規化する |

**Phase 8 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P8-1` | metadata schema と default scope を固定する | §2.4、§9.6 | `org/*`、`location/*`、`quota/*` | default scope 欠落、単一 JSON 統合 | default org/group/location/quota が作成される | metadata fixture |
| `TASK-P8-2` | Phase 7 migration / rollback を固定する | §9.1.23、§9.6 | migration / metadata | partial migration success | preflight、migration、rollback marker が固定 | migration fixture |
| `TASK-P8-3` | Admin API scope 拡張を固定する | §6.4、§9.5 | `http/admin/*` | `/admin` wrapper 破壊 | org/group/location/quota/usage API が admin schema で動く | admin snapshots |
| `TASK-P8-4` | Turso Platform API `/v1/*` を固定する | §6.4、§9.5、Phase 8 詳細 | `http/platform/*` | `/admin` body 流用 | Turso wrapper / casing / status が snapshot 一致 | Turso snapshots |
| `TASK-P8-5` | Platform token を固定する | §6.4、§9.1.18 | `auth/*`、`token/*` | JWT と混同、secret 再表示 | create/validate/revoke が規定通り | platform token transcript |
| `TASK-P8-6` | DB token / auth rotate を固定する | Phase 8 詳細、§5.4 | `http/platform/*`、`token/*` | rotate が Platform token を失効 | DB/group token 発行・rotate が固定 | auth rotate snapshots |
| `TASK-P8-7` | scope precedence を固定する | §9.1.18 | `auth/*`、`http/*` | org/grp/dbs 順序揺れ | scope denial matrix が pass | scope matrix |
| `TASK-P8-8` | quota / usage / block policy を固定する | §9.1.18、Phase 8 詳細 | `quota/*`、`usage/*` | usage 不明で成功、status 混同 | quota/block precedence が固定 | quota matrix |
| `TASK-P8-9` | legacy fallback と unique 制約を固定する | §6.4、§9.6 | metadata / validation | legacy rename、重複許可 | legacy 接続と新規 validation が両立 | legacy fixture |
| `TASK-P8-10` | Turso snapshot artifact / secret scan を固定する | §9.1.33、Phase 8 snapshot 表 | tests / artifact | snapshot 手動改変、secret 混入 | snapshot completeness と scan が pass | snapshot audit |
| `TASK-P8-11` | Phase 8 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が migration/API/quota/snapshot を再現できる | handoff checklist |

**Phase 8 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P8-1` | Phase 7 data-dir で Phase 8 起動 | default scope 付与、legacy DB/token 維持 | migration transcript |
| `SCN-P8-2` | migration preflight で壊れた metadata | 書き込み前に起動失敗 | preflight failure fixture |
| `SCN-P8-3` | migration 中断 marker あり | 起動失敗。自動上書きしない | rollback marker fixture |
| `SCN-P8-4` | `POST /admin/v1/organizations` / list/detail/delete | Admin schema、201/200/204、unique 制約 | admin org snapshot |
| `SCN-P8-5` | `POST /admin/v1/locations`、`POST /admin/v1/groups` | location/group 関連と not found が固定 | admin group/location snapshot |
| `SCN-P8-6` | Phase 7 互換 `POST /admin/v1/databases {"name":"db2"}` | default org/group/location が付与され成功 | legacy admin DB snapshot |
| `SCN-P8-7` | Phase 8 新規 DB 名 uppercase / underscore / 64 超過 | HTTP 400 `INVALID_DB_NAME` | name validation snapshot |
| `SCN-P8-8` | `/admin/v1/quotas` / `/admin/v1/usage` | quota/usage schema、scope query、usage unavailable が固定 | quota/usage snapshot |
| `SCN-P8-9` | org/grp/dbs scope が衝突 | precedence 通り allow/deny。存在漏洩なし | scope precedence matrix |
| `SCN-P8-10` | quota exceeded write | `/v1/*` は 402、admin/hrana は 403、code は `QUOTA_EXCEEDED` | quota status snapshot |
| `SCN-P8-11` | block_reads / block_writes / delete_protection | operation 前に拒否し metadata/DB を変更しない | block policy matrix |
| `SCN-P8-12` | `GET /v1/locations` | 200 Turso 互換 locations wrapper | Turso snapshot |
| `SCN-P8-13` | `/v1/organizations/{org}/groups` create/list/detail/config | Turso wrapper / field casing が一致 | Turso group snapshots |
| `SCN-P8-14` | `/v1/organizations/{org}/databases` create/list/detail/delete/config | Turso wrapper / field casing / status が一致 | Turso DB snapshots |
| `SCN-P8-15` | `/v1/organizations/{org}/databases/{db}/auth/tokens` | 200 `{"jwt":string}`、secret は作成時のみ | DB token snapshot |
| `SCN-P8-16` | DB/group auth rotate | 対象 DB/group token は失効、Platform token は有効 | rotate transcript |
| `SCN-P8-17` | `/v1/auth/api-tokens/{tokenName}` create/validate/delete | `adlpt_` token、作成時のみ secret、revoke 後 invalid | platform token transcript |
| `SCN-P8-18` | route priority: auth/tokens、configuration、detail | 詳細 route と特殊 route を取り違えない | route priority snapshot |
| `SCN-P8-19` | unknown field/query/null/duplicate query | `INVALID_REQUEST` | request validation snapshots |
| `SCN-P8-20` | unsupported Turso API audit logs / transfer / branch seed | 501 / 405 / 400 が固定 | unsupported snapshots |
| `SCN-P8-21` | snapshot directory secret scan | JWT、Bearer、adlpt_、admin token 生値なし | secret scan result |
| `SCN-P8-22` | Phase 1〜7 regression + SDK CRUD | すべて pass | regression transcript |

**Phase 8 禁止事項：**

| 状態 | 判定 |
|------|------|
| Turso 互換 `/v1/*` の wrapper / field casing を `/admin/v1/*` と混同する | merge 不可 |
| Phase 7 legacy DB/token を rename または削除して migration する | merge 不可 |
| migration preflight 失敗後に partial metadata を書く | merge 不可 |
| Platform token を JWT として扱う、または `adlpt_` secret を再表示する | merge 不可 |
| DB/group auth rotate で Platform API token を失効する | merge 不可 |
| usage unavailable を quota allow として成功扱いにする | merge 不可 |
| `/v1/*` quota 超過 status と admin/hrana quota 超過 status を混同する | merge 不可 |
| org/grp/dbs/a/scope precedence を endpoint ごとに変える | merge 不可 |
| UUID/timestamp/JWT/host/request id を未正規化のまま snapshot 比較する | Phase 未完了 |
| Turso snapshot artifact、migration fixture、legacy fallback、secret scan、Phase 1〜7 regression なしで完了扱いにする | Phase 未完了 |

**Phase 8 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | organization/group/location/quota/usage、Admin API 拡張、Turso Platform API `/v1/*`、Platform token、DB token/rotate、metadata migration、scope/quota 判定 |
| `excluded_scope` | WebSocket、ATTACH、metrics API、replication、backup/restore、branch、extension、HA、内製化 |
| `atomic_task_result` | `TASK-P8-1`〜`TASK-P8-11` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P8-1`〜`SCN-P8-22` の pass/fail、artifact path |
| `migration_result` | Phase 7 metadata preflight、migration、rollback marker、legacy fallback、restart 復元 |
| `metadata_result` | organizations/groups/locations/quotas/usage/databases/tokens の schema、unique、atomic update、破損時挙動 |
| `admin_api_result` | `/admin/v1/*` organization/group/location/quota/usage と既存 DB/token 拡張の snapshot |
| `turso_platform_result` | `/v1/*` wrapper、field casing、status、route priority、unsupported API、snapshot completeness |
| `auth_scope_result` | Platform token、DB token、org/grp/dbs/a precedence、auth rotate、scope denial matrix |
| `quota_usage_result` | quota exceeded、usage unavailable、block_reads、block_writes、delete_protection、allow_attach の precedence |
| `compatibility_baseline_result` | Phase 1〜7 regression、TypeScript SDK default/path CRUD、Turso snapshot diff、legacy metadata fallback |
| `secret_redaction_result` | response/log/snapshot/artifact に admin token、Platform token、JWT、secret、raw claim、SQL args が残らない scan |
| `coverage_closure_result` | migration、metadata、admin API、Platform API、auth/scope、quota、legacy、unsupported、regression の gap 0 件 |
| `review_handoff_result` | 第三者が migration、Admin API、Turso API、scope、quota、snapshot、secret scan を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 8 で Turso Cloud 互換の organization/group/location/quota/usage と `/v1/*` が公開される release note |
| `precision_closure_result` | organization/group/location/quota/usage、`/v1/*` wrapper、legacy metadata migration、scope auth、quota enforcement、Turso snapshot、Phase 1〜7 regression の artifact path、reviewer 再現 command、`P8-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 9 完全実装精度固定契約：**

Phase 9 は hrana-ws v3 WebSocket surface と interactive transaction を完成させる Phase である。Phase 9 で成功応答してよい新規外部 API は `GET /v3/baton` と `GET /{db-name}/v3/baton` の WebSocket upgrade のみであり、ATTACH、metrics API、replication、backup/restore、branch、extension、HA を実装してはならない。Phase 9 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 9 固定仕様 |
|------|------------------|
| 実装範囲 | WebSocket upgrade、subprotocol、hello auth、stream lifecycle、execute/batch/sequence/describe、store_sql/close_sql、interactive transaction、disconnect rollback |
| endpoint | `GET /v3/baton` は `default` DB、`GET /{db-name}/v3/baton` は path DB。unknown DB は upgrade 前に HTTP 404 `DB_NOT_FOUND` |
| upgrade validation | `Connection: upgrade`、`Upgrade: websocket`、`Sec-WebSocket-Key`、`Sec-WebSocket-Version: 13` 必須。不正は HTTP 400 |
| subprotocol | client 提示順を保持し、`hrana3`、`hrana2`、`hrana1` の最初に一致した値を選択する。`hrana3-protobuf` は Phase 9 では選択しない |
| auth | upgrade 後の `hello` で JWT 認証する。hello 失敗時は `hello_error` を返し、未処理 request へ response を返さず close |
| hello pipeline | client は hello 応答前に request を送ってよい。server は hello 認証成功後、受信順で処理する |
| message size | 1 frame 最大 1 MiB。超過は close code 1009 |
| unknown field | hrana wire object の unknown field は互換のため無視する。管理 API の unknown field 拒否方針を適用しない |
| request_id | connection 内で response と 1:1 対応する。重複 request_id は許可するが、response は該当 request の順序と body に対応させる |
| stream lifecycle | `open_stream` 後だけ execute/batch/sequence/describe/store_sql/close_sql を処理する。close 後 stream_id 再利用は新規 open_stream まで不可 |
| transaction | stream ごとに dedicated connection または同等 isolation を持つ。BEGIN/COMMIT/ROLLBACK は同一 stream に閉じる |
| rollback | close_stream、connection close、server shutdown で open transaction がある場合は rollback を試行する。COMMIT 成功応答前の切断は success と扱わない |
| batch/sequence | `batch` は step 順序を保持して結果/エラーを返す。`sequence` は SQL text 全体を execute_batch に渡し、自前 semicolon split 禁止 |
| describe | SQL を実行せず parameter/column metadata を返す。prepare error は `response_error` |
| store_sql | `sql_id` は connection 内だけ有効。未登録 id 参照、二重登録、close 後参照は `INVALID_REQUEST` |
| cursor API | `open_cursor` / `fetch_cursor` / `close_cursor` は Phase 9 では `NOT_IMPLEMENTED` の `response_error` とし、connection は維持する |
| permission | JWT `a` / `dbs` / org / group / quota / block policy は Phase 8 の precedence を維持し、operation ごとに判定する |
| persistence | WebSocket session、stream、store_sql は永続化しない。SQL commit 済みデータだけ DB に残る |

**Phase 9 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P9-1` | WebSocket upgrade / route を固定する | §6.3、§9.5 | `ws/*`、`http/mod.rs` | HTTP route 破壊、unknown DB upgrade | default/path upgrade が規定通り | upgrade snapshots |
| `TASK-P9-2` | subprotocol / frame validation を固定する | §9.4.2 Phase 9 | `ws/*` | protobuf 選択、size 無制限 | subprotocol と close code が固定 | subprotocol transcript |
| `TASK-P9-3` | hello auth と message ordering を固定する | §6.3、§9.1.18 | `ws/session.rs` | hello 前 request 実行、auth leak | hello_ok/error と pipelined request が固定 | hello transcript |
| `TASK-P9-4` | stream lifecycle を固定する | §9.1.28、§9.4.1 | `ws/stream.rs` | open 前 execute、close 後実行 | stream state 禁止遷移が error | stream matrix |
| `TASK-P9-5` | execute/batch/sequence/describe を固定する | §9.1.28、§6.3 | `ws/requests.rs`、`hrana/*` | result order 破壊、自前 split | SQL result/error mapping が互換 | result snapshots |
| `TASK-P9-6` | interactive transaction を固定する | §9.1.28 | `ws/stream.rs`、`db/*` | stream 間 tx 混線、commit 前 success | commit/rollback/disconnect が固定 | transaction fixture |
| `TASK-P9-7` | store_sql / close_sql を固定する | §6.3、§9.4.1 | `ws/sql_cache.rs` | global cache、close 後参照成功 | connection-local SQL cache が固定 | store_sql snapshot |
| `TASK-P9-8` | cursor / unknown request を固定する | §9.4.2 Phase 9 | `ws/requests.rs` | cursor success、connection 強制 close | cursor は NOT_IMPLEMENTED、unknown は INVALID_REQUEST | unsupported snapshot |
| `TASK-P9-9` | SDK WebSocket regression を固定する | §9.1.1、§9.8 | tests / artifact | manual transcript のみ | TypeScript SDK transaction が pass | SDK transcript |
| `TASK-P9-10` | Phase 9 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が WS/tx/rollback を再現できる | handoff checklist |

**Phase 9 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P9-1` | `GET /v3/baton` valid upgrade | HTTP 101、default DB WebSocket session | upgrade transcript |
| `SCN-P9-2` | `GET /{db-name}/v3/baton` valid / unknown DB | valid は 101、unknown は upgrade 前 404 `DB_NOT_FOUND` | path upgrade snapshot |
| `SCN-P9-3` | invalid upgrade headers / version | HTTP 400。WebSocket session を開始しない | invalid upgrade snapshot |
| `SCN-P9-4` | subprotocol `hrana3-protobuf, hrana3, hrana2` | `hrana3` を選択。protobuf は選択しない | subprotocol snapshot |
| `SCN-P9-5` | valid JWT hello | `hello_ok` | hello ok transcript |
| `SCN-P9-6` | invalid / expired / revoked JWT hello | `hello_error` + close、未処理 request response なし | hello error transcript |
| `SCN-P9-7` | hello 前に request を送る | hello 成功後に受信順で処理。hello 失敗時は response なし | pipelined hello transcript |
| `SCN-P9-8` | open_stream 前 execute | `response_error` `INVALID_REQUEST` | stream validation snapshot |
| `SCN-P9-9` | execute SELECT / INSERT on open stream | hrana-ws result mapping が snapshot 一致 | execute snapshot |
| `SCN-P9-10` | batch with mixed success/error | step 順序維持、connection 維持 | batch snapshot |
| `SCN-P9-11` | sequence with semicolon in string literal | execute_batch 境界を維持し自前 split しない | sequence snapshot |
| `SCN-P9-12` | describe valid / invalid SQL | 実行せず metadata または `response_error` | describe snapshot |
| `SCN-P9-13` | TypeScript SDK transaction commit | commit 後 SELECT で反映 | SDK tx commit transcript |
| `SCN-P9-14` | transaction rollback | rollback 後 SELECT で未反映 | tx rollback transcript |
| `SCN-P9-15` | open tx 中に connection close | rollback され、再接続後に未反映 | disconnect rollback transcript |
| `SCN-P9-16` | multi stream transaction + read | stream 間 state が混線しない | multi stream transcript |
| `SCN-P9-17` | store_sql、execute by sql_id、close_sql、再参照 | close 後再参照は `INVALID_REQUEST` | store_sql transcript |
| `SCN-P9-18` | cursor request | `response_error` `NOT_IMPLEMENTED`、connection 維持 | cursor snapshot |
| `SCN-P9-19` | 1 MiB 超過 frame | close code 1009 | frame size transcript |
| `SCN-P9-20` | ro token write / block_writes / quota exceeded | Phase 8 precedence 通り拒否 | permission matrix |
| `SCN-P9-21` | unknown field in hrana message | 互換のため無視し、既知 field で処理 | forward compatibility snapshot |
| `SCN-P9-22` | Phase 1〜8 regression + WS SDK regression | すべて pass | regression transcript |

**Phase 9 禁止事項：**

| 状態 | 判定 |
|------|------|
| `hrana3-protobuf` を選択または protobuf schema なしで実装する | merge 不可 |
| hello 認証前に SQL request を実行する | merge 不可 |
| hello 失敗時に未処理 request へ response を返す | merge 不可 |
| stream 間で transaction state / connection state を共有して混線させる | merge 不可 |
| close_stream / disconnect 後に open transaction を残す | merge 不可 |
| COMMIT 成功応答前の切断を success と扱う | merge 不可 |
| `sequence` を ad hoc semicolon split する | merge 不可 |
| store_sql cache を connection 外へ永続化または共有する | merge 不可 |
| cursor API を Phase 9 で成功応答にする | merge 不可 |
| WebSocket transcript、transaction rollback fixture、SDK regression、secret scan なしで完了扱いにする | Phase 未完了 |

**Phase 9 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | WebSocket upgrade、subprotocol、hello auth、stream lifecycle、execute/batch/sequence/describe、store_sql/close_sql、interactive transaction、disconnect rollback |
| `excluded_scope` | ATTACH、metrics API、replication、backup/restore、branch、extension、HA、protobuf WebSocket |
| `atomic_task_result` | `TASK-P9-1`〜`TASK-P9-10` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P9-1`〜`SCN-P9-22` の pass/fail、artifact path |
| `upgrade_result` | default/path upgrade、invalid headers、unknown DB、subprotocol、frame size close code |
| `hello_auth_result` | valid/invalid/expired/revoked JWT、pipelined hello、hello failure close の transcript |
| `stream_result` | open/close、open 前 request、close 後 request、multi stream isolation |
| `sql_result_mapping_result` | execute、batch、sequence、describe、SQL error、unknown field の wire snapshot |
| `transaction_result` | commit、rollback、disconnect rollback、close_stream rollback、COMMIT 前切断 |
| `store_sql_result` | store_sql、sql_id execute、close_sql、unknown id、double register、connection-local boundary |
| `compatibility_baseline_result` | TypeScript SDK WebSocket transaction、Phase 1〜8 regression、HTTP SDK regression |
| `secret_redaction_result` | WebSocket transcript/log/artifact に JWT、Bearer、Platform/admin token、SQL args 生値が残らない scan |
| `coverage_closure_result` | upgrade、hello、stream、SQL、transaction、store_sql、permission、unsupported、regression の gap 0 件 |
| `review_handoff_result` | 第三者が WebSocket upgrade、SDK transaction、rollback、store_sql、secret scan を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 9 で hrana-ws v3 `/v3/baton` と `/{db-name}/v3/baton` が公開され、interactive transaction が利用可能になる release note |
| `precision_closure_result` | WebSocket upgrade、hello/auth、stream/cursor lifecycle、interactive transaction commit/rollback、close cleanup、SDK WS transcript、Phase 1〜8 regression の artifact path、reviewer 再現 command、`P9-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 10 完全実装精度固定契約：**

Phase 10 は管理下 DB 間の ATTACH と、Phase 1〜10 の HTTP/WebSocket/DB 実行を観測するインメモリ metrics API を完成させる Phase である。Phase 10 で成功応答してよい新規外部 API は `GET /admin/v1/metrics` のみであり、replication、backup/restore、branch、extension、Prometheus、metrics 永続化、HA を実装してはならない。Phase 10 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 10 固定仕様 |
|------|-------------------|
| 実装範囲 | ATTACH SQL interception、managed DB path resolution、ATTACH policy、hrana-http/ws 適用、in-memory metrics counters/gauges、`GET /admin/v1/metrics` |
| ATTACH 対象 | `ATTACH DATABASE '<db-name>' AS <alias>` と `ATTACH '<db-name>' AS <alias>` のみ。single quote DB name のみ許可 |
| parser | SQL tokenizer または SQLite prepare 前の限定 parser を使う。正規表現だけの判定、semicolon split、文字列リテラル/コメント/quoted identifier 内の誤検出は禁止 |
| DB name | ATTACH DB 名は Phase 8 metadata に存在する管理下 DB のみ。任意 path、絶対 path、相対 path、URL、空、未登録 DB は成功させない |
| alias | alias は `^[A-Za-z_][A-Za-z0-9_]{0,63}$`。quote、dot、slash、空、SQLite reserved collision は `INVALID_REQUEST` |
| path rewrite | libsql / SQLite へ渡す前に管理下 DB の実 path に解決する。client supplied path をそのまま渡さない |
| auth/scope | source DB と attach target DB の両方に JWT/org/group/db scope が必要。どちらか scope 外なら `ORG_SCOPE_DENIED` または `PERMISSION_DENIED` |
| block policy | `allow_attach=false`、`block_reads=true`、`block_writes=true`、ro token の優先順位を §9.4.2 の Phase 10 表に従って固定する |
| transaction | active transaction 中の ATTACH/DETACH は SQLite の結果に従うが、任意 path validation と permission 判定は必ず先に行う |
| protocol coverage | hrana-http `execute` / `sequence` と hrana-ws `execute` / `batch` / `sequence` の全経路で同一 ATTACH policy を適用する |
| metrics API | `GET /admin/v1/metrics` は Admin token 必須。body あり、unknown query、非 GET は拒否する |
| metrics persistence | Phase 10 metrics はプロセス内のみ。再起動で reset される。永続 metrics と Prometheus は Phase 17 |
| metrics update | 成功/失敗 HTTP request、WebSocket connection、SQL execution、SQL error、auth denial、storage busy を規定 counter に反映する |
| counter boundary | SQL execution counter は libSQL に渡した step のみ加算する。validation で拒否した SQL は SQL execution counter に含めない |
| gauge boundary | `size_bytes` / `wal_size_bytes` は response 時に filesystem から取得する。取得失敗は WARN + field `0` |
| secret | metrics label、log、snapshot に token、JWT、SQL args、生 SQL bind 値、絶対 extension path を含めない |

**Phase 10 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P10-1` | ATTACH parser 境界を固定する | §9.1.28、§9.4.2 Phase 10 | `attach/*` | regex-only、semicolon split | parser cases が誤検出なし | parser fixture |
| `TASK-P10-2` | managed DB path resolution を固定する | §3.4、§9.6 | `attach/*`、`db/manager.rs` | client path pass-through | 管理下 DB のみ path 解決 | path resolution snapshot |
| `TASK-P10-3` | ATTACH policy precedence を固定する | §9.1.18、§9.4.2 | `attach/*`、`auth/*` | scope/block/quota 順序揺れ | allow/deny matrix が固定 | policy matrix |
| `TASK-P10-4` | hrana-http/ws 適用を固定する | §6.2、§6.3、§9.1.28 | `http/pipeline.rs`、`ws/*` | HTTP/WS 差分 | 全 SQL 経路で同一 policy | protocol matrix |
| `TASK-P10-5` | metrics schema を固定する | §6.4、§9.5 | `metrics.rs`、`http/admin/metrics.rs` | Prometheus 混入、永続化 | JSON schema が snapshot 一致 | metrics snapshot |
| `TASK-P10-6` | metrics update points を固定する | §9.1.17、§9.4.1 | `metrics.rs`、HTTP/WS hooks | lost increment、二重 decrement | 成功/失敗/WS close が反映 | counter fixture |
| `TASK-P10-7` | metrics auth / validation を固定する | §6.4、§9.1.18 | `http/admin/metrics.rs` | auth bypass、body 黙認 | Admin token / invalid request が固定 | metrics auth snapshot |
| `TASK-P10-8` | secret redaction / labels を固定する | §9.1.17、§9.1.18 | metrics / logs / tests | token/SQL args label | secret scan が pass | secret scan artifact |
| `TASK-P10-9` | Phase 10 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が ATTACH/metrics を再現できる | handoff checklist |

**Phase 10 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P10-1` | `ATTACH DATABASE 'db_b' AS b` on managed DB | 成功し cross DB SELECT 可能 | attach success transcript |
| `SCN-P10-2` | `ATTACH '/etc/passwd' AS evil` | `INVALID_REQUEST` または `PERMISSION_DENIED`、path 未使用 | arbitrary path snapshot |
| `SCN-P10-3` | `ATTACH DATABASE '../db' AS x` | `INVALID_DB_NAME` / `INVALID_REQUEST` | traversal snapshot |
| `SCN-P10-4` | 未登録 DB を ATTACH | `DB_NOT_FOUND` | missing DB snapshot |
| `SCN-P10-5` | invalid alias / quoted alias / dot alias | `INVALID_REQUEST` | alias matrix |
| `SCN-P10-6` | SQL string/comment 内の `ATTACH` | ATTACH と誤検出しない | parser cases artifact |
| `SCN-P10-7` | `allow_attach=false` の source/target | `PERMISSION_DENIED` | allow_attach snapshot |
| `SCN-P10-8` | scope 外 target DB | `ORG_SCOPE_DENIED` または `PERMISSION_DENIED` | scope denial snapshot |
| `SCN-P10-9` | `block_reads=true` target を read source にする | `PERMISSION_DENIED` | block read snapshot |
| `SCN-P10-10` | ro token で ATTACH 後 write | `PERMISSION_DENIED` | ro attach write snapshot |
| `SCN-P10-11` | hrana-http execute/sequence ATTACH | HTTP 経路で policy 一致 | HTTP attach transcript |
| `SCN-P10-12` | hrana-ws execute/batch/sequence ATTACH | WebSocket 経路で policy 一致 | WS attach transcript |
| `SCN-P10-13` | `GET /admin/v1/metrics` valid Admin token | 200 metrics JSON、managed DB を含む | metrics snapshot |
| `SCN-P10-14` | metrics no auth / bad token | 401 `AUTH_REQUIRED` / `AUTH_INVALID` | metrics auth snapshot |
| `SCN-P10-15` | metrics with body / unknown query / non GET | `INVALID_REQUEST` / `METHOD_NOT_ALLOWED` | metrics validation snapshot |
| `SCN-P10-16` | HTTP success/failure and SQL success/error | counters が規定通り増える | counter update transcript |
| `SCN-P10-17` | WebSocket connect/close | `connections_active` が二重 decrement なしで戻る | WS metrics transcript |
| `SCN-P10-18` | server restart | Phase 10 metrics は reset される | reset transcript |
| `SCN-P10-19` | metrics secret scan | token、JWT、SQL args、生 bind 値が残らない | secret scan result |
| `SCN-P10-20` | Phase 1〜9 regression + SDK HTTP/WS | すべて pass | regression transcript |

**Phase 10 禁止事項：**

| 状態 | 判定 |
|------|------|
| 正規表現だけで ATTACH を判定する | merge 不可 |
| client supplied path を SQLite / libsql に直接渡す | merge 不可 |
| 任意 path、相対 path、絶対 path、URL を ATTACH 成功させる | merge 不可 |
| hrana-http と hrana-ws で ATTACH policy が異なる | merge 不可 |
| `allow_attach=false`、scope denial、block policy、ro token の precedence を証跡なしで実装する | Phase 未完了 |
| Phase 10 metrics を永続化または Prometheus endpoint として公開する | merge 不可。Phase 17 対象 |
| metrics label / response / artifact に secret、JWT、SQL args、生 bind 値を含める | merge 不可 |
| WebSocket close 時に `connections_active` を二重 decrement する、または decrement 漏れする | Phase 未完了 |
| ATTACH allow/deny、任意 path 拒否、metrics counter snapshot、Phase 1〜9 regression なしで完了扱いにする | Phase 未完了 |

**Phase 10 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | ATTACH SQL interception、managed DB path resolution、ATTACH policy、hrana-http/ws 適用、in-memory metrics、`GET /admin/v1/metrics` |
| `excluded_scope` | replication、backup/restore、branch、extension、Prometheus、metrics 永続化、HA、内製化 |
| `atomic_task_result` | `TASK-P10-1`〜`TASK-P10-9` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P10-1`〜`SCN-P10-20` の pass/fail、artifact path |
| `attach_parser_result` | parser cases、string/comment/quoted identifier 誤検出なし、semicolon split なし |
| `attach_policy_result` | allow_attach、scope、block_reads、block_writes、ro/rw、quota、unknown DB、invalid alias/path の matrix |
| `protocol_coverage_result` | hrana-http execute/sequence、hrana-ws execute/batch/sequence の ATTACH transcript |
| `metrics_schema_result` | `GET /admin/v1/metrics` schema、auth、validation、counter/gauge field、filesystem failure handling |
| `metrics_update_result` | HTTP/WS/SQL success/failure、WebSocket close、storage/auth errors の counter update artifact |
| `compatibility_baseline_result` | Phase 1〜9 regression、TypeScript SDK HTTP/WS transcript、Turso / libSQL compatibility 差分なし |
| `secret_redaction_result` | metrics/log/snapshot/artifact に token、JWT、SQL args、生 bind 値が残らない scan |
| `coverage_closure_result` | ATTACH parser、path、policy、protocol、metrics、auth、secret、regression の gap 0 件 |
| `review_handoff_result` | 第三者が ATTACH allow/deny、任意 path 拒否、metrics counter、secret scan を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 10 で管理下 DB の ATTACH と `GET /admin/v1/metrics` が公開されるが、metrics は再起動で reset される release note |
| `precision_closure_result` | managed ATTACH allow/deny、arbitrary path reject、metrics counter/gauge、admin auth、secret redaction、Phase 1〜9 regression の artifact path、reviewer 再現 command、`P10-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 8〜10 の境界決定：**

- location、organization/group、quota/usage は Phase 8 で完了条件に含める
- WebSocket の transaction/session 維持は Phase 9
- クロス DB クエリ、ATTACH policy、metrics は Phase 10
- Phase 9 の WebSocket 実装中に metrics hook を入れてよいが、metrics API 完了条件には含めない

#### Phase 11〜15：レプリケーション・WAL アーカイブ・バックアップ・ブランチ

| Phase | 変更対象 | API 契約 | 永続化 | エラー/ログ | テスト契約 |
|-------|----------|----------|--------|-------------|------------|
| Phase 11 | `replication/primary.rs`, `http/replication.rs`, `state.rs`, `config.rs` | primary role で `/replication/v1/log`, `/snapshot`, `/heartbeat`, `/status` を提供する。replica loop は未完了でよい | replication state はプロセス内。snapshot response は DB の整合した byte stream を返す | replication token 不正は `AUTH_INVALID`。from_frame 不正は `INVALID_REQUEST`。checksum を必ず出す | API contract tests、snapshot header、heartbeat status、SSE/long-poll 挙動 |
| Phase 12 | `replication/replica.rs`, redirect middleware, `http/health.rs` | replica role で primary から snapshot/WAL を取得する。replica 書き込みは 307 redirect。primary 到達不能時の挙動を固定する | replica 側 DB に WAL 適用済み状態を保存する。再起動後は最後の frame から再開する | checksum mismatch は frame skip + ERROR log + 再取得対象。health は `ok`/`degraded` を返す | TC-4-1〜TC-4-5。停止/復帰、primary down、multi replica |
| Phase 13 | `wal/archive.rs`, `wal/manifest.rs`, cleanup task, `config.rs` | 外部 API 追加は不要。archive/cleanup は内部機能 | `wal-archive/manifest.json`, frame files, snapshot file を atomic update。retention cleanup は manifest と file を同時整合 | manifest 破損は起動失敗。frame missing は ERROR。cleanup の削除件数を INFO log | TC-5-8。retention、manifest/files consistency、CRC mismatch detection |
| Phase 14 | `http/admin/backup.rs`, restore service, PITR service | backup/restore/PITR API を §6.4 通り実装する。restore/PITR は admin auth 必須 | restore 前に元 DB を退避し、失敗時は必ず rollback。PITR は snapshot + WAL replay | `PITR_NOT_ENABLED`, `FRAME_NOT_FOUND`, `RESTORE_INTEGRITY_FAILED`, `RESTORE_FRAME_CORRUPT` を使用 | TC-5-1〜TC-5-7。同時書き込み、不正 file、範囲外、CRC 破壊、rollback |
| Phase 15 | `http/admin/branches.rs`, branch metadata, `db/manager.rs`, PITR helper | branch CRUD API を実装し、branch DB は `{db}___{branch}` として通常 pipeline でアクセスする | `meta/branches.json` と branch DB directory を atomic に整合。削除時は metadata と directory の両方を消す | branch 名不正は `INVALID_DB_NAME`、`___` 衝突は `DB_RESERVED_NAME`。削除済み branch pipeline は `DB_NOT_FOUND` | TC-6-1〜TC-6-7。current/timestamp/frame branch、独立書き込み、再起動復元 |

**Phase 11 完全実装精度固定契約：**

Phase 11 は primary role の replication 送信側 API を完成させる Phase である。Phase 11 で成功応答してよい新規外部 API は primary port 上の `/replication/v1/log`、`/replication/v1/snapshot`、`/replication/v1/heartbeat`、`/replication/v1/status` のみであり、replica apply、write redirect、replica health lag、WAL archive retention、backup/restore、branch、HA を実装してはならない。Phase 11 実装者は下表を Phase packet、atomic task ledger、scenario matrix、Done receipt、review handoff、operator behavior delta に転記してから実装する。

| 項目 | Phase 11 固定仕様 |
|------|-------------------|
| 実装範囲 | `--role primary`、primary port、replication token auth、WAL frame stream、snapshot stream、heartbeat receive、primary status |
| role | `standalone` は既定。`primary` のみ replication API を起動する。`replica` は Phase 12 まで起動拒否または明示 unsupported |
| primary port | `--primary-port` 既定 `8082`。client SDK 用 HTTP port と混同しない。port bind 失敗は起動失敗 |
| auth token | replication token が設定されている場合は `Authorization: Bearer <token>` 完全一致必須。prefix/空白/大小文字補正は禁止 |
| auth disabled | primary role で replication token 未設定の場合は replication API を `AUTH_REQUIRED` とする。外部公開状態で無認証成功させない |
| `/replication/v1/log` | `GET` + query `from_frame` 必須。SSE で frame を frame_no 昇順に返し、追いついたら接続維持して heartbeat を送る |
| `from_frame` | 1 以上の integer。0、負数、非数値、重複 query、空 query は `INVALID_REQUEST` |
| frame_no | primary 内で単調増加。DB ごとの frame 順序と全体 stream 順序を transcript に残す |
| checksum | frame payload bytes の CRC32。SSE data、snapshot header、future archive manifest で同じ計算式を使う |
| SSE format | frame は `event: frame`、`id: <frame_no>`、`data: <json>`。heartbeat は `event: heartbeat`。frame bytes は base64 |
| `/replication/v1/snapshot` | 整合した SQLite snapshot byte stream を返す。`Content-Type: application/octet-stream`、base frame header、checksum header 必須 |
| snapshot consistency | snapshot 生成中の write があっても snapshot 内は一貫。temp snapshot は成功/失敗後に cleanup される |
| `/replication/v1/heartbeat` | replica_id、synced_frame、last_seen_primary_frame を受け付ける。unknown replica_id は登録、既存は上書き更新 |
| heartbeat validation | `synced_frame` が primary 最大 frame を超える場合は `INVALID_REQUEST`。負数/非数値/null は拒否 |
| `/replication/v1/status` | primary role、current_frame、connected replica summary、write_mode、auth enabled を返す。secret は返さない |
| write mode | Phase 11 の `async` は ACK を待たない。`sync` は quorum/ACK semantics 未完成なら起動拒否し、silent async fallback しない |
| quota/block | replication apply は Phase 12。Phase 11 primary stream は quota/block 判定で frame 提供を止めない |

**Phase 11 原子タスク台帳：**

| Task ID | Goal | Input contracts | Change targets | Forbidden changes | Completion condition | Verification |
|---------|------|-----------------|----------------|-------------------|----------------------|--------------|
| `TASK-P11-1` | primary role / config validation を固定する | §4、§8.4、§9.13 | `config.rs`、`main.rs` | silent fallback、port 混同 | primary 起動条件が固定 | config fixture |
| `TASK-P11-2` | replication auth を固定する | §9.1.18、§10.1 | `http/replication.rs` | token 補正、secret log | auth matrix が規定 status/code | auth snapshot |
| `TASK-P11-3` | WAL frame model を固定する | §3.6、§9.1.28 | `replication/primary.rs` | frame_no 巻き戻り、checksum なし | frame_no / CRC32 が固定 | frame transcript |
| `TASK-P11-4` | `/replication/v1/log` SSE を固定する | §9.5、Phase 11 詳細 | `http/replication.rs` | JSON polling 代替、順序崩れ | SSE event/id/data が固定 | SSE snapshot |
| `TASK-P11-5` | snapshot stream を固定する | §9.1.23、Phase 11 詳細 | `replication/snapshot.rs` | 不整合 file copy、header 欠落 | consistent byte stream + headers | snapshot artifact |
| `TASK-P11-6` | heartbeat API を固定する | Phase 11 詳細 | `http/replication.rs` | invalid frame 受理、secret response | replica state summary が更新 | heartbeat snapshot |
| `TASK-P11-7` | primary status API を固定する | §9.5、Phase 11 詳細 | `http/replication.rs` | secret 出力、role 混同 | status schema が固定 | status snapshot |
| `TASK-P11-8` | long-poll / cleanup / shutdown を固定する | §9.1.25、§9.4.1 | replication runtime | leaked temp snapshot、hung connection | heartbeat/close/cleanup が再現可能 | lifecycle artifact |
| `TASK-P11-9` | Phase 11 Done receipt / handoff / operator delta | §9.1.33、§9.1.51、§9.1.52 | docs / PR artifact | PR description だけの根拠 | 第三者が replication API を再現できる | handoff checklist |

**Phase 11 シナリオマトリクス：**

| Scenario ID | 入力 | 期待結果 | Evidence |
|-------------|------|----------|----------|
| `SCN-P11-1` | `--role primary --primary-port 8082` | primary port 起動、client port と分離 | startup transcript |
| `SCN-P11-2` | primary port bind 失敗 | 起動失敗、partial service success なし | bind failure fixture |
| `SCN-P11-3` | replication token 欠落 / 不一致 / valid | `AUTH_REQUIRED` / `AUTH_INVALID` / success | auth matrix |
| `SCN-P11-4` | `GET /replication/v1/log` without `from_frame` | `INVALID_REQUEST` | query validation snapshot |
| `SCN-P11-5` | `from_frame=0`、負数、非数値、重複 query | `INVALID_REQUEST` | from_frame matrix |
| `SCN-P11-6` | `from_frame=1` with existing frames | SSE frame_no 昇順、CRC32 付き | SSE frame transcript |
| `SCN-P11-7` | stream が current frame に追いつく | connection 維持、heartbeat event 送信 | long-poll transcript |
| `SCN-P11-8` | write 後に log stream 継続 | 新 frame が次の frame_no で流れる | live stream transcript |
| `SCN-P11-9` | frame payload CRC32 再計算 | response checksum と一致 | checksum artifact |
| `SCN-P11-10` | `GET /replication/v1/snapshot` | 200 octet-stream、base frame/checksum headers | snapshot header artifact |
| `SCN-P11-11` | snapshot 中に concurrent write | snapshot は一貫し、write は後続 frame になる | consistency transcript |
| `SCN-P11-12` | snapshot temp file left by interrupted run | 起動時 cleanup、metadata 変更なし | cleanup fixture |
| `SCN-P11-13` | `POST /replication/v1/heartbeat` unknown replica | 登録され status に反映 | heartbeat transcript |
| `SCN-P11-14` | heartbeat `synced_frame` > primary max | `INVALID_REQUEST` | heartbeat validation snapshot |
| `SCN-P11-15` | `GET /replication/v1/status` | role/current_frame/replicas/write_mode/auth_enabled、secret なし | status snapshot |
| `SCN-P11-16` | `--replication-write-mode sync` without quorum contract | 起動拒否。async へ silent fallback しない | write mode fixture |
| `SCN-P11-17` | replica role / replica apply endpoints | Phase 11 では成功応答なし | unsupported snapshot |
| `SCN-P11-18` | Phase 1〜10 regression + SDK HTTP/WS | すべて pass | regression transcript |

**Phase 11 禁止事項：**

| 状態 | 判定 |
|------|------|
| replication token 未設定 primary で replication API を無認証成功させる | merge 不可 |
| Bearer token を trim / lowercase / 補正して受け付ける | merge 不可 |
| frame_no を巻き戻す、重複させる、checksum なしで送る | merge 不可 |
| SSE ではなく独自 JSON polling だけで `/log` 完了扱いにする | merge 不可 |
| snapshot を単純 file copy し、一貫性と base frame を証明しない | merge 不可 |
| snapshot / frame bytes / replication token を log や artifact に生出力する | merge 不可 |
| `sync` write mode を quorum 未定義のまま async と同じ挙動にする | merge 不可 |
| replica apply、write redirect、replica health lag を Phase 11 完了条件に混ぜる | merge 不可。Phase 12 対象 |
| replication API snapshot、frame/checksum transcript、snapshot consistency、secret scan なしで完了扱いにする | Phase 未完了 |

**Phase 11 Done receipt 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `implemented_scope` | primary role、primary port、replication auth、`/log` SSE、`/snapshot`、`/heartbeat`、`/status`、frame_no、CRC32 |
| `excluded_scope` | replica apply、write redirect、replica health lag、WAL archive retention、backup/restore、branch、HA、内製化 |
| `atomic_task_result` | `TASK-P11-1`〜`TASK-P11-9` がすべて pass、open task 0 件 |
| `scenario_matrix_result` | `SCN-P11-1`〜`SCN-P11-18` の pass/fail、artifact path |
| `config_result` | role、primary_port、replication_auth_token、write_mode、invalid config、secret redaction |
| `replication_auth_result` | no auth、bad format、不一致、valid、token redaction の matrix |
| `frame_stream_result` | SSE event/id/data、frame_no monotonic、CRC32、long-poll heartbeat、live append |
| `snapshot_result` | octet-stream、base frame/checksum headers、concurrent write consistency、temp cleanup |
| `heartbeat_status_result` | heartbeat validation、replica summary、status schema、secret 不在 |
| `compatibility_baseline_result` | Phase 1〜10 regression、TypeScript SDK HTTP/WS transcript、Turso / libSQL compatibility 差分なし |
| `secret_redaction_result` | response/log/snapshot/artifact に replication token、JWT、frame bytes 生値、SQL args が残らない scan |
| `coverage_closure_result` | config、auth、SSE、snapshot、heartbeat、status、cleanup、unsupported、regression の gap 0 件 |
| `review_handoff_result` | 第三者が primary 起動、SSE、snapshot、heartbeat、status、secret scan を再現できる command と artifact |
| `operator_behavior_delta_result` | Phase 11 で primary replication API が公開されるが replica apply / redirect はまだ利用不可である release note |
| `precision_closure_result` | replica registration、WAL stream auth/range/checksum、snapshot consistency、retention、lag metadata、SDK/API compatibility、Phase 1〜10 regression の artifact path、reviewer 再現 command、`P11-PRECISION-CLOSURE`、closure fields（ambiguity / failure / review_handoff / na / go_no_go / regression_inheritance / determinism / assertion_binding / negative_surface / evidence_integrity / operator_observability）すべて pass、§9.11.2 canonical / manifest / reproduction / review algorithm / freeze / reconciliation / remediation / cross-reference ledger 準拠、未解決判断 0 件 |

**Phase 6〜11 完全実装精度正規化契約：**

Phase 6〜11 は外部 API、metadata、JWT scope、WebSocket state、ATTACH、metrics、replication の境界が広がるため、後続 Phase と同じ Done 判定規則を適用する。実装者は「主要 happy path が動く」ことを完了根拠にしてはならない。各 Phase の Done receipt は、原子タスク、シナリオ、禁止事項、互換 baseline、metadata migration / rollback、secret redaction、review handoff、operator behavior delta、precision closure をすべて埋める。

| 項目 | 固定仕様 | 完了不可条件 |
|------|----------|--------------|
| heading normalization | Phase 6〜11 の仕様見出しは `原子タスク台帳`、`シナリオマトリクス`、`Done receipt 必須項目` を正とする | 旧見出しだけを参照して Done 判定する |
| artifact path | Done receipt の pass/fail は artifact path または再現 command を必ず持つ | 口頭説明、PR description、スクリーンショットのみ |
| regression chain | Phase 6 は Phase 1〜5、Phase 7 は Phase 1〜6、以後同様に直前 Phase までの regression をすべて通す | 直前 Phase regression skip、影響なし根拠なし |
| compatibility baseline | libSQL SDK HTTP/WS transcript、Turso Cloud 管理 API snapshot、hrana wire snapshot、legacy metadata fixture のうち該当面を Done receipt に含める | curl smoke のみ、snapshot なし |
| metadata migration | metadata schema を増やす Phase は legacy fixture、atomic write、restart restore、rollback / failure behavior を artifact 化する | parse 失敗上書き、migration 証跡なし |
| auth and scope | Admin token、JWT DB scope、organization/group scope、replication token は互いに混線させず、権限 matrix を artifact 化する | token 種別混同、scope bypass |
| secret redaction | response/stdout/stderr/log/artifact に JWT、Admin token、replication token、SQL args、raw frame bytes が残らない scan を必須にする | scan なし、秘匿値混入 |
| protocol lifecycle | WebSocket、ATTACH、replication stream は open/close/error/restart/reconnect の lifecycle を scenario に含める | happy path のみ |
| future boundary | Phase 6〜11 で backup/restore/PITR/branch/extension/HA/内製化を完成扱いにしない | 未来 Phase の成功応答を Done に含める |
| reviewer reproducibility | 第三者が clean checkout から command で再現できる handoff を必須にする | ローカル状態依存、手順欠落 |
| bug-zero readiness | Done receipt に coverage gap、未解決判断、仕様未確定、既知 flaky が 0 件であることを明記する | 未解決判断を実装者判断へ先送り |

**Phase 11/12 の境界決定：**

- Phase 11 は primary が WAL/snapshot を提供するところまで
- Phase 12 は replica がそれを消費し、health/redirect を完成させるところまで
- write-mode `sync` の完全な quorum/ack semantics は Phase 12 で定義されている範囲のみ実装する。未定義なら async と同じ挙動にしてはならず、起動時に設定エラーとする

**Phase 13/14 の境界決定：**

- Phase 13 は WAL archive を作るだけで、restore API は実装しない
- Phase 14 は archive を使って backup/restore/PITR を公開 API として完成させる
- restore/PITR は破壊的操作なので、途中失敗時の rollback 成功までが API 成功条件である

**Phase 15 の境界決定：**

- branch は通常 DB と同じ routing/auth/backup policy に従う
- branch merge、copy-on-write 最適化、外部 storage 連携は Phase 15 対象外
- branch 名と DB 名は同じ validation を使い、`___` を含む名前は禁止する

#### Phase 16〜19：拡張・監視・HA・内製化

| Phase | 変更対象 | API/CLI 契約 | 永続化 | エラー/ログ | テスト契約 |
|-------|----------|--------------|--------|-------------|------------|
| Phase 16 | `extension/*`, `http/admin/extensions.rs`, `config.rs` | `GET/POST/DELETE /admin/v1/extensions` を追加し、許可済み SQLite 拡張だけをロードする。任意パス指定は禁止し、拡張名は manifest 登録名のみ許可する | `meta/extensions.json` に拡張名、version、sha256、enabled、loaded_at を atomic update。拡張 binary は `{data-dir}/extensions/{name}/{version}/` 配下のみ | 未登録拡張は `EXTENSION_NOT_ALLOWED`、署名不一致は `EXTENSION_SIGNATURE_INVALID`、ロード失敗は `EXTENSION_LOAD_FAILED`。拡張 path と secret はログに出さない | TC-16-1〜TC-16-6。allowlist、署名、load/unload、restart、任意パス拒否、SDK regression |
| Phase 17 | `metrics.rs`, `http/admin/metrics.rs`, `http/admin/prometheus.rs`, `usage/*` | `GET /admin/v1/metrics` に永続 counter を追加し、`GET /admin/v1/metrics/prometheus` を追加する。Prometheus endpoint は Admin token 必須 | `meta/metrics-snapshot.json` を定期 atomic update。`usage.json` と quota 判定に使う storage usage を同じ計測源に統一する | snapshot 破損は WARN 後に再計測。Prometheus 出力失敗は `INTERNAL_ERROR`。metric label に secret/token/SQL args を含めない | TC-17-1〜TC-17-5。再起動後 counter 復元、Prometheus schema、quota usage 整合、破損復旧、秘匿 |
| Phase 18 | `ha/*`, `replication/*`, `http/health.rs`, `config.rs` | `GET /ha/v1/status`、`POST /ha/v1/promote`、`POST /ha/v1/demote` を追加する。HA 管理 API は Admin token + HA token 必須 | `meta/ha-state.json` に node_id、term、leader_id、last_applied_frame、role を atomic update。split-brain 防止のため term は単調増加のみ | leader 不明は `HA_NO_LEADER`、split-brain 検出は `HA_SPLIT_BRAIN`、昇格不能は `HA_PROMOTION_FAILED`。term/leader 変更は INFO、矛盾は ERROR | TC-18-1〜TC-18-7。leader election、promotion/demotion、primary down、network partition、restart、redirect、split-brain rejection |
| Phase 19 | `adlaire-wal`, `adlaire-storage`, `db/sqld_adapter.rs`, `hrana/*` | 外部 API は変更しない。内製 crate への切り替えは config flag で段階的に行い、既定は直前 Phase と同じ挙動にする | 新規 metadata 追加は禁止。必要な場合は別 Phase として仕様追加する。rollback は config flag を戻すことで可能にする | 互換性差分は `INTERNAL_ERROR` で隠さず、既存 §7.3 code に写像する。性能回帰が閾値を超えた場合は完了不可 | TC-19-1〜TC-19-6。Turso Cloud / libSQL SDK 互換、WAL consistency、crash recovery、rollback flag、performance baseline、Phase 1〜18 regression |

**Phase 16〜19 の境界決定：**

- Phase 16 は SQLite 拡張ロードのみ。HA、内製化、任意 SQL parser 変更は含めない
- Phase 17 は監視と usage 永続化のみ。quota policy 自体の変更は Phase 8 契約を変更しない限り禁止
- Phase 18 は HA と failover のみ。multi-primary write は対象外
- Phase 19 は内部実装差し替えのみ。wire format、admin API、metadata schema、JWT claim を変更してはならない

### 9.4.1 Phase 9〜19 実装精度固定表

Phase 9 以降は状態、再試行、rollback、snapshot の不足がバグ修正 PR を生むため、下表を各 Phase の実装契約に追加する。ここに書かれた項目は設計メモではなく完了条件である。

| Phase | 固定する境界 | 成功条件 | 失敗時の固定挙動 | 必須 snapshot / artifact |
|-------|--------------|----------|------------------|--------------------------|
| 9 WebSocket | upgrade、hello、stream、transaction、store_sql | hello 後のみ request を処理し、stream_id ごとに connection と tx 状態を分離する | hello 前 request は protocol error、接続 close 時の open tx は rollback、unknown message は response_error | `tests/snapshots/phase9_ws/messages.json` |
| 10 ATTACH/metrics | ATTACH SQL 解決、DB alias、counter 更新点 | 管理下 DB だけ attach し、metrics は HTTP/WS/pipeline の実行後に増加する | 任意 path、未登録 DB、不正 alias は成功させない。metrics 取得失敗は `INTERNAL_ERROR` ではなく対象 field を 0 または明示 error | `tests/snapshots/phase10_metrics/metrics.json` |
| 11 replication primary | frame_no、CRC32、SSE、snapshot headers | `from_frame` 以降を順序通り返し、snapshot は整合した DB byte stream と header を返す | frame 不在は `FRAME_NOT_FOUND`、checksum 計算不能は 500 ではなく起動/stream 失敗として記録 | `tests/snapshots/phase11_replication/api.json` |
| 12 replica/redirect | replica state、catchup、write redirect、primary down | 最後に適用した frame から再開し、replica write は規定 redirect または規定 error | checksum mismatch は破棄して再取得。primary 不達時は write を成功扱いにしない | `tests/snapshots/phase12_replica/health_redirect.json` |
| 13 WAL archive | manifest、frame file、retention cleanup | manifest と frame/snapshot file が双方向に一致する | manifest 破損は起動失敗。orphan file は WARN 後 cleanup。missing frame は PITR 対象外ではなく起動失敗 | `tests/snapshots/phase13_archive/manifest.json` |
| 14 backup/restore/PITR | restore transaction、temp layout、integrity_check | committed 前に元 DB を保持し、成功後は integrity_check が `ok` | 失敗時は必ず rollback。rollback 不能なら起動失敗 marker を残し成功応答しない | `tests/snapshots/phase14_restore/results.json` |
| 15 branch | source selector、branch name、metadata/file commit | branch DB directory 準備後に metadata active commit する | partial branch は起動時 cleanup。metadata active で DB directory 不在は起動失敗 | `tests/snapshots/phase15_branch/branches.json` |
| 16 extension | manifest、sha256、load boundary | sha256 一致、allowlist 一致、固定 directory 内だけ load | load 失敗時は metadata 追加なし。delete は metadata のみ削除し binary は残す | `tests/snapshots/phase16_extension/extensions.json` |
| 17 metrics persistence | counter snapshot、Prometheus text、usage source | snapshot は 30 秒ごとと shutdown 時に書く。usage は Phase 8 `usage.json` と同源 | snapshot 破損は WARN 後 0 から再計測。quota 判定には破損 snapshot を使わない | `tests/snapshots/phase17_metrics/prometheus.txt` |
| 18 HA | term、leader、candidate、operator promotion | term は単調増加。candidate から primary は operator promote 必須 | split-brain は `HA_SPLIT_BRAIN`。自動 primary 昇格は禁止 | `tests/snapshots/phase18_ha/status.json` |
| 19 internal adapter | config flag、shadow/active、rollback、baseline | default は libsql。adapter active でも API/metadata/JWT/wire 差分ゼロ | 性能回帰または snapshot 差分があれば未完了。silent fallback 禁止 | `tests/snapshots/phase19_internal/compat.json` |

**実装精度チェックリスト：**

```
IC-1: §9.1.2 の横断 validation を該当 endpoint 全てで実施
IC-2: §9.4.1 の snapshot/artifact を生成し、dynamic 値を正規化
IC-3: state transition の禁止遷移をテストで発火
IC-4: partial write / interrupted request / process kill 後の再起動結果を固定
IC-5: 既存 Phase の compatibility snapshot に差分がない
IC-6: Phase 外 route は success response を返さない
```

### 9.4.2 Phase 9〜19 ゼロバグ実装補完契約

Phase 9〜19 で実装者が独自判断しやすい境界は、以下を優先仕様として固定する。各 Phase 詳細節と矛盾する場合は、本節を優先し、詳細節も同じ PR で修正する。ここに書かれた項目は推奨ではなく完了条件である。

**Protocol / wire 互換固定表：**

| Phase | 対象 | 固定仕様 | テスト artifact |
|-------|------|----------|-----------------|
| 9 | WebSocket subprotocol | `Sec-WebSocket-Protocol` は client 提示順を保持して解釈し、server が Phase 9 で対応する `hrana3`、`hrana2`、`hrana1` のうち最初に一致したものを選択する。`hrana3-protobuf` は Phase 9 では未対応のため選択しない。protobuf 対応を追加する場合は先に依存と schema を本仕様へ追加する | `phase9_ws/subprotocols.json` |
| 9 | unknown protocol field | Hrana wire protocol の JSON object に含まれる unknown field は互換のため無視する。管理 API の unknown field 拒否方針を WebSocket message に適用してはならない | `phase9_ws/forward_compat.json` |
| 9 | message ordering | client は hello 応答前に request を送ってよい。server は hello 認証完了後に受信順で処理し、hello が失敗した場合は未処理 request へ response を返さず close する | `phase9_ws/pipelined_hello.json` |
| 9 | cursor API | `open_cursor` / `fetch_cursor` / `close_cursor` を受信した場合は Phase 9 では response_error `NOT_IMPLEMENTED` とし、connection は維持する。unknown request type は response_error `INVALID_REQUEST` | `phase9_ws/cursor_not_implemented.json` |
| 10 | ATTACH SQL parse | SQL text の簡易文字列分割で ATTACH を判定しない。SQL tokenizer または SQLite prepare 前の限定 parser で、文字列リテラル内の `ATTACH` を無視する | `phase10_attach/parser_cases.json` |
| 11 | replication stream | SSE は `id:{frame_no}` と `event: wal` を必須にし、`Last-Event-ID` があれば `from_frame=max(query.from_frame, last_event_id+1)` として再開する | `phase11_replication/sse_resume.txt` |
| 14 | backup body | backup は SQLite Online Backup API 相当の snapshot を返す。単純な `data.db` file copy は、共有 lock と WAL checkpoint 整合が証明できない限り禁止 | `phase14_restore/backup_headers.json` |
| 17 | Prometheus text | 出力は UTF-8、LF 改行、末尾 LF 必須。各 metric は `HELP`、`TYPE`、samples の順で grouping する。`TYPE` は最初の sample より前に 1 回だけ出す | `phase17_metrics/prometheus.txt` |

**Phase 10 ATTACH / metrics 優先順位固定表：**

| 条件 | 優先順位 | 結果 |
|------|----------|------|
| attach 対象 DB が存在しない | 1 | `404 DB_NOT_FOUND` |
| JWT / org / group / db scope 外 | 2 | `403 ORG_SCOPE_DENIED` |
| `allow_attach=false` | 3 | `403 PERMISSION_DENIED` |
| `block_reads=true` の DB を read source にする | 4 | `403 PERMISSION_DENIED` |
| `block_writes=true` の DB へ write する | 5 | `403 PERMISSION_DENIED` |
| ro token で attach 先へ write | 6 | `403 PERMISSION_DENIED` |
| alias 不正、quote 不正、任意 path | 7 | `400 INVALID_REQUEST` または DB 名 validation 由来の `INVALID_DB_NAME` |

metrics は成功・失敗の両方で `http_requests_total` と `errors_total` を更新する。SQL execution counter は libSQL に渡した step のみ加算し、validation で拒否した SQL は加算しない。WebSocket は connection close 時に `connections_active` を必ず decrement し、二重 decrement は禁止する。

**Phase 11〜15 data lifecycle 固定表：**

| Phase | 操作 | commit 順序 | crash 後の扱い |
|-------|------|-------------|----------------|
| 11 | replication snapshot | temp snapshot 作成、integrity_check、header 生成、stream 開始 | temp snapshot は起動時 cleanup。metadata は変更しない |
| 12 | replica apply | frame checksum 検証、apply、fsync、`replica-state.json` 更新 | state より進んだ frame は再検証してから再適用。重複 frame は idempotent skip |
| 13 | archive append | frame file fsync、manifest tmp fsync、rename、directory fsync | manifest にない frame は orphan として WARN 後 cleanup。manifest にある file 欠損は起動失敗 |
| 14 | restore/PITR | upload temp、verify、runtime close、old rename、new rename、directory fsync、reopen、success response | `restore-failed.json` があれば起動失敗。operator が手動復旧するまで自動上書き禁止 |
| 15 | branch create | branch dir temp、DB 構築、integrity_check、runtime open、`branches.json` commit、dir finalize | metadata active で dir 不在は起動失敗。dir だけ存在し metadata なしは cleanup |

**Phase 14 backup / restore request 固定表：**

| API | Content-Type | body limit | lock | success |
|-----|--------------|------------|------|---------|
| `GET /admin/v1/databases/{name}/backup` | response `application/octet-stream` | response streaming。メモリ全読み込み禁止 | source read lock は Online Backup API の step 中だけ | 200 + SQLite snapshot bytes |
| `POST /admin/v1/databases/{name}/restore` | request `application/octet-stream` | `restore_max_bytes` 未定義時は min(DB size x2, 1GiB)。超過は `413 PAYLOAD_TOO_LARGE` | DB exclusive restore lock。read は旧 DB で継続、write は `503 STORAGE_BUSY` | 204 empty body |
| `POST /admin/v1/databases/{name}/restore/point-in-time` | request `application/json` | JSON body 最大 64KiB | DB exclusive restore lock | 204 empty body |

restore/PITR は `delete_protection=true` の DB では `403 ORG_SCOPE_DENIED` とする。`block_writes=true` の DB では `403 PERMISSION_DENIED` とする。quota は restore 後サイズを事前推定し、超過する場合は commit 前に `QUOTA_EXCEEDED` を返す。

**Phase 15 branch / Turso seed 接続固定表：**

| 入力 | Phase 15 の扱い |
|------|-----------------|
| `/admin/v1/databases/{db}/branches` | 正式 branch API。`from` selector を必須にする |
| `/v1/organizations/{org}/databases` with `seed.type:"database"` | Phase 15 で Turso 互換 branch create に昇格してよい。ただし `branch_name` または Turso 互換の branch field が明記されない request は `INVALID_REQUEST` |
| source DB `delete_protection=true` | branch create は許可、source DB delete は active branch がある限り `403 ORG_SCOPE_DENIED` |
| source DB `block_reads=true` | branch create は `403 PERMISSION_DENIED` |
| source DB quota | branch DB は作成時に source の database quota を継承する。branch 作成で organization/group quota を超える場合は `QUOTA_EXCEEDED` |
| token scope | source DB token は branch DB へ自動拡張しない。branch 用 token は別途発行する |

**Phase 16 extension 状態遷移固定表：**

| 状態 | 意味 | 許可遷移 |
|------|------|----------|
| `registered` | manifest に存在し binary/sha256 検証済み、未ロード | `loading`、`deleted` |
| `loading` | load 試行中。API success response 前の一時状態 | `loaded`、`load_failed` |
| `loaded` | 新規 connection に load 対象 | `disabled`、`deleted` |
| `load_failed` | load 失敗。既存 connection へ影響なし | `loading`、`deleted` |
| `disabled` | manifest に残すが新規 connection に load しない | `loading`、`deleted` |
| `deleted` | manifest から削除済み。binary は残ってよい | 復帰禁止。再登録は新 record として扱う |

`extensions.json` の `loaded` boolean だけで状態を表現してはならない。Phase 16 実装時は `state` field を追加し、migration で既存 `loaded:true` は `state:"loaded"`、`loaded:false` は `state:"registered"` に変換する。

**Phase 17 Prometheus 固定表：**

| 項目 | 固定仕様 |
|------|----------|
| metric order | metric name 昇順。各 metric 内は label set の辞書順 |
| HELP escape | backslash と LF を escape |
| label escape | backslash、double quote、LF を escape |
| sample value | finite number のみ。取得不能値を `NaN` にせず、該当 sample を出さない |
| route label | route pattern のみ。raw path、DB 名入り path、query string は禁止 |
| content negotiation | `Accept` 未指定、`*/*`、`text/plain` は 200。その他は 406 `NOT_ACCEPTABLE` |

**Phase 18 HA 昇格固定表：**

| 状態 | write 可否 | 自動遷移 | operator API |
|------|------------|----------|--------------|
| `primary` | 可 | split-brain 検出時は `candidate` に降格して write 停止 | demote 可 |
| `replica` | 不可。leader 判明時は 307 redirect | heartbeat timeout で `candidate` | promote 可。ただし最新 frame 到達が必須 |
| `candidate` | 不可 | primary へ自動昇格禁止 | promote/demote 可 |
| `standalone` | 可。ただし HA enabled の場合は起動時に `candidate` へ移行 | なし | promote で primary |

promotion は `last_applied_frame >= leader_known_frame`、`term >= stored_term`、`ha-state.json` commit 成功、replication apply queue empty の全条件を満たす場合だけ成功する。どれか 1 つでも満たさない場合は `HA_PROMOTION_FAILED` とし、write を開始しない。

**Phase 19 adapter 切替固定表：**

| Mode | 実行内容 | success condition | failure |
|------|----------|-------------------|---------|
| `libsql` | 既存 production path | Phase 1〜18 regression pass | 失敗時は通常のテスト失敗 |
| `shadow` | libsql を正として adapter を副実行し、結果・error code・rows affected・last_insert_rowid を比較 | 差分ゼロ。latency は記録のみ | 差分が 1 件でもあれば Phase 19 未完了 |
| `active` | adapter 経路を response に使用 | shadow で差分ゼロ、snapshot 差分ゼロ、p95 latency 2 倍以内 | silent fallback 禁止。失敗時は error を返し rollback 手順へ |

Phase 19 では storage write path の active 化は禁止する。`active` にできるのは WAL checkpoint control と executor adapter boundary までとし、metadata migration を伴う変更は Phase 20 以降の仕様改訂なしに実装してはならない。

### 9.5 全 API endpoint 契約表

この表は実装対象 endpoint のインデックスである。詳細 schema は §6 および各 Phase 節を正とするが、認証・status・永続化・冪等性で迷った場合はこの表を優先する。

| Method / Path | Phase | 認証 | Request | Success | 主な Error | 永続化 | 冪等性 |
|---------------|-------|------|---------|---------|------------|--------|--------|
| `GET /v2/health` | 3 / 12 拡張 | 不要 | body なし | 200 JSON。Phase 3 は `{status:"ok"}`、Phase 12 以降は role/lag を追加 | 500 `INTERNAL_ERROR` | なし | Yes |
| `POST /v2/pipeline` | 3 | Phase 4 以降 JWT。Phase 3 は認証無効のみ | hrana-http v2 `PipelineRequest` | 200 `PipelineResponse`。SQL error は results 内 error | 400 `INVALID_REQUEST`, 401 auth 系, 404 `DB_NOT_FOUND`, 503 `STORAGE_BUSY` | SQL 書き込み時のみ DB | No |
| `POST /{db-name}/v2/pipeline` | 6 | JWT | path DB + hrana-http v2 | 200 `PipelineResponse` | 400 `INVALID_DB_NAME`, 404 `DB_NOT_FOUND`, auth 系, storage 系 | SQL 書き込み時のみ対象 DB | No |
| `GET /v3/baton` | 9 | WebSocket hello JWT | WebSocket upgrade | 101 Switching Protocols | 400 upgrade 不正, hello error `AUTH_*` | session 内 SQL 書き込み時のみ DB | 接続単位 |
| `GET /{db-name}/v3/baton` | 9 | WebSocket hello JWT | path DB + WebSocket upgrade | 101 Switching Protocols | 400/404/auth 系 | session 内 SQL 書き込み時のみ対象 DB | 接続単位 |
| `GET /admin/v1/databases` | 7 | Admin token | body なし | 200 `{databases:[...]}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/databases` | 7 | Admin token | `{name}` | 201 `DbInfo` | 400 `INVALID_DB_NAME`/`DB_RESERVED_NAME`, 409 `DB_ALREADY_EXISTS` | `databases.json`, DB directory | No |
| `GET /admin/v1/databases/{name}` | 7 | Admin token | body なし | 200 `DbInfo` | 400 invalid name, 404 `DB_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/databases/{name}` | 7 | Admin token | body なし | 204 empty body | 400 invalid/reserved, 404 `DB_NOT_FOUND` | `databases.json`, DB directory deletion | Yes: missing DB remains 404 |
| `GET /admin/v1/tokens` | 7 | Admin token | body なし | 200 `{tokens:[...]}` token value は返さない | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/tokens` | 7 | Admin token | `{access, expiry?, dbs?}` | 201 `{id, token, ...}`。token は作成時のみ返す | 400 `INVALID_REQUEST` | `tokens.json` | No |
| `GET /admin/v1/tokens/{id}` | 7 | Admin token | body なし | 200 token metadata。JWT 文字列は返さない | 404 `TOKEN_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/tokens/{id}` | 7 | Admin token | body なし | 204 empty body | 404 `TOKEN_NOT_FOUND` | `tokens.json`, in-memory revoke set | Yes: 既に revoked は 204 |
| `GET /admin/v1/organizations` | 8 | Admin token | body なし | 200 `{organizations:[...]}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/organizations` | 8 | Admin token | `{name, slug?}` | 201 `OrganizationInfo` | 400 `INVALID_REQUEST`, 409 `ORG_ALREADY_EXISTS` | `organizations.json` | No |
| `GET /admin/v1/organizations/{org}` | 8 | Admin token | body なし | 200 `OrganizationInfo` | 404 `ORG_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/organizations/{org}` | 8 | Admin token | body なし | 204 empty body | 404 `ORG_NOT_FOUND`, 403 `ORG_SCOPE_DENIED` | `organizations.json`, related group/quota metadata | Yes: missing org remains 404 |
| `GET /admin/v1/groups` | 8 | Admin token | query `organization?` | 200 `{groups:[...]}` | 404 `ORG_NOT_FOUND` | なし | Yes |
| `POST /admin/v1/groups` | 8 | Admin token | `{organization, name, slug?, location?}` | 201 `GroupInfo` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND`/`LOCATION_NOT_FOUND`, 409 `GROUP_ALREADY_EXISTS` | `groups.json` | No |
| `GET /admin/v1/groups/{group}` | 8 | Admin token | body なし | 200 `GroupInfo` | 404 `GROUP_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/groups/{group}` | 8 | Admin token | body なし | 204 empty body | 404 `GROUP_NOT_FOUND`, 403 `ORG_SCOPE_DENIED` | `groups.json`, related quota metadata | Yes: missing group remains 404 |
| `GET /admin/v1/locations` | 8 | Admin token | body なし | 200 `{locations:[...]}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/locations` | 8 | Admin token | `{name, provider?, region?, primary?}` | 201 `LocationInfo` | 400 `INVALID_REQUEST`, 409 `LOCATION_ALREADY_EXISTS` | `locations.json` | No |
| `GET /admin/v1/locations/{location}` | 8 | Admin token | body なし | 200 `LocationInfo` | 404 `LOCATION_NOT_FOUND` | なし | Yes |
| `DELETE /admin/v1/locations/{location}` | 8 | Admin token | body なし | 204 empty body | 404 `LOCATION_NOT_FOUND`, 403 `ORG_SCOPE_DENIED` | `locations.json` | Yes: missing location remains 404 |
| `GET /admin/v1/quotas` | 8 | Admin token | query `organization?`, `group?`, `database?` | 200 `{quotas:[...]}` | 404 `ORG_NOT_FOUND`/`GROUP_NOT_FOUND`/`DB_NOT_FOUND` | なし | Yes |
| `PUT /admin/v1/quotas/{scope}` | 8 | Admin token | `{storage_bytes, rows?, write_ops_per_minute?}` | 200 `QuotaInfo` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND`/`GROUP_NOT_FOUND`/`DB_NOT_FOUND` | `quotas.json` | Yes |
| `GET /admin/v1/usage` | 8 | Admin token | query `organization?`, `group?`, `database?` | 200 `{usage:[...]}` | 404 `ORG_NOT_FOUND`/`GROUP_NOT_FOUND`/`DB_NOT_FOUND`, 503 `USAGE_UNAVAILABLE` | `usage.json` snapshot | Yes |
| `GET /v1/auth/validate` | 8 | Platform token | body なし | 200 `{"exp":integer}` | 401 auth 系 | なし | Yes |
| `POST /v1/auth/api-tokens/{tokenName}` | 8 | Platform token | body `{organization?}` または body なし | 200 `{"name","id","token"}` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND` | `tokens.json` | No |
| `DELETE /v1/auth/api-tokens/{tokenName}` | 8 | Platform token | body なし | 200 `{"token":"{tokenName}"}` | 404 `TOKEN_NOT_FOUND` | `tokens.json` | Yes: revoked は 200 |
| `GET /v1/locations` | 8 | Platform token | body なし | 200 `{"locations":{code:name}}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `GET /v1/organizations` | 8 | Platform token | body なし | 200 `[TursoOrganizationInfo]` | 401 `AUTH_REQUIRED` | なし | Yes |
| `PATCH /v1/organizations/{organizationSlug}` | 8 | Platform token | `{overages?, require_mfa?}` | 200 `{"organization":TursoOrganizationInfo}` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND` | `organizations.json` | Yes |
| `GET /v1/organizations/{organizationSlug}/usage` | 8 | Platform token | body なし | 200 `{"organization":TursoOrganizationUsage}` | 404 `ORG_NOT_FOUND`, 503 `USAGE_UNAVAILABLE` | なし | Yes |
| `GET /v1/organizations/{organizationSlug}/groups` | 8 | Platform token | body なし | 200 `{"groups":[TursoGroupInfo]}` | 404 `ORG_NOT_FOUND` | なし | Yes |
| `POST /v1/organizations/{organizationSlug}/groups` | 8 | Platform token | `{name, location}` | 200 `{"group":TursoGroupInfo}` | 400 `INVALID_REQUEST`, 404 `ORG_NOT_FOUND`/`LOCATION_NOT_FOUND`, 409 `GROUP_ALREADY_EXISTS` | `groups.json` | No |
| `GET /v1/organizations/{organizationSlug}/groups/{groupName}` | 8 | Platform token | body なし | 200 `{"group":TursoGroupInfo}` | 404 `GROUP_NOT_FOUND` | なし | Yes |
| `PATCH /v1/organizations/{organizationSlug}/groups/{groupName}/configuration` | 8 | Platform token | `{delete_protection}` | 200 `{"delete_protection":boolean}` | 400 `INVALID_REQUEST`, 404 `GROUP_NOT_FOUND` | `groups.json` | Yes |
| `POST /v1/organizations/{organizationSlug}/groups/{groupName}/auth/rotate` | 8 | Platform token | body なし | 200 empty body | 404 `GROUP_NOT_FOUND` | `tokens.json` DB/group token revoke | Yes |
| `POST /v1/organizations/{organizationSlug}/groups/{groupName}/transfer` | 8 | Platform token | any | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `GET /v1/organizations/{organizationSlug}/databases` | 8 | Platform token | query `group?`, `schema?`, `parent?` | 200 `{"databases":[TursoDatabaseInfo]}` | 404 `ORG_NOT_FOUND` | なし | Yes |
| `POST /v1/organizations/{organizationSlug}/databases` | 8 | Platform token | `{name, group, size_limit?}` | 200 `{"database":TursoDatabaseInfo}` | 400 `INVALID_DB_NAME`/`INVALID_REQUEST`, 404 `GROUP_NOT_FOUND`, 409 `DB_ALREADY_EXISTS` | `databases.json`, DB directory | No |
| `GET /v1/organizations/{organizationSlug}/databases/{databaseName}` | 8 | Platform token | body なし | 200 `{"database":TursoDatabaseInfo}` | 404 `DB_NOT_FOUND` | なし | Yes |
| `DELETE /v1/organizations/{organizationSlug}/databases/{databaseName}` | 8 | Platform token | body なし | 200 `{"database":"{databaseName}"}` | 404 `DB_NOT_FOUND` | `databases.json`, DB directory, DB token revoke | Yes: missing DB remains 404 |
| `PATCH /v1/organizations/{organizationSlug}/databases/{databaseName}/configuration` | 8 | Platform token | `{size_limit?, delete_protection?, block_reads?, block_writes?, allow_attach?}` | 200 configuration JSON | 400 `INVALID_REQUEST`, 404 `DB_NOT_FOUND` | `databases.json`, `quotas.json` | Yes |
| `POST /v1/organizations/{organizationSlug}/databases/{databaseName}/auth/tokens` | 8 | Platform token | query `expiration?`, `authorization?`; body `{permissions?}` | 200 `{"jwt":string}` | 400 `INVALID_REQUEST`, 404 `DB_NOT_FOUND` | `tokens.json` | No |
| `POST /v1/organizations/{organizationSlug}/databases/{databaseName}/auth/rotate` | 8 | Platform token | body なし | 200 empty body | 404 `DB_NOT_FOUND` | `tokens.json` DB token revoke | Yes |
| `/v1/organizations/{organizationSlug}/members*` | 8 | Platform token | any | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `/v1/organizations/{organizationSlug}/invites*` | 8 | Platform token | any | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `/v1/organizations/{organizationSlug}/plans` | 8 | Platform token | body なし | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `GET /v1/organizations/{organizationSlug}/audit-logs` | 8 | Platform token | query `page?`, `page_size?` | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `/v1/organizations/{organizationSlug}/databases/{databaseName}/stats` | 8 | Platform token | body なし | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `/v1/upload` | 8 | Database token | binary | 501 `NOT_IMPLEMENTED` | 501 `NOT_IMPLEMENTED` | なし | Yes |
| `GET /admin/v1/metrics` | 10 | Admin token | body なし | 200 metrics JSON | 401 `AUTH_REQUIRED` | なし | Yes |
| `GET /replication/v1/log?from_frame=N` | 11 | replication token | query `from_frame` | 200 SSE frames | 400 `INVALID_REQUEST`, 401 `AUTH_INVALID`, 404 `FRAME_NOT_FOUND` | なし | 接続単位 |
| `GET /replication/v1/snapshot` | 11 | replication token | query/body なし | 200 octet-stream + replication headers | auth 系, 500 | なし | Yes |
| `POST /replication/v1/heartbeat` | 11 | replication token | `{replica_id, synced_frame}` | 200 `{primary_frame, lag_frames}` | 400 `INVALID_REQUEST`, auth 系 | primary in-memory replica status | Yes |
| `GET /replication/v1/status` | 11 | replication token | body なし | 200 primary replication status | auth 系 | なし | Yes |
| `GET /admin/v1/databases/{name}/backup` | 14 | Admin token | body なし | 200 octet-stream SQLite backup | 404 `DB_NOT_FOUND`, auth 系 | なし | Yes |
| `POST /admin/v1/databases/{name}/restore` | 14 | Admin token | octet-stream SQLite file | 204 empty body | 413 `PAYLOAD_TOO_LARGE`, 409 `RESTORE_INTEGRITY_FAILED`, 404 `DB_NOT_FOUND`, 403 `ORG_SCOPE_DENIED`/`PERMISSION_DENIED`/`QUOTA_EXCEEDED` | target DB replace + rollback temp | No |
| `POST /admin/v1/databases/{name}/restore/point-in-time` | 14 | Admin token | `{timestamp}` または `{frame_no}` | 204 empty body | 503 `PITR_NOT_ENABLED`, 404 `FRAME_NOT_FOUND`, 409 `RESTORE_FRAME_CORRUPT`, 403 `ORG_SCOPE_DENIED`/`PERMISSION_DENIED`/`QUOTA_EXCEEDED` | target DB replace + rollback temp | No |
| `GET /admin/v1/databases/{name}/branches` | 15 | Admin token | body なし | 200 `{branches:[...]}` | 404 `DB_NOT_FOUND` | なし | Yes |
| `POST /admin/v1/databases/{name}/branches` | 15 | Admin token | `{branch_name, from}` | 201 branch metadata | invalid/reserved name, `FRAME_NOT_FOUND` | `branches.json`, branch DB directory | No |
| `DELETE /admin/v1/databases/{name}/branches/{branch}` | 15 | Admin token | body なし | 204 empty body | 404 `DB_NOT_FOUND` | `branches.json`, branch DB directory deletion | Yes: missing branch remains 404 |
| `GET /admin/v1/extensions` | 16 | Admin token | body なし | 200 `{extensions:[...]}` | 401 `AUTH_REQUIRED` | なし | Yes |
| `POST /admin/v1/extensions` | 16 | Admin token | `{name, version, sha256, enabled?}` | 201 `ExtensionInfo` | 400 `INVALID_REQUEST`, 403 `EXTENSION_NOT_ALLOWED`, 409 `EXTENSION_ALREADY_EXISTS` | `extensions.json` | No |
| `DELETE /admin/v1/extensions/{name}` | 16 | Admin token | body なし | 204 empty body | 404 `EXTENSION_NOT_FOUND` | `extensions.json` | Yes: missing extension remains 404 |
| `GET /admin/v1/metrics/prometheus` | 17 | Admin token | body なし | 200 text/plain Prometheus exposition | 401 `AUTH_REQUIRED`, 406 `NOT_ACCEPTABLE`, 500 `INTERNAL_ERROR` | なし | Yes |
| `GET /ha/v1/status` | 18 | Admin token + HA token | body なし | 200 `HaStatus` | 401 auth 系, 503 `HA_NO_LEADER` | なし | Yes |
| `POST /ha/v1/promote` | 18 | Admin token + HA token | `{node_id, term}` | 200 `HaStatus` | 409 `HA_PROMOTION_FAILED`/`HA_SPLIT_BRAIN` | `ha-state.json` | No |
| `POST /ha/v1/demote` | 18 | Admin token + HA token | `{node_id, term}` | 200 `HaStatus` | 409 `HA_PROMOTION_FAILED` | `ha-state.json` | No |

### 9.6 永続化ファイル契約表

| Path | Phase | Owner | 初期値 | 更新方式 | fsync | 破損時挙動 | Backup 対象 |
|------|-------|-------|--------|----------|-------|------------|-------------|
| `{data-dir}/.lock` | 2 | `ProcessLock` | 空ファイル可 | open + flock。内容は意味を持たない | 不要 | flock が取れれば続行。削除不要 | No |
| `{data-dir}/meta/databases.json` | 2 / 6 / 7 / 8 | `DbManager` | `{"databases":[]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/tokens.json` | 2 / 4 / 7 | `AuthState` / token 管理 | `{"tokens":[]}` | tmp write + fsync + rename | 必須 | 起動失敗。空で上書きしない | Yes |
| `{data-dir}/meta/organizations.json` | 8 | organization 管理 | `{"organizations":[{"id":"default","name":"default","slug":"default","created_at":...}]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/groups.json` | 8 | group 管理 | `{"groups":[{"id":"default","organization":"default","name":"default","slug":"default","location":"default","delete_protection":false}]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/locations.json` | 8 | location 管理 | `{"locations":[{"id":"default","name":"default","provider":"self-hosted","region":"local","primary":true}]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/quotas.json` | 8 | quota 管理 | `{"quotas":[]}` | tmp write + fsync + rename | 必須 | 起動失敗。自動修復しない | Yes |
| `{data-dir}/meta/usage.json` | 8 | usage snapshot | `{"usage":[],"updated_at":null}` | tmp write + fsync + rename | 任意。fsync 失敗時は WARN + `USAGE_UNAVAILABLE` | 破損時は起動 WARN 後に再計測。空上書きは禁止 | No |
| `{data-dir}/meta/phase8-migration.json` | 8 | migration marker | migration 完了時のみ作成 | tmp write + fsync + rename | 必須 | marker 破損時は起動失敗。再 migration しない | Yes |
| `{data-dir}/meta/migration-backup/phase8-{timestamp}/` | 8 | migration backup | migration 開始前に作成 | copy + fsync | 必須 | 復元不能なら起動失敗 | No |
| `{data-dir}/meta/branches.json` | 2 / 15 | branch 管理 | `{"branches":[]}` | tmp write + fsync + rename | 必須 | Phase 15 以降は起動失敗。Phase 14 以前は初期化のみ | Yes |
| `{data-dir}/databases/{name}/data.db` | 2+ | libsql / DbManager | libsql 作成 | libsql commit | libsql に委譲 | integrity_check NG なら起動失敗 | Yes |
| `{data-dir}/databases/{name}/data.db-wal` | 2+ | SQLite WAL | SQLite 作成 | SQLite WAL | SQLite に委譲 | SQLite recovery に委譲。integrity_check で検出 | Yes |
| `{data-dir}/meta/replica-state.json` | 12 | replica sync | replica 起動時に作成 | tmp write + fsync + rename | 必須 | 起動失敗。last_applied_frame を推測で進めない | Yes |
| `{data-dir}/databases/{name}/wal-archive/manifest.json` | 13 | WAL archive | archive 有効時に作成 | tmp write + fsync + rename | 必須 | 起動失敗。PITR/backup API は使わない | Yes |
| `{data-dir}/databases/{name}/wal-archive/frame-*.bin` | 13 | WAL archive | なし | create + write + fsync | 必須 | manifest と不整合なら ERROR。PITR 対象から除外または起動失敗を Phase 13 で固定 | Yes |
| `{data-dir}/databases/{name}/wal-archive/snapshot-*.db` | 13 | WAL archive | なし | copy + fsync + rename | 必須 | PITR 不可。manifest 整合性検査で検出 | Yes |
| restore temp dir | 14 | restore/PITR | API 実行時のみ | temp write + fsync + rename/swap | 必須 | 中断時は元 DB を復元。残骸は次回起動時に cleanup して WARN | No |
| `{data-dir}/databases/{name}/restore-failed.json` | 14 | restore/PITR | rollback 不能時のみ | tmp write + fsync + rename | 必須 | 存在する場合は起動失敗。operator が手動復旧するまで自動修復しない | Yes |
| `{data-dir}/databases/{db}___{branch}/data.db` | 15 | branch 管理 | branch 作成時 | libsql commit | libsql に委譲 | branch metadata と不整合なら WARN + branch 無効化、または起動失敗を Phase 15 で固定 | Yes |
| `{data-dir}/meta/extensions.json` | 16 | extension 管理 | `{"extensions":[]}` | tmp write + fsync + rename | 必須 | 起動失敗。未登録拡張を自動許可しない | Yes |
| `{data-dir}/extensions/{name}/{version}/` | 16 | extension binary | extension 登録時 | create + write/copy + fsync | 必須 | sha256 不一致ならロード禁止 | Yes |
| `{data-dir}/meta/metrics-snapshot.json` | 17 | metrics 永続化 | `{"counters":{},"gauges":{},"updated_at":null}` | tmp write + fsync + rename | 任意。失敗時 WARN | 破損時 WARN 後に 0 から再計測。quota usage は `usage.json` を正とする | No |
| `{data-dir}/meta/ha-state.json` | 18 | HA state | `{"node_id":null,"term":0,"leader_id":null,"role":"standalone","last_applied_frame":0}` | tmp write + fsync + rename | 必須 | 起動失敗。split-brain 防止のため自動初期化しない | Yes |

**永続化の禁止事項：**

- metadata 更新後に file 更新する順序は禁止。作成時は file を先に準備し、metadata を最後に commit する
- 削除時は runtime map から外し、DB close を確認し、directory 削除し、metadata 更新する。途中失敗時は再起動後に一貫した状態へ復旧できること
- JSON metadata を partial write してはならない。必ず tmp file を使う
- 破損 metadata を空初期値で上書きしてはならない

### 9.7 エラーコード使用契約表

| Code | 使用 Phase | 使用 API | Retry | Client action |
|------|------------|----------|-------|---------------|
| `AUTH_REQUIRED` | 4+ / admin 7+ | JWT/API/Admin/replication 認証必須 endpoint | No | Authorization header を付ける |
| `AUTH_INVALID` | 4+ | JWT/Admin/replication token 検証 | No | token を再発行または設定修正 |
| `AUTH_EXPIRED` | 4+ | JWT | No | token を再発行 |
| `AUTH_DISABLED` | 8+ | 認証無効時に許可されない管理・HA・extension 操作 | No | サーバー設定を変更 |
| `PERMISSION_DENIED` | 4+ / 10 | ro 書き込み、任意パス ATTACH 等 | No | 権限または request を変更 |
| `DB_NOT_FOUND` | 6+ | DB path, 管理 API, branch/backup | No | DB 名を確認または作成 |
| `TOKEN_NOT_FOUND` | 7+ | token get/delete | No | token id を確認 |
| `ORG_NOT_FOUND` | 8+ | organization get/delete, group/quota scope | No | organization id/slug を確認または作成 |
| `GROUP_NOT_FOUND` | 8+ | group get/delete, DB/token/quota scope | No | group id/slug を確認または作成 |
| `LOCATION_NOT_FOUND` | 8+ | location get/delete, DB/group create | No | location id/slug を確認または作成 |
| `ORG_ALREADY_EXISTS` | 8+ | organization create | No | 別の organization name/slug を使う |
| `GROUP_ALREADY_EXISTS` | 8+ | group create | No | 同一 organization 内で別の group name/slug を使う |
| `LOCATION_ALREADY_EXISTS` | 8+ | location create | No | 別の location name を使う |
| `QUOTA_EXCEEDED` | 8+ | write/import/restore/replication apply/branch create。`/v1/*` では HTTP 402、それ以外は HTTP 403 | No | quota を増やすか使用量を削減 |
| `USAGE_UNAVAILABLE` | 8+ | usage API, quota 判定不能時 | Yes | usage 再計測またはサーバーログ確認 |
| `ORG_SCOPE_DENIED` | 8+ | organization/group scope 外の admin/JWT 操作 | No | token scope または対象 scope を修正 |
| `DB_ALREADY_EXISTS` | 7+ | DB create | No | 別名を使う |
| `INVALID_DB_NAME` | 6+ | DB/branch create/path validation | No | name を修正 |
| `DB_RESERVED_NAME` | 6+ / 15 | `meta`, `admin`, `___` 含有名 | No | name を修正 |
| `NOT_IMPLEMENTED` | all | 未来 Phase の stub endpoint | No | 対象 Phase 実装後に再試行 |
| `INVALID_REQUEST` | 3+ | JSON/schema/query/body 不正 | No | request を修正 |
| `NOT_ACCEPTABLE` | 17+ | Prometheus など media type 固定 endpoint の Accept 不一致 | No | `Accept` を修正 |
| `PAYLOAD_TOO_LARGE` | 14+ | restore/upload body size limit 超過 | No | request body を小さくする |
| `SQLITE_ERROR` | 3+ | hrana results 内 | Depends | SQL を修正。busy は `STORAGE_BUSY` を使う |
| `SQLITE_CONSTRAINT` | 3+ | hrana results 内 | No | data/constraint を修正 |
| `STORAGE_BUSY` | 3+ | DB write/read lock timeout | Yes | backoff retry |
| `REPLICATION_TIMEOUT` | 11+ | sync write / replication ACK | Yes | retry または replication 状態確認 |
| `PITR_NOT_ENABLED` | 14+ | PITR/branch from timestamp/frame when archive disabled | No | `wal_retention_days` を有効化 |
| `FRAME_NOT_FOUND` | 11+ / 14+ / 15 | replication log, PITR, branch | Depends | frame range/retention を確認 |
| `RESTORE_INTEGRITY_FAILED` | 14+ | restore | No | backup file を確認 |
| `RESTORE_FRAME_CORRUPT` | 14+ | PITR | No | archive corruption を復旧 |
| `EXTENSION_NOT_ALLOWED` | 16+ | extension create/load | No | allowlist と manifest を確認 |
| `EXTENSION_NOT_FOUND` | 16+ | extension get/delete/load | No | extension 名を確認 |
| `EXTENSION_ALREADY_EXISTS` | 16+ | extension create | No | version または name を変更 |
| `EXTENSION_SIGNATURE_INVALID` | 16+ | extension create/load | No | sha256/署名を確認 |
| `EXTENSION_LOAD_FAILED` | 16+ | extension load | Depends | extension binary と SQLite ABI を確認 |
| `HA_NO_LEADER` | 18+ | HA status/write redirect | Yes | leader election 状態を確認 |
| `HA_SPLIT_BRAIN` | 18+ | HA promote/status | No | partition を解消し、operator 判断 |
| `HA_PROMOTION_FAILED` | 18+ | HA promote/demote | Depends | node health と term を確認 |
| `INTERNAL_ERROR` | all | 未分類内部エラー | Depends | server log を確認 |

### 9.8 Phase 別テストマトリクス

| Phase | 正常系 | Invalid request | Auth/permission | Persistence/restart | Crash/rollback | Regression |
|-------|--------|-----------------|-----------------|---------------------|----------------|------------|
| 1 | CLI help, config merge | unknown flag, missing `--data` | n/a | n/a | n/a | build/test |
| 2 | data dir init, DB open | invalid config, short secret | n/a | restart opens same DB | double lock / crash leaves flock releasable | Phase 1 |
| 3 | health, CREATE/INSERT/SELECT | malformed JSON, unknown type, bad value | auth disabled only | inserted data survives restart | SIGINT releases lock | Phase 1〜2 |
| 4 | valid JWT, token create | malformed JWT, bad expiry | missing/bad/expired/revoked/ro-write | tokens survive restart | token write atomicity | Phase 1〜3 |
| 5 | TS SDK CRUD, logs | n/a | auth cases from Phase 4 | TC-5 restart | graceful shutdown timeout | Phase 1〜4 |
| 6 | multi DB route isolation | invalid/reserved DB name | global JWT applies | databases.json survives restart | DB create/delete partial failure recovery | Phase 1〜5 |
| 7 | admin DB/token CRUD | malformed admin body | admin token, DB scoped JWT | tokens/databases survive restart | revoke/write atomicity | Phase 1〜6 |
| 8 | Turso Cloud 互換 management model と `/v1/*` snapshot | malformed location/org/group/quota/Turso body | admin/platform auth, ownership/scope permission | metadata migration and legacy fallback survive restart | partial metadata migration rollback | Phase 1〜7 |
| 9 | ws hello/open/execute/tx | invalid frame/order/stream | hello auth, ro-write | committed tx survives restart | disconnect rolls back open tx | Phase 1〜8 |
| 10 | ATTACH managed DB, metrics | arbitrary path ATTACH | admin metrics auth | metrics reset acceptable | n/a | Phase 1〜9 |
| 11 | primary replication APIs | bad from_frame/body | replication token | snapshot consistent | log stream disconnect/reconnect | Phase 1〜10 |
| 12 | replica catchup/redirect | bad primary URL | replication token | replica resumes from last frame | primary down / replica restart | Phase 1〜11 |
| 13 | archive manifest/frame cleanup | bad retention config | n/a | manifest survives restart | partial archive write recovery | Phase 1〜12 |
| 14 | backup/restore/PITR | bad restore file/body | admin auth | restored DB survives restart | restore failure rollback | Phase 1〜13 |
| 15 | branch create/list/delete | invalid branch name | admin/JWT on branch DB | branch survives restart | branch create/delete partial failure | Phase 1〜14 |
| 16 | extension register/list/delete/load | arbitrary path, bad sha256 | admin auth | extensions.json survives restart | failed load rollback | Phase 1〜15 |
| 17 | persistent metrics, Prometheus output | invalid metric request | admin metrics auth | metrics snapshot survives restart | corrupt snapshot recovery | Phase 1〜16 |
| 18 | leader election, promote/demote, redirect | stale term, bad node | admin + HA token | ha-state survives restart | partition / split-brain rejection | Phase 1〜17 |
| 19 | internal crate switch via config flag | invalid flag combination | n/a | no metadata migration | rollback flag restores previous path | Phase 1〜18 |

### 9.9 実装禁止事項

- 仕様にない endpoint を成功応答付きで公開しない
- Phase 外機能を「ついで」に実装しない。前倒しする場合は仕様の Phase 境界を先に変更する
- `unwrap()` / `expect()` で request 由来・disk 由来・network 由来の失敗を panic にしない
- invalid config を黙って default に fallback しない。空文字を無効扱いにする場合は仕様に明記する
- DB/branch/token metadata と実ファイルを不整合なまま成功応答しない
- 認証 secret、JWT、admin token、replication token、生 SQL 引数値、backup contents をログに出さない
- SQL 文字列を ad hoc split して複数 statement として処理しない。`sequence` は `execute_batch()` に渡す
- 任意ファイルパスを SQL/API から開かない。DB 名は必ず validation と管理 metadata 照合を通す
- Web フレームワークを導入しない。HTTP ルーティングは hyper ベースの自前実装を維持する
- `INTERNAL_ERROR` で仕様済みエラーを隠さない。対応する code がある場合は必ずそれを使う

### 9.10 Phase 16〜19 固定タスク

Phase 16〜19 は本節の固定タスクを完了条件とする。追加の仕様変更 PR なしに、ここへ未記載の API、metadata、error code、外部依存を追加してはならない。

| Phase | 実装タスク |
|-------|------------|
| Phase 16 | T16-1 extension manifest schema、T16-2 allowlist/sha256 検証、T16-3 load/unload API、T16-4 任意パス拒否、T16-5 restart 復元、T16-6 TC-16-1〜TC-16-6 |
| Phase 17 | T17-1 metrics snapshot writer、T17-2 Prometheus endpoint、T17-3 usage/quota 計測統合、T17-4 snapshot 破損復旧、T17-5 TC-17-1〜TC-17-5 |
| Phase 18 | T18-1 HA state、T18-2 leader election、T18-3 promote/demote API、T18-4 write redirect、T18-5 split-brain rejection、T18-6 restart recovery、T18-7 TC-18-1〜TC-18-7 |
| Phase 19 | T19-1 config flag、T19-2 adlaire-wal adapter、T19-3 storage boundary adapter、T19-4 executor boundary adapter、T19-5 rollback flag、T19-6 TC-19-1〜TC-19-6 |

### 9.11 Definition of Ready / Definition of Done

各 Phase の実装を始める前に Ready を満たし、merge 前に Done を満たすこと。
Done 判定は §9.1.33 の Done receipt を正とし、下表の Done は Phase 固有の最低条件として扱う。

| Phase | Definition of Ready | Definition of Done |
|-------|---------------------|--------------------|
| 1 | workspace 名、binary 名、CLI subcommand 名、必須 flag が決まっている | help 出力、config merge skeleton、build/test が通る |
| 2 | data-dir 構成、metadata 初期値、lock 方式、libsql open 設定が決まっている | 初回起動/再起動/二重起動拒否/integrity_check が通る |
| 3 | hrana-http request/response、HTTP status 境界、auth disabled 条件が決まっている | `/v2/health` と `/v2/pipeline` の正常/異常/永続化テストが通る |
| 4 | JWT claims、token record schema、expiry/revoke/access 仕様が決まっている | token create、JWT verify、revoke、ro/rw permission tests が通る |
| 5 | ログ field、秘匿対象、統合テスト環境、SDK version が決まっている | Phase 1〜5 TC と SDK 互換、永続化、ログ形式が通る |
| 6 | DB 名 validation、path routing、databases.json migration 方針が決まっている | multi DB routing、分離、再起動復元、invalid name tests が通る |
| 7 | admin API schema、admin auth、token CRUD、DB scope claim が決まっている | DB/token CRUD、DB scoped auth、revoke immediate tests が通る |
| 8 | Turso Cloud 互換の location、organization/group、quota/usage、`/v1/*` Platform API、metadata、auth、migration、error が決まっている | 互換管理モデル、metadata migration、quota/usage、権限、Turso snapshot、legacy fallback、Phase 1〜7 regression、SDK 互換 tests が通る |
| 9 | hrana-ws message schema、stream lifecycle、transaction lifecycle が決まっている | WebSocket handshake/execute/tx/store_sql tests が通る |
| 10 | ATTACH rewrite policy、metrics schema、counter 更新点が決まっている | managed ATTACH、path rejection、metrics auth/counter tests が通る |
| 11 | primary role、replication token、frame format、snapshot headers が決まっている | replication API contract、SSE/snapshot/heartbeat/status tests が通る |
| 12 | replica state persistence、redirect policy、primary-down behavior が決まっている | catchup, redirect, restart, primary-down, multi replica tests が通る |
| 13 | manifest schema、frame naming、retention cleanup、consistency check が決まっている | archive write, cleanup, corruption detection, restart tests が通る |
| 14 | restore transaction model、rollback temp layout、PITR selector schema が決まっている | backup/restore/PITR/rollback/corrupt archive tests が通る |
| 15 | branch metadata schema、branch naming、source selector、delete semantics が決まっている | branch create/list/delete/isolation/restart tests が通る |
| 16 | extension allowlist、manifest、署名/sha256、API schema、任意パス拒否が決まっている | extension CRUD/load/unload、署名検証、restart、任意パス拒否、Phase 1〜15 regression が通る |
| 17 | metrics snapshot schema、Prometheus schema、usage 計測源、破損時復旧方針が決まっている | metrics 永続化、Prometheus endpoint、usage/quota 整合、破損復旧、Phase 1〜16 regression が通る |
| 18 | HA token、node_id、term、leader election、promote/demote、split-brain policy が決まっている | leader election、failover、redirect、restart、partition、split-brain rejection、Phase 1〜17 regression が通る |
| 19 | 切り替える内製 crate、config flag、rollback flag、性能基準、互換テスト範囲が決まっている | Turso Cloud / libSQL SDK 互換、crash recovery、rollback、performance baseline、Phase 1〜18 regression が通る |

### 9.11.1 Phase 1〜19 実装精度索引

本索引は Phase 実装開始時の入口である。実装者は対象 Phase の行にある `詳細節`、`固定契約`、`台帳`、`シナリオ`、`Done receipt`、`横断契約`、`regression`、`precision closure` と、§9.11.3〜§9.11.13 の ambiguity / failure / review handoff / N/A / Go-No-Go / regression inheritance / deterministic / assertion / negative surface / evidence integrity / operator observability closure を Phase packet に転記してから実装する。1 つでも未定義、未読、未転記、または Phase packet / Done receipt / 実装差分と不一致がある場合は、実装開始禁止または Phase 未完了とする。

| Phase | 詳細節 | 固定契約 | 台帳 | シナリオ | Done receipt | 横断契約 | 必須 regression | precision closure |
|-------|--------|----------|------|----------|--------------|----------|-----------------|-------------------|
| 1 | `### Phase 1` | Phase 1 完全実装精度固定契約 | Phase 1 原子タスク台帳 | Phase 1 シナリオマトリクス | Phase 1 Done receipt 必須項目 | Phase 1〜5 完全実装精度正規化契約 | build/test | help/config/no persistence/stub token |
| 2 | `### Phase 2` | Phase 2 完全実装精度固定契約 | Phase 2 原子タスク台帳 | Phase 2 シナリオマトリクス | Phase 2 Done receipt 必須項目 | Phase 1〜5 完全実装精度正規化契約 | Phase 1 regression | data-dir/lock/metadata/default DB/WAL/integrity/restart/no HTTP |
| 3 | `### Phase 3` | Phase 3 完全実装精度固定契約 | Phase 3 原子タスク台帳 | Phase 3 シナリオマトリクス | Phase 3 Done receipt 必須項目 | Phase 1〜5 完全実装精度正規化契約 | Phase 1〜2 regression | hrana-http v2 schema/wire snapshot/SDK transcript/restart persistence |
| 4 | `### Phase 4` | Phase 4 完全実装精度固定契約 | Phase 4 原子タスク台帳 | Phase 4 シナリオマトリクス | Phase 4 Done receipt 必須項目 | Phase 1〜5 完全実装精度正規化契約 | Phase 1〜3 regression | auth matrix/permission matrix/token persistence/secret redaction |
| 5 | `### Phase 5` | Phase 5 完全実装精度固定契約 | Phase 5 原子タスク台帳 | Phase 5 シナリオマトリクス | Phase 5 Done receipt 必須項目 | Phase 1〜5 完全実装精度正規化契約 | Phase 1〜4 regression | JSONL/request log/SDK CRUD/restart/secret scan/unsupported surface |
| 6 | `### Phase 6` | Phase 6 完全実装精度固定契約 | Phase 6 原子タスク台帳 | Phase 6 シナリオマトリクス | Phase 6 Done receipt 必須項目 | Phase 6〜11 完全実装精度正規化契約 | Phase 1〜5 regression | DB name/path routing/default compatibility/isolation/metadata/restart |
| 7 | `### Phase 7` | Phase 7 完全実装精度固定契約 | Phase 7 原子タスク台帳 | Phase 7 シナリオマトリクス | Phase 7 Done receipt 必須項目 | Phase 6〜11 完全実装精度正規化契約 | Phase 1〜6 regression | Admin auth/DB CRUD/token CRUD/DB scope/revoke/metadata consistency |
| 8 | `### Phase 8` | Phase 8 完全実装精度固定契約 | Phase 8 原子タスク台帳 | Phase 8 シナリオマトリクス | Phase 8 Done receipt 必須項目 | Phase 6〜11 完全実装精度正規化契約 | Phase 1〜7 regression | org/group/location/quota/usage/Turso wrapper/migration/scope/quota |
| 9 | `### Phase 9` | Phase 9 完全実装精度固定契約 | Phase 9 原子タスク台帳 | Phase 9 シナリオマトリクス | Phase 9 Done receipt 必須項目 | Phase 6〜11 完全実装精度正規化契約 | Phase 1〜8 regression | WebSocket upgrade/hello/stream/transaction/close/SDK WS |
| 10 | `### Phase 10` | Phase 10 完全実装精度固定契約 | Phase 10 原子タスク台帳 | Phase 10 シナリオマトリクス | Phase 10 Done receipt 必須項目 | Phase 6〜11 完全実装精度正規化契約 | Phase 1〜9 regression | ATTACH allow/deny/path rejection/metrics/admin auth/redaction |
| 11 | `### Phase 11` | Phase 11 完全実装精度固定契約 | Phase 11 原子タスク台帳 | Phase 11 シナリオマトリクス | Phase 11 Done receipt 必須項目 | Phase 6〜11 完全実装精度正規化契約 | Phase 1〜10 regression | replica registration/WAL stream/checksum/snapshot/retention/compatibility |
| 12 | `### Phase 12` | Phase 12 完全実装精度固定契約 | Phase 12 原子タスク台帳 | Phase 12 シナリオマトリクス | Phase 12 Done receipt 必須項目 | Phase 12〜19 完全実装精度正規化契約 | Phase 1〜11 regression | replica state/snapshot bootstrap/WAL catch-up/redirect/primary down/multi replica |
| 13 | `### Phase 13` | Phase 13 完全実装精度固定契約 | Phase 13 原子タスク台帳 | Phase 13 シナリオマトリクス | Phase 13 Done receipt 必須項目 | Phase 12〜19 完全実装精度正規化契約 | Phase 1〜12 regression | manifest/archive/snapshot/retention/corruption/disabled mode |
| 14 | `### Phase 14` | Phase 14 完全実装精度固定契約 | Phase 14 原子タスク台帳 | Phase 14 シナリオマトリクス | Phase 14 Done receipt 必須項目 | Phase 12〜19 完全実装精度正規化契約 | Phase 1〜13 regression | backup consistency/restore rollback/PITR/startup recovery/policy precedence |
| 15 | `### Phase 15` | Phase 15 完全実装精度固定契約 | Phase 15 原子タスク台帳 | Phase 15 シナリオマトリクス | Phase 15 Done receipt 必須項目 | Phase 12〜19 完全実装精度正規化契約 | Phase 1〜14 regression | branch metadata/create/delete recovery/routing isolation/seed compatibility |
| 16 | `### Phase 16` | Phase 16 完全実装精度固定契約 | Phase 16 原子タスク台帳 | Phase 16 シナリオマトリクス | Phase 16 Done receipt 必須項目 | Phase 12〜19 完全実装精度正規化契約 | Phase 1〜15 regression | extension manifest/allowlist/sha256/path rejection/load/SQL bypass |
| 17 | `### Phase 17` | Phase 17 完全実装精度固定契約 | Phase 17 原子タスク台帳 | Phase 17 シナリオマトリクス | Phase 17 Done receipt 必須項目 | Phase 12〜19 完全実装精度正規化契約 | Phase 1〜16 regression | metrics snapshot/Prometheus/counter restore/usage-quota/label redaction |
| 18 | `### Phase 18` | Phase 18 完全実装精度固定契約 | Phase 18 原子タスク台帳 | Phase 18 シナリオマトリクス | Phase 18 Done receipt 必須項目 | Phase 12〜19 完全実装精度正規化契約 | Phase 1〜17 regression | HA state/term/promote/demote/redirect/split-brain/restart |
| 19 | `### Phase 19` | Phase 19 完全実装精度固定契約 | Phase 19 原子タスク台帳 | Phase 19 シナリオマトリクス | Phase 19 Done receipt 必須項目 | Phase 12〜19 完全実装精度正規化契約 | Phase 1〜18 regression | config flags/shadow-active-rollback/adapter boundary/SDK/performance/crash recovery |

**索引の判定規則：**

| 状態 | 判定 |
|------|------|
| 対象 Phase の詳細節、固定契約、台帳、シナリオ、Done receipt、横断契約のいずれかを Phase packet に転記していない | 実装開始禁止 |
| Phase packet の `scope`、`regression_set`、`precision_closure_result` が本索引と一致しない | 実装開始禁止 |
| Done receipt の `implemented_scope`、`excluded_scope`、`atomic_task_result`、`scenario_matrix_result`、`precision_closure_result` が本索引と一致しない | Phase 未完了 |
| Phase packet、Done receipt、`P{phase}-PRECISION-CLOSURE` のいずれかで §9.11.3〜§9.11.13 の closure result を相互参照できない | Phase 未完了 |
| 本索引にない Phase 外機能、未来 API、metadata、config、dependency を実装差分へ含める | merge 不可 |
| 本索引の必須 regression を実行せず、影響なし理由も Phase packet / Done receipt にない | Phase 未完了 |
| 索引、Phase 詳細節、§9.2、§9.4、§9.8、§9.11、§9.17 が矛盾する | 仕様修正 PR に戻す |

### 9.11.2 Phase 1〜19 Contract ID / artifact path 固定契約

本節は Phase packet、受入 manifest、原子タスク台帳、シナリオマトリクス、test、artifact、Done receipt を同じ契約 ID で接続するための命名規則である。実装者は対象 Phase のすべての証跡に Contract ID を割り当て、artifact path と Done receipt field を 1 対 1 で対応させる。後付け ID、PR description だけの対応表、環境依存 path は Phase 完了根拠として扱わない。

**Contract ID 標準形式：**

| Contract ID | 用途 | 必須対応先 |
|-------------|------|------------|
| `P{phase}-API-{name}` | HTTP / WebSocket / CLI / Admin / Platform / replication / HA endpoint の request/response 契約 | API snapshot、error snapshot、SDK transcript |
| `P{phase}-PERSIST-{name}` | metadata、DB file、WAL、archive、backup、branch、extension、metrics、HA state、internal adapter state の永続化契約 | persistence fixture、recovery log、restart transcript |
| `P{phase}-AUTH-{name}` | JWT、Admin token、Platform token、replication token、HA token、scope、quota、block policy の認証認可契約 | auth matrix、permission matrix、redaction scan |
| `P{phase}-COMPAT-{name}` | Turso Cloud、libSQL SDK、hrana wire、legacy metadata、previous Phase との互換契約 | compatibility diff、SDK transcript、snapshot diff |
| `P{phase}-RECOVERY-{name}` | crash、rollback、restart、partial failure、operator_required、startup recovery の契約 | recovery log、failure injection artifact |
| `P{phase}-REDACTION-{name}` | secret、token、SQL args、raw path、frame bytes、backup body、absolute path の秘匿契約 | secret scan result |
| `P{phase}-REGRESSION-{name}` | 当該 Phase と過去 Phase の regression 契約 | CI output、release-check output、regression transcript |
| `P{phase}-PRECISION-CLOSURE` | Done receipt の `precision_closure_result` を証明する最終閉鎖契約 | precision closure artifact、review handoff、ambiguity closure、failure closure、N/A closure、Go-No-Go、regression inheritance、determinism、assertion binding、negative surface、evidence integrity、operator observability、open decision 0 件 |

`{phase}` は整数だけを使い、`P01` のような 0 padding はしない。`{name}` は ASCII lowercase、数字、hyphen のみを許可し、space、underscore、slash、日本語、timestamp、random ID、local username、host name を含めてはならない。例: `P14-RECOVERY-restore-rollback`、`P19-COMPAT-sdk-transcript`。

**artifact path 標準形：**

| Artifact 種別 | Path | 必須内容 |
|---------------|------|----------|
| request fixture | `tests/fixtures/phase-{phase}/{contract-id}/request.json` | method、path、query、header subset、body、normalization rule |
| response snapshot | `tests/snapshots/phase-{phase}/{contract-id}/response.json` | status、header subset、body、error code、redaction |
| scenario artifact | `tests/artifacts/phase-{phase}/{contract-id}/scenario-{scenario-id}.json` | scenario ID、input、expected、actual、pass/fail、artifact generator |
| persistence fixture | `tests/fixtures/phase-{phase}/{contract-id}/persistence.json` | file path、schema version、before/after、fsync、rollback、restart result |
| recovery log | `tests/artifacts/phase-{phase}/{contract-id}/recovery.log` | failure injection、detected state、rollback/recovery result、operator action |
| compatibility diff | `tests/artifacts/phase-{phase}/{contract-id}/compat.diff` | Turso / libSQL SDK / hrana / previous Phase の正規化済み差分 |
| SDK transcript | `tests/artifacts/phase-{phase}/{contract-id}/sdk-transcript.txt` | SDK name/version、command、request/response、exit code |
| CI output | `tests/artifacts/phase-{phase}/{contract-id}/ci.txt` | command、environment、exit code、pass/fail summary |
| secret scan | `tests/artifacts/phase-{phase}/{contract-id}/secret-scan.txt` | scan command、対象 path、検出 0 件、redaction rule |
| performance baseline | `tests/artifacts/phase-{phase}/{contract-id}/performance.json` | p95、RSS、DB size、WAL size、workload、baseline ratio |
| phase packet | `docs/phase-evidence/phase-{phase}/packet.md` | 本索引、Contract ID、Task ID、Scenario ID、regression、Done gate |
| Done receipt | `docs/phase-evidence/phase-{phase}/done.md` | Done receipt fields、evidence index、failure closure、precision closure |

**Task / Scenario / Done receipt 対応規則：**

| 対象 | 固定規則 |
|------|----------|
| `TASK-P{phase}-{number}` | 1 つ以上の Contract ID を `owner_contracts` として持つ。owner なし task は merge 不可 |
| `SCN-P{phase}-{number}` | 1 つ以上の Contract ID と artifact path を持つ。artifact なし scenario は pass 扱い禁止 |
| `atomic_task_result` | 全 task ID、owner Contract ID、verification command、artifact path、pass/fail を列挙する |
| `scenario_matrix_result` | 全 scenario ID、owner Contract ID、artifact path、pass/fail、N/A reason を列挙する |
| `compatibility_baseline_result` | `P{phase}-COMPAT-*` の artifact をすべて参照する |
| `secret_redaction_result` | `P{phase}-REDACTION-*` の secret scan をすべて参照する |
| `regression_result` | `P{phase}-REGRESSION-*` の CI / regression transcript をすべて参照する |
| `precision_closure_result` | 必ず `P{phase}-PRECISION-CLOSURE` を参照し、coverage gap、open task、open scenario、open decision、known flaky が 0 件であることを示す。さらに `ambiguity_closure_result`、`failure_closure_result`、`review_handoff_result`、`na_closure_result`、`go_no_go_result`、`regression_inheritance_result`、`determinism_result`、`assertion_binding_result`、`negative_surface_result`、`evidence_integrity_result`、`operator_observability_result` の全 field、artifact path、reviewer 再現 command、open count 0 を含める |

**precision_closure_result 標準テンプレート：**

`precision_closure_result` は以下の 11 field をこの名前で持つ。各 field は `status`、`contract_id`、`artifact_path`、`reviewer_command`、`open_count`、`source_section`、`blocking_rule` を必ず含める。field 名、key 名、Contract ID、artifact path、source section のいずれかが本表と一致しない場合は、実装者が意図を推測する余地が残るため Phase 未完了とする。

| field | source_section | artifact_path | blocking_rule |
|-------|----------------|---------------|---------------|
| `ambiguity_closure_result` | §9.11.3 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/ambiguity-closure.json` | open ambiguity 0 件 |
| `failure_closure_result` | §9.11.4 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/failure-closure.json` | open failure 0、known flaky 0、unverified 0、missing artifact 0 件 |
| `review_handoff_result` | §9.11.5 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/review-handoff.json` | third-party reproduction pass、oral context 0 件 |
| `na_closure_result` | §9.11.6 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/na-closure.json` | rootless N/A 0、manual only pass 0、skipped required verification 0 件 |
| `go_no_go_result` | §9.11.7 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/go-no-go.json` | entry_go true、exit_go true、blocking item 0 件 |
| `regression_inheritance_result` | §9.11.8 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/regression-inheritance.json` | inherited regression failure 0、missing previous Phase regression 0 件 |
| `determinism_result` | §9.11.9 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/determinism.json` | flaky 0、nondeterministic artifact 0 件 |
| `assertion_binding_result` | §9.11.10 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/assertion-binding.json` | missing assertion 0、oracle binding pass |
| `negative_surface_result` | §9.11.11 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/negative-surface.json` | unsupported success 0、denial redaction pass |
| `evidence_integrity_result` | §9.11.12 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/evidence-integrity.json` | stale artifact 0、manifest mismatch 0、missing integrity 0 件 |
| `operator_observability_result` | §9.11.13 | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/operator-observability.json` | operator action gap 0、observability pass、operator secret leak 0 件 |

各 field の `status` は `pass` 固定、`contract_id` は `P{phase}-PRECISION-CLOSURE` 固定、`open_count` は `0` 固定とする。`reviewer_command` は第三者がその field の artifact を生成または検証できる具体 command を記録し、`artifact_path` は上表の標準 path と完全一致させる。`status: pass` だけ、PR description だけ、手元実行ログだけ、または artifact path のない closure field は pass 扱いしてはならない。

**precision_closure_result canonical receipt format：**

Done receipt に記録する `precision_closure_result` は、以下の canonical object を正とする。`closure_fields` は配列ではなく固定 key の object とし、key 漏れ、順序依存、別名、要約だけの pass を禁止する。

| key | 必須値 |
|-----|--------|
| `phase` | 対象 Phase 番号。整数のみ |
| `contract_id` | `P{phase}-PRECISION-CLOSURE` |
| `summary_status` | `pass`。ただし 11 closure field すべてが `status: pass`、全 `open_count: 0`、artifact path 全件存在、reviewer command 全件再現可能、unsupported / negative / operator observability が pass の場合だけ許可 |
| `closure_fields` | `ambiguity_closure_result`、`failure_closure_result`、`review_handoff_result`、`na_closure_result`、`go_no_go_result`、`regression_inheritance_result`、`determinism_result`、`assertion_binding_result`、`negative_surface_result`、`evidence_integrity_result`、`operator_observability_result` を固定 key として持つ object |
| `artifact_manifest` | 11 field の `artifact_path`、生成 command、sha256 または content hash、生成日時の source、secret scan result を列挙する |
| `reviewer_reproduction` | 11 field の `reviewer_command` と expected exit code、expected artifact path、再現不能時の failure classification を列挙する |
| `open_counts` | coverage gap、open task、open scenario、open decision、known flaky、open ambiguity、open failure、unverified、missing artifact、rootless N/A、blocking item、regression failure、unsupported success、operator action gap をすべて `0` として列挙する |
| `generated_at_source` | artifact の生成元を `ci`、`release-check`、`local-docker` のいずれかで示す。時刻だけ、手入力、PR description は不可 |

canonical object は Done receipt 本文にそのまま貼れる Markdown table または JSON object とする。ただし JSON object を使う場合も key 名は本表と完全一致させ、`closure_fields` の 11 key を省略してはならない。

**precision closure artifact manifest schema：**

`artifact_manifest` は 11 closure field と 1:1 で対応する entry を持つ。各 entry は以下の key を必ず持ち、field 名、artifact path、reviewer command が `closure_fields` の同名 field と一致しなければならない。

| key | 必須値 |
|-----|--------|
| `field` | 11 closure field のいずれか。別名、短縮名、配列 index は不可 |
| `artifact_path` | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/{artifact-name}.json` |
| `producer_command` | artifact を生成した再実行可能 command |
| `producer_exit_code` | `0` |
| `content_hash` | 正規化済み artifact 本文の hash |
| `hash_algorithm` | `sha256` 固定 |
| `generated_at_source` | `ci`、`release-check`、`local-docker` のいずれか |
| `secret_scan_result` | `pass`。secret/token/JWT/SQL args/raw path/frame bytes の検出 0 件 |
| `redaction_policy` | 適用した redaction rule 名、または不要な場合 `none_with_reason` |
| `reviewer_command` | reviewer が artifact と hash と secret scan を再検証できる command |

`content_hash` の対象は正規化済み artifact 本文だけとする。local absolute path、timestamp、hostname、username、非決定的 temporary path、実行順序で変わる ID は hash 対象から除外し、必要な場合は正規化 rule を artifact 内に記録する。`secret_scan_result` を pass にするには、scan command、対象 path、検出 0 件、redaction policy を manifest entry に残さなければならない。

**precision closure reviewer reproduction schema：**

`reviewer_reproduction` は 11 closure field と 1:1 で対応する entry を持つ。各 entry は以下の key を必ず持ち、artifact と hash の両方を同じ command または連続 command で検証できなければならない。

| key | 必須値 |
|-----|--------|
| `field` | 11 closure field のいずれか。`artifact_manifest.field` と一致 |
| `reviewer_command` | reviewer が artifact existence、content hash、secret scan result を再検証できる command |
| `working_directory` | repo root 相対または `repo-root`。local absolute path は不可 |
| `environment_profile` | `ci`、`release-check`、`local-docker` のいずれか |
| `expected_exit_code` | `0` |
| `expected_artifact_path` | `artifact_manifest.artifact_path` と完全一致 |
| `expected_hash_algorithm` | `sha256` |
| `expected_content_hash` | `artifact_manifest.content_hash` と完全一致 |
| `timeout_seconds` | 正の整数。未指定、`0`、無制限は不可 |
| `failure_classification` | `not_run`、`command_failed`、`artifact_missing`、`hash_mismatch`、`secret_scan_failed`、`environment_gap`、`spec_gap` のいずれか |

`environment_profile` が `ci`、`release-check`、`local-docker` 以外の場合、その reproduction entry は無効とする。`failure_classification` は失敗時の分類であり、`summary_status: pass` の場合は全 entry が実行済みで、`failure_classification` に `not_run` または `environment_gap` を含んではならない。

**precision_closure_result review algorithm：**

レビュアーと CI は以下の順序で `precision_closure_result` を評価する。各 step は fail fast とし、失敗した step より後続の step を pass 扱いしてはならない。`summary_status` は最後の step でのみ評価し、途中 step の代替証跡にしてはならない。

| step | 確認内容 | failure `review_result` |
|------|----------|-------------------------|
| 1 | `contract_id` が `P{phase}-PRECISION-CLOSURE` と完全一致する | `fail_contract_mismatch` |
| 2 | canonical key（`phase`、`contract_id`、`summary_status`、`closure_fields`、`artifact_manifest`、`reviewer_reproduction`、`open_counts`、`generated_at_source`）が全て存在する | `fail_missing_key` |
| 3 | `closure_fields` に 11 closure field が固定 key object として全て存在する | `fail_missing_field` |
| 4 | 11 field の `artifact_path` が標準 path と完全一致し、Contract ID と Phase 番号を含む | `fail_artifact_path` |
| 5 | `artifact_manifest` が 11 field の artifact path、生成 command、hash、secret scan result と一致する | `fail_manifest_mismatch` |
| 6 | `reviewer_reproduction` の各 command が expected exit code で終了し、expected artifact path を生成または検証できる | `fail_reproduction` |
| 7 | `open_counts` と各 field の `open_count` が全て `0` で一致する | `fail_open_count` |
| 8 | `summary_status` が `pass` であり、step 1〜7 の失敗が 0 件である | `fail_summary_status` |

`review_result` は `pass`、`fail_contract_mismatch`、`fail_missing_key`、`fail_missing_field`、`fail_artifact_path`、`fail_manifest_mismatch`、`fail_reproduction`、`fail_open_count`、`fail_summary_status` のいずれかだけを許可する。`warning`、`accepted_risk`、`manual pass`、`pass with notes`、空欄は Phase 完了根拠にしてはならない。

**precision closure phase packet freeze checklist：**

実装者は対象 Phase の実装開始前に、Phase packet へ以下の freeze checklist を転記する。freeze checklist は実装 PR の開始境界であり、実装中に closure field 名、artifact path、review command、open count target、summary_status 条件を暗黙変更してはならない。

| freeze key | 必須内容 |
|------------|----------|
| `phase` | 対象 Phase 番号 |
| `contract_id` | `P{phase}-PRECISION-CLOSURE` |
| `closure_field_set` | 11 closure field 名の完全な一覧 |
| `artifact_manifest_schema` | `field`、`artifact_path`、`producer_command`、`producer_exit_code`、`content_hash`、`hash_algorithm`、`generated_at_source`、`secret_scan_result`、`redaction_policy`、`reviewer_command` |
| `reviewer_reproduction_schema` | `field`、`reviewer_command`、`working_directory`、`environment_profile`、`expected_exit_code`、`expected_artifact_path`、`expected_hash_algorithm`、`expected_content_hash`、`timeout_seconds`、`failure_classification` |
| `review_algorithm_steps` | step 1〜8 と failure `review_result` の対応 |
| `allowed_review_result_values` | `pass`、`fail_contract_mismatch`、`fail_missing_key`、`fail_missing_field`、`fail_artifact_path`、`fail_manifest_mismatch`、`fail_reproduction`、`fail_open_count`、`fail_summary_status` |
| `open_count_targets` | coverage gap、open task、open scenario、open decision、known flaky、open ambiguity、open failure、unverified、missing artifact、rootless N/A、blocking item、regression failure、unsupported success、operator action gap がすべて `0` |
| `forbidden_shortcuts` | `summary_status: pass` だけ、manual only、see CI、配列 `closure_fields`、artifact manifest 省略、reviewer reproduction 省略、open count 省略、freeze 後の暗黙変更 |

freeze 後に closure field 名、artifact path、review command、open count target、review algorithm、allowed review result、`summary_status` 条件を変更する場合は、実装 PR 内の判断で処理せず、仕様修正 PR に戻す。Done receipt は Phase packet の freeze checklist と完全一致しなければならない。

**precision closure done receipt reconciliation gate：**

Phase 完了時の Done receipt は、Phase packet freeze checklist と `precision_closure_result` の実証を以下の gate で照合する。照合対象は Phase packet freeze checklist、canonical receipt format、artifact manifest schema、reviewer reproduction schema、review algorithm result、open count targets とする。Done receipt 側で freeze と違う値へ書き換えること、または実装後に pass 条件を緩めることを禁止する。

| reconciliation field | pass 条件 |
|----------------------|-----------|
| `freeze_match_result` | Phase packet の freeze checklist と Done receipt の `phase`、`contract_id`、11 field、schema、review algorithm、allowed review result が完全一致 |
| `canonical_format_result` | Done receipt が canonical receipt format の全 key を持ち、`closure_fields` が固定 key object である |
| `manifest_match_result` | `artifact_manifest` が 11 field と 1:1 対応し、path、hash、secret scan、redaction policy が closure field と一致 |
| `reproduction_match_result` | `reviewer_reproduction` が 11 field と 1:1 対応し、全 command が expected exit code、expected artifact path、expected content hash と一致 |
| `review_algorithm_result` | step 1〜8 を順序通り実行し、`review_result = pass` である |
| `open_count_match_result` | Phase packet の `open_count_targets` と Done receipt の `open_counts` がすべて `0` で一致 |
| `final_reconciliation_result` | 上記 6 field がすべて `pass`。1 件でも `fail`、`missing`、`not_run`、`manual_only` があれば `fail` |

`final_reconciliation_result = pass` の場合だけ Phase 完了候補にできる。`freeze_match_result`、`canonical_format_result`、`manifest_match_result`、`reproduction_match_result`、`review_algorithm_result`、`open_count_match_result` のいずれかを省略した Done receipt は、機能が動作していても Phase 未完了とする。

**precision closure reconciliation failure remediation map：**

reconciliation field が `pass` 以外になった場合、実装者は以下の修正先を先に解消する。失敗 field を `N/A` に逃がすこと、`final_reconciliation_result` だけを pass にすること、artifact だけを差し替えて manifest / hash / reviewer reproduction を更新しないことを禁止する。

| failed field | 修正先 |
|--------------|--------|
| `freeze_match_result` | Phase packet freeze checklist、または仕様修正 PR。Done receipt 側だけを書き換えない |
| `canonical_format_result` | Done receipt の canonical object。key 名、固定 key object、summary 条件を修正 |
| `manifest_match_result` | artifact manifest、artifact generator、content hash、secret scan、redaction policy を同時修正 |
| `reproduction_match_result` | reviewer reproduction command、environment profile、expected artifact path、expected content hash、timeout を同時修正 |
| `review_algorithm_result` | review algorithm の失敗 step と `review_result` を一致させ、fail fast 順序を修正 |
| `open_count_match_result` | open item closure、failure record、N/A 根拠、regression failure、operator action gap を先に閉じる |
| `final_reconciliation_result` | 上記 6 field の失敗を先に解消する。final だけ pass にしてはならない |

`reproduction_match_result` の失敗を `environment_gap` のまま pass にしてはならない。環境差分が原因の場合は、`environment_profile`、再現 command、toolchain、Docker / CI / release-check 差分、artifact path を仕様または Phase packet に反映し、再実行で pass するまで Phase 未完了とする。

**precision closure remediation order：**

複数の reconciliation failure が同時に出た場合は、以下の順序で修正する。先順位の失敗が残っている間は、後順位 field を pass にしてはならない。

| order | field | 理由 |
|-------|-------|------|
| 1 | `freeze_match_result` | 実装開始前の固定契約が正しくなければ、後続 artifact / review の意味が確定しない |
| 2 | `canonical_format_result` | Done receipt の構造が固定されなければ、manifest / reproduction を機械照合できない |
| 3 | `manifest_match_result` | artifact の path、hash、secret scan が確定しなければ、reproduction の期待値が決まらない |
| 4 | `reproduction_match_result` | reviewer が再現できなければ、review algorithm の pass 根拠にならない |
| 5 | `review_algorithm_result` | step 順序と failure result が確定しなければ、open count / final を評価できない |
| 6 | `open_count_match_result` | open item が残る限り final pass は許可しない |
| 7 | `final_reconciliation_result` | 上記 6 field が pass になった後だけ評価する |

manifest / reproduction を先に直して `freeze_match_result` の失敗を隠すこと、open count が残っている状態で `final_reconciliation_result` を pass にすること、複数失敗を 1 つの `environment_gap` にまとめることを禁止する。

**precision closure cross-reference ledger：**

Phase packet、Done receipt、artifact manifest、reviewer reproduction、review algorithm、reconciliation result は、同じ closure を以下の ledger entry で相互参照する。ledger にない artifact、manifest entry、reproduction entry、review step、reconciliation field は Phase 完了根拠にしてはならない。

| ledger key | 必須値 |
|------------|--------|
| `phase` | 対象 Phase 番号 |
| `contract_id` | `P{phase}-PRECISION-CLOSURE` |
| `closure_field` | 11 closure field のいずれか |
| `artifact_path` | 対象 `closure_field` の標準 artifact path |
| `manifest_entry_id` | `PCR-P{phase}-{closure-field}-manifest` |
| `reproduction_entry_id` | `PCR-P{phase}-{closure-field}-reproduction` |
| `review_step` | `artifact_path` は step 4、manifest は step 5、reproduction は step 6、open count は step 7 に対応 |
| `reconciliation_field` | `manifest_match_result`、`reproduction_match_result`、`review_algorithm_result`、`open_count_match_result` のいずれか |
| `source_section` | 対応する §9.11.3〜§9.11.13 の節番号 |

`manifest_entry_id` と `reproduction_entry_id` の `{closure-field}` は `ambiguity-closure`、`failure-closure`、`review-handoff`、`na-closure`、`go-no-go`、`regression-inheritance`、`determinism`、`assertion-binding`、`negative-surface`、`evidence-integrity`、`operator-observability` のいずれかとする。別 Phase の Contract ID、別 Phase の artifact path、または `source_section` と `closure_field` の不一致を参照してはならない。

**Phase receipt closure reference audit：**

Phase 1〜19 の各 `precision_closure_result` は、§9.11.2 の canonical、manifest、reproduction、review algorithm、freeze、reconciliation、remediation、cross-reference ledger への準拠を同一文言で参照しなければならない。この audit は、個別 Phase の Done receipt が §9.11.2 の更新から取り残されることを防ぐための横断 gate である。

| audit field | pass 条件 |
|-------------|----------|
| `phase_receipt_reference_count` | Phase 1〜19 の個別 Done receipt にある `precision_closure_result` のうち、§9.11.2 closure reference set を含む行が 19 件 |
| `missing_phase_receipt_references` | §9.11.2 closure reference set を参照しない Phase 番号が 0 件 |
| `extra_phase_receipt_references` | Phase 1〜19 個別 Done receipt 以外に、同じ closure reference set を誤って完了根拠として置いた箇所が 0 件 |
| `mismatched_reference_text` | 19 件すべての §9.11.2 closure reference set 文言が同一である |
| `phase_receipt_reference_audit_result` | 上記 4 field がすべて pass |

§9.11.2 の canonical、manifest、reproduction、review algorithm、freeze、reconciliation、remediation、cross-reference ledger のいずれかを更新する PR は、同じ PR で Phase 1〜19 個別 Done receipt の `precision_closure_result` 行と本 audit を更新する。更新しない場合は merge 不可とする。

**命名不一致時の判定：**

| 状態 | 判定 |
|------|------|
| Contract ID、Task ID、Scenario ID、artifact path、Done receipt field のいずれかが相互参照できない | Phase 未完了 |
| artifact path に Contract ID が含まれない | Phase 未完了 |
| `precision_closure_result` が `P{phase}-PRECISION-CLOSURE` を参照しない | Phase 未完了 |
| `precision_closure_result` に §9.11.3〜§9.11.13 に対応する 11 個の closure field が 1 つでも欠ける | Phase 未完了 |
| `phase_receipt_reference_count` が 19 ではない | Phase 未完了 |
| `missing_phase_receipt_references`、`extra_phase_receipt_references`、`mismatched_reference_text` が 0 件でない | Phase 未完了 |
| §9.11.2 の precision closure 契約を更新したのに Phase 1〜19 個別 Done receipt の `precision_closure_result` 行を同時更新しない | merge 不可 |
| Phase ごとに §9.11.2 closure reference set の文言が揺れている | Phase 未完了 |
| closure field 名、key 名、`source_section`、`contract_id` が標準テンプレートと一致しない | Phase 未完了 |
| closure field の `artifact_path` が `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/...` の標準形でない | Phase 未完了 |
| closure field が pass でも artifact path、reviewer 再現 command、open count 0 のいずれかを示さない | merge 不可 |
| closure field が `status: pass` だけで、必須 key、artifact、reviewer command、blocking rule の実証を持たない | merge 不可 |
| reviewer が同じ command で artifact を再生成または検証できない | merge 不可 |
| `closure_fields` を配列にして順序依存にする、または固定 key object 以外で表現する | Phase 未完了 |
| `summary_status: pass` だけで 11 field、artifact manifest、reviewer reproduction、open counts を省略する | merge 不可 |
| `artifact_manifest` と closure field の `artifact_path` が一致しない | Phase 未完了 |
| `artifact_manifest` entry が 11 closure field と 1:1 対応しない | Phase 未完了 |
| manifest entry の `hash_algorithm` が未指定、`sha256` 以外、または `content_hash` の対象が不明 | merge 不可 |
| `content_hash` に local absolute path、timestamp、hostname、username、非決定的 temporary path が混入している | merge 不可 |
| `secret_scan_result` が未実行、manual only、抽象記載、または scan command / 対象 path / redaction policy を欠く | merge 不可 |
| `reviewer_command` が `see CI`、`manual`、`確認済み` など抽象表現だけで、再現可能な command ではない | merge 不可 |
| `reviewer_reproduction` entry が 11 closure field と 1:1 対応しない | Phase 未完了 |
| `reviewer_command` が artifact existence と content hash の両方を検証しない | merge 不可 |
| `timeout_seconds` が未指定、`0`、負数、または無制限である | Phase 未完了 |
| `environment_profile` が `ci`、`release-check`、`local-docker` 以外である | Phase 未完了 |
| `failure_classification` が許可値以外、または `environment_gap` を pass 扱いしている | merge 不可 |
| `not_run` が 1 件でもある状態で `summary_status: pass` とする | merge 不可 |
| review algorithm の step を飛ばす、順序を入れ替える、fail 後に後続 step で pass 扱いする | merge 不可 |
| `review_result` が許可値以外、または失敗 step と一致しない | Phase 未完了 |
| `summary_status` を step 1〜7 の代替証跡として扱う | merge 不可 |
| Phase packet に precision closure freeze checklist がない | 実装開始禁止 |
| freeze checklist と Done receipt の `precision_closure_result` が一致しない | Phase 未完了 |
| freeze 後に closure field 名、artifact path、review command、open count target、review algorithm、allowed review result、`summary_status` 条件を暗黙変更する | merge 不可 |
| 実装 PR 内で `summary_status` の pass 条件を緩める | merge 不可 |
| Done receipt に reconciliation field が 1 つでも欠ける | Phase 未完了 |
| reconciliation field が `pass` 以外、または `fail`、`missing`、`not_run`、`manual_only` を含む | Phase 未完了 |
| `final_reconciliation_result` が `pass` でない、または 6 field の pass を根拠にしていない | Phase 未完了 |
| Done receipt 側で freeze checklist と異なる値へ書き換える | merge 不可 |
| reconciliation failure を `N/A`、`accepted_risk`、`manual_only` で回避する | merge 不可 |
| failed field を残したまま `final_reconciliation_result` だけを pass にする | merge 不可 |
| artifact を差し替えて manifest、content hash、reviewer reproduction を更新しない | merge 不可 |
| reproduction 失敗を `environment_gap` のまま pass 扱いする | merge 不可 |
| remediation order を飛ばす、または先順位 failure を残したまま後順位 field を pass にする | merge 不可 |
| manifest / reproduction を先に直して `freeze_match_result` の失敗を隠す | merge 不可 |
| open count が残っている状態で `final_reconciliation_result` を pass にする | merge 不可 |
| 複数 failure を 1 つの `environment_gap` にまとめる | merge 不可 |
| cross-reference ledger にない artifact、manifest entry、reproduction entry、review step、reconciliation field を Phase 完了根拠にする | Phase 未完了 |
| `manifest_entry_id` または `reproduction_entry_id` が `PCR-P{phase}-{closure-field}-manifest` / `PCR-P{phase}-{closure-field}-reproduction` の形式でない | Phase 未完了 |
| 別 Phase の Contract ID、artifact path、manifest entry、reproduction entry を参照する | merge 不可 |
| `source_section` と `closure_field` の組み合わせが §9.11.3〜§9.11.13 の対応と一致しない | Phase 未完了 |
| `未解決判断 0 件` だけで `precision_closure_result` を pass とする | merge 不可 |
| Contract ID に timestamp、random ID、local username、host name、absolute path 由来文字列が含まれる | merge 不可 |
| artifact が生成されていないのに Done receipt で pass とする | merge 不可 |
| snapshot だけを更新し、対応する Contract ID、oracle、manifest、Done receipt を更新しない | merge 不可 |
| 同じ Contract ID が複数の意味を持つ、または同じ意味に複数 ID を割り当てる | 仕様修正 PR に戻す |

### 9.11.3 Phase 1〜19 ambiguity closure 最低判断表

本節は Phase 実装前に閉じるべき最低判断を定義する。§9.1.49 の ambiguity closure matrix は本表を下限とし、対象 Phase の Phase packet に `AMB-P{phase}-{surface}-{number}` 形式の ambiguity ID として転記する。`P{phase}-PRECISION-CLOSURE` は open ambiguity 0 件を証明しなければならない。

| Phase | ambiguity closure 最低判断 |
|-------|----------------------------|
| 1 | CLI command 名、必須 flag、config precedence、stub token create の出力、no persistence の確認方法 |
| 2 | data-dir layout、metadata 初期値、lock 競合、integrity_check failure、restart 時の既存 file 扱い |
| 3 | hrana request body validation、SQL error の HTTP status、baton/base_url、named_args 非対応、close 後処理 |
| 4 | secret source precedence、Bearer header strictness、JWT claim required/optional、revoked/unknown token、ro/rw SQL 分類 |
| 5 | JSON Lines field、request_id、SDK version、secret redaction、restart persistence、unsupported future surface |
| 6 | DB name validation、default route と path route の優先順位、存在しない DB、metadata/directory 不整合、DB isolation |
| 7 | Admin token strictness、DB CRUD status/body、token JWT 再表示禁止、revoke 即時反映、DB scope precedence |
| 8 | Turso wrapper field casing、organization/group/location/quota precedence、legacy metadata migration、usage unavailable、Platform token redaction |
| 9 | WebSocket subprotocol、hello 前 request、stream lifecycle、transaction rollback、store_sql scope、cursor unsupported |
| 10 | ATTACH parser 境界、任意 path 拒否、source/target scope、metrics counter 加算点、WebSocket close gauge |
| 11 | primary role 起動条件、replication token required、from_frame validation、frame_no/checksum、snapshot consistency、sync mode 未定義時挙動 |
| 12 | replica state schema、snapshot bootstrap、WAL gap/duplicate/checksum mismatch、redirect status、primary down、multi replica state |
| 13 | archive manifest schema、frame naming、retention cleanup order、partial write recovery、corrupt/orphan file、disabled mode |
| 14 | backup consistency、restore temp layout、commit/rollback order、PITR selector 解決、corrupt frame、startup recovery marker |
| 15 | branch name/internal name、source selector、create/delete rollback、runtime map、source delete denial、Turso seed compatibility |
| 16 | extension allowlist、canonical path、symlink rejection、sha256 mismatch、existing connection 非 retroactive、SQL bypass rejection |
| 17 | metrics snapshot schema、flush race、corrupt/future snapshot、Prometheus Accept、label redaction、usage/quota source |
| 18 | HA token/Admin token 境界、term monotonic、promote precondition、demote failure、leader unknown、split-brain、restart primary state |
| 19 | internal flag precedence、shadow diff handling、active mode gate、rollback no migration、performance threshold、adapter boundary、SDK transcript |

**ambiguity ID / 証跡規則：**

| 項目 | 固定仕様 |
|------|----------|
| ambiguity ID | `AMB-P{phase}-{surface}-{number}`。例: `AMB-P14-restore-1` |
| selected decision | 採用した判断を 1 つだけ記録する。複数案併記のまま実装してはならない |
| rejected options | 却下案と却下理由を記録する。理由なし却下は禁止 |
| decision basis | 参照する仕様節、Turso Cloud / libSQL SDK baseline、security / durability / compatibility 根拠 |
| affected contracts | 関連する Contract ID、Task ID、Scenario ID、artifact path |
| reopen trigger | upstream Turso 変更、SDK 変更、schema 変更、error code 変更、operator behavior 変更など再検討条件 |
| evidence | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/ambiguity-{ambiguity-id}.json` |

**ambiguity closure 判定規則：**

| 状態 | 判定 |
|------|------|
| `TBD`、`TODO`、`FIXME`、`未定`、`後で決める` が Phase packet / Done receipt / artifact に残る | 実装開始禁止 |
| `実装者判断`、`必要に応じて`、`適宜`、`可能なら` を完了条件に使う | merge 不可 |
| 本表の最低判断に対応する ambiguity ID が Phase packet に存在しない | 実装開始禁止 |
| ambiguity ID に selected decision、rejected options、decision basis、affected contracts、evidence が揃っていない | Phase 未完了 |
| open ambiguity が 1 件以上ある状態で `precision_closure_result` を pass にする | merge 不可 |
| ambiguity closure が PR description のみで仕様本文または phase evidence に存在しない | 仕様として扱わない |

### 9.11.4 Phase 1〜19 failure / resume closure 最低表

本節は Phase 実装中に失敗、flaky、未検証、artifact 欠落、blocked contract、途中中断が発生した場合の最低閉鎖条件を定義する。実装者は失敗を発見した時点で failure record を作成し、同一 PR 内で修正または仕様修正へ戻す。`P{phase}-PRECISION-CLOSURE` は open failure 0、open flaky 0、open unverified 0、missing artifact 0、blocked contract 0 を証明しなければならない。

| Phase | failure / resume closure 最低対象 |
|-------|-----------------------------------|
| 1 | CLI parse failure、config precedence failure、stub token mismatch、unexpected persistence、help snapshot drift |
| 2 | data-dir partial init、lock conflict、metadata parse failure、DB open failure、integrity_check failure、restart mismatch |
| 3 | malformed JSON、hrana result mismatch、SQL error mapping、close behavior、restart persistence、SDK smoke failure |
| 4 | secret resolution failure、JWT validation failure、permission classifier mismatch、token atomicity、revoke state、secret leak |
| 5 | JSONL parse failure、request log field missing、SDK transcript failure、restart data loss、unsupported future route success、secret scan hit |
| 6 | DB name validation miss、path route/default route mix、metadata/directory inconsistency、DB isolation failure、restart restore failure |
| 7 | admin auth bypass、DB CRUD atomicity failure、token JWT leak、revoke delayed effect、DB scope mismatch、concurrency lost update |
| 8 | metadata migration failure、legacy fallback failure、Turso snapshot drift、scope/quota precedence mismatch、Platform token leak |
| 9 | WebSocket upgrade failure、hello ordering bug、stream state leak、transaction rollback failure、store_sql scope leak、SDK WS failure |
| 10 | ATTACH parser false positive/negative、arbitrary path success、scope/block mismatch、metrics counter drift、WebSocket gauge leak |
| 11 | primary port/config failure、replication auth bypass、frame_no/checksum mismatch、SSE ordering drift、snapshot inconsistency、heartbeat/status leak |
| 12 | replica state corruption、resume frame mismatch、WAL gap/duplicate handling、checksum mismatch unresolved、redirect drift、primary down ambiguity |
| 13 | manifest/file inconsistency、partial archive write、retention unsafe delete、corrupt/orphan file handling、disabled mode drift |
| 14 | backup inconsistency、restore partial commit、rollback failure、PITR range/frame corruption、startup recovery marker ambiguity |
| 15 | branch metadata/runtime mismatch、branch create/delete partial failure、source delete denial miss、seed compatibility drift、restart recovery failure |
| 16 | extension path/symlink bypass、sha mismatch handling、load_failed recovery drift、SQL load bypass、absolute path leak |
| 17 | metrics snapshot corruption、counter lost increment、flush race、Prometheus format drift、label secret leak、usage/quota source mismatch |
| 18 | HA term regression、promote/demote commit failure, leader unknown drift、split-brain miss、restart primary write leak、partition ambiguity |
| 19 | invalid flag fallback、shadow diff unresolved、active mode gate miss、rollback needs migration、performance threshold miss、SDK transcript drift |

**failure record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `failure_id` | `FAIL-P{phase}-{surface}-{number}` 形式の一意 ID |
| `owner_contract` | 対応する Contract ID。横断失敗は複数可 |
| `failure_class` | `command_failure`、`flaky`、`unverified`、`missing_artifact`、`secret_leak`、`spec_conflict`、`regression`、`blocked_contract` のいずれか |
| `first_seen` | command、scenario ID、artifact path、observed result |
| `root_cause` | 仕様不足、実装不一致、環境差分、oracle 不一致、互換差分、secret 混入など |
| `resolution` | code fix、spec fix、oracle fix、environment fix、not_a_defect のいずれか。`not_a_defect` は仕様本文参照必須 |
| `rerun_evidence` | 修正後の command、exit code、artifact path、secret scan、regression result |
| `resume_point` | 再開する execution step、直前 successful command、再生成する artifact |
| `status` | Phase Done 時は `closed` のみ。`open`、`deferred`、`known_issue`、`flaky_but_passed` は禁止 |

**resume closure 固定規則：**

| 状態 | 固定仕様 |
|------|----------|
| 中断後に再開する | Phase packet、manifest、Contract ID、Task ID、Scenario ID、artifact path、Done receipt を再確認する |
| 再開時に仕様 version が進んでいる | 新 version に合わせて Phase packet / manifest / Contract ID / Done receipt を再固定する |
| artifact を再生成する | 旧 artifact を参照した Done receipt を更新し、Contract ID と生成 command を一致させる |
| failure を仕様変更で対象外へ移す | 仕様本文、manifest、Contract ID、Scenario ID、artifact path、Done receipt を同じ PR で更新する |
| flaky が再実行で通った | 原因、失敗ログ、deterministic 化修正、再実行 artifact がなければ未完了 |

**failure / resume 判定規則：**

| 状態 | 判定 |
|------|------|
| open failure、open flaky、open unverified、missing artifact、blocked contract が 1 件以上ある | Phase 未完了 |
| `known issue`、`後で検証`、`別 PR で修正`、`flaky but passed` を Done receipt に残す | merge 不可 |
| failure record が PR description のみで phase evidence にない | 仕様として扱わない |
| resume point が不明なまま中断・引継ぎ・再開する | 実装再開禁止 |
| failure closure と `P{phase}-PRECISION-CLOSURE` の open count が一致しない | Phase 未完了 |

### 9.11.5 Phase 1〜19 review handoff / third-party reproducibility 最低表

本節は Phase 実装 PR を実装者以外が再現レビューするための最低条件を定義する。実装者は対象 Phase の review handoff packet を作成し、第三者レビュアーが仕様本文、Phase packet、artifact、再現 command だけで Done / Not Done / Spec correction required を判定できる状態にしなければならない。PR description、チャット履歴、口頭説明、実装者の記憶、実装者固有のローカル path、未共有 secret を判定材料にしてはならない。

| Phase | third-party reproducibility 最低対象 |
|-------|--------------------------------------|
| 1 | clean checkout build、CLI help、invalid flag、config precedence、stub token、no persistence evidence |
| 2 | data-dir init、process lock、metadata 初期化、default DB open、WAL / integrity、restart、no HTTP evidence |
| 3 | `GET /v2/health`、`POST /v2/pipeline`、hrana wire snapshot、SQL error、close behavior、restart、SDK smoke |
| 4 | auth enabled / disabled、Bearer strictness、JWT claim、ro/rw permission、token create、revoke persistence、secret redaction |
| 5 | JSONL log parse、request_id、SDK CRUD transcript、restart persistence、unsupported future surface、secret scan、Phase 1〜4 regression |
| 6 | default route、path DB route、DB name validation、isolation、metadata / directory consistency、restart、Phase 1〜5 regression |
| 7 | Admin auth、DB CRUD、token CRUD、DB scope JWT、revoke immediate effect、metadata concurrency、Phase 1〜6 regression |
| 8 | legacy metadata migration、organization / group / location、quota / usage、Turso Platform API wrapper、scope precedence、snapshot、secret scan |
| 9 | WebSocket upgrade、hello ordering、stream lifecycle、interactive transaction、rollback、store_sql scope、SDK WS transcript |
| 10 | managed ATTACH allow / deny、arbitrary path rejection、source / target scope、metrics counter、WebSocket gauge、secret redaction |
| 11 | primary role startup、replication auth、SSE log stream、snapshot consistency、heartbeat、status、frame_no / checksum |
| 12 | primary + 2 replicas、snapshot bootstrap、WAL catch-up、write redirect、primary down、checksum mismatch、restart |
| 13 | archive manifest、frame write、restart check、retention cleanup、partial write recovery、corrupt / orphan handling、disabled mode |
| 14 | backup consistency、restore rollback、PITR success、PITR corrupt / outside range、startup recovery、policy precedence |
| 15 | current branch、PITR branch、branch delete recovery、routing isolation、source delete denial、restart、seed compatibility |
| 16 | extension register、allowlist / sha256、load / unload / delete、restart、path / symlink rejection、SQL bypass rejection |
| 17 | metrics snapshot restore、Prometheus output、usage / quota boundary、flush race、corrupt / future snapshot recovery、label redaction |
| 18 | promote、demote、redirect、leader unknown、split-brain rejection、restart primary state、network partition、Phase 1〜17 regression |
| 19 | config flags、shadow mode diff、active mode gate、rollback flag、SDK transcript、performance baseline、crash recovery、Phase 1〜18 regression |

**review handoff packet 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `handoff_id` | `HANDOFF-P{phase}-{number}` 形式の一意 ID |
| `spec_version` | 対象仕様書 version。Phase Done 時点の `V.{累積番号}` と一致させる |
| `reading_order` | レビュアーが読む順番。最低でも §0、§1.4、§9.1.51、§9.11.1〜§9.11.13、対象 Phase 詳細節、Phase packet、Done receipt |
| `reproduction_commands` | clean checkout から実行できる command。各 command は working directory、env、fixture、expected exit code を持つ |
| `expected_artifacts` | command ごとの生成 artifact path、Contract ID、Scenario ID、snapshot / transcript / log の対応 |
| `decision_criteria` | Done / Not Done / Spec correction required の判定条件。失敗時に参照する仕様節を含める |
| `failure_classification` | §9.11.4 の `failure_class` と failure record 作成条件 |
| `oral_context_free_evidence` | PR description、チャット履歴、口頭補足なしで判定できる証跡一覧 |
| `reviewer_result` | 第三者が実行した command、exit code、artifact、判定、差分有無、再実行要否 |

**review handoff artifact path 固定規則：**

| Artifact | Path | 必須内容 |
|----------|------|----------|
| handoff document | `docs/phase-evidence/phase-{phase}/review-handoff.md` | reading order、再現 command、判定基準、禁止事項、artifact index |
| handoff result | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/review-handoff.json` | handoff ID、実行 command、exit code、artifact、reviewer result、open item count |
| handoff transcript | `tests/artifacts/phase-{phase}/P{phase}-PRECISION-CLOSURE/review-handoff.txt` | 第三者実行ログ、stdout/stderr summary、secret redaction 済み transcript |

**review handoff 判定規則：**

| 状態 | 判定 |
|------|------|
| 対象 Phase の review handoff packet が存在しない | Phase 未完了 |
| 本節の最低対象の一部が `N/A` で、仕様本文に根拠がない | Phase 未完了 |
| 再現 command に実装者固有の absolute path、未共有 secret、手動 GUI 操作、口頭手順が必要 | merge 不可 |
| expected exit code、expected artifact path、decision criteria のいずれかが欠けている | Phase 未完了 |
| reviewer が PR description、チャット履歴、口頭説明を読まないと判定できない | Phase 未完了 |
| `reviewer_result` に open item、unknown、not reproduced、manual only が残る | merge 不可 |
| `P{phase}-PRECISION-CLOSURE` が review handoff result を参照しない | Phase 未完了 |

### 9.11.6 Phase 1〜19 N/A / manual exception / skip closure 最低表

本節は Phase 実装 PR で `N/A`、`not_applicable`、`manual only`、`skip`、`環境都合で未実行` を使う場合の最低閉鎖条件を定義する。実装者は対象 Phase の必須 contract、scenario、verification、artifact を省略してはならない。省略が許されるのは、仕様本文で対象外と固定済み、または該当 Phase では到達不能であることを証跡付きで示せる場合だけである。

| Phase | N/A / skip closure 最低対象 |
|-------|-----------------------------|
| 1 | DB / HTTP / JWT / metadata 永続化を N/A にする根拠、CLI / config / no persistence の manual only 禁止 |
| 2 | HTTP / JWT / multi DB を N/A にする根拠、data-dir / lock / metadata / restart 検証の skip 禁止 |
| 3 | JWT / WebSocket / Admin API / path DB route を N/A にする根拠、hrana HTTP / SDK smoke の manual only 禁止 |
| 4 | Admin API / DB scope / WebSocket を N/A にする根拠、auth / permission / redaction 検証の skip 禁止 |
| 5 | 新規 API / metadata 変更を N/A にする根拠、SDK / log / restart / secret scan の manual only 禁止 |
| 6 | 管理 API / Turso API / WebSocket を N/A にする根拠、routing / isolation / metadata restart の skip 禁止 |
| 7 | Turso Platform API / organization / quota を N/A にする根拠、Admin auth / scope / revoke / concurrency の skip 禁止 |
| 8 | WebSocket / ATTACH / replication / backup を N/A にする根拠、Turso wrapper / migration / quota / secret scan の manual only 禁止 |
| 9 | ATTACH / metrics / replication を N/A にする根拠、WebSocket transaction / rollback / SDK WS の skip 禁止 |
| 10 | replication / backup / Prometheus を N/A にする根拠、ATTACH denial / path rejection / metrics counter の skip 禁止 |
| 11 | replica apply / HA / backup を N/A にする根拠、primary replication API / snapshot / checksum の manual only 禁止 |
| 12 | archive / PITR / branch / HA を N/A にする根拠、primary + replicas / redirect / checksum mismatch の skip 禁止 |
| 13 | restore / branch / extension を N/A にする根拠、archive manifest / retention / corruption / restart の skip 禁止 |
| 14 | branch / extension / HA を N/A にする根拠、backup / restore rollback / PITR / startup recovery の manual only 禁止 |
| 15 | merge / diff / COW / extension を N/A にする根拠、branch lifecycle / source delete denial / seed compatibility の skip 禁止 |
| 16 | upload / Wasm / runtime global load を N/A にする根拠、extension allowlist / path rejection / SQL bypass の skip 禁止 |
| 17 | alerting / remote write / HA を N/A にする根拠、metrics snapshot / Prometheus / quota / redaction の manual only 禁止 |
| 18 | multi-primary / internal adapter を N/A にする根拠、promote / demote / split-brain / partition / restart の skip 禁止 |
| 19 | 外部 API 変更 / metadata migration を N/A にする根拠、shadow / active / rollback / SDK / performance / crash recovery の skip 禁止 |

**N/A record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `na_id` | `NA-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 対象 Phase 番号 |
| `target_contract` | N/A にする Contract ID、Scenario ID、verification command、artifact のいずれか |
| `reason_section` | N/A の根拠となる仕様本文 section。PR description だけの理由は禁止 |
| `why_not_required` | 対象外、未来 Phase、到達不能、別 contract で完全代替のいずれかを選び、理由を 1 つに固定する |
| `risk_if_wrong` | N/A 判断が誤っていた場合の互換、永続化、security、運用、データ損失リスク |
| `alternative_evidence` | 代替 test、snapshot、artifact、または到達不能を証明する unsupported / denial artifact |
| `reopen_trigger` | Turso Cloud / SDK 変更、Phase scope 変更、metadata schema 変更、error/status 変更、operator behavior 変更など |
| `reviewer_result` | 第三者が N/A 根拠、代替証跡、open item 0 件を確認した結果 |

**manual exception / skip 固定規則：**

| 対象 | 固定仕様 |
|------|----------|
| `manual only` | 自動化できない外部サービス状態の補助証跡に限る。Phase 完了の主証跡にはできない |
| `skip` | 仕様本文に skip 条件、代替 artifact、再実行条件、reopen trigger がある場合だけ許可する |
| `not_applicable` | `NA-P{phase}-{surface}-{number}` と仕様本文 section を持たない場合は無効 |
| `environment unavailable` | CI / Docker / local のいずれかで代替 command が固定されていない限り N/A 理由にできない |
| `future phase` | 未来 Phase の節番号、拒否 status/body、unsupported scenario artifact を必ず示す |

**N/A / manual / skip 判定規則：**

| 状態 | 判定 |
|------|------|
| 仕様本文根拠なしに `N/A`、`not_applicable`、`skip` を使う | Phase 未完了 |
| regression、security、persistence、compatibility、redaction、rollback を manual only で pass にする | merge 不可 |
| `後で検証`、`環境がない`、`時間がない`、`今回は不要`、`実装者判断` を N/A 理由にする | merge 不可 |
| skip した verification に代替 artifact または unsupported / denial artifact がない | Phase 未完了 |
| N/A record が Done receipt、Phase packet、review handoff、precision closure と相互参照できない | Phase 未完了 |
| `P{phase}-PRECISION-CLOSURE` に `open_na_without_spec_reason = 0`、`manual_only_pass = 0`、`skipped_required_verification = 0` がない | Phase 未完了 |

### 9.11.7 Phase 1〜19 go / no-go closure 最低表

本節は Phase 実装開始、Phase 完了、merge、release / rollout の Go / No-Go 判定を固定する。実装者は Phase packet、artifact、Done receipt、review handoff、precision closure の各証跡を集めたうえで、対象 Phase の `go_no_go_record` を作成しなければならない。Go / No-Go 判定は「実装者の感触」ではなく、open item が 0 であることと再現可能な証跡で決める。

| Phase | entry Go 最低条件 | exit Go 最低条件 | No-Go 最低条件 |
|-------|-------------------|------------------|----------------|
| 1 | CLI / config / no persistence の scope と対象外が固定済み | build、help、invalid flag、config precedence、no persistence が再現済み | DB / HTTP / JWT / metadata 永続化が成功応答を持つ |
| 2 | data-dir、lock、metadata、default DB の contract が固定済み | init、lock、WAL、integrity、restart、no HTTP が再現済み | partial init、lock bypass、restart mismatch、HTTP route success |
| 3 | hrana HTTP endpoint、wire schema、SDK smoke の oracle が固定済み | health、pipeline、SQL error、restart、SDK transcript が再現済み | JWT / WebSocket / Admin API success、wire snapshot drift |
| 4 | JWT secret、claim、permission、token create、redaction が固定済み | auth matrix、permission matrix、revoke、secret scan が再現済み | auth bypass、scope mismatch、token / secret leak |
| 5 | log、SDK、restart、unsupported surface、regression set が固定済み | JSONL、SDK CRUD、restart、secret scan、Phase 1〜4 regression が再現済み | unsupported future surface success、flaky / manual only pass |
| 6 | DB routing、name validation、isolation、metadata migration が固定済み | default/path route、isolation、metadata consistency、restart が再現済み | DB 混線、metadata/directory 不整合、未来 API success |
| 7 | Admin auth、DB CRUD、token CRUD、DB scope、revoke が固定済み | admin matrix、scope matrix、revoke immediate、concurrency が再現済み | admin bypass、token JWT leak、lost update |
| 8 | Turso model、org/group/location/quota/usage、migration が固定済み | migration、Turso wrapper、quota、snapshot、secret scan が再現済み | Turso snapshot drift、legacy migration failure、quota bypass |
| 9 | WebSocket protocol、stream、transaction、rollback、SDK WS が固定済み | upgrade、hello、tx commit/rollback、store_sql、SDK WS が再現済み | stream state leak、rollback failure、SDK WS drift |
| 10 | ATTACH policy、path rejection、metrics counter、auth が固定済み | ATTACH allow/deny、path rejection、metrics、redaction が再現済み | arbitrary path success、counter drift、scope bypass |
| 11 | primary role、replication auth、frame/checksum、snapshot が固定済み | primary startup、SSE、snapshot、heartbeat、status が再現済み | replication auth bypass、checksum drift、snapshot inconsistency |
| 12 | replica state、catch-up、redirect、primary down、multi replica が固定済み | primary + 2 replica、redirect、restart、checksum mismatch が再現済み | replica corruption、primary down ambiguity、redirect drift |
| 13 | archive manifest、frame write、retention、corruption handling が固定済み | archive write、restart、retention cleanup、corruption、disabled mode が再現済み | manifest/file inconsistency、unsafe delete、corrupt file success |
| 14 | backup、restore temp、rollback、PITR、startup recovery が固定済み | backup、restore rollback、PITR、corrupt/range outside、startup recovery が再現済み | partial commit、rollback 不能隠蔽、restore-failed 未処理 |
| 15 | branch metadata、source selector、delete、routing、seed が固定済み | current/PITR branch、delete recovery、restart、source delete denial が再現済み | branch/source 混線、delete partial failure、seed drift |
| 16 | extension allowlist、sha256、path、load timing、SQL bypass が固定済み | register/load/delete/restart/path rejection/SQL rejection が再現済み | symlink/path bypass、sha mismatch success、SQL direct load |
| 17 | metrics snapshot、Prometheus、usage/quota、redaction が固定済み | snapshot restore、Prometheus、quota boundary、corrupt recovery が再現済み | counter loss、format drift、label secret leak |
| 18 | HA token、term、promote/demote、split-brain、partition が固定済み | promote/demote/redirect/split-brain/restart/partition が再現済み | term regression、multi leader、primary write leak |
| 19 | adapter flags、shadow/active/rollback、SDK、performance、crash recovery が固定済み | shadow、active gate、rollback、SDK、performance、crash recovery が再現済み | wire/API drift、rollback migration required、performance threshold miss |

**go_no_go_record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `gate_id` | `GATE-P{phase}-{number}` 形式の一意 ID |
| `phase` | 対象 Phase 番号 |
| `entry_go` | 実装開始条件がすべて満たされている場合のみ `true` |
| `exit_go` | Done receipt、artifact、handoff、precision closure がすべて満たされている場合のみ `true` |
| `blocking_items` | open failure、open ambiguity、open N/A、missing artifact、manual only pass、regression skip、reviewer unresolved の件数 |
| `required_evidence` | Phase packet、manifest、scenario、artifact、failure closure、N/A closure、review handoff、Done receipt、precision closure の path |
| `reviewer_decision` | `Done`、`Not Done`、`Spec correction required` のいずれか |
| `release_decision` | `release_ready`、`phase_done_only`、`blocked` のいずれか。Phase Done と release ready を混同しない |

**Go / No-Go 判定規則：**

| 状態 | 判定 |
|------|------|
| `entry_go` が `true` でない | 実装開始禁止 |
| `exit_go` が `true` でない | Phase 未完了 |
| `blocking_items` が 1 件以上ある | merge 不可 |
| open failure、open ambiguity、根拠なし N/A、artifact 欠落、reviewer 再現不可、manual only 完了、regression 未実行が残る | No-Go |
| Turso Cloud / libSQL SDK 互換差分に仕様本文の理由、snapshot、client impact がない | No-Go |
| Phase Done だが rollout readiness、operator runbook、rollback、release note が未完了 | `phase_done_only`。運用投入不可 |
| `P{phase}-PRECISION-CLOSURE` に `entry_go = true`、`exit_go = true`、`blocking_items = 0`、`reviewer_decision = Done` がない | Phase 未完了 |

### 9.11.8 Phase 1〜19 regression inheritance / non-regression closure 最低表

本節は後続 Phase が過去 Phase の外部 contract、永続化、認証認可、SDK 互換、運用挙動を壊していないことを証明する最低条件を定義する。実装者は対象 Phase で追加した機能だけを検証して完了扱いにしてはならない。対象 Phase より前に Done となった Phase の regression は、仕様本文に根拠がある除外を除き、対象 Phase の Done receipt と `P{phase}-PRECISION-CLOSURE` に継承して記録する。

| Phase | inherited regression 最低対象 |
|-------|-------------------------------|
| 1 | なし。ただし CLI / config / no persistence の snapshot は以後の基準にする |
| 2 | Phase 1 CLI / config / no persistence |
| 3 | Phase 1 CLI / config、Phase 2 data-dir / lock / metadata / restart |
| 4 | Phase 1〜3 CLI、data-dir、hrana HTTP、SDK smoke、restart |
| 5 | Phase 1〜4 CLI、data-dir、hrana HTTP、auth disabled / enabled、secret redaction |
| 6 | Phase 1〜5 default route、SDK HTTP、auth、logs、restart、unsupported future surface |
| 7 | Phase 1〜6 default/path route、DB isolation、auth、SDK HTTP、metadata restart |
| 8 | Phase 1〜7 Admin API、DB CRUD、token CRUD、scope、legacy metadata、SDK HTTP |
| 9 | Phase 1〜8 HTTP route、Admin / Turso API、auth/scope/quota、SDK HTTP、metadata migration |
| 10 | Phase 1〜9 HTTP、WebSocket、transaction rollback、Admin / Turso API、secret redaction |
| 11 | Phase 1〜10 HTTP、WebSocket、ATTACH denial、metrics、Admin / Turso API、SDK transcript |
| 12 | Phase 1〜11 replication primary API、HTTP/WS SDK、Admin / Turso API、metrics、secret redaction |
| 13 | Phase 1〜12 primary/replica、redirect、checksum mismatch、HTTP/WS SDK、Admin / Turso API |
| 14 | Phase 1〜13 archive、replication、HTTP/WS SDK、Admin / Turso API、metadata / file consistency |
| 15 | Phase 1〜14 backup/restore/PITR、archive、replication、HTTP/WS SDK、Admin / Turso API |
| 16 | Phase 1〜15 branch、backup/restore/PITR、HTTP/WS SDK、Admin / Turso API、secret redaction |
| 17 | Phase 1〜16 extension、branch、backup/restore/PITR、metrics boundary、HTTP/WS SDK |
| 18 | Phase 1〜17 metrics、extension、branch、backup/restore/PITR、replication、HTTP/WS SDK |
| 19 | Phase 1〜18 full regression。CLI、config、metadata、HTTP、WebSocket、auth/scope/quota、Admin / Turso API、replication、archive、backup、branch、extension、metrics、HA、SDK transcript |

**regression_inheritance_record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `regression_id` | `REG-P{phase}-INHERIT-{number}` 形式の一意 ID |
| `phase` | 対象 Phase 番号 |
| `inherited_phases` | 継承対象 Phase 範囲。例: `1-8` |
| `required_surfaces` | CLI、config、data-dir、metadata、hrana HTTP、hrana WebSocket、auth/scope/quota、Admin / Turso API、replication、backup、branch、extension、metrics、HA、SDK transcript、secret redaction のうち該当面 |
| `commands` | clean checkout から実行できる regression command、expected exit code、timeout、Docker / CI / local 差分 |
| `artifact_paths` | regression transcript、snapshot、compat diff、secret scan、failure record、review handoff への path |
| `allowed_exclusions` | 仕様本文 section、N/A record、代替 artifact を持つ除外だけを列挙する。空の場合は `none` |
| `result` | `pass`、`fail`、`not_applicable_with_spec_reason` のいずれか。`not_run`、`manual_only`、`assumed_unaffected` は禁止 |

**Non-regression 判定規則：**

| 状態 | 判定 |
|------|------|
| 本節の inherited regression を実行していない | Phase 未完了 |
| `影響なし`、`コードを触っていない`、`たぶん関係ない` だけを理由に regression を skip する | merge 不可 |
| regression failure を別 PR、後続 Phase、既知問題として残す | merge 不可 |
| snapshot を更新しただけで compatibility diff、oracle 更新理由、client impact がない | Phase 未完了 |
| allowed exclusion に仕様本文 section、N/A record、代替 artifact がない | Phase 未完了 |
| secret redaction regression を省略する | merge 不可 |
| `P{phase}-PRECISION-CLOSURE` に inherited Phase 範囲、required surfaces、`regression_failure_count = 0`、artifact path がない | Phase 未完了 |

**canonical phase transition handoff audit：**

Phase N から Phase N+1 へ進む場合、対象 Phase の readiness packet は、直前 Phase の Done receipt と regression inheritance を参照した transition handoff を持たなければならない。transition handoff がない場合、後続 Phase は `ready_to_implement = true` にできない。Phase 1 は直前 Phase がないため `source_phase = none` とし、CLI / config / no persistence baseline を初期基準として記録する。

| transition field | 必須内容 |
|------------------|----------|
| `source_phase` | 継承元 Phase。通常は `Phase {N}`、Phase 1 のみ `none` |
| `target_phase` | 実装開始する Phase。通常は `Phase {N+1}` |
| `source_done_receipt` | 継承元 Phase の Done receipt path、commit SHA、PR 番号 |
| `source_phase_done_final_result` | 継承元 Phase の `phase_done_final_result`。Phase 1 以外は `pass` 必須 |
| `inherited_contract_ids` | 継承する API、persistence、security、compatibility、regression、precision closure の Contract ID |
| `inherited_artifacts` | 継承元の artifact path、expected hash、producer command、secret scan result |
| `inherited_regression_set` | §9.11.8 の inherited regression command、expected exit code、timeout、artifact path |
| `compatibility_baseline_snapshot` | Turso Cloud / libSQL SDK / previous Phase baseline の snapshot path、version、refresh trigger |
| `operator_delta_carryover` | 継承元の operator behavior delta、release note、rollback / monitoring / runbook の扱い |
| `known_blockers` | 後続 Phase へ持ち越す blocker。実装開始可能な場合は `none` |
| `transition_ready_result` | 下記 pass 条件をすべて満たす場合のみ `pass` |

`transition_ready_result = pass` にできるのは、Phase 1 以外では `source_phase_done_final_result = pass`、inherited regression 未実行 0 件、stale artifact 0 件、known blocker 0 件、compatibility baseline 未更新 0 件、operator delta 未継承 0 件、source Done receipt と target readiness packet の Contract ID / artifact path / regression set 不一致 0 件の場合だけである。`known_blockers` が `none` でない場合、または source Phase の Done receipt が古い仕様 version を参照している場合は、target Phase の実装開始を禁止する。

transition handoff を更新する場合は、target Phase の readiness packet、受入 manifest、regression inheritance record、compatibility baseline、review handoff、operator delta を同じ PR で更新する。source Phase の artifact を差し替えた場合は、target Phase の inherited artifact hash と regression command も同時更新しなければならない。

### 9.11.9 Phase 1〜19 deterministic execution / flaky prevention closure 最低表

本節は Phase 実装・検証・artifact 生成が実行環境、時刻、乱数、port、timeout、並行実行順、local path に依存して揺れないことを証明する最低条件を定義する。実装者は flaky を retry で隠してはならない。すべての snapshot、transcript、fixture、log、review handoff artifact は、正規化規則と deterministic な待機条件を持たなければならない。

| Phase | deterministic closure 最低対象 |
|-------|--------------------------------|
| 1 | CLI help / error snapshot、config precedence、stdout/stderr ordering、no persistence path normalization |
| 2 | data-dir temp path、process lock timing、metadata write order、WAL busy timeout、restart fixture |
| 3 | HTTP bind port、request id、health response、pipeline response order、SQL error snapshot、SDK smoke timeout |
| 4 | JWT token id / expiry、secret source precedence、auth denial log、permission matrix ordering、redaction snapshot |
| 5 | JSONL log field ordering、request_id normalization、SDK CRUD transcript、restart timing、secret scan output |
| 6 | DB name fixture、default/path route ordering、directory scan order、metadata list order、isolation transcript |
| 7 | Admin API list ordering、token create id normalization、revoke timing、concurrency race control、scope matrix |
| 8 | migration backup path、organization/group/location ordering、quota usage clock、Turso snapshot normalization |
| 9 | WebSocket connection id、stream id、transaction timing、rollback on disconnect、store_sql id normalization |
| 10 | ATTACH source ordering、path canonicalization、metrics counter increment order、WebSocket gauge timing |
| 11 | frame_no ordering、checksum fixture、SSE event order、snapshot timestamp、heartbeat interval / timeout |
| 12 | replica catch-up wait condition、redirect timing、primary down timeout、multi replica ordering、checksum mismatch artifact |
| 13 | archive frame filename order、manifest `created_at` clock、retention clock、partial write fixture、cleanup order |
| 14 | backup snapshot boundary、restore temp name、PITR selector clock、timeout / shutdown race、rollback artifact |
| 15 | branch internal id、source snapshot boundary、branch list order、delete race、seed compatibility snapshot |
| 16 | extension path normalization、sha256 fixture、load timing、existing connection non-retroactive check、delete ordering |
| 17 | metrics flush interval、shutdown flush race、counter monotonicity、Prometheus text ordering、label normalization |
| 18 | heartbeat clock、failover timeout、term ordering、promote/demote race、partition fixture、leader redirect timing |
| 19 | shadow diff ordering、adapter selection log、performance workload seed、crash recovery timing、rollback transcript |

**determinism_record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `determinism_id` | `DET-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 対象 Phase 番号 |
| `nondeterministic_inputs` | timestamp、random、port、host、absolute path、username、request id、connection id、thread scheduling、network timing など揺れる入力 |
| `normalization_rules` | artifact 上の置換規則。例: `<normalized-id>`、`<normalized-port>`、`<normalized-path>` |
| `fixed_seed_or_clock` | fixed seed、mock clock、manifest clock、fixture clock、または不要理由 |
| `timeout_policy` | timeout 値、待機条件、retry 禁止/許可条件、failure 時の error code / artifact |
| `race_control` | lock、event wait、barrier、deterministic scheduler、または race 対象外の仕様根拠 |
| `artifact_evidence` | snapshot、transcript、log、flaky report、environment.txt、ci.txt、review handoff path |
| `flaky_result` | `pass` または `fail`。`passed_after_retry`、`unknown`、`not_run` は禁止 |

**Deterministic / flaky 判定規則：**

| 状態 | 判定 |
|------|------|
| flaky を retry 成功だけで pass にする | merge 不可 |
| timestamp、random、local username、host、absolute path、ephemeral port が未正規化のまま artifact に残る | Phase 未完了 |
| fixed sleep だけで成立する race / timeout / heartbeat / replication / HA test | review failure |
| timeout 上限、待機条件、failure artifact が未定義 | Phase 未完了 |
| list / map / JSON field / Prometheus line / manifest entry の順序が未定義 | Phase 未完了 |
| flaky 原因、失敗ログ、deterministic 化修正、再実行 artifact が揃っていない | merge 不可 |
| `P{phase}-PRECISION-CLOSURE` に `flaky_count = 0`、`nondeterministic_artifact_count = 0`、`determinism_result = pass` がない | Phase 未完了 |

### 9.11.10 Phase 1〜19 assertion / oracle binding closure 最低表

本節は Phase 実装 PR の test、snapshot、fixture、transcript が、仕様上の oracle と具体的な assertion で結び付いていることを証明する最低条件を定義する。test が存在していても、重要 field を assert していない、expected と actual の対応がない、snapshot 更新だけで差分を吸収している場合は Phase 完了扱いにしない。

| Phase | assertion / oracle binding 最低対象 |
|-------|-------------------------------------|
| 1 | CLI exit code、stdout/stderr、help text、invalid flag error、config precedence、no persistence file count |
| 2 | data-dir layout、lock conflict status、metadata before/after、DB file/WAL existence、integrity result、restart result |
| 3 | HTTP status、Content-Type、hrana response schema、SQL error body、close behavior、SDK transcript |
| 4 | auth status/error code、Bearer header handling、JWT claim、ro/rw permission、token create output、redaction |
| 5 | JSONL field set、request_id、SDK CRUD result、restart data equality、unsupported response、secret scan result |
| 6 | route selected DB、default compatibility、DB name validation、isolation query result、metadata/directory consistency |
| 7 | Admin status/body、DB CRUD metadata、token CRUD response、DB scope denial、revoke effect、concurrency result |
| 8 | Turso wrapper schema、organization/group/location/quota fields、usage response、legacy migration artifact、scope/quota denial |
| 9 | WebSocket upgrade headers、message schema、stream state、transaction commit/rollback、store_sql behavior、SDK WS transcript |
| 10 | ATTACH allow/deny result、arbitrary path denial、metrics counter values、auth/scope denial、redaction |
| 11 | replication status/body、SSE event fields、frame_no/checksum、snapshot headers/body、heartbeat/status response |
| 12 | replica state before/after、redirect status/header/body、primary down behavior、checksum mismatch error、multi replica result |
| 13 | archive manifest fields、frame filename/checksum、retention deletion set、corrupt/orphan response、disabled mode response |
| 14 | backup body checksum、restore temp/commit/rollback trace、PITR selected frame、corrupt/range error、startup recovery |
| 15 | branch metadata fields、source selector、routing isolation query、delete recovery artifact、seed compatibility response |
| 16 | extension manifest fields、sha256 check、load/delete result、path/symlink denial、SQL bypass denial、restart state |
| 17 | metrics snapshot fields、Prometheus text lines、counter restore values、quota/usage consistency、label redaction |
| 18 | HA state fields、term monotonicity、promote/demote response、redirect behavior、split-brain denial、partition result |
| 19 | adapter flag parsing、shadow diff artifact、active gate assertion、rollback transcript、SDK compatibility、performance threshold |

**assertion_binding_record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `assertion_id` | `ASSERT-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 対象 Phase 番号 |
| `owner_contract` | 対応する Contract ID。複数 contract を曖昧にまとめない |
| `oracle_path` | expected snapshot、fixture、transcript、baseline、manifest など正解 artifact の path |
| `actual_artifact_path` | 実行結果 artifact の path。oracle と 1 対 1 または明示 matrix で対応させる |
| `asserted_fields` | status、code、headers、body schema、metadata、file state、log、metric、redaction など実際に assert する field |
| `normalization_rules` | 比較前に適用する正規化規則。正規化で意味差分を消してはならない |
| `comparison_command` | expected と actual を比較する command、expected exit code、差分保存先 |
| `result` | `pass` または `fail`。`not_asserted`、`snapshot_updated_only`、`manual_checked` は禁止 |

**Assertion / oracle binding 判定規則：**

| 状態 | 判定 |
|------|------|
| test は存在するが status だけ、または process exit code だけを assert している | Phase 未完了 |
| snapshot / fixture / expected を更新したが assertion ID、oracle path、comparison command を更新していない | merge 不可 |
| expected と actual の artifact path が対応しない | Phase 未完了 |
| error、auth、persistence、redaction、compatibility、rollback の assertion が欠落している | merge 不可 |
| `actual` をそのまま `expected` にコピーして oracle とする | merge 不可 |
| assertion failure を snapshot 更新で消し、仕様本文の差分理由を残さない | merge 不可 |
| `P{phase}-PRECISION-CLOSURE` に `missing_assertion_count = 0`、`oracle_binding_result = pass` がない | Phase 未完了 |

### 9.11.11 Phase 1〜19 negative surface / unsupported denial closure 最低表

本節は対象外、未来 Phase、unsupported、拒否系入力が success response、partial success、metadata 変更、file 変更、runtime map 変更を起こさないことを証明する最低条件を定義する。実装者は正常系 API だけで Phase 完了扱いにしてはならない。各 Phase は、その Phase で成功応答を許可しない surface を negative surface として固定し、拒否時の status / error / persistence no-op / redaction を artifact で証明する。

| Phase | negative surface / unsupported denial 最低対象 |
|-------|-----------------------------------------------|
| 1 | DB open、HTTP listen、JWT secret、metadata file、future serve subcommand、unknown CLI flag |
| 2 | HTTP route、JWT、multi DB route、Admin API、Turso API、invalid data-dir / lock bypass |
| 3 | JWT required mode、WebSocket route、Admin API、path DB route、unknown hrana body shape、unsupported SQL protocol feature |
| 4 | Admin API、DB scope JWT、WebSocket、Turso API、bad Bearer、expired/revoked token、ro write |
| 5 | future API、new metadata schema、unsupported route/config、secret in log/artifact、SDK unsupported mode |
| 6 | Admin API success、Turso API success、WebSocket success、invalid DB name、path traversal、default/path ambiguity |
| 7 | Turso Platform `/v1/*` success、organization/group/quota fields、WebSocket、backup、branch、invalid Admin body/query |
| 8 | WebSocket、ATTACH、replication、backup/restore、branch、extension、HA、unsupported Turso body feature / upload / seed |
| 9 | ATTACH、metrics Prometheus、replication、backup、branch、unsupported WebSocket message / ordering |
| 10 | replication、backup/restore、branch、Prometheus、arbitrary ATTACH path、scope/quota bypass |
| 11 | replica apply、write redirect、backup、branch、HA、bad replication token、invalid frame range |
| 12 | archive/PITR/branch/HA success、stale replica write、bad redirect target、checksum bypass |
| 13 | restore/PITR/branch/extension success、unsafe retention delete、corrupt archive success、disabled mode write |
| 14 | branch/extension/HA/internal adapter success、restore partial success、rollback failure hidden、invalid backup body |
| 15 | merge/diff/COW/extension success、source delete bypass、branch name collision、seed without branch selector |
| 16 | upload/Wasm/runtime global load success、symlink/path bypass、SQL direct load、default Turso mode pollution |
| 17 | alerting/remote write/HA success、unsupported Prometheus Accept、label secret exposure、counter mutation on denied request |
| 18 | multi-primary write、external consensus dependency、auto primary without operator、stale term promote、split-brain write |
| 19 | wire/API/schema/JWT claim change、SQL parser replacement success、metadata migration、rollback requiring data-dir edit |

**negative_surface_record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `negative_id` | `NEG-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 対象 Phase 番号 |
| `surface` | API path、WebSocket message、CLI flag、config key、SQL statement、filesystem path、background operation |
| `forbidden_input` | success response を許可しない request / command / config / SQL / path / body field |
| `expected_status_or_error` | HTTP status、hrana error、WebSocket close/error、CLI exit code、startup failure、health state |
| `persistence_noop` | 拒否時に metadata、DB file、WAL、archive、branch、extension、metrics、HA state が変化しない証跡 |
| `redaction_evidence` | token、body、SQL、raw path、upload binary、secret が log / artifact / error body に出ない証跡 |
| `artifact_path` | denial snapshot、before/after fixture、secret scan、review handoff artifact の path |
| `result` | `pass` または `fail`。`manual_only`、`not_run`、`accepted_risk` は禁止 |

**Negative surface 判定規則：**

| 状態 | 判定 |
|------|------|
| unsupported、対象外、未来 Phase surface が 2xx、success body、partial success、runtime active を返す | merge 不可 |
| 拒否時に metadata、file、runtime map、counter、token state、branch state が変化する | Phase 未完了 |
| 404、501、400、403、405、startup failure、WebSocket error の使い分けが仕様本文にない | 実装開始禁止 |
| denial artifact に token、request body、SQL args、raw path、upload binary、secret が残る | merge 不可 |
| negative scenario が normal scenario のみで代替されている | Phase 未完了 |
| future Phase に昇格する場合、Scope in/out、unsupported 表、Contract ID、oracle、artifact を同じ PR で更新していない | merge 不可 |
| `P{phase}-PRECISION-CLOSURE` に `negative_surface_failure_count = 0`、`unsupported_success_count = 0`、`denial_redaction_result = pass` がない | Phase 未完了 |

### 9.11.12 Phase 1〜19 evidence integrity / artifact manifest closure 最低表

本節は Phase 実装 PR の artifact が対象 Phase、Contract ID、生成 command、対象 commit、manifest、Done receipt と相互参照できることを証明する最低条件を定義する。古い artifact、別 commit の artifact、PR description だけの証跡、secret scan 対象外の artifact、Contract ID がない artifact を Phase 完了根拠にしてはならない。

| Phase | evidence integrity 最低対象 |
|-------|-----------------------------|
| 1 | CLI help/error snapshot、config precedence fixture、no persistence evidence、Phase packet、Done receipt |
| 2 | data-dir tree、lock fixture、metadata fixture、integrity/restart transcript、recovery log |
| 3 | HTTP request/response snapshot、hrana wire fixture、SQL error snapshot、SDK transcript |
| 4 | auth matrix、permission matrix、token create artifact、secret redaction scan、restart transcript |
| 5 | JSONL log artifact、SDK CRUD transcript、unsupported surface snapshot、Phase 1〜4 regression output |
| 6 | routing snapshot、DB isolation transcript、metadata/directory fixture、Phase 1〜5 regression output |
| 7 | Admin API snapshot、token CRUD artifact、scope/revoke matrix、concurrency artifact、Phase 1〜6 regression output |
| 8 | Turso Platform snapshot、legacy migration fixture、quota/usage artifact、compat diff、secret scan |
| 9 | WebSocket transcript、transaction rollback artifact、store_sql snapshot、SDK WS transcript、secret scan |
| 10 | ATTACH allow/deny fixture、path rejection snapshot、metrics counter artifact、redaction scan |
| 11 | replication SSE transcript、snapshot artifact、frame/checksum fixture、heartbeat/status snapshot |
| 12 | replica state fixture、redirect snapshot、primary down artifact、checksum mismatch transcript |
| 13 | archive manifest、frame/snapshot checksum artifact、retention cleanup log、corruption fixture |
| 14 | backup artifact、restore rollback log、PITR replay fixture、startup recovery log、secret scan |
| 15 | branch metadata fixture、create/delete recovery log、routing isolation transcript、seed compatibility snapshot |
| 16 | extension manifest、sha256 fixture、load/delete transcript、path/symlink denial snapshot、SQL rejection artifact |
| 17 | metrics snapshot、Prometheus text artifact、counter restore fixture、quota/usage diff、label redaction scan |
| 18 | HA state fixture、term/promotion log、split-brain artifact、partition transcript、restart recovery log |
| 19 | shadow diff artifact、adapter selection log、rollback transcript、SDK compatibility transcript、performance baseline |

**evidence_integrity_record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `evidence_id` | `EVID-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 対象 Phase 番号 |
| `commit_sha` | artifact を生成した commit SHA。Done receipt の commit SHA と一致させる |
| `contract_id` | artifact が証明する Contract ID。複数 contract を含む場合は matrix で明示する |
| `artifact_path` | repository 内の deterministic path。timestamp、random、local username、host 名、absolute path を含めない |
| `artifact_kind` | snapshot、fixture、transcript、ci、release-check、secret-scan、compat-diff、recovery-log、performance-baseline、handoff のいずれか |
| `generated_by_command` | artifact を生成した command、expected exit code、environment、normalization step |
| `normalized` | `true` または `false`。`false` の場合は repository に保存不可 |
| `secret_scan_result` | artifact 全体を対象にした secret scan の result と path |
| `stale_check_result` | manifest、Done receipt、Contract ID、commit SHA、artifact mtime ではなく内容対応で stale でないことを示す結果 |

**Evidence integrity 判定規則：**

| 状態 | 判定 |
|------|------|
| artifact の `commit_sha` が Done receipt / PR commit と一致しない | Phase 未完了 |
| manifest、Done receipt、Phase packet、artifact path、Contract ID が相互参照できない | merge 不可 |
| stale artifact、過去 Phase artifact、別 branch artifact を再利用して pass にする | merge 不可 |
| artifact path または artifact 内容に Contract ID がない | Phase 未完了 |
| secret scan が artifact 全体ではなく一部 file だけを対象にしている | merge 不可 |
| raw artifact を repository に保存する、または normalization 前後の対応がない | merge 不可 |
| `P{phase}-PRECISION-CLOSURE` に `stale_artifact_count = 0`、`missing_evidence_integrity_count = 0`、`artifact_manifest_result = pass` がない | Phase 未完了 |

### 9.11.13 Phase 1〜19 operator action / observability closure 最低表

本節は Phase 実装 PR で障害、劣化、復旧、rollback、operator_required が発生した時に、運用者が health、log、metric、artifact、runbook から状態と必要操作を判断できることを証明する最低条件を定義する。実装者は API が動くことだけで Phase 完了扱いにしてはならない。障害が silent に隠れる、health が `ok` のまま、log / metric / operator action が未定義、または secret を含む運用証跡は Phase 完了不可である。

| Phase | operator action / observability 最低対象 |
|-------|------------------------------------------|
| 1 | CLI 起動失敗、config parse failure、bind failure、no persistence confirmation、stderr redaction |
| 2 | data-dir init failure、process lock conflict、metadata corruption、integrity_check failure、restart recovery |
| 3 | HTTP bind failure、malformed request、SQL error、shutdown timeout、health response |
| 4 | auth disabled warning、bad/expired/revoked token、permission denial、token persistence failure、secret redaction |
| 5 | JSONL log contract、SDK regression failure、restart data loss、unsupported future surface hit、secret scan failure |
| 6 | DB not found、DB name invalid、metadata/directory mismatch、DB isolation violation、restart restore failure |
| 7 | Admin auth failure、token revoke failure、concurrent DB update conflict、scope denial、metadata lost update |
| 8 | migration failure、quota exceeded、usage unavailable、Turso unsupported endpoint、legacy fallback failure |
| 9 | WebSocket upgrade failure、transaction rollback failure、disconnect recovery、store_sql failure、SDK WS drift |
| 10 | ATTACH denial、arbitrary path attempt、metrics counter drift、scope/quota denial、WebSocket gauge leak |
| 11 | replication token denial、frame checksum mismatch、snapshot failure、heartbeat stale、primary role misconfig |
| 12 | replica lag、primary down、redirect unavailable、checksum mismatch、replica state corruption |
| 13 | archive manifest mismatch、missing/corrupt frame、retention cleanup failure、disabled mode write attempt |
| 14 | backup failure、restore rollback、PITR range/corruption、restore-failed marker、startup recovery |
| 15 | branch create/delete failure、source delete denial、branch route recovery、seed compatibility failure |
| 16 | extension load_failed、sha256 mismatch、missing binary、path/symlink denial、SQL bypass attempt |
| 17 | metrics snapshot corruption、flush failure、Prometheus render failure、invalid sample、quota/usage mismatch |
| 18 | leader unknown、candidate state、promotion failure、demotion failure、split-brain、network partition |
| 19 | shadow diff、active mode failure、rollback failure、performance threshold miss、adapter crash recovery |

**operator_action_record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `operator_id` | `OP-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 対象 Phase 番号 |
| `trigger_condition` | operator action または observability が必要になる条件 |
| `observable_signal` | API response、health、log、metric、artifact、startup failure のどれで検出できるか |
| `health_state` | `ok`、`degraded`、`unavailable`、`recovering`、`rollback_required`、`operator_required`、`blocked` のいずれか |
| `log_event` | log level、event name、必須 field、禁止 field、redaction rule |
| `metric_or_counter` | metric / counter / gauge / Prometheus sample。未提供の場合は仕様本文の不要理由 |
| `operator_action` | retry、restart、rollback、manual cleanup、config change、token rotate、restore、promote/demote、none のいずれか |
| `recovery_boundary` | 自動復旧してよい範囲、operator 承認が必要な境界、success response 禁止条件 |
| `evidence_path` | health snapshot、log artifact、metric snapshot、runbook、review handoff artifact の path |

**Operator / observability 判定規則：**

| 状態 | 判定 |
|------|------|
| `operator_required`、`rollback_required`、`degraded`、`unavailable` が health / log / metric / artifact のいずれにも出ない | Phase 未完了 |
| 自動復旧してよい条件と禁止条件が仕様本文にない | 実装開始禁止 |
| rollback、restart、manual cleanup、promote/demote、token rotate の手順がない破壊的操作を公開する | merge 不可 |
| operator log、release note、runbook、artifact に token、JWT、SQL args、backup body、absolute path secret が残る | merge 不可 |
| ERROR / WARN log があるが operator action、client action、retry 可否が不明 | Phase 未完了 |
| Phase Done だが release note / behavior delta / operator runbook が未完了 | `phase_done_only`。運用投入不可 |
| `P{phase}-PRECISION-CLOSURE` に `operator_action_gap_count = 0`、`observability_result = pass`、`operator_secret_leak_count = 0` がない | Phase 未完了 |

### 9.12 PR レビュー観点

PR レビューでは以下を必ず確認する。該当しない項目は PR description に `N/A` と理由を書く。

| 観点 | 確認内容 |
|------|----------|
| Phase 境界 | その PR が対象 Phase のスコープ内か。対象外機能を成功応答付きで公開していないか |
| API 契約 | method/path/auth/status/body/error が §9.5 と一致しているか |
| 永続化 | metadata と file の更新順、atomic update、fsync、rollback が §9.6 と一致しているか |
| エラー | 仕様済み error code を使っているか。`INTERNAL_ERROR` で隠していないか |
| 認証/認可 | JWT/Admin/replication token の境界が正しいか。ro/rw/DB scope が正しいか |
| ログ/秘匿 | secret/token/SQL args/backup contents をログに出していないか |
| 再起動互換 | 既存 metadata で起動できるか。migration が必要なら仕様化されているか |
| 並行性 | concurrent request、DB lock、shutdown 中 request の挙動が決まっているか |
| テスト | §9.8 の該当列を満たしているか。regression target が落ちていないか |
| 後方互換 | 既存 endpoint、metadata、config、SDK 互換を壊していないか |

### 9.13 設定値契約表

| 設定 | CLI | Env | TOML | Default | Phase | 不正値時 |
|------|-----|-----|------|---------|-------|----------|
| data dir | `--data` | なし | 書かない | 必須 | 1 | clap error / 起動失敗 |
| API port | `--port` | なし | `[server] port` | `8080` | 1 | 起動失敗 |
| admin port | `--admin-port` | なし | `[server] admin_port` | `8081` | 1 | 起動失敗 |
| config path | `--config` | なし | n/a | `{data}/config.toml` | 1 | 読み込み/parse 失敗で起動失敗 |
| JWT secret | `--auth-jwt-secret` | `ADLAIRE_JWT_SECRET` | `[auth] jwt_secret` | 認証無効 | 4 | 32 bytes 未満は起動失敗 |
| JWT secret file | `--auth-jwt-secret-file` | なし | `[auth] jwt_secret_file` | なし | 4 | 読み込み失敗/短すぎは起動失敗 |
| admin token | `--admin-auth-token` | `ADLAIRE_ADMIN_TOKEN` | `[admin] auth_token` | 管理 API 認証無効 | 7 | 空文字は未指定扱い |
| log level | `--log-level` | `ADLAIRE_LOG_LEVEL` | `[server] log_level` | `info` | 5 | 不正値は起動失敗を原則とする。fallback する場合は WARN 必須 |
| skip integrity | `--skip-integrity-check` | なし | `[storage] skip_integrity_check` | `false` | 2 | boolean parse 失敗で起動失敗 |
| busy timeout | `--busy-timeout` | なし | `[server] busy_timeout_ms` | `5000` | 2 | `0` は許可しない。起動失敗 |
| shutdown timeout | `--shutdown-timeout` | なし | `[server] shutdown_timeout` | `30` | 3 | `0` は即時 abort として明記しない限り起動失敗 |
| replication role | `--role` | なし | なし | `standalone` | 11 | unknown role は起動失敗 |
| primary port | `--primary-port` | なし | なし | `8082` | 11 | bind 失敗で起動失敗 |
| primary URL | `--primary-url` | なし | なし | replica では必須 | 12 | replica で未指定/parse 失敗なら起動失敗 |
| replication token | `--replication-auth-token` | なし | なし | なし | 11 | required mode で未指定なら起動失敗 |
| replication write mode | `--replication-write-mode` | なし | `[replication] write_mode` | `async` | 11 | unknown は起動失敗。未定義 sync semantics は起動失敗 |
| HA node id | `--ha-node-id` | なし | `[ha] node_id` | `standalone` | 18 | validation 失敗なら起動失敗 |
| HA token | `--ha-token` | `ADLAIRE_HA_TOKEN` | `[ha] token` | なし | 18 | HA API 有効時に未指定なら起動失敗 |
| HA failover timeout | `--ha-failover-timeout-ms` | なし | `[ha] failover_timeout_ms` | `10000` | 18 | `1000` 未満は起動失敗 |
| internal WAL | `--internal-wal` | なし | `[internal] wal` | `libsql` | 19 | `libsql` / `adlaire` 以外は起動失敗 |
| internal storage | `--internal-storage` | なし | `[internal] storage` | `libsql` | 19 | `libsql` / `adlaire-readonly` 以外は起動失敗 |
| internal executor | `--internal-executor` | なし | `[internal] executor` | `libsql` | 19 | `libsql` / `adlaire-adapter` 以外は起動失敗 |
| WAL mode | なし | なし | `[storage] wal_mode` | `passive` | 2 | unknown は起動失敗 |
| WAL retention | なし | なし | `[storage] wal_retention_days` | `0` | 13 | parse 失敗で起動失敗 |
| replication sync timeout | なし | なし | `[replication] sync_timeout_ms` | `5000` | 11 | `0` は起動失敗 |

#### 9.13.1 対象 Phase 前の設定値固定契約

設定値契約表に存在するが、現在の実装 Phase より後の Phase に属する設定値は、先取り実装の根拠にしてはならない。対象 Phase 前に指定された場合の挙動は下表を正とする。

| 設定分類 | 対象 Phase 前に指定された場合 | 理由 |
|----------|------------------------------|------|
| security / auth / HA / replication token | 起動失敗 | secret を受理して未使用にすると運用者が保護済みと誤認するため |
| persistence / WAL / restore / branch / archive | 起動失敗 | 値を受理して未適用にすると durability と recovery の保証が曖昧になるため |
| observability / log output / metrics | 明示 WARN を出して無効化。ただし仕様表に WARN 可と書かれている場合だけ | 監視値は data path を変えないが、silent ignore は禁止 |
| internal adapter | 起動失敗 | 互換性差分と rollback flag が未定義の状態で production path を変えないため |
| commented sample key | 指定不可。TOML に実値として書かれた場合は上記分類に従う | sample コメントは実装許可ではないため |

対象 Phase 前に設定値を受け取っても default と同じ挙動で silent ignore してはならない。起動失敗時は `INVALID_CONFIG` 相当の起動エラーとして扱い、HTTP error code には変換しない。WARN 許可の設定でも、log には key 名、対象 Phase、無効化理由だけを出し、secret 値や path の絶対値は出力しない。

対象 Phase 到達時は、§9.13 の default、不正値、優先順位、§9.1.6 の Phase 別証跡を満たすまで完了扱いにしない。設定値を実装した PR は、指定あり/なし、不正値、対象 Phase 前 fixture、config/env/CLI 優先順位のテストを必須とする。

### 9.14 セキュリティ境界表

| 境界 | 信頼しない入力 | 必須対策 | 禁止事項 |
|------|----------------|----------|----------|
| public HTTP | method/path/header/body | size limit、JSON validation、auth、timeout | panic、secret log、silent fallback |
| hrana SQL | SQL text、args、named_args | parameter conversion、write permission、ATTACH interception | SQL split、任意 path open |
| admin API | path name、JSON body、admin token | Bearer 完全一致、DB name validation、atomic update | token value の再表示、auth bypass |
| Turso Platform API | organizationSlug、databaseName、groupName、query、platform token | Bearer 完全一致、Turso name validation、wrapper snapshot、unsupported API 分類 | field casing 変更、success status 変更、body/token log |
| WebSocket | upgrade headers、frames、stream_id | hello auth、message size limit、stream lifecycle validation | hello 前 request 処理、open tx 放置 |
| replication API | token、frame_no、frame bytes | replication token、CRC32、range validation | checksum 無視、unauthenticated stream |
| extension | extension name/version/sha256/filename | allowlist、sha256、固定 directory、manifest validation | 任意 path load、SQL からの直接 load、未署名拡張 |
| HA | HA token、node_id、term、leader_id | HA token、term 単調増加、operator promotion、split-brain rejection | unauthenticated promotion、自動 primary 昇格、古い term の採用 |
| internal switch | internal flags、adapter output | 起動時 validation、rollback flag、互換 snapshot test | silent fallback、metadata migration、wire/API 差分 |
| filesystem | metadata JSON、DB files、archive files | canonical data_dir join、atomic update、integrity check | path traversal、自動上書き修復 |
| backup/restore | uploaded DB、PITR selector | temp restore、integrity_check、rollback | 元 DB の直接上書き、失敗後の不整合 |
| branch | branch name、source frame/time | DB name validation、source existence check | `___` 含有名、metadata 先行 commit |
| config/env | TOML/env/CLI values | priority rule、type validation、secret length check | invalid default fallback、secret logging |

### 9.15 互換性ルール

- 互換性判断で迷う場合は §1.5 の優先順位を正とし、Turso Cloud 互換と libSQL SDK 互換を内製化都合より優先する
- hrana-http v2 と hrana-ws v3 の wire format は後方互換を維持する
- `/v2/pipeline` は Phase 6 以降も常に `default` DB を対象とする
- 新 field を response に追加する場合は、既存 field を削除・rename しない
- metadata JSON に field を追加する場合は、古い field を読み飛ばせるようにし、既存 file の migration path を仕様化する
- config の default 値を変更する場合は、migration note と regression test を追加する
- error `code` を変更してはならない。message は詳細化してよいが、client が code で分岐できる状態を維持する
- TypeScript `@libsql/client` 互換は Phase 5 以降の regression target とする
- backup/restore/PITR/branch のファイル形式を変える場合は、旧形式読み込み可否と不可の場合の明示エラーを仕様化する
- Adlaire 独自拡張を追加する場合は、Turso 互換 mode の response、metadata、error、SDK 挙動に差分を出さない。差分が必要な場合は mode 分離、互換 snapshot、migration、rollback を仕様化してから実装する

**互換優先の merge 不可条件：**

| 状態 | 判定 |
|------|------|
| Turso 互換 mode の API path、method、request、response wrapper、field casing が仕様差分なしに変わる | merge 不可 |
| libSQL SDK regression が失敗し、差分理由と修正方針が Done receipt にない | merge 不可 |
| metadata schema の rename / delete / required field 追加に migration と rollback がない | merge 不可 |
| error code、HTTP status、retry policy が §7.3 と矛盾する | merge 不可 |
| Adlaire 拡張 mode の挙動が既定 mode に混入する | merge 不可 |
| 内製 crate への切り替えに shadow diff、rollback flag、compat snapshot がない | merge 不可 |
| Turso Cloud との差分を `INTERNAL_ERROR`、generic 500、またはログだけで隠す | Phase 未完了 |

### 9.16 実装順序ルール

各 Phase の実装は原則として次の順に行う。
詳細な step、handoff state、中断・再開、前倒し実装の扱いは §9.1.34 を正とする。

1. 仕様内の schema / error / persistence 契約を確定する
2. 永続化 schema と migration / recovery を実装する
3. core service logic を実装する
4. HTTP/WebSocket/CLI API を公開する
5. 正常系と異常系テストを追加する
6. restart / rollback / regression test を追加する
7. README や運用メモを更新する

**順序例外禁止：**

- 永続化 schema 未確定のまま API を先に公開しない
- rollback 方針未確定のまま破壊的 API を実装しない
- auth 方針未確定のまま管理 API / replication API を公開しない
- tests がない状態で Phase 完了扱いにしない

### 9.17 Phase 実装前チェックリスト

各 Phase の実装 PR は、コード変更前にこの表を満たしていることを確認する。1つでも `未定義` がある場合、その PR は実装 PR ではなく仕様修正 PR として扱う。

| 確認項目 | 必須状態 | 未定義時の扱い |
|----------|----------|----------------|
| Phase スコープ | 対象機能と対象外が §9.2 / §9.4 に明記され、resource identity / naming / path boundary が §9.1.26 に従って固定されている | 仕様追記まで実装しない |
| Phase packet | §9.1.32 に従い、scope、target surface、API、永続化、security、concurrency、compatibility、evidence、regression、unsupported behavior、completion gate が 1 セットで固定されている | 実装開始禁止 |
| Phase 受入 manifest | §9.1.10 の必須 fields が実装開始前に固定されている | 実装 PR として扱わない |
| Evidence artifact | §9.1.11 の保存先、命名、正規化、secret scan が固定されている | 証跡生成まで完了扱いにしない |
| Phase Done receipt | §9.1.33 に従い、packet、manifest、contract map、evidence index、regression result、failure closure、compatibility / redaction result が一致している | Phase 完了扱いにしない |
| Phase execution state | §9.1.34 に従い、current step、completed/open contracts、last command、allowed/forbidden changes、next command、blocking decision が明記されている | 中断・引継ぎ・再開を行わない |
| Acceptance oracle | §9.1.35 に従い、API/error/persistence/migration/SDK/unsupported/security/compatibility/regression の正解 artifact、正規化、更新条件が固定されている | snapshot / fixture / expected を更新しない |
| Defect classification | §9.1.36 に従い、spec gap、implementation bug、regression bug、compatibility diff、oracle gap、environment gap、security gap、persistence gap が分類され、`open:0` になっている | Phase 完了扱いにしない |
| Operational state / recovery runbook | §9.1.37 に従い、healthy、degraded、unavailable、recovering、rollback_required、operator_required、blocked の API / write / health / log / operator action が固定されている | failure path を実装しない |
| Dependency graph | §9.1.38 に従い、各 Contract ID の prerequisite、blocks、status、evidence、not_applicable reason が固定され、未完了 prerequisite が 0 件になっている | dependent contract を実装・公開・完了扱いにしない |
| Invariant ledger | §9.1.39 に従い、durability、metadata/file consistency、auth/scope/quota、compatibility、error surface、operational state、redaction、dependency/prerequisite の invariant と regression guard が固定され、violation が 0 件になっている | 実装開始禁止。違反がある場合は Phase 完了扱いにしない |
| Scenario matrix | §9.1.40 に従い、normal、invalid request、auth/scope/quota、persistence/restart、rollback/recovery、concurrency/idempotency、compatibility、unsupported、redaction、operational の scenario と evidence が固定され、manual only / not run / 根拠なし N/A が 0 件になっている | 実装開始禁止。ケース漏れがある場合は Phase 完了扱いにしない |
| Resource lifecycle | §9.1.41 に従い、resource type、state、allowed/forbidden transition、entry/exit condition、API behavior、write policy、commit order、recovery behavior、state evidence が固定されている | 実装開始禁止。状態遷移未定義または forbidden transition 未検証の場合は Phase 完了扱いにしない |
| Schema registry | §9.1.42 に従い、request、response、metadata、config、JWT claim、WebSocket message、artifact、log/metric の field-level schema、required/null/default/migration/compatibility/redaction が固定されている | 実装開始禁止。field 意味ズレ、根拠なし null/省略、migration 未定義の場合は Phase 完了扱いにしない |
| Decision precedence | §9.1.43 に従い、複数条件同時成立時の precedence、selected behavior、losing behavior、error/status/client action、compatibility 差分が固定されている | 実装開始禁止。分岐順の実装依存、存在漏洩、commit 後拒否、protocol 間不一致がある場合は Phase 完了扱いにしない |
| Coverage closure | §9.1.44 に従い、全 Contract ID / schema ID / scenario ID / decision ID が test、artifact、oracle、regression、N/A 理由へ対応している | 実装開始禁止。coverage gap、manual only pass、根拠なし N/A、旧 Phase regression 漏れがある場合は Phase 完了扱いにしない |
| Change impact / drift control | §9.1.45 に従い、実装中の scope、API、schema、error、metadata、auth、test、oracle、compatibility 変更が change ID、同時更新範囲、承認状態、closure evidence で閉じている | 実装開始禁止。packet freeze 後の暗黙変更、snapshot だけ更新、互換影響未評価、migration / rollback 未評価がある場合は Phase 完了扱いにしない |
| Rollout readiness | §9.1.46 に従い、startup、shutdown、restart、rollback、health、operator action、compatibility、data safety、blocked release reason が固定されている | release / deploy / production enable 禁止。Phase Done だけで rollout ready 扱い、rollback 未検証、operator_required 隠蔽、環境差分未記録の場合は運用投入不可 |
| Compatibility baseline | §9.1.47 に従い、Turso Cloud、libSQL SDK、hrana、legacy metadata、previous Phase の baseline、snapshot / SDK version、refresh trigger、差分分類、証跡が固定されている | 実装開始禁止。実装都合の snapshot 更新、SDK transcript 欠落、upstream 未確認、自己ホスト差分理由なし、古い baseline のまま Done / rollout ready は不可 |
| Security abuse / bypass resistance | §9.1.48 に従い、attack surface、untrusted input、required control、bypass attempt、expected denial、redaction、audit/log、quota/rate、persistence no-op が固定されている | 実装開始禁止。auth/scope/quota bypass、secret 漏洩、path traversal、commit 後拒否、replay 二重処理、manual only security case がある場合は Phase 完了扱いにしない |
| Ambiguity closure / implementation decision | §9.1.49 に従い、implementation question、candidate options、selected decision、rejected options、decision basis、affected contracts、edge cases、reopen trigger が固定されている | 実装開始禁止。TBD、実装判断、根拠なし N/A、open ambiguity、error/status/commit order/redaction の未決定がある場合は Phase 完了扱いにしない |
| Atomic implementation task ledger | §9.1.50 に従い、task ID、input contracts、change targets、forbidden changes、completion condition、verification command、rollback condition、dependency が固定されている | 実装開始禁止。巨大 task、task ID なし差分、検証なし task、task 外変更、rollback 未定義、open task がある場合は Phase 完了扱いにしない |
| Review handoff / independent reproducibility | §9.1.51 に従い、reading order、reproduction commands、expected artifacts、decision criteria、failure classification、oral context free evidence が固定されている | Phase 完了扱いにしない。口頭説明、PR description だけの根拠、local only 再現、artifact 欠落、レビュアー判断任せの N/A / snapshot 更新は禁止 |
| Operator-facing behavior delta | §9.1.52 に従い、audience、external surface、before/after、compatibility delta、operator action、migration/config、rollback、release note、evidence が固定されている | Phase 完了扱いにしない。release note なし、operator 影響未分類、互換差分未記載、運用手順なし、rollback 不明、log/health/metric sample 欠落は禁止 |
| Phase precision closure index | §9.11.1〜§9.11.2 に従い、対象 Phase の詳細節、固定契約、台帳、シナリオ、Done receipt、横断契約、regression、`P{phase}-PRECISION-CLOSURE`、artifact path が Phase packet で相互参照できる | 実装開始禁止。索引未転記、Contract ID 未接続、artifact path 未確定、Done receipt field との不一致がある場合は Phase 完了扱いにしない |
| Phase ambiguity / failure / handoff closure | §9.11.3〜§9.11.5 に従い、open ambiguity、open failure、known flaky、unverified item、review handoff 欠落が 0 件である | merge 不可。判断未確定、失敗未分類、flaky 放置、レビュアー再現不能、口頭説明依存がある場合は Phase 完了扱いにしない |
| Phase N/A / Go-No-Go / regression closure | §9.11.6〜§9.11.8 に従い、根拠なし N/A、blocking item、regression failure、過去 Phase regression 漏れが 0 件であり、Go/No-Go 判定と継承 regression が Done receipt に接続されている | Phase 未完了。N/A 理由不足、No-Go 未解消、旧 Phase 影響未検証、regression 継承漏れがある場合は実装完了扱いにしない |
| Phase deterministic / assertion / negative / evidence / operator closure | §9.11.9〜§9.11.13 に従い、deterministic でない検証、assertion 未接続、unsupported success、stale/missing artifact、operator action gap が 0 件である | Phase 未完了。flaky、oracle なし pass、negative surface 未否認、artifact manifest 不整合、log/health/metric/operator 手順欠落がある場合は完了扱いにしない |
| 仕様矛盾 | §9.1.12 の優先順位に従い、矛盾箇所が同じ PR で解消されている | 実装 PR として扱わない |
| 仕様内相互参照 | §9.1.29 に従い、Phase 番号、TC ID、Task ID、API 契約 ID、error code、persistence key、evidence 名、manifest 参照が一致している | 仕様修正 PR に戻す |
| Verification command | §9.1.13 / §9.1.24 の command 分類、順序、exit code、artifact、toolchain、Docker/CI/local 差分が固定されている | 検証完了扱いにしない |
| Failure closure | §9.1.14 の失敗、flaky、未検証、artifact 欠落、secret 混入が同一 PR で閉じている | merge 不可 |
| 後方互換 / migration | §9.1.15 の互換影響、migration plan、rollback、旧形式 fixture が固定されている | 既存契約を変更しない |
| 設定解決 / validation | §9.1.19 に従い、CLI/env/TOML/default/secret file の優先順位、不正値、対象 Phase 前挙動、秘匿が固定されている | config 実装を開始しない |
| API 契約 | §9.1.20 / §9.1.27 / §9.1.28 に従い、method/path/auth/request/success/error/schema/validation/serialization/list order/pagination/cursor/filtering/SQL result mapping が §9.5 または各 API 節に明記されている | route を追加しない |
| Error code | §9.1.22 に従い、失敗条件ごとの `code`、status、wire surface、retry、client action、precedence が §7.3 / §9.7 に存在する | 先に error code と retry 契約を追加する |
| 永続化 | §9.1.21 / §9.1.26 に従い、ファイル名、schema、atomic update、fsync、directory sync、rollback、破損時挙動、path normalization、recovery evidence が §9.6 に明記されている | 書き込み処理を実装しない |
| 認証/認可 | §9.1.18 に従い、必要 token、scope、ro/rw、org/group、quota、block policy、拒否条件 precedence が明記されている | success response を返す API を公開しない |
| Backup / restore / PITR | Phase 13〜15 の backup、restore、PITR、branch seed は §9.1.30 に従い、temp layout、lock、commit/rollback、checksum、quota/block precedence、recovery marker、redaction が固定されている | 破壊的 API を公開しない |
| Branch lifecycle | Phase 15 の branch create/delete/seed/routing は §9.1.31 に従い、metadata/file commit 順序、source selector、isolation、token scope、quota、restart recovery が固定されている | branch API を公開しない |
| ログ/秘匿 | §9.1.17、§12、§9.14 に従い、出力 field、request id、audit 相当記録、秘匿対象、redaction evidence が明記されている | request/SQL/token をログに出す実装を入れない |
| 並行性 / job lifecycle | §9.1.16 / §9.1.25 / §9.1.28 に従い、同時 request、resource lock、idempotency、shutdown、transaction、SQL execution、long-running operation の扱いが定義されている | 並行実行で状態を変更する処理を入れない |
| 後方互換 / Turso 追従 | §9.1.23 に従い、既存 endpoint/schema/config への影響、Turso 差分分類、SDK 影響、migration path が明記されている | 既存契約を変更しない |
| テスト | §9.8 と §9.1.24 / §9.1.27 / §9.1.28 に従い、該当 Phase 行に正常/異常/認可/永続化/障害系、list/pagination/cursor/filter evidence、SQL/result/transaction evidence、CI / release-check / secret scan evidence がある | 完了扱いにしない |
| 運用 | config、metrics、health、rollback、job status / recovery 手順が必要な Phase では明記されている | 運用 API を公開しない |

**Phase implementation readiness packet audit：**

Phase 1〜19 の実装開始前に、実装者は対象 Phase の readiness packet を作成し、以下の audit field をすべて `pass` にしなければならない。readiness packet は `docs/phase-evidence/phase-{phase}/packet.md` または PR description の同等表を正とし、仕様本文、Phase packet、受入 manifest、Done receipt、artifact manifest、review handoff の間で値が揺れてはならない。

| audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `phase_packet_result` | §9.1.32 と §9.17 の Phase packet が対象 Phase 番号、scope in/out、target surface、unsupported behavior、completion gate を持つ | 実装開始禁止 |
| `acceptance_manifest_result` | §9.1.10 の Phase 受入 manifest が Contract ID、Task ID、Scenario ID、Evidence path、Status を全件持つ | 実装開始禁止 |
| `oracle_result` | §9.1.35 の acceptance oracle が API、error、persistence、migration、SDK、unsupported、security、compatibility、regression の期待値を持つ | 実装開始禁止 |
| `scenario_matrix_result` | §9.1.40 と個別 Phase の scenario matrix が正常系、異常系、認可、永続化、rollback、concurrency、compatibility、unsupported、redaction、operational を網羅する | 実装開始禁止 |
| `schema_registry_result` | §9.1.42 に従い、追加・変更する request、response、metadata、config、JWT claim、WebSocket message、artifact、log、metric の field-level schema が固定されている | 実装開始禁止 |
| `decision_precedence_result` | §9.1.43 に従い、複数条件同時成立時の status、error code、client action、commit/rollback 順序が固定されている | 実装開始禁止 |
| `coverage_closure_result` | §9.1.44 に従い、Contract ID、schema ID、scenario ID、decision ID が test、artifact、oracle、regression、N/A 理由へ接続され、coverage gap が 0 件 | 実装開始禁止 |
| `artifact_paths_result` | §9.1.11 に従い、readiness packet、受入 manifest、Done receipt、artifact manifest、review handoff の artifact path、hash、secret scan、reviewer command が一致している | 実装開始禁止 |
| `failure_remediation_result` | §9.1.14 に従い、検出済み failure が分類、root cause、修正範囲、再検証 command、再生成 artifact、reviewer command、closure result で閉じている | 実装開始禁止 |
| `transition_ready_result` | §9.11.8 に従い、source Phase Done receipt、inherited contracts、artifacts、regression、compatibility baseline、operator delta、known blockers が target Phase readiness packet に継承されている | 実装開始禁止 |
| `review_handoff_result` | §9.1.51 に従い、第三者が clean checkout から同じ command と artifact で Done / Not Done を判定できる | Phase 完了扱い禁止 |
| `operator_delta_result` | §9.1.52 に従い、operator から見える before/after、migration/config、rollback、health/log/metric、release note、evidence が固定されている | Phase 完了扱い禁止 |
| `precision_closure_index_result` | §9.11.1〜§9.11.13 と §9.17 の precision closure 系 field が Phase packet、Done receipt、artifact manifest、reviewer reproduction で相互参照できる | 実装開始禁止 |
| `change_impact_closure_result` | §9.1.45 に従い、change impact matrix の affected sections / matrices / tests / artifacts / compatibility / migration / rollback / approval / cross-reference がすべて閉じている | 実装開始禁止 |
| `rollout_readiness_closure_result` | §9.1.46 に従い、startup / shutdown / restart / rollback / health / operator / observability / compatibility / data safety / release blocker がすべて閉じている | 実装開始禁止 |
| `compatibility_baseline_closure_result` | §9.1.47 に従い、Turso Cloud / libSQL SDK / hrana / legacy metadata / previous Phase baseline、snapshot version、SDK transcript、refresh trigger、diff reason、mode boundary がすべて閉じている | 実装開始禁止 |
| `security_abuse_closure_result` | §9.1.48 に従い、attack surface、untrusted input、required control、bypass attempt、expected denial、redaction、audit/log、quota/rate、persistence no-op、security regression がすべて閉じている | 実装開始禁止 |
| `configuration_environment_closure_result` | §9.1.19 / §9.1.24 に従い、config key、precedence、invalid config、secret/path validation、target Phase 前挙動、redaction、toolchain、Docker/CI/local、release-check、environment artifact がすべて閉じている | 実装開始禁止 |
| `ambiguity_atomic_task_closure_result` | §9.1.49 / §9.1.50 に従い、implementation question、selected decision、rejected options、decision basis、edge cases、reopen trigger、task ID、task boundary、verification、dependency / rollback がすべて閉じている | 実装開始禁止 |
| `review_operator_closure_result` | §9.1.51 / §9.1.52 に従い、reading order、reproduction command、expected artifact、decision criteria、failure classification、oral-context-free evidence、operator surface、behavior delta、operator action、release note / rollback がすべて閉じている | 実装開始禁止 |
| `merge_readiness_closure_result` | §9.1.52 の merge readiness closure に従い、base branch freshness、PR diff scope、artifact commit match、CI rerun、regression rerun、stale snapshot、review reapproval、merge blocker がすべて閉じている | merge 不可 |
| `dependency_provenance_closure_result` | §9.1.52 の dependency provenance closure に従い、dependency inventory、version pin、license policy、security advisory、build toolchain、generated artifact provenance、binary extension provenance、runtime surface、rollback/removal がすべて閉じている | 実装開始禁止 |
| `upgrade_data_compatibility_closure_result` | §9.1.52 の upgrade / downgrade / data compatibility closure に従い、source version inventory、upgrade path、downgrade boundary、data file compatibility、metadata schema compatibility、config compatibility、client request compatibility、failure injection upgrade、rollback after upgrade がすべて閉じている | 実装開始禁止 |
| `performance_capacity_closure_result` | §9.1.52 の performance / capacity / resource limit closure に従い、workload profile、latency baseline、memory baseline、storage growth、large input limit、concurrency capacity、backpressure timeout、quota capacity、performance regression がすべて閉じている | Phase 完了扱い禁止 |
| `incident_recovery_closure_result` | §9.1.52 の incident response / disaster recovery / operator runbook closure に従い、incident classification、detection signal、RTO/RPO、operator runbook、safe mode、backup/restore drill、replication/HA recovery、post-incident evidence、customer impact がすべて閉じている | Phase 完了扱い禁止 |
| `contract_versioning_closure_result` | §9.1.52 の external contract versioning / deprecation / sunset closure に従い、contract surface inventory、versioning policy、deprecation policy、sunset policy、breaking change、compat mode、SDK/client impact、metadata/error/config contract、migration notice がすべて閉じている | 実装開始禁止 |
| `machine_contract_artifact_closure_result` | §9.1.52 の machine-readable contract / snapshot / artifact template closure に従い、OpenAPI contract、JSON Schema、wire snapshot、Turso observed snapshot、SDK transcript、artifact template、CI machine verification、contract drift detection、reviewer replay contract がすべて閉じている | 実装開始禁止 |
| `phase_execution_sequence_closure_result` | §9.1.52 の phase execution sequence / stop condition / evidence handoff closure に従い、phase entry sequence、pre-implementation freeze、task order、stop condition、mid-phase change control、evidence handoff sequence、phase exit sequence、blocked state、resume state がすべて閉じている | 実装開始禁止 |
| `verdict_normalization_closure_result` | §9.1.52 の verdict vocabulary / pass-fail normalization / reviewer decision closure に従い、verdict vocabulary、pass condition、fail condition、blocked condition、not applicable condition、manual review condition、partial result prohibition、reviewer decision trace、done/not_done mapping がすべて閉じている | 実装開始禁止 |
| `release_handoff_closure_result` | §9.1.52 の release handoff / rollout decision / rollback evidence closure に従い、release candidate inventory、rollout decision、rollback evidence、operator release note、compatibility release delta、data safety release、post-release monitoring、release blocker、release handoff trace がすべて閉じている | Phase 完了扱い禁止 |
| `defect_prevention_closure_result` | §9.1.52 の defect taxonomy / root cause / recurrence prevention closure に従い、defect taxonomy、severity/priority、root cause、fix scope、regression prevention、recurrence test、spec feedback、defect escape analysis、known defect zero がすべて閉じている | Phase 完了扱い禁止 |
| `artifact_layout_closure_result` | §9.1.52 の artifact layout / schema file / snapshot storage closure に従い、artifact root layout、phase artifact naming、schema file location、snapshot storage、transcript storage、hash manifest、artifact update policy、artifact retention、artifact replay path がすべて閉じている | 実装開始禁止 |
| `upstream_observation_closure_result` | §9.1.52 の upstream observation / Turso snapshot refresh / compatibility drift closure に従い、upstream surface inventory、observation command、observation cadence、snapshot normalization、compatibility drift、drift classification、refresh decision、self-host delta、upstream reference trace がすべて閉じている | 実装開始禁止 |
| `ownership_approval_closure_result` | §9.1.52 の ownership / approval authority / review escalation closure に従い、phase owner、reviewer role、approval authority、escalation path、reapproval trigger、change authority boundary、merge authority、operator acceptance authority、audit signoff trace がすべて閉じている | 実装開始禁止 |
| `operational_readiness_closure_result` | §9.1.52 の operational readiness / SLO / alert threshold / capacity acceptance closure に従い、SLO target、alert threshold、health signal、capacity acceptance、monitoring dashboard、degradation policy、on-call runbook、operator acknowledgement、operational exception がすべて閉じている | 実装開始禁止 |
| `ci_command_matrix_closure_result` | §9.1.52 の CI command matrix / required verification command / shard determinism closure に従い、required command inventory、command execution order、CI shard mapping、local replay command、timeout budget、dependency cache、failure artifact、rerun policy、command drift detection がすべて閉じている | 実装開始禁止 |
| `state_invariant_closure_result` | §9.1.52 の lifecycle state / transition matrix / data invariant closure に従い、lifecycle state inventory、allowed transition matrix、forbidden transition matrix、persistence invariant、idempotency invariant、concurrency invariant、recovery invariant、rollback invariant、invariant violation handling がすべて閉じている | 実装開始禁止 |
| `compatibility_delta_closure_result` | §9.1.52 の compatibility delta / self-host variance / rebaseline closure に従い、upstream behavior baseline、self-host delta inventory、compatible delta classification、API response delta、error delta、persistence side-effect delta、SDK compatibility delta、operator-visible delta、delta approval / rebaseline がすべて閉じている | 実装開始禁止 |
| `artifact_contract_sync_closure_result` | §9.1.52 の artifact / contract / schema / snapshot synchronization closure に従い、contract artifact inventory、schema artifact inventory、snapshot artifact inventory、readiness packet template sync、Done receipt template sync、artifact hash / generated commit sync、stale artifact detection、machine validation command、artifact regeneration trigger がすべて閉じている | 実装開始禁止 |
| `review_checklist_closure_result` | §9.1.52 の reviewer checklist / evidence order / approval-rejection closure に従い、reviewer checklist inventory、review evidence order、required reviewer command、blocking finding taxonomy、non-blocking finding taxonomy、reviewer replay scope、approval checklist、rejection checklist、checklist drift detection がすべて閉じている | 実装開始禁止 |
| `post_merge_verification_closure_result` | §9.1.52 の post-merge / post-release verification / defect watch closure に従い、post-merge verification scope、post-release verification command、deployment smoke test、compatibility recheck、artifact availability check、monitoring signal check、rollback readiness recheck、defect watch window、post-merge failure escalation がすべて閉じている | Phase 完了扱い禁止 |
| `exception_deferral_closure_result` | §9.1.52 の exception / deferral / known limitation closure に従い、exception inventory、deferral reason、deferral owner、expiry / revisit trigger、known limitation classification、Done impact classification、unsupported vs future phase distinction、exception approval、exception removal condition がすべて閉じている | 実装開始禁止 |
| `migration_compatibility_closure_result` | §9.1.52 の migration / upgrade / downgrade compatibility closure に従い、migration inventory、forward migration contract、backward / downgrade policy、metadata version mapping、backup / restore compatibility、WAL / snapshot compatibility、zero-downtime / maintenance mode policy、migration failure handling、migration replay / rollback evidence がすべて閉じている | 実装開始禁止 |
| `configuration_secret_closure_result` | §9.1.52 の configuration / secret / credential / token closure に従い、configuration key inventory、default / required / forbidden value、environment override policy、secret inventory、secret redaction policy、credential rotation policy、token scope / expiry policy、configuration drift detection、insecure configuration failure handling がすべて閉じている | 実装開始禁止 |
| `data_retention_privacy_closure_result` | §9.1.52 の data retention / privacy / deletion closure に従い、data classification、retention policy、delete policy、backup retention、snapshot retention、log retention、artifact retention privacy、personal / sensitive data、retention exception がすべて閉じている | 実装開始禁止 |
| `documentation_runbook_closure_result` | §9.1.52 の documentation / runbook / guide synchronization closure に従い、documentation inventory、API documentation sync、operator runbook sync、migration guide sync、rollback guide sync、release note sync、configuration documentation sync、troubleshooting documentation、documentation stale detection がすべて閉じている | 実装開始禁止 |
| `sbom_vulnerability_license_closure_result` | §9.1.52 の SBOM / vulnerability / license release closure に従い、SBOM inventory、lockfile / SBOM sync、vulnerability scan、license distribution、container image attestation、GitHub Action supply chain、binary artifact origin、exception waiver、rescan trigger がすべて閉じている | 実装開始禁止 |
| `telemetry_signal_contract_closure_result` | §9.1.52 の telemetry / signal / observability contract closure に従い、signal inventory、log field contract、metric name contract、health status contract、request / trace correlation、redaction signal contract、failure signal mapping、operator-visible signal、telemetry artifact contract がすべて閉じている | 実装開始禁止 |
| `phase_done_final_result` | §9.1.33 に従い、Done receipt final audit の readiness、manifest、artifact、failure、regression、handoff、operator、precision closure、zero-bug がすべて pass である | Phase 完了扱い禁止 |
| `readiness_audit_result` | 上記 49 field がすべて `pass`。ただし `phase_done_final_result` は実装完了時に評価する。`missing`、`partial`、`manual_only`、`not_run`、`N/A` 根拠なし、または値不一致が 0 件 | `pass` 以外は実装開始禁止 |

`readiness_audit_result` は Phase 実装開始の入口 gate であり、実装後の Done receipt で初めて埋めてはならない。実装中に scope、API、schema、error、persistence、auth、compatibility、artifact path、review command、operator behavior のいずれかが変わる場合は、同じ PR で readiness packet、受入 manifest、oracle、scenario matrix、schema registry、precision closure を更新し、再度 `readiness_audit_result = pass` にする。更新しないまま code、test、snapshot、artifact だけを変更した場合は merge 不可とする。

**readiness audit 不一致時の判定：**

| 状態 | 判定 |
|------|------|
| `readiness_audit_result` が `pass` ではない | 実装開始禁止 |
| readiness packet と仕様本文の Phase 番号、Contract ID、Task ID、Scenario ID、artifact path が一致しない | 実装開始禁止 |
| Phase packet にある scope / unsupported behavior と受入 manifest または Done receipt が一致しない | Phase 未完了 |
| acceptance oracle または scenario matrix に存在しない成功応答を実装する | merge 不可 |
| schema registry に存在しない field、default、nullable、omittable、redaction rule を実装する | merge 不可 |
| decision precedence 未定義のまま複数拒否条件、commit/rollback、auth/quota、resource state を実装する | merge 不可 |
| coverage closure に未接続の Contract ID、schema ID、scenario ID、decision ID がある | Phase 未完了 |
| change impact の affected sections、affected matrices、affected tests、affected artifacts、compatibility、migration、rollback、approval、cross-reference のいずれかが未更新または旧値を参照する | 実装開始禁止 |
| rollout readiness の startup、shutdown、restart、rollback、health、operator、observability、compatibility、data safety、release blocker のいずれかが未評価または artifact 未接続である | 実装開始禁止 |
| compatibility baseline の upstream source、observed behavior、Adlaire behavior、compatibility class、snapshot version、SDK transcript、refresh trigger、diff reason、mode boundary のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| security abuse の attack surface、untrusted input、required control、bypass attempt、expected denial、redaction、audit/log、quota/rate、persistence no-op、security regression のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| configuration / environment の config key、precedence、invalid config、secret/path validation、target Phase 前挙動、redaction、toolchain、Docker/CI/local、release-check、environment artifact のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| ambiguity / atomic task の implementation question、selected decision、rejected options、decision basis、edge cases、reopen trigger、task ID、task boundary、verification、dependency / rollback のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| review handoff / operator delta の reading order、reproduction command、expected artifact、decision criteria、failure classification、oral-context-free evidence、operator surface、behavior delta、operator action、release note / rollback のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| merge readiness の base branch freshness、PR diff scope、artifact commit match、CI rerun、regression rerun、stale snapshot、review reapproval、merge blocker のいずれかが未確定または PR head commit に接続していない | merge 不可 |
| dependency provenance の inventory、version pin、license policy、security advisory、build toolchain、generated artifact provenance、binary extension provenance、runtime surface、rollback/removal のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| upgrade / downgrade / data compatibility の source version inventory、upgrade path、downgrade boundary、data file compatibility、metadata schema compatibility、config compatibility、client request compatibility、failure injection upgrade、rollback after upgrade のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| performance / capacity / resource limit の workload profile、latency baseline、memory baseline、storage growth、large input limit、concurrency capacity、backpressure timeout、quota capacity、performance regression のいずれかが未確定または artifact 未接続である | Phase 未完了 |
| incident response / disaster recovery / operator runbook の incident classification、detection signal、RTO/RPO、operator runbook、safe mode、backup/restore drill、replication/HA recovery、post-incident evidence、customer impact のいずれかが未確定または artifact 未接続である | Phase 未完了 |
| external contract versioning / deprecation / sunset の contract surface inventory、versioning policy、deprecation policy、sunset policy、breaking change、compat mode、SDK/client impact、metadata/error/config contract、migration notice のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| machine-readable contract / snapshot / artifact template の OpenAPI contract、JSON Schema、wire snapshot、Turso observed snapshot、SDK transcript、artifact template、CI machine verification、contract drift detection、reviewer replay contract のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| phase execution sequence / stop condition / evidence handoff の phase entry sequence、pre-implementation freeze、task order、stop condition、mid-phase change control、evidence handoff sequence、phase exit sequence、blocked state、resume state のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| verdict vocabulary / pass-fail normalization / reviewer decision の verdict vocabulary、pass condition、fail condition、blocked condition、not applicable condition、manual review condition、partial result prohibition、reviewer decision trace、done/not_done mapping のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| release handoff / rollout decision / rollback evidence の release candidate inventory、rollout decision、rollback evidence、operator release note、compatibility release delta、data safety release、post-release monitoring、release blocker、release handoff trace のいずれかが未確定または artifact 未接続である | Phase 未完了 |
| defect taxonomy / root cause / recurrence prevention の defect taxonomy、severity/priority、root cause、fix scope、regression prevention、recurrence test、spec feedback、defect escape analysis、known defect zero のいずれかが未確定または artifact 未接続である | Phase 未完了 |
| artifact layout / schema file / snapshot storage の artifact root layout、phase artifact naming、schema file location、snapshot storage、transcript storage、hash manifest、artifact update policy、artifact retention、artifact replay path のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| upstream observation / Turso snapshot refresh / compatibility drift の upstream surface inventory、observation command、observation cadence、snapshot normalization、compatibility drift、drift classification、refresh decision、self-host delta、upstream reference trace のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| ownership / approval authority / review escalation の phase owner、reviewer role、approval authority、escalation path、reapproval trigger、change authority boundary、merge authority、operator acceptance authority、audit signoff trace のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| operational readiness / SLO / alert threshold / capacity acceptance の SLO target、alert threshold、health signal、capacity acceptance、monitoring dashboard、degradation policy、on-call runbook、operator acknowledgement、operational exception のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| CI command matrix / required verification command / shard determinism の required command inventory、command execution order、CI shard mapping、local replay command、timeout budget、dependency cache、failure artifact、rerun policy、command drift detection のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| lifecycle state / transition matrix / data invariant の lifecycle state inventory、allowed transition matrix、forbidden transition matrix、persistence invariant、idempotency invariant、concurrency invariant、recovery invariant、rollback invariant、invariant violation handling のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| compatibility delta / self-host variance / rebaseline の upstream behavior baseline、self-host delta inventory、compatible delta classification、API response delta、error delta、persistence side-effect delta、SDK compatibility delta、operator-visible delta、delta approval / rebaseline のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| artifact / contract / schema / snapshot synchronization の contract artifact inventory、schema artifact inventory、snapshot artifact inventory、readiness packet template sync、Done receipt template sync、artifact hash / generated commit sync、stale artifact detection、machine validation command、artifact regeneration trigger のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| reviewer checklist / evidence order / approval-rejection の reviewer checklist inventory、review evidence order、required reviewer command、blocking finding taxonomy、non-blocking finding taxonomy、reviewer replay scope、approval checklist、rejection checklist、checklist drift detection のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| post-merge / post-release verification / defect watch の post-merge verification scope、post-release verification command、deployment smoke test、compatibility recheck、artifact availability check、monitoring signal check、rollback readiness recheck、defect watch window、post-merge failure escalation のいずれかが未確定または artifact 未接続である | Phase 完了扱い禁止 |
| exception / deferral / known limitation の exception inventory、deferral reason、deferral owner、expiry / revisit trigger、known limitation classification、Done impact classification、unsupported vs future phase distinction、exception approval、exception removal condition のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| migration / upgrade / downgrade compatibility の migration inventory、forward migration contract、backward / downgrade policy、metadata version mapping、backup / restore compatibility、WAL / snapshot compatibility、zero-downtime / maintenance mode policy、migration failure handling、migration replay / rollback evidence のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| configuration / secret / credential / token の configuration key inventory、default / required / forbidden value、environment override policy、secret inventory、secret redaction policy、credential rotation policy、token scope / expiry policy、configuration drift detection、insecure configuration failure handling のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| data retention / privacy / deletion の data classification、retention policy、delete policy、backup retention、snapshot retention、log retention、artifact retention privacy、personal / sensitive data、retention exception のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| documentation / runbook / guide synchronization の documentation inventory、API documentation sync、operator runbook sync、migration guide sync、rollback guide sync、release note sync、configuration documentation sync、troubleshooting documentation、documentation stale detection のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| SBOM / vulnerability / license release の SBOM inventory、lockfile / SBOM sync、vulnerability scan、license distribution、container image attestation、GitHub Action supply chain、binary artifact origin、exception waiver、rescan trigger のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| telemetry / signal / observability contract の signal inventory、log field contract、metric name contract、health status contract、request / trace correlation、redaction signal contract、failure signal mapping、operator-visible signal、telemetry artifact contract のいずれかが未確定または artifact 未接続である | 実装開始禁止 |
| artifact path、hash、secret scan、reviewer command が readiness packet、受入 manifest、Done receipt、artifact manifest、review handoff の間で一致しない | Phase 未完了 |
| failure が未分類、root cause 未記載、再検証 command 未記載、artifact / regression / secret scan 再実行漏れのまま残る | Phase 未完了 |
| source Phase の Done receipt、regression、artifact hash、compatibility baseline、operator delta、known blocker が target Phase readiness packet に継承されていない | 実装開始禁止 |
| review handoff が local only、口頭説明、PR description の自由文、または実装者環境だけに依存する | Phase 未完了 |
| operator delta が health、log、metric、rollback、migration/config、release note のいずれかを欠く | Phase 未完了 |
| precision closure index と readiness packet の Contract ID、artifact path、review command が一致しない | Phase 未完了 |
| `phase_done_final_result` が `pass` でない、または known bugs / unverified / missing evidence / open failure / unsupported success / rootless N/A / stale artifact が 0 件でない | Phase 未完了 |

**Phase 間の前倒し実装ルール：**

- 未来 Phase の内部型や helper を先に置くことは許可する。ただし外部 API、永続化 schema、成功応答を公開してはならない
- 未来 Phase の route を先に置く場合は 501 `NOT_IMPLEMENTED` のみ許可する。200/201/204/307 を返してはならない
- 未来 Phase の永続化ファイルを先に初期化する場合は、初期値・破損時挙動・読み飛ばし可否を §9.6 に明記する
- Phase 完了判定は「コードが存在する」ではなく「該当 Phase の完了ゲートとテスト契約を満たす」で行う

**仕様変更 PR と実装 PR の分離基準：**

| 変更内容 | PR 種別 |
|----------|---------|
| Phase 境界、API schema、error code、永続化 schema、認証境界を変える | 仕様変更 PR |
| 既に仕様化済みの契約をコードへ反映する | 実装 PR |
| 仕様と実装の不一致を見つけ、仕様が正しい場合 | 実装修正 PR |
| 仕様と実装の不一致を見つけ、仕様を変える必要がある場合 | 仕様変更 PR を先行 |
| テストだけで契約を固定できる場合 | 実装 PR。ただし期待値は仕様内の契約を参照する |


## 10. Phase仕様の責務

Phase 1〜19 の詳細は `docs/spec/phase-01.md` 〜 `docs/spec/phase-19.md` に分割して管理する。各 Phase ファイルは、その Phase の実装契約、完了条件、禁止事項、証跡、review handoff を固定する正本である。HTML 生成時は以下の include をこの順序で展開する。

<!-- include: docs/spec/phase-01.md -->
<!-- include: docs/spec/phase-02.md -->
<!-- include: docs/spec/phase-03.md -->
<!-- include: docs/spec/phase-04.md -->
<!-- include: docs/spec/phase-05.md -->
<!-- include: docs/spec/phase-06.md -->
<!-- include: docs/spec/phase-07.md -->
<!-- include: docs/spec/phase-08.md -->
<!-- include: docs/spec/phase-09.md -->
<!-- include: docs/spec/phase-10.md -->
<!-- include: docs/spec/phase-11.md -->
<!-- include: docs/spec/phase-12.md -->
<!-- include: docs/spec/phase-13.md -->
<!-- include: docs/spec/phase-14.md -->
<!-- include: docs/spec/phase-15.md -->
<!-- include: docs/spec/phase-16.md -->
<!-- include: docs/spec/phase-17.md -->
<!-- include: docs/spec/phase-18.md -->
<!-- include: docs/spec/phase-19.md -->
