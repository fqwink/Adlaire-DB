# 実装統制仕様

Phase 実装判断、共通完了条件、DoR / DoD、証跡、バグ修正ゼロ化、レビュー統制を固定する仕様である。

### 9.1 Phase 共通統制契約

フェーズ単位で機能を積み上げる。内製化は Phase 19 から開始し、Phase 18 以前は production path の内部差し替えを行わない（§3.5.4）。

| フェーズ | 内容 | テストケース | 実装タスク |
|----------|------|------------|----------|
| **Phase 1** | ビルド基盤・CLI | — | T-1〜T-2 (2件) |
| **Phase 2** | データディレクトリ・libsql 統合 | — | T-3〜T-4 (2件) |
| **Phase 3** | HTTP サーバー・hrana パイプライン | TC-1, TC-2, TC-6 (3件) | T-5〜T-6 (2件) |
| **Phase 4** | JWT 認証・token コマンド | TC-3 (1件) | T-7〜T-9 (3件) |
| **Phase 5** | ログ・統合テスト | TC-4, TC-5 (2件) | T-10〜T-11 (2件) |
| **Phase 6** | マルチ DB ルーター・DB マネージャ | — | T2-1〜T2-2 (2件) |
| **Phase 7** | 管理 API・トークン CRUD・DB スコープ JWT | TC-2-1〜TC-2-6（TC-2-5b 含む）(7件) | T2-3〜T2-6 (4件) |
| **Phase 8** | Turso Cloud 互換管理モデル | TC-8-1〜TC-8-24 (24件) | T8-1〜T8-24 (24件) |
| **Phase 9** | WebSocket（hrana-ws v3） | TC-3-1〜TC-3-4 (4件) | T3-1〜T3-5 (5件) |
| **Phase 10** | ATTACH DB・メトリクス | TC-3-5, TC-3-6 (2件) | T3-6〜T3-8 (3件) |
| **Phase 11** | レプリケーション基盤（WAL ストリーム・スナップショット） | — | T4-1〜T4-3 (3件) |
| **Phase 12** | レプリカ同期・書き込みリダイレクト | TC-4-1〜TC-4-5 (5件) | T4-4〜T4-7 (4件) |
| **Phase 13** | WAL アーカイブ・manifest 管理 | TC-5-8 (1件) | T5-1〜T5-4 (4件) |
| **Phase 14** | バックアップ・リストア・PITR API | TC-5-1〜TC-5-7 (7件) | T5-5〜T5-9 (5件) |
| **Phase 15** | ブランチ作成・一覧・削除 | TC-6-1〜TC-6-7 (7件) | T6-1〜T6-7 (7件) |
| **Phase 16** | SQLite 拡張ロード | TC-16-1〜TC-16-6 (6件) | T16-1〜T16-6 (6件) |
| **Phase 17** | メトリクス永続化・外部監視連携 | TC-17-1〜TC-17-5 (5件) | T17-1〜T17-5 (5件) |
| **Phase 18** | HA・自動フェイルオーバー | TC-18-1〜TC-18-7 (7件) | T18-1〜T18-7 (7件) |
| **Phase 19** | libSQL 内部コンポーネント段階的内製化 | TC-19-1〜TC-19-6 (6件) | T19-1〜T19-6 (6件) |


### 9.0 実装判断ルール

実装中に仕様の解釈で迷った場合は、以下の順で判断する。

1. 本仕様書の「固定制約」「設計不変条件」「API 仕様」「エラー定義」を優先する
2. 同じ Phase 内の「スコープ」「対象外」「完了ゲート」を優先する
3. 既存実装と仕様が違う場合は仕様を優先する。ただし仕様変更が必要な場合は作業ルールの変更承認フローに従う
4. Phase に明記されていない機能は、その Phase では実装しない。必要なら次 Phase の対象として仕様に追記してから実装する
5. 互換性判断で迷う場合は §1.5 に従い、`Turso Cloud 互換 > libSQL SDK 互換 > 既存 Adlaire 後方互換 > 自己ホスト最適化 > 内製化都合 > Adlaire 独自拡張` の順で優先する
6. libSQL/Turso の hrana ワイヤ互換、Turso Cloud の管理 API・metadata・auth・error 互換を優先する。ただしセルフホスト運用・データ永続性・セキュリティ制約を破ってはならない
7. 内製化都合で API、レスポンス、認証、metadata、永続化形式、エラー形式を変更してはならない。変更が必要な場合は、先に Turso Cloud 追従差分として仕様書を改訂する
8. エラー形式で迷う場合は §7.3 の `code` を使う。新しいエラーが必要な場合は先に §7.3 へ追加する
9. JSON schema で迷う場合は「省略可能」「null 可」「空配列可」を本文に明記する。明記がないフィールドは必須とする
10. 永続化を伴う処理は、成功応答前にファイル内容とメタデータの両方が整合していることを必須とする

**曖昧語の扱い：**

| 表現 | 実装上の扱い |
|------|--------------|
| 「必須」 | Phase 完了条件に含める。未実装なら Phase 未完了 |
| 「任意」 | 実装してよいが、完了条件には含めない。未実装でも API 契約を壊してはならない |
| 「対象外」 | その Phase では実装しない。route を生やす場合は 501 または仕様で定めた stub 応答に限定する |
| 「将来」「検討」 | 実装禁止。対象 Phase が明記されるまでコードに入れない |
| 「スタブ」 | レスポンス形式・ステータス・ログ有無を本仕様に従って固定する |
| 「よい」「できる」「可能」 | 明示された条件下でだけ許可。条件、Phase、完了ゲートが同じ節にない場合は実装根拠にしない |
| 「原則」 | 例外条件が同じ節または参照先表に明記されている場合だけ例外を許可。例外条件がなければ MUST と同じ |
| 「推奨」「非推奨」 | 運用 guidance であり、実装完了条件にはしない。ただし security / persistence / compatibility に関係する場合は §9 の契約表を優先する |
| 「未実装」 | 指定 Phase に到達するまで成功応答を返してはならない。設定値や route を受け取る場合の挙動は §9.0.1 と §9.13.1 に従う |
| 「など」 | 例示であり、実装範囲を拡張しない。列挙外の機能は仕様未定義として扱う |

**仕様の正本性：**

| 種別 | 実装上の優先度 | 扱い |
|------|----------------|------|
| 本文の MUST / MUST NOT 相当の記述 | 最優先 | 実装・テスト・レビューで必ず満たす |
| §9 の判断ルール・契約表・完了ゲート | 最優先 | 各 Phase の merge 判定に使用する |
| API schema / error code / 永続化 schema の表 | 高 | コード例より優先する |
| Rust コード例 | 中 | 実装方針の参考。本文・表と矛盾する場合は本文・表を正とする |
| 将来方針・候補・検討事項 | 低 | 実装根拠にしてはならない。実装する場合は先に仕様変更 PR を作る |

**コード例・stub の扱い：**

| 対象 | 実装上の扱い |
|------|--------------|
| Rust コード例の `unwrap()` / `expect()` | 説明用に限定する。request、disk、network、metadata、config、auth、replication、backup、extension、HA 由来の失敗を production path で panic にしてはならない |
| Rust コード例の `not_implemented()` | 対象 Phase 前の stub 例であり、対象 Phase 完了後の production path に残してはならない |
| コメント内の Phase 名 | 実装境界の説明であり、本文・表の Phase 契約と矛盾する場合は本文・表を優先する |
| sample config の commented key | その key の実装許可ではない。§9.13 に Phase と挙動がない限り実装禁止 |

**未定義に遭遇した場合の処理：**

1. API の method/path/auth/status/body/error が未定義なら、実装を開始しない
2. 永続化ファイル、更新順序、rollback、破損時挙動が未定義なら、実装を開始しない
3. security boundary、secret handling、ログ秘匿対象が未定義なら、実装を開始しない
4. 既存仕様にない success response を返す route は追加しない
5. やむを得ず先行実装が必要な場合は、同じ PR の先頭 commit で仕様を更新し、その後に実装 commit を積む
6. 仕様変更を伴う PR は、PR description に「変更した契約」「影響 Phase」「追加テスト」を明記する

#### 9.0.1 stub / NOT_IMPLEMENTED 固定契約

未来 Phase の route、CLI、config、内部 hook を先に配置する場合は、本節を必ず満たす。stub は success response ではなく、互換性・探索性・routing 明確化のための失敗応答である。

| 対象 | 許可条件 | 必須応答 | 禁止 |
|------|----------|----------|------|
| HTTP route stub | §9.5 または該当 Phase 節に method/path が定義済み | `501 NOT_IMPLEMENTED` と `{"error":"not implemented","code":"NOT_IMPLEMENTED"}`。body なしが明記された Turso 互換 endpoint は該当節を優先 | 2xx、部分成功、metadata 更新、DB file 更新 |
| unsupported method | path は既知だが method が未定義 | `405 METHOD_NOT_ALLOWED` と `METHOD_NOT_ALLOWED` | 501 で method 不一致を隠す |
| unknown path | path が仕様未定義 | `404 ENDPOINT_NOT_FOUND` | 501 で未知 path を許可済みに見せる |
| CLI stub | subcommand 名と Phase が定義済み | 起動前 validation 後に非 0 終了。stderr に対象 Phase と未実装理由を出す | 0 終了、token/secret/metadata 生成 |
| config stub | §9.13.1 で対象 Phase 前の扱いが定義済み | 起動失敗または明示 WARN のどちらか。silent ignore 禁止 | 成功扱いで値を保存、将来 Phase の挙動を部分実行 |
| internal hook stub | production path から到達しない、または到達時に仕様済み error を返す | `NOT_IMPLEMENTED` または対象 error code | panic、unwrap、silent no-op |

stub の log は WARN `not implemented endpoint` / `not implemented command` / `not implemented config` のいずれかとし、request body、Authorization、JWT、admin/platform/replication/HA token、生 SQL args、backup body、extension path を出力してはならない。

対象 Phase の完了 PR では、その Phase の completion path にある stub をすべて実装済み処理へ置き換える。stub が残る場合は、対象外節または unsupported 固定表に残存理由、status、body、log、test を明記する。

#### 9.0.2 実装前仕様固定プロトコル

各 Phase の実装開始前に、実装者は該当 Phase について以下の契約表を作成または既存節から確認し、全項目が仕様書本文に存在することを確認する。1 つでも欠ける場合、その Phase の実装を開始してはならない。

| 契約カテゴリ | 固定必須項目 | 欠けている場合の扱い |
|--------------|--------------|----------------------|
| API surface | method、path、path parameter、query parameter、request body、success status、success body、error status、error body、auth 種別 | 仕様未確定。実装禁止 |
| Permission | admin/JWT/platform/replication/HA token のどれを使うか、ro/rw/scope/org/group/quota の判定順序 | 仕様未確定。実装禁止 |
| Persistence | 更新ファイル、atomic update 手順、fsync 対象、rollback 手順、起動時 recovery、破損時挙動 | 仕様未確定。実装禁止 |
| Compatibility | Turso Cloud との差分、libSQL SDK 影響、legacy metadata 互換、snapshot artifact の正規化方法 | 仕様未確定。実装禁止 |
| Observability | INFO/WARN/ERROR の発火条件、metric 名、secret/log redaction、audit 相当の記録有無 | 仕様未確定。実装禁止 |
| Test evidence | 正常系、異常系、権限系、永続化系、再起動系、互換系、concurrency 系の test ID | 仕様未確定。実装禁止 |

**実装前チェックリスト：**

1. 該当 Phase の「対象外」に書かれた機能を実装予定に含めていない
2. 追加 endpoint は §9.5 または該当 Phase 節に method/path 単位で存在する
3. 追加 error code は §7.3 に存在し、HTTP status と message 粒度が固定されている
4. 追加 metadata field は §9.6 または該当 Phase 節に型、必須/任意、default、migration、破損時挙動が固定されている
5. 追加 config/flag/env は §4 と §8.4 に default、優先順位、不正値エラーが固定されている
6. 追加外部 crate は §3.3.5 に用途、導入 Phase、セキュリティ影響が固定されている
7. Turso Cloud 互換に関係する変更は snapshot artifact と差分理由が固定されている
8. 実装 PR の完了条件として実行する test command と期待結果が固定されている

上記チェックリストの「固定」は、仕様書本文に追記済みであることを意味する。PR description だけに書かれた判断は仕様として扱わない。

### 9.1 全 Phase 共通の完了条件

各 Phase は、個別ゲートに加えて以下をすべて満たすこと。

1. `cargo build` と `cargo test` が成功する
2. その Phase で追加・変更した API の正常系、異常系、権限系、永続化系テストが追加されている
3. 既存 Phase のテストケースがリグレッションしない
4. API レスポンスの HTTP status、JSON body、`code` が §6・§7 と一致している
5. 永続化ファイルは tmp 書き込み、fsync、rename の順でアトミックに更新する。ただし対象 FS で fsync が利用できない場合は起動時に WARN ではなくエラーとする
6. メタデータ破損、DB 破損、設定不正は起動成功扱いにしない
7. ログには秘密情報、JWT、生 SQL 引数の値、管理トークンを出さない
8. 新しい設定値を追加した場合は、CLI/config/env の優先順位、デフォルト値、不正値エラーを §4 と §8.4 に追記する
9. 新しい永続化ファイルを追加した場合は、ディレクトリ構成、初期値、破損時挙動、バックアップ対象かどうかを §3.2 または該当 Phase に追記する
10. 新しい外部 crate を追加する場合は、採用理由、代替案、対象 Phase、セキュリティ影響を仕様書に明記する

#### 9.1.1 バグ修正ゼロ化ゲート

各 Phase の実装 PR は、以下を満たすまで完了扱いにしてはならない。1 つでも未実施または失敗がある場合は、その PR 内で修正し、後続の「バグ修正 PR」へ持ち越さない。

| ゲート | 必須判定 |
|--------|----------|
| Contract exhaustiveness | §9.5 の該当 endpoint 全てについて、method/path/auth/request/success/error/persistence/idempotency のテストが存在する |
| Error exhaustiveness | §7.3 / §9.7 に定義された該当 Phase の error code 全てについて、発火テストが存在する |
| Unknown/null validation | 管理 API は unknown field、余分 body、不正 null、不正 query、不正 path decode を `INVALID_REQUEST` として拒否する |
| Persistence atomicity | metadata 更新中の失敗注入で、partial write、空上書き、metadata/file 不整合が起きない |
| Restart recovery | 正常更新後、異常終了後、metadata 破損後の起動結果が §9.6 と一致する |
| Auth boundary | 認証なし、不正 token、scope 外、ro/rw の境界を全 endpoint で確認する |
| Secret leakage | stdout/stderr/log file/HTTP response に secret、JWT、admin token、replication token、HA token、生 SQL args、backup body、extension 絶対 path が出ない |
| Concurrency | 同一 resource への同時 create/delete/update で二重作成、lost update、破損 metadata が起きない |
| Regression | 当該 Phase より前の Phase の TC がすべて通る |
| Compatibility | Phase 5 以降は TypeScript `@libsql/client` の CRUD regression、Phase 9 以降は WebSocket regression、Phase 12 以降は replication regression が通る |
| Turso Platform compatibility | Phase 8 以降は `/v1/*` の response wrapper、field casing、status code、query mapping が Turso 互換 snapshot と一致する |
| Legacy fallback | Phase 8 以降は Phase 1〜7 で作られた legacy DB/token metadata が読み取り・接続・削除でき、新規作成 validation と混同されない |

**失敗時の扱い：**

- ゲート失敗はすべて当該 Phase の未完了として扱う
- test skip で通過扱いにしてはならない
- flaky test は `retry` で隠さず、原因を修正してから完了扱いにする
- 外部環境依存で自動化できない検証は、手順、期待値、実行ログ保存先を仕様書または PR description に固定する

#### 9.1.1a バグ修正持ち越し禁止リスト

以下の状態を含む PR は、実装が一見動作していても Phase 完了として扱わない。該当する場合は同一 PR 内で修正し、後続 PR へ「バグ修正」として分離してはならない。

| 禁止状態 | 判定 |
|----------|------|
| TODO/FIXME による契約未実装 | Phase 未完了 |
| error mapping の暫定 `INTERNAL_ERROR` 代用 | Phase 未完了 |
| request validation の一部未実装 | Phase 未完了 |
| auth/scope/quota/org/group 判定の一部 endpoint 未適用 | Phase 未完了 |
| metadata migration の片方向のみ実装 | Phase 未完了 |
| rollback 不能な destructive operation | Phase 未完了 |
| crash/restart 後の挙動未検証 | Phase 未完了 |
| concurrency test 未実施 | Phase 未完了 |
| Turso Platform snapshot 未更新または未比較 | Phase 8 以降は Phase 未完了 |
| TypeScript SDK regression 未実行 | Phase 5 以降は Phase 未完了 |
| WebSocket regression 未実行 | Phase 9 以降は Phase 未完了 |
| replication regression 未実行 | Phase 12 以降は Phase 未完了 |
| backup/restore rollback 未検証 | Phase 14 以降は Phase 未完了 |
| HA split-brain test 未実行 | Phase 18 以降は Phase 未完了 |
| internal adapter rollback flag 未検証 | Phase 19 は Phase 未完了 |

「後で検証する」「既知の軽微な不具合」「現時点では通る想定」は完了根拠として認めない。完了根拠は、実行済み command、保存された snapshot、または自動テスト結果のいずれかでなければならない。

#### 9.1.2 全 Phase 共通実装固定契約

本節は Phase 1〜19 の全実装に適用する。個別 Phase 節がより厳しい条件を定義する場合は個別 Phase 節を優先する。個別 Phase 節が沈黙している場合は下表を正とする。

| 項目 | 固定契約 |
|------|----------|
| JSON object | request body が JSON API の場合、body は object 必須。array/scalar/null は `400 INVALID_REQUEST` |
| unknown field | 管理 API、Turso Platform API、HA、extension、backup/restore/branch API では拒否。hrana protocol body は hrana 仕様に従う |
| field 省略 | schema で `?` が付いた field だけ省略可。明記なし field は必須 |
| `null` | schema で `null 可` と明記された field だけ許可。省略可能 field に `null` を送っても省略扱いにしない |
| 空文字 | token、name、id、path parameter、query value の空文字は禁止。明記された free-form text field だけ許可 |
| 空配列 | schema で空配列可と明記された field だけ許可。filter 配列、scope 配列、batch 配列は空を `INVALID_REQUEST` とする |
| query | unknown key、duplicate key、percent decode 不能、型変換不能は `INVALID_REQUEST` |
| pagination | `limit` は 1〜500。未指定は 100。`cursor` は base64url 文字列とし、decode 不能・期限切れ・対象 resource 不一致は `INVALID_REQUEST` |
| timestamp | response と metadata は RFC3339 UTC 秒精度。比較用 snapshot では placeholder 正規化。request の未来 timestamp は endpoint が許可しない限り `INVALID_REQUEST` |
| UUID/id | 外部互換で UUID が必要な field は UUID v4 文字列。内部 prefix id は `^[a-z]+_[a-zA-Z0-9_-]{1,80}$` |
| Content-Type | JSON body は `application/json` 必須。`charset=utf-8` は許可。octet-stream endpoint は `application/octet-stream` 必須 |
| Accept | 未指定、`*/*`、`application/json` は許可。その他を厳密拒否する場合は endpoint 節に明記する |
| response body | 204 は body なし。JSON response は object/array の最上位型を schema に固定し、成功時に error field を混在させない |
| error body | HTTP API の error は `{"error": "...", "code": "..."}` のみ。追加 debug field は返さない |
| idempotency | GET は副作用禁止。DELETE は表に明記された挙動に従う。POST は idempotency key を仕様化していない限り非冪等 |
| concurrency | 同一 resource の create/update/delete は resource 単位 lock を取る。lost update と二重作成は禁止 |
| atomic write | tmp file 書き込み、file fsync、rename、parent directory fsync の順を必須とする |
| multi-file commit | 複数 file 更新は prepare、fsync、rename、commit marker の順で行う。途中失敗時は旧状態または明示 rollback 状態だけを起動可能にする |
| startup corruption | 必須 metadata の JSON parse 失敗、schema 違反、unique constraint 違反は起動失敗。空初期値で上書きしない |
| cleanup | temp/backup/orphan file の cleanup は起動成功後に WARN を出して行う。cleanup 失敗は元データを破壊しない限り起動失敗にしない |
| logging | request body、SQL args、JWT、admin/platform/replication/HA token、backup binary、extension 絶対 path は出力禁止 |
| test artifact | snapshot は動的値を正規化し、status/header subset/body を含める。生成場所は Phase 節で固定する |

**状態遷移固定契約：**

resource の状態を持つ Phase 9 以降の機能は、以下の状態名を使う。未定義状態を追加する場合は、先に本表へ追記する。

| Resource | 状態 | 許可遷移 | 禁止 |
|----------|------|----------|------|
| WebSocket session | `new` → `hello_ok` → `closing` → `closed` | `hello_ok` 後だけ request 処理可 | `new` で SQL 実行、`closed` から復帰 |
| WebSocket stream | `open` → `tx_active` → `open` → `closed` | BEGIN で `tx_active`、COMMIT/ROLLBACK で `open` | close 後 execute、接続 close 後 tx 放置 |
| replication frame | `generated` → `served` → `acked` → `archived` | frame_no は単調増加 | checksum 未検証 ack、番号巻き戻り |
| restore job | `prepared` → `verifying` → `committed` または `rolled_back` | committed 前は元 DB を保持 | 元 DB 直接上書き、rollback 不能 |
| branch | `creating` → `active` → `deleting` → `deleted` | active だけ pipeline 接続可 | metadata 先行 active、削除済み接続 |
| extension | `registered` → `loaded` → `disabled` または `deleted` | sha256 検証後のみ loaded | 未検証 load、SQL から直接 load |
| HA node | `standalone` / `replica` / `candidate` / `primary` | candidate から primary は operator promote 必須 | 自動 primary 昇格、古い term 採用 |
| internal adapter | `disabled` → `shadow` → `active` → `rollback` | shadow で互換 snapshot 通過後だけ active | API/metadata 差分を伴う active |

**ゼロバグ横断テスト：**

各 Phase 実装 PR は既存 TC に加えて、該当する横断テストを実施する。

| ID | 対象 Phase | 検証内容 |
|----|------------|----------|
| ZB-1 | all | unknown/null/empty/duplicate query/body validation が固定契約通り |
| ZB-2 | 2+ | metadata atomic write 失敗注入で旧状態または rollback 状態に戻る |
| ZB-3 | 3+ | error body が `error` と `code` だけで、secret/debug field を含まない |
| ZB-4 | 7+ | 同一 resource の concurrent create/delete/update で unique constraint と metadata 整合性が壊れない |
| ZB-5 | 8+ | Turso snapshot が dynamic placeholder 正規化後に一致する |
| ZB-6 | 9+ | connection/session/job の途中切断で未完了 write が成功扱いにならない |
| ZB-7 | 11+ | frame/checksum/manifest の不整合を検出し、silent success しない |
| ZB-8 | 14+ | restore/branch/adapter rollback が再起動後も整合する |

#### 9.1.3 実装 PR 証跡フォーマット

各 Phase の実装 PR は、PR description または同梱された test artifact に以下を残す。記録がない項目は未実施として扱う。

| 項目 | 必須内容 |
|------|----------|
| Phase | 対象 Phase、影響する前後 Phase、対象外として確認した機能 |
| Contract changed | 追加/変更した endpoint、metadata、config、error code、permission、migration |
| Compatibility | Turso Cloud / libSQL SDK / legacy metadata への影響と確認結果 |
| Test commands | 実行した command、終了コード、失敗時の修正内容 |
| Snapshot artifacts | API snapshot、Turso Platform snapshot、Prometheus snapshot、WebSocket transcript など該当 Phase の artifact |
| Persistence evidence | atomic write、rollback、restart recovery、corruption handling の検証結果 |
| Security evidence | secret redaction、auth failure、scope denial、quota denial の検証結果 |
| Regression evidence | 当該 Phase より前の TC が通ったこと |

証跡は「何を実装したか」ではなく「仕様のどの契約を満たしたか」で記述する。仕様書にない判断を PR description で補って完了扱いにしてはならない。

#### 9.1.4 Phase 完了不可条件

以下に該当する PR は、該当機能が動作して見える場合でも Phase 完了として扱わない。修正は同一 PR 内で完了させ、後続の「バグ修正」へ分離してはならない。

| 完了不可条件 | 判定 |
|--------------|------|
| 対象 Phase の endpoint、metadata、config、error、permission、test artifact のいずれかが仕様未定義 | 実装 PR ではなく仕様修正 PR とする |
| success response は返るが異常系、権限系、quota/scope 系、破損系、再起動系のいずれかが未検証 | Phase 未完了 |
| TODO/FIXME/stub/unimplemented/panic/unwrap により production path が残る | Phase 未完了 |
| `INTERNAL_ERROR`、`INVALID_REQUEST`、`PERMISSION_DENIED` を仕様未定義エラーの代用にしている | Phase 未完了 |
| 仕様上の `対象外` 機能に成功応答を返している | Phase 未完了 |
| metadata migration が旧形式、新形式、破損形式、rollback 形式のいずれかを検証していない | Phase 未完了 |
| destructive operation に rollback、restart recovery、concurrency test がない | Phase 未完了 |
| Turso Cloud 互換対象 endpoint の snapshot がない、または snapshot 差分理由が仕様本文にない | Phase 未完了 |
| TypeScript SDK / WebSocket / replication / backup / HA / internal adapter の該当 regression が未実行 | Phase 未完了 |
| log redaction の確認なしに token、JWT、SQL args、backup body、extension path を扱う | Phase 未完了 |
| flaky test、手動確認、目視確認、想定結果だけを完了根拠にしている | Phase 未完了 |

「軽微」「一時的」「後で直す」「既知課題」「仕様上問題ないはず」という記述は完了根拠として無効である。Phase 完了は、仕様本文、実装、テスト、証跡が揃った場合だけ認める。

#### 9.1.5 実装差分リスク分類

実装 PR は、変更内容を下表のリスク分類に必ず割り当てる。複数に該当する場合はすべての必須証跡を満たす。分類できない変更は仕様未定義として扱い、先に本表へ分類を追加する。

| リスク分類 | 該当例 | 必須証跡 |
|------------|--------|----------|
| API contract | endpoint 追加、status/body/header/error 変更、query/path/body validation 変更 | endpoint snapshot、正常系/異常系/unknown/null/duplicate query test |
| Auth/permission | JWT claim、admin/platform token、DB scope、org/group scope、quota gate、ro/rw 判定 | auth matrix、scope denial、secret redaction、対象外 resource rejection |
| Metadata/migration | metadata field 追加、schema 変更、legacy migration、unique constraint 変更 | old/new/corrupt metadata fixture、migration log、rollback/restart recovery |
| Persistence/rollback | DB 作成削除、restore、branch、WAL archive、extension binary、HA state 更新 | atomic write failure injection、fsync/rename evidence、rollback evidence、concurrency test |
| Compatibility | Turso Platform API、hrana schema、SDK 挙動、legacy DB 名、response wrapper 変更 | Turso snapshot、TypeScript SDK regression、legacy fixture、差分理由 |
| Replication/HA | WAL frame、snapshot、redirect、leader election、promotion/demotion、term 更新 | frame/checksum transcript、lag/health snapshot、split-brain rejection、restart recovery |
| Observability | log field、metric、Prometheus output、request id、audit 相当 record | log redaction sample、metric snapshot、secret 非含有確認 |
| Internal adapter | WAL/storage/executor adapter、shadow/active/rollback flag、performance baseline | Phase 1〜18 regression、adapter diff snapshot、rollback flag test、baseline comparison |

PR は「変更なし」として分類を省略してはならない。仕様書のみの PR であっても、影響する分類と後続実装で必要になる証跡を明記する。

#### 9.1.6 Phase 別証跡チェックリスト

各 Phase 実装 PR は、§9.1.3 の共通証跡に加えて下表の証跡を残す。該当 Phase の証跡が欠ける場合は Phase 未完了とする。

| Phase | 必須証跡 |
|-------|----------|
| Phase 1 | CLI help snapshot、invalid flag stderr、workspace build result |
| Phase 2 | data-dir 初期化 tree、二重起動拒否、integrity_check 成功/失敗、metadata 破損起動失敗 |
| Phase 3 | `/v2/health` snapshot、pipeline success/error snapshot、malformed JSON、hrana error body、Web framework 不使用確認 |
| Phase 4 | JWT valid/expired/revoked/bad signature、ro write denial、token secret redaction、tokens.json atomic update |
| Phase 5 | JSON Lines log snapshot、Authorization/SQL args 非出力、TypeScript SDK CRUD、restart persistence |
| Phase 6 | default fallback、`/{db}/v2/pipeline`、DB 名 validation、複数 DB 分離、databases.json recovery |
| Phase 7 | admin auth matrix、DB CRUD、token CRUD、DB scope ro/rw、revoke 即時反映、concurrent create/delete |
| Phase 8 | Turso Platform snapshot、organization/group/location/quota migration、legacy metadata fixture、quota exceeded denial、scope denial |
| Phase 9 | WebSocket hello/subprotocol transcript、stream transaction、store_sql/close_sql、close rollback、multi stream |
| Phase 10 | ATTACH allow/deny、任意 path 拒否、metrics counter/gauge snapshot、HTTP/WS/pipeline 更新点 |
| Phase 11 | replication log/snapshot/heartbeat/status snapshot、frame_no 単調増加、CRC32、replication token denial |
| Phase 12 | replica catch-up、307 redirect、primary down behavior、lag health、checksum mismatch recovery |
| Phase 13 | manifest/frame 双方向整合、retention cleanup、orphan/missing/corrupt frame、restart recovery |
| Phase 14 | backup artifact、restore rollback、PITR 範囲外、CRC 破壊、restore lock、元 DB 保持 |
| Phase 15 | branch create/list/delete、timestamp/frame branch、独立書き込み、source delete denial、restart recovery |
| Phase 16 | extension manifest、sha256/署名検証、allowlist、任意 path 拒否、load/unload/restart |
| Phase 17 | metrics snapshot 永続化、Prometheus text snapshot、quota usage 整合、破損 snapshot recovery、secret 非含有 |
| Phase 18 | leader election、promotion/demotion、network partition、split-brain rejection、term 単調増加、redirect/health |
| Phase 19 | adapter shadow/active/rollback、Phase 1〜18 regression、SDK 互換、WAL consistency、performance baseline |

証跡は repository 内の test fixture、snapshot、CI log、または PR description の実行結果として追跡可能でなければならない。ローカルで確認しただけの説明は証跡として扱わない。

#### 9.1.7 契約トレーサビリティ固定契約

実装 PR は、変更した仕様契約、対応するテスト、保存された証跡を 1 対 1 以上で追跡できなければならない。追跡不能な契約は未検証として扱い、Phase 完了不可とする。

| 契約種別 | ID 形式 | 例 | 必須対応 |
|----------|---------|----|----------|
| API 契約 | `API-P{phase}-{kebab-name}` | `API-P8-turso-database-create` | endpoint test、snapshot、error test |
| 永続化契約 | `PERSIST-P{phase}-{kebab-name}` | `PERSIST-P14-restore-rollback` | atomic write test、restart/rollback fixture |
| エラー契約 | `ERR-{code}` | `ERR-QUOTA_EXCEEDED` | 発火 test、HTTP status/body snapshot |
| 設定契約 | `CFG-P{phase}-{kebab-name}` | `CFG-P11-replication-write-mode` | valid/invalid/default/priority test |
| セキュリティ境界 | `SEC-{boundary}` | `SEC-admin-api` | auth denial、secret redaction、scope denial |
| 互換契約 | `COMPAT-P{phase}-{kebab-name}` | `COMPAT-P8-turso-wrapper` | SDK regression または Turso snapshot |
| 横断ゼロバグ | `ZB-{number}` | `ZB-4` | 該当 Phase の横断 test |

契約 ID は PR description、test 名、snapshot/fixture path のいずれかに含める。完全一致が難しい場合は PR description の traceability table で対応を明示する。

**PR 完了時の追跡表フォーマット：**

| Contract ID | 実装対象 | Test ID / command | Evidence path | Regression | N/A 理由 |
|-------------|----------|-------------------|---------------|------------|----------|
| `API-Px-name` | endpoint / config / metadata / auth | `TC-*` / `ZB-*` / command | snapshot / fixture / log | Phase 1〜x | 該当なしの場合だけ理由を書く |

追跡表の各行は、少なくとも `Contract ID`、`Test ID / command`、`Evidence path` を持つ。`N/A` は、その契約種別が変更対象外である場合だけ許可する。失敗した test、未生成 artifact、手元確認だけの項目を `N/A` にしてはならない。

**テスト種別の最低要件：**

| 契約種別 | 単体テストのみ | snapshot のみ | 手動確認のみ | 必須最低ライン |
|----------|----------------|---------------|--------------|----------------|
| API 契約 | 不可 | 不可 | 不可 | integration test + response snapshot |
| 永続化契約 | 不可 | 不可 | 不可 | failure injection または fixture + restart test |
| エラー契約 | 不可 | 可。ただし発火 test とセット | 不可 | 発火 test + status/body assertion |
| 設定契約 | 不可 | 不可 | 不可 | valid/invalid/default/priority test |
| セキュリティ境界 | 不可 | 不可 | 不可 | auth/scope denial + redaction test |
| Turso 互換 | 不可 | 可。ただし strict compare 必須 | 不可 | Turso snapshot + regression |
| 破壊的操作 | 不可 | 不可 | 不可 | rollback/restart/concurrency test |
| 内部 adapter | 不可 | 不可 | 不可 | shadow/rollback/regression/performance artifact |

手動確認は、自動化できない外部環境依存の補助証跡としてのみ許可する。認証、永続化、破壊的操作、Turso 互換、secret redaction、rollback、concurrency は手動確認だけで完了扱いにしてはならない。

#### 9.1.8 契約カバレッジ固定契約

§9.5 API endpoint 契約表、§9.6 永続化ファイル契約表、§9.7 エラーコード使用契約表、§9.13 設定値契約表、§9.14 セキュリティ境界表は契約 ID の発生源である。実装 PR は、対象 Phase の該当行すべてに契約 ID を割り当て、テストと証跡でカバーしなければならない。

| Source section | Contract ID | Contract summary | Required tests | Required evidence | Owner phase |
|----------------|-------------|------------------|----------------|-------------------|-------------|
| `§9.5` | `API-P{phase}-{kebab-name}` | method/path/auth/status/body/error/persistence/idempotency | normal / invalid / auth / error / idempotency | response snapshot, request fixture, log redaction sample | endpoint の Phase |
| `§9.6` | `PERSIST-P{phase}-{kebab-name}` | file path/schema/update/fsync/corruption/backup | atomic write / corruption / restart / rollback | metadata fixture, crash fixture, recovery log | first write Phase |
| `§9.7` | `ERR-{code}` | code/status/retry/client action |発火 test / status-body assertion / retry decision | error snapshot, triggering fixture | first use Phase |
| `§9.13` | `CFG-P{phase}-{kebab-name}` | CLI/env/TOML/default/invalid handling | default / override priority / invalid / target Phase before-after | config fixture, stderr/log sample | config Phase |
| `§9.14` | `SEC-{boundary}` | untrusted input / required control / forbidden behavior | allow / deny / redaction / bypass attempt | auth matrix, redaction log, denied response | first exposed Phase |

既存表の行に明示 ID が書かれていない場合でも、実装 PR では上表の形式で ID を割り当てる。割り当てた ID は PR description だけでなく、該当する test 名、snapshot path、fixture path、または仕様本文のいずれかに残す。

**契約 ID 欠落時の扱い：**

| 欠落状態 | 判定 |
|----------|------|
| 対象 Phase の §9.5 endpoint に API 契約 ID がない | Phase 未完了 |
| 更新する §9.6 persistence 行に PERSIST 契約 ID がない | Phase 未完了 |
| 発火する §9.7 error code に ERR 契約 ID の test がない | Phase 未完了 |
| 追加/変更する §9.13 config に CFG 契約 ID がない | Phase 未完了 |
| 触れる §9.14 security boundary に SEC 契約 ID の deny/redaction test がない | Phase 未完了 |
| PR description の一時 ID だけで、仕様本文・test・artifact のどこにも残らない | Phase 未完了 |

Phase 完了時は、対象 Phase の API / persistence / error / config / security / compatibility 契約に未カバー行が 0 件でなければならない。`N/A` は、その契約が対象外である理由が §9.2、§9.4、該当 Phase 節、または unsupported 固定表に明記されている場合だけ許可する。

#### 9.1.9 Phase 実装 PR ライフサイクル固定契約

Phase 実装 PR は下表の順序で進める。順序を飛ばした PR は Phase 完了として扱わない。

| Step | Gate | 必須状態 | 失敗時 |
|------|------|----------|--------|
| 1 | Ready | §9.11 の Definition of Ready と §9.17 の実装前チェックリストを満たす | 実装開始禁止。仕様修正 PR に戻す |
| 2 | Contract mapping | §9.1.7 / §9.1.8 の Contract ID、test ID、evidence path を先に割り当てる | 実装開始禁止 |
| 3 | Implementation | 対象 Phase のみ実装し、対象外機能に成功応答を返さない | Phase 未完了 |
| 4 | Evidence generation | snapshot、fixture、log、CI output を生成し secret scan を通す | Phase 未完了 |
| 5 | Regression | 対象 Phase 以前の regression と該当 SDK/Turso/replication/HA/internal tests を通す | Phase 未完了 |
| 6 | Review | §9.12、§9.1.7、§9.1.8 の traceability と coverage を確認する | Phase 未完了 |
| 7 | Merge | 未カバー契約、未検証、既知不具合、未生成証跡が 0 件 | merge 不可 |

**順序違反時の扱い：**

| 違反 | 判定 |
|------|------|
| 実装後に Contract ID を後付けし、test/evidence へ反映していない | Phase 未完了 |
| Contract ID はあるが evidence path が存在しない | Phase 未完了 |
| regression 未完了のまま review/merge へ進む | merge 不可 |
| 仕様未確定のまま実装を開始した | 実装 PR ではなく仕様修正 PR として扱う |
| snapshot 差分を実装都合だけで更新した | review failure |
| `N/A` 理由が仕様本文に存在しない | review failure |

**レビュー自動判定チェック：**

PR review / CI / release-check は最低限、以下を機械的に確認する。自動化されていない場合は Phase 完了不可であり、手動確認だけで代替してはならない。

| Check | 必須判定 |
|-------|----------|
| traceability table | PR description に Contract ID / Test ID / Evidence path が存在する |
| coverage | 対象 Phase の契約 ID に未カバーが 0 件 |
| N/A validation | `N/A` の理由が仕様本文の対象外・unsupported・該当なしに対応している |
| artifact existence | snapshot / fixture / log / CI output の path が存在する |
| secret scan | artifact に JWT、Bearer token、admin/platform/replication/HA token、生 SQL args、backup body が含まれない |
| regression result | 対象 Phase 以前の regression が全件成功している |
| zero-bug gate | 既知不具合、未検証、未生成証跡、TODO/FIXME production path が 0 件 |

Phase 完了 PR は、上記 check がすべて成功しなければならない。1 件でも失敗した場合は、その PR 内で修正し、後続のバグ修正 PR に持ち越さない。

#### 9.1.10 Phase 受入 manifest 固定契約

各 Phase 実装 PR は、実装開始前に Phase 受入 manifest を作成し、PR description または `docs/phase-evidence/phase-{phase}.md` に固定する。manifest は「この Phase が何を実装し、何を実装しないか」「どの契約をどの test / artifact で証明するか」を 1 箇所で読める形にする。

manifest がない Phase 実装 PR は、§9.1.9 の Step 2 `Contract mapping` 未完了として扱う。実装者がコードを読まないと完了条件を判断できる状態は不可とする。

**必須 manifest fields：**

| Field | 必須内容 | 欠落時 |
|-------|----------|--------|
| `Phase` | 対象 Phase 番号と Phase 名 | 実装開始禁止 |
| `Scope in` | この PR で成功応答まで実装する機能一覧 | 実装開始禁止 |
| `Scope out` | 未来 Phase、unsupported、明示対象外の機能一覧と根拠 section | 実装開始禁止 |
| `Contract map` | §9.1.7 / §9.1.8 の Contract ID、対象仕様 section、test ID、evidence path | Phase 未完了 |
| `Endpoint map` | method、path、auth、status、body、error code、idempotency | route 公開禁止 |
| `Persistence map` | file path、schema version、write timing、fsync、corruption、rollback | 書き込み処理実装禁止 |
| `Config map` | CLI/env/TOML/default/invalid/priority/対象 Phase 前挙動 | config 実装禁止 |
| `Security map` | auth boundary、scope、secret redaction、denial case、bypass attempt | success response 公開禁止 |
| `Compatibility map` | Turso/libSQL SDK 互換確認、差分理由、snapshot source | 互換完了不可 |
| `Regression set` | 対象 Phase 以前の regression command と期待結果 | merge 不可 |
| `Manual exception` | 自動化できない確認の理由、owner、期限、代替自動化 Phase | 手動確認のみ不可 |
| `Zero-bug declaration` | 既知不具合、未検証、未生成証跡、TODO/FIXME production path が 0 件である宣言 | merge 不可 |

**manifest の固定フォーマット：**

```markdown

### Phase {phase} Acceptance Manifest

| Field | Value |
|-------|-------|
| Phase | P{phase}: {name} |
| Scope in | ... |
| Scope out | ... |
| Regression set | ... |
| Manual exception | none / ... |
| Zero-bug declaration | known bugs: 0 / unverified: 0 / missing evidence: 0 / production TODO-FIXME: 0 |

| Contract ID | Source section | Implementation target | Test ID / command | Evidence path | Status |
|-------------|----------------|-----------------------|-------------------|---------------|--------|
| API-Px-name | §9.5 / §6.x | endpoint / behavior | TC-* | artifacts/... | planned/pass |
```

`Status` は実装開始前は `planned`、PR 完了時は `pass` または仕様本文に根拠がある `N/A` のみ許可する。`todo`、`later`、`manual only`、`unknown`、空欄は Phase 完了不可である。

**manifest 更新ルール：**

| 変更 | 必須対応 |
|------|----------|
| 実装中に endpoint / schema / error / config / security 境界が変わった | 先に manifest と仕様本文を更新し、Contract ID と test/evidence を再割当する |
| 実装中に対象外機能が必要になった | `Scope out` から `Scope in` へ移す前に仕様変更 PR を先行する |
| artifact path が変わった | manifest、PR description、test output を同時に更新する |
| 手動確認が増えた | `Manual exception` に理由、owner、期限、代替自動化 Phase を追加する |
| regression を削った | 削除理由と代替 test を仕様本文に明記するまで merge 不可 |

Phase 完了 review では、manifest、仕様本文、test 名、artifact path、PR description の 5 点が一致していなければならない。1 つでも不一致がある場合は、実装の正しさではなく受入条件の未確定として扱い、Phase 未完了に戻す。

#### 9.1.11 Evidence artifact 固定契約

各 Phase 実装 PR は、§9.1.10 の Phase 受入 manifest に記載した `Evidence path` に、実際の artifact を生成しなければならない。artifact は「実装が仕様を満たしたこと」を後から再確認できる証跡であり、PR description の文章や手元ログだけでは代替できない。

artifact path は deterministic に固定し、実装者、実行環境、実行時刻によって変わってはならない。timestamp、random ID、host 名、absolute path、local username を path に含めることは禁止する。

**artifact 保存規則：**

| 種別 | 保存先 | 必須命名 | 必須内容 |
|------|--------|----------|----------|
| request fixture | `tests/fixtures/phase-{phase}/{contract-id}/request.json` | Contract ID を directory に含める | request method/path/header subset/body |
| response snapshot | `tests/snapshots/phase-{phase}/{contract-id}/response.json` | Contract ID を directory に含める | status、header subset、body、error code |
| persistence fixture | `tests/fixtures/phase-{phase}/{contract-id}/persistence.json` | Contract ID を directory に含める | file path、schema version、before/after、fsync/rollback 結果 |
| recovery / rollback log | `tests/artifacts/phase-{phase}/{contract-id}/recovery.log` | Contract ID を directory に含める | 起動、破損検出、復旧、rollback の要点 |
| compatibility diff | `tests/artifacts/phase-{phase}/{contract-id}/compat.diff` | Contract ID を directory に含める | Turso/libSQL SDK との差分、正規化後比較結果 |
| CI output | `tests/artifacts/phase-{phase}/{contract-id}/ci.txt` | Contract ID を directory に含める | 実行 command、exit code、pass/fail summary |
| secret scan result | `tests/artifacts/phase-{phase}/{contract-id}/secret-scan.txt` | Contract ID を directory に含める | scan command、対象 path、検出 0 件の結果 |
| performance baseline | `tests/artifacts/phase-{phase}/{contract-id}/performance.json` | Contract ID を directory に含める | p95、RSS、DB size、WAL size、測定条件 |

`contract-id` は小文字化せず、§9.1.7 / §9.1.8 で割り当てた Contract ID と完全一致させる。ファイルシステム都合で大文字小文字が不安定になる環境を考慮し、同一 directory 内に大文字小文字だけが異なる Contract ID を作ってはならない。

**canonical artifact path manifest audit：**

Phase implementation packet の `artifact_paths` は、以下のカテゴリを固定 key として持たなければならない。該当しないカテゴリは空欄にせず、仕様本文の対象外根拠 section と `N/A` 理由を明記する。根拠なし `N/A`、PR description だけの path、実行時に変わる path は Phase 完了根拠にしてはならない。

| artifact category | 必須内容 |
|-------------------|----------|
| `request_fixture_paths` | request fixture の path、対象 Contract ID、source section |
| `response_snapshot_paths` | response snapshot の path、status/code/body 正規化 rule |
| `persistence_fixture_paths` | metadata / file / DB / WAL / manifest の before/after fixture path |
| `recovery_log_paths` | restart、rollback、corruption、operator_required の log / trace path |
| `compatibility_diff_paths` | Turso Cloud / libSQL SDK / previous Phase との差分 artifact path |
| `ci_output_paths` | format、unit、integration、SDK、Turso、release-check の output path |
| `secret_scan_paths` | artifact / log / snapshot / packet に対する secret scan result path |
| `review_handoff_paths` | reviewer が clean checkout で再現する handoff packet / command / expected artifact path |
| `precision_closure_paths` | `P{phase}-PRECISION-CLOSURE` の 11 closure field に対応する artifact path |

各 artifact entry は以下の field を必ず持つ。

| field | 必須値 |
|-------|--------|
| `contract_id` | §9.1.7 / §9.1.8 / §9.11 の Contract ID と完全一致 |
| `source_section` | artifact が証明する仕様 section |
| `artifact_path` | repository root 相対 path。local absolute path、timestamp、random ID、username、hostname を含めない |
| `producer_command` | artifact を生成した再実行可能 command |
| `expected_hash` | 正規化済み artifact 本文の sha256 |
| `secret_scan_result` | `pass` または blocking failure。未実行、manual only、対象外根拠なし `N/A` は不可 |
| `normalized_fields` | 正規化した timestamp、request id、path、host、port、secret、SQL args、payload の一覧 |
| `reviewer_command` | reviewer が artifact existence、hash、secret scan を再検証できる command |

`artifact_paths_result = pass` にできるのは、readiness packet、Phase 受入 manifest、Done receipt、artifact manifest、review handoff の `artifact_path` がすべて一致し、Contract ID 未接続 0 件、`expected_hash` 未定義 0 件、secret scan 未実行 0 件、raw local path / timestamp / username / hostname 混入 0 件の場合だけである。1 件でも不一致がある場合は Phase 未完了とし、artifact だけを差し替えて manifest、Done receipt、review handoff を更新しないことを禁止する。

**正規化必須 field：**

| Field | 正規化値 | 理由 |
|-------|----------|------|
| timestamp / datetime | `<normalized-time>` | 実行時刻差分を禁止 |
| request id / trace id | `<normalized-id>` | 実行ごとの差分を禁止 |
| absolute path | `<normalized-path>` | local 環境差分と user name 混入を禁止 |
| host / port | `<normalized-host>` / `<normalized-port>` | CI と local 差分を禁止 |
| JWT / Bearer token / secret | `<redacted-secret>` | secret 混入禁止 |
| SQL bind value が機微情報の場合 | `<redacted-arg>` | user data 混入禁止 |
| backup body / replication payload の機微 field | `<redacted-payload>` | 大容量 secret / data 混入禁止 |

正規化前の raw artifact は repository にコミットしてはならない。raw artifact が必要な検証は CI 内一時領域だけに保存し、repository に残す artifact は正規化後のみとする。

**artifact 欠落・不一致時の扱い：**

| 状態 | 判定 |
|------|------|
| manifest の `Evidence path` に実ファイルが存在しない | Phase 未完了 |
| artifact path に Contract ID が含まれない | Phase 未完了 |
| artifact 内容が該当 Contract ID と無関係 | review failure |
| artifact に secret / token / raw SQL args / local username が含まれる | merge 不可 |
| snapshot が実装都合で更新され、仕様差分理由がない | review failure |
| artifact が timestamp / random ID を含む path に保存される | Phase 未完了 |
| PR description だけに証跡があり repository に artifact がない | Phase 未完了 |
| 手動確認ログのみで自動 test / snapshot がない | Phase 未完了。ただし §9.1.10 の Manual exception が仕様本文に根拠付きである場合のみ補助証跡として許可 |

artifact は Phase 完了 PR と同じ commit に含める。後続 PR で artifact だけを追加して Phase 完了扱いにすることは禁止する。artifact を更新する場合は、対応する仕様 section、Contract ID、test ID、manifest、PR description を同時に更新しなければならない。

#### 9.1.12 仕様矛盾解消固定契約

仕様書内の複数 section が同じ対象について異なる挙動、status、error code、永続化 schema、設定優先順位、認証境界、Phase 境界、証跡要件を示す場合、その状態を仕様矛盾とみなす。仕様矛盾がある対象は、実装者の判断で実装してはならない。

**仕様優先順位：**

| 優先順位 | Source | 適用範囲 |
|----------|--------|----------|
| 1 | §1.4 設計不変条件 | 全 Phase、全機能の最上位制約 |
| 2 | §9.1〜§9.17 の実装固定契約 | Phase 完了条件、契約 ID、証跡、ゼロバグ判定 |
| 3 | §9.2 Phase 別完了ゲート / §9.4 unsupported 固定表 | Phase 境界、対象外、前倒し可否 |
| 4 | §9.5 API endpoint 契約表 / §9.6 永続化ファイル契約表 / §9.7 エラーコード使用契約表 / §9.13 設定値契約表 / §9.14 セキュリティ境界表 | 実装対象の具体契約 |
| 5 | 各 Phase 詳細節 | Phase 内の補足仕様 |
| 6 | Rust コード例、JSON 例、CLI 例、説明文中の例 | 実装参考。上位契約と矛盾する場合は上位契約を正とする |

上位 Source と下位 Source が矛盾する場合は、上位 Source を一時的な正とする。ただし、下位 Source を放置したまま実装 PR を進めてはならない。矛盾を発見した PR は、実装前に仕様修正 PR として矛盾箇所を解消する。

**矛盾の種類と必須対応：**

| 矛盾 | 必須対応 |
|------|----------|
| Phase 境界と詳細節が異なる | §9.2 / §9.4 と詳細節を同じ PR で修正する |
| API status/body と error code 表が異なる | §9.5、§9.7、該当 API 節、snapshot 期待値を同時更新する |
| persistence schema と Phase 詳細が異なる | §9.6、該当 Phase 節、migration / rollback 契約を同時更新する |
| config default / priority が複数箇所で異なる | §9.13、CLI/env/TOML 例、manifest の Config map を同時更新する |
| security boundary と API 説明が異なる | §9.14、該当 API 節、deny/redaction test を同時更新する |
| Turso Cloud 互換方針と自己ホスト差分が異なる | §1.4、§9.4、該当 Phase 節、compatibility diff を同時更新する |
| コード例 / JSON 例 / CLI 例だけが本文と異なる | 例を本文に合わせる。本文を変える場合は上位契約も更新する |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 矛盾を PR description だけで説明し、仕様本文を直さない | Phase 未完了 |
| 上位 Source だけを直し、下位 Source の矛盾を残す | review failure |
| test 期待値だけを直し、仕様本文を直さない | review failure |
| manifest だけを直し、Contract ID / artifact path / 仕様 section を直さない | Phase 未完了 |
| 「実装上はこちらを採用」として仕様矛盾を残す | merge 不可 |
| 未来 Phase で直す前提で現在 Phase の成功応答を公開する | merge 不可 |

矛盾解消後は、該当 Contract ID、manifest、test name、artifact path、PR description の追跡表が同じ挙動を指していなければならない。1 つでも古い仕様を参照している場合は、矛盾未解消として扱う。

#### 9.1.13 Verification command 固定契約

各 Phase 実装 PR は、完了判定に使う verification command を実装開始前に Phase 受入 manifest の `Regression set` と PR description に固定する。固定されていない command の成功は完了根拠として扱わない。

verification command は deterministic に実行できなければならない。実行者のローカル環境、手順記憶、IDE、手動クリック、外部サービスの一時状態に依存する確認は、§9.1.10 の `Manual exception` に根拠がない限り完了根拠として使えない。

**必須 command 分類：**

| 分類 | 必須 command | 適用 Phase | 必須 artifact |
|------|--------------|------------|---------------|
| format / lint | repository 標準の format / lint command。存在しない場合は manifest に `N/A` 理由を明記 | all | `tests/artifacts/phase-{phase}/{contract-id}/ci.txt` |
| unit | 対象 crate/module の unit test | all implementation phases | `ci.txt` |
| integration | HTTP / WebSocket / CLI / persistence の integration test | Phase 3 以降、または外部 API を持つ Phase | `ci.txt`、request fixture、response snapshot |
| SDK compatibility | libSQL client SDK 互換 test | hrana / client API を変更する Phase | SDK transcript、compat diff |
| Turso compatibility | Turso Cloud snapshot / behavior compare | Turso Cloud 互換 API / metadata / auth / quota / branch / platform API を変更する Phase | `compat.diff` |
| persistence / restart / rollback | fsync、restart、corruption、rollback、migration test | 永続化 schema / metadata / backup / branch / HA を変更する Phase | persistence fixture、recovery log |
| security / secret scan | auth denial、scope denial、redaction、artifact secret scan | 認証、認可、secret、artifact を扱う Phase | secret-scan result、denied response |
| release-check | release-check script または同等の repository 標準 release validation | release / packaging / CI 完了判定を行う Phase | `ci.txt` |

該当する分類が存在するのに command が未定義の場合、その Phase は未完了である。`N/A` は、対象機能が §9.2、§9.4、該当 Phase 節、または unsupported 固定表で明示対象外の場合だけ許可する。

**実行順序：**

| Order | Gate | 失敗時 |
|-------|------|--------|
| 1 | format / lint | 実装修正。後続 gate に進まない |
| 2 | unit | 実装修正。integration に進まない |
| 3 | integration | 実装または仕様矛盾を修正する |
| 4 | SDK / Turso compatibility | 互換差分を仕様化するか実装修正する |
| 5 | persistence / restart / rollback | 永続化・migration・rollback 契約を修正する |
| 6 | security / secret scan | secret 混入を除去し、deny/redaction test を追加する |
| 7 | release-check | release / packaging / CI 契約を修正する |

上記順序を飛ばして後段 command だけを成功させても Phase 完了扱いにしない。前段 command の失敗を `known issue`、`flaky`、`後で確認` として残すことは禁止する。

**exit code と証跡：**

| 状態 | 判定 |
|------|------|
| command の exit code が `0` | 完了根拠として利用可 |
| command の exit code が非 `0` | Phase 未完了 |
| command 未実行 | Phase 未完了 |
| command 名、引数、対象 path が artifact に残っていない | Phase 未完了 |
| CI output artifact と manifest の `Regression set` が一致しない | review failure |
| ローカル実行の口頭説明だけで artifact がない | Phase 未完了 |
| flaky test を再実行で通したが失敗ログを残していない | review failure |
| `--ignored`、`--skip`、filter で対象 test を外した | Phase 未完了。ただし除外理由が仕様本文にある場合のみ `N/A` 可 |

verification command の追加・削除・引数変更は、manifest、PR description、CI output artifact、該当 Contract ID の追跡表を同時に更新しなければならない。検証 command を変更しただけで仕様本文を更新しない PR は Phase 完了不可である。

#### 9.1.14 Failure closure 固定契約

各 Phase 実装 PR は、検証中に発見した失敗、flaky、未検証、artifact 欠落、secret 混入、仕様矛盾、TODO/FIXME production path を同一 PR 内で解消しなければならない。後続 PR、別 issue、運用メモ、口頭説明へ持ち越した時点で、その Phase は未完了である。

failure closure は「失敗を隠す」ことではなく、失敗の原因、修正、再検証、証跡を同じ追跡単位で閉じることを意味する。修正後に pass した結果だけでなく、発見した失敗の分類と再発防止の evidence を残す。

**closure 必須対象：**

| 対象 | 必須 closure | 未完了判定 |
|------|--------------|------------|
| verification command failure | 原因、修正 commit、再実行 command、exit code 0 の artifact | 非 0 exit code、未再実行、artifact 欠落 |
| flaky test | flaky 原因、deterministic 化修正、失敗ログ、再実行 pass artifact | 再実行成功だけ、失敗ログなし、原因未特定 |
| unverified contract | Contract ID、追加 test、evidence path、manifest 更新 | `N/A` で隠す、手動確認のみ |
| missing artifact | 正規化済み artifact 生成、manifest / PR description path 更新 | PR description の説明のみ |
| secret leakage | 漏洩 artifact 削除、redaction 修正、secret scan 再実行 | secret を含む artifact が残る |
| spec conflict | §9.1.12 に従う同時仕様修正、test / artifact 更新 | 上位仕様だけ修正、下位矛盾放置 |
| TODO/FIXME/stub/unimplemented/panic/unwrap production path | production path から除去、または対象 Phase 前 stub 契約へ移動 | 完了 PR に残存 |
| regression failure | 失敗原因、修正、対象 Phase 以前の regression 再実行 | regression 範囲縮小、skip、後続対応 |

**canonical failure remediation and re-review audit：**

Phase 実装中または review 中に失敗を検出した場合、実装者は失敗を以下の分類へ必ず割り当て、分類ごとの順序で修正、再検証、artifact 再生成、review handoff 更新を行う。分類できない失敗は `unknown` として処理せず、先に仕様本文へ failure class を追加する。

| failure class | 標準修正順序 |
|---------------|--------------|
| `spec_conflict` | §9.1.12 に従い仕様矛盾を解消し、Contract ID、manifest、oracle、scenario、artifact path を同時更新する |
| `contract_mapping_gap` | Contract ID / Task ID / Scenario ID / source section の未接続を先に閉じ、test と artifact を再割当する |
| `artifact_missing` | artifact を正規化済みで生成し、manifest、Done receipt、review handoff、hash を同時更新する |
| `hash_mismatch` | artifact 正規化 rule、producer command、expected hash を再生成し、古い hash を残さない |
| `secret_leak` | 漏洩 artifact を除去し、redaction rule を修正し、secret scan を全 artifact に再実行する |
| `command_failed` | failed command の root cause を修正し、同一 command を exit code 0 で再実行する |
| `flaky_or_timeout` | sleep / timing / network 依存を deterministic wait、fixture、timeout 契約へ置換し、失敗ログと pass artifact を両方残す |
| `compatibility_diff` | Turso Cloud / libSQL SDK / previous Phase との差分を仕様化または実装修正し、compat diff と SDK transcript を再生成する |
| `regression_failure` | 対象 Phase 以前の regression を修正後に全件再実行し、範囲縮小、skip、後続対応を禁止する |
| `operator_observability_gap` | health、log、metric、operator action、release note、rollback evidence を補完し、operator delta を更新する |

各 failure entry は以下の field を必ず持つ。

| field | 必須値 |
|-------|--------|
| `failure_id` | `FAIL-P{phase}-{n}` 形式の一意 ID |
| `failure_class` | 上記 failure class のいずれか |
| `source_contract_id` | 失敗した Contract ID。複数ある場合は全件列挙 |
| `failed_command` | 失敗を検出した command。artifact 欠落など command がない場合は検出手順 |
| `failed_artifact_path` | 失敗ログ、差分、欠落 path、secret scan result の path |
| `root_cause` | 原因。`unknown`、`環境差分`、`一時的` だけでは不可 |
| `fix_scope` | 修正対象。仕様、実装、test、artifact、manifest、review handoff、operator delta のどれを更新したか |
| `required_reverification` | 再実行する command、対象 Contract ID、期待 exit code、必要 artifact |
| `regenerated_artifacts` | 再生成した artifact path と expected hash |
| `reviewer_command` | reviewer が失敗再現不可または修正後 pass を確認できる command |
| `closure_result` | `pass` のみ完了扱い。`partial`、`not_run`、`manual_only`、`accepted_risk` は不可 |

`failure_remediation_result = pass` にできるのは、未分類 failure 0 件、`root_cause` 未記載 0 件、再実行 command 未記載 0 件、artifact 再生成漏れ 0 件、regression 再実行漏れ 0 件、secret scan 再実行漏れ 0 件、`closure_result != pass` 0 件の場合だけである。失敗を修正した場合は、対応する readiness packet、Phase 受入 manifest、Done receipt、artifact manifest、review handoff、operator delta を同じ PR で更新しなければならない。

**禁止語句と扱い：**

| PR / manifest / artifact に残る語句 | 判定 |
|-------------------------------------|------|
| `known issue` / `known bug` | merge 不可 |
| `later` / `follow-up` / `next PR` | Phase 未完了 |
| `temporary` / `workaround` | 仕様本文に期限・owner・解消 Phase がない限り Phase 未完了 |
| `flaky but passed` | merge 不可 |
| `manual checked` | §9.1.10 の Manual exception がない限り Phase 未完了 |
| `TODO` / `FIXME` / `unimplemented` / `panic!` / unchecked `unwrap` in production path | Phase 未完了 |
| `後で修正` / `後で検証` / `一旦 merge` | merge 不可 |

**closure artifact 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `Failure ID` | `FAIL-P{phase}-{surface}-{number}` の形式 |
| `Related Contract ID` | 失敗に対応する Contract ID。横断失敗は `ZB-*` も併記 |
| `Failure class` | command failure / flaky / unverified / artifact / secret / spec conflict / production TODO / regression |
| `Root cause` | 実装、仕様、test、fixture、環境差分のいずれか |
| `Fix summary` | 同一 PR 内での修正内容 |
| `Re-run command` | §9.1.13 の command と完全一致 |
| `Evidence path` | pass artifact、失敗ログ、修正後 snapshot / fixture |
| `Closure status` | `closed` のみ許可。`open`、`deferred`、`accepted risk` は Phase 未完了 |

failure closure の追跡表は PR description または `docs/phase-evidence/phase-{phase}.md` に残す。closure artifact が存在しない失敗は、修正済みであっても Phase 完了根拠として扱わない。失敗を見つけた後に仕様変更で対象外へ移す場合も、§9.1.12 に従って仕様本文、manifest、Contract ID、test、artifact を同時更新しなければならない。

#### 9.1.15 Backward compatibility / migration 固定契約

既存 endpoint、wire schema、metadata schema、config default、error code、JWT claim、SDK 互換挙動、永続化 file path を変更する Phase 実装 PR は、実装開始前に後方互換と migration の契約を固定しなければならない。既存データ、既存 config、既存 client request が存在する状態で起動・接続・操作できることを完了条件に含める。

後方互換は「新規環境で動く」ことではない。旧仕様で生成された metadata / config / token / DB path / request fixture を読み、必要な migration を行い、失敗時に安全に rollback または起動失敗できることを意味する。

**互換影響の分類：**

| 変更対象 | 必須互換契約 | 必須 evidence |
|----------|--------------|---------------|
| API endpoint | 旧 method/path/query/body/header の扱い、deprecated field、unknown field 方針、status/error mapping | old/new request fixture、response snapshot、SDK regression |
| hrana wire schema | 旧 SDK request、unknown field、baton/stream/args 互換、error response 互換 | libSQL SDK transcript、wire snapshot |
| metadata schema | schema version、追加 field default、旧形式 migration、破損時挙動、rollback | old/new/corrupt fixture、migration log、restart test |
| config default / priority | 旧 config 読み込み、default 変更理由、CLI/env/TOML 優先順位、invalid value | old/new config fixture、stderr/log snapshot |
| error code / status | 旧 client が期待する status/code、retry 可否、Turso Cloud 差分 | error snapshot、client action matrix |
| JWT / token claims | 旧 token の扱い、claim 追加時 default、scope 解決順、revoke 互換 | old/new token fixture、auth denial test |
| persistence path | 旧 directory/file 名、移動手順、fsync、rollback、partial migration 検出 | path fixture、failure injection、recovery log |
| SDK / Turso compatibility | 旧 SDK version、Turso snapshot 差分、自己ホスト差分理由 | SDK transcript、compat diff |

**migration plan 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `Migration ID` | `MIG-P{phase}-{name}` の形式 |
| `Source schema` | 旧 schema version、旧 file path、旧 field 一覧 |
| `Target schema` | 新 schema version、新 field、default、必須/任意 |
| `Trigger` | 起動時、API 実行時、admin command 実行時のいずれで migration するか |
| `Atomicity` | tmp write、fsync、rename、commit marker、multi-file commit 順序 |
| `Rollback` | commit 前失敗、commit 後失敗、再起動後検出時の扱い |
| `Idempotency` | 再実行時に二重変換・二重削除・二重課金を起こさない条件 |
| `Compatibility window` | 旧形式を読み続ける Phase、または削除禁止理由 |
| `Evidence` | old/new/corrupt fixture、restart test、rollback log、CI output |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 旧 metadata / config / token を手動修正前提にする | Phase 未完了 |
| 新規環境だけで test し、旧形式 fixture がない | Phase 未完了 |
| migration 失敗時に部分更新済み状態で起動成功する | merge 不可 |
| rollback plan なしで destructive operation を行う | merge 不可 |
| 旧 endpoint / field を仕様本文なしに削除する | review failure |
| error status / code を変更し、旧 client action を定義しない | Phase 未完了 |
| Turso Cloud 互換差分を自己ホスト都合だけで隠す | merge 不可 |
| migration artifact が Contract ID / Migration ID と紐づかない | Phase 未完了 |

破壊的変更が必要な場合は、同じ PR で §9.2、§9.5、§9.6、§9.7、§9.13、§9.14、該当 Phase 詳細、manifest、test、artifact を更新し、既存利用者への互換維持策または段階的移行策を明記する。互換維持策がない破壊的変更は、実装都合があっても Phase 完了として扱わない。

#### 9.1.16 Concurrency / idempotency / shutdown 固定契約

状態変更を伴う Phase 実装 PR は、同時 request、resource lock、idempotency、shutdown 中の扱いを実装開始前に固定しなければならない。並行実行で lost update、二重作成、二重削除、二重 revoke、partial commit、成功応答後 rollback が発生する可能性が残る場合、その Phase は未完了である。

**resource lock 境界：**

| 対象 | lock 単位 | 同時実行時の固定挙動 | 必須 evidence |
|------|-----------|----------------------|---------------|
| DB create / delete / update | DB name + organization/group scope | 同一 DB への create/update/delete は直列化。競合 create は `409 DB_ALREADY_EXISTS`、削除中 update は `404 DB_NOT_FOUND` または `409 STORAGE_BUSY` | concurrency test、metadata before/after fixture |
| token create / revoke | token id + tokens.json | revoke は冪等 204。create は token id 重複禁止。revoke と auth check の順序を固定 | auth race test、tokens fixture |
| quota / usage update | organization/group/DB quota key | usage 計算と write 判定の間で quota 超過を見逃さない。超過時は commit 前に拒否 | quota race test、denied response |
| backup | source DB | read lock または Online Backup API 相当の一貫 snapshot。backup 中 write の可否を Phase 節で固定 | backup consistency artifact |
| restore / PITR | target DB exclusive lock | restore 中 write は `503 STORAGE_BUSY`。read は旧 DB 継続または `503` のどちらかを Phase 節で固定 | restore lock test、rollback log |
| branch create / delete | source DB + branch DB | 同名 branch create は `409 DB_ALREADY_EXISTS`。delete は明記された場合のみ冪等 204 | branch race fixture |
| migration | migration id + metadata set | commit marker 前の中断は rollback または起動失敗。commit marker 後は再実行しない | interrupted migration fixture |
| WebSocket transaction | connection id + stream id | stream ごとに transaction 状態を分離。connection close 時の open tx は rollback | WebSocket transcript |
| metrics / counters | metric key | lost increment 禁止。restart 永続化対象は snapshot write と shutdown write の競合を固定 | counter race artifact |

**HTTP method idempotency：**

| Method | 既定 | 例外を許可する条件 |
|--------|------|--------------------|
| `GET` | 副作用禁止。同一 request の再試行で状態を変更しない | metrics read 時の内部 read counter など、仕様本文に副作用が明記される場合のみ |
| `POST` | 非冪等。重複 request は二重作成または二重実行を防ぐ契約が必要 | endpoint ごとに idempotency key、natural key、または conflict response が仕様化される場合 |
| `PATCH` | resource version または lock により lost update を防ぐ | 全 field が上書きではなく merge semantics として仕様化され、競合 test がある場合 |
| `DELETE` | endpoint 表で冪等 204 と明記された場合だけ冪等。未記載なら存在しない resource は 404 | Turso Cloud 互換 endpoint が冪等削除を要求する場合 |

**shutdown 中の固定挙動：**

| 状態 | 必須挙動 |
|------|----------|
| shutdown signal 受信後の新規 request | listener を閉じる。受信済みで処理未開始の request は `503 STORAGE_BUSY` または connection close のどちらかを Phase 節で固定 |
| commit 前の write request | 成功応答を返してはならない。rollback 可能なら rollback、不能なら recovery marker を残す |
| commit 後・応答前の write request | 再試行時に二重実行しない。idempotency / conflict / read-after-write で結果を確認できること |
| WebSocket open transaction | connection close または shutdown timeout で rollback。commit 完了前の close を成功扱いにしない |
| restore / migration / backup 中 | restore/migration は commit marker と rollback を優先。backup は一貫 snapshot 以外を返さない |
| shutdown timeout 超過 | 途中成功応答を作らない。次回起動時 recovery が完了するまで該当 resource を成功扱いにしない |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 同一 resource の並行 create で二重 metadata が作成される | merge 不可 |
| read-modify-write に lock または version check がない | Phase 未完了 |
| commit 前に success response を返す | merge 不可 |
| shutdown 中の partial write を次回起動で成功扱いにする | merge 不可 |
| retry により token revoke、quota charge、restore、branch create が二重適用される | Phase 未完了 |
| concurrency test が手動確認のみ | Phase 未完了 |
| lock timeout / busy error が §9.7 の error code に紐づかない | review failure |

並行性に関係する仕様変更は、manifest の `Endpoint map`、`Persistence map`、`Regression set`、該当 Contract ID、concurrency artifact を同時更新する。並行性 test がない状態で「単体では動く」ことを Phase 完了根拠にしてはならない。

#### 9.1.17 Observability / audit / redaction 固定契約

各 Phase 実装 PR は、ログ、request id、trace id、監査相当記録、metric、秘匿、運用証跡の扱いを実装開始前に固定しなければならない。状態変更、認証、権限拒否、quota、replication、backup/restore、branch、extension、HA、internal adapter に関係する変更は、観測不能または secret 漏洩の可能性が残る場合、その Phase は未完了である。

**request id / trace id：**

| 項目 | 固定仕様 |
|------|----------|
| request id 生成 | inbound に `x-request-id` がある場合は安全な長さ・文字種に正規化して使用する。ない場合は server が生成する |
| trace id | Phase 内で分散 trace を実装しない場合でも、log / artifact では `request_id` で同一 request を追跡できること |
| response header | public/admin API は `x-request-id` を返す。WebSocket は transcript artifact に connection id と stream id を残す |
| snapshot 正規化 | request id / trace id / connection id は artifact 上で `<normalized-id>` に置換する |
| 禁止 | request id に Authorization、JWT、SQL、path の絶対値、user input body 全体を入れてはならない |

**ログ level 固定表：**

| Level | 発火条件 | 禁止内容 |
|-------|----------|----------|
| `INFO` | 起動、停止、request 完了、admin 操作成功、migration 完了、backup/restore/branch 成功 | token、SQL args、backup body、secret value |
| `WARN` | auth disabled、unsupported/stub、非推奨 config、recoverable corruption、retryable replication lag、Phase 前 config 無効化 | request body 全体、secret、絶対 extension path |
| `ERROR` | 起動失敗、永続化破損、migration/restore rollback 不能、secret scan failure、HA split-brain、unrecoverable adapter error | panic backtrace に secret/raw path/body を含めること |
| `DEBUG` / `TRACE` | 開発補助。production default では無効 | INFO/WARN/ERROR で禁止された値すべて |

**出力禁止 field：**

| 種別 | 禁止対象 | 代替表現 |
|------|----------|----------|
| auth | Authorization header、JWT、Bearer token、admin/platform/replication/HA token | `<redacted-secret>` |
| SQL | SQL bind args、生 SQL args、user data row value | count、type、statement kind |
| backup / restore | backup body、uploaded DB body、replication frame bytes | byte length、checksum、frame_no |
| filesystem | extension 絶対 path、data-dir 絶対 path、local username | logical resource name、`<normalized-path>` |
| config | secret file contents、env secret value | key name、source type、redacted marker |
| error | upstream error message 内の secret / path / SQL args | sanitized error code + redacted message |

**audit 相当記録：**

| 操作 | 必須記録 | Phase 対象外の場合 |
|------|----------|-------------------|
| token create / revoke | actor scope、target token id、result、request id。secret は記録しない | manifest に対象外理由を記載 |
| DB create / delete / configuration update | actor scope、organization/group/db、operation、result | Phase 前 route は 501 stub log のみ |
| quota / usage enforcement | db/org/group、limit、usage、denied/allowed、request id | quota 未実装 Phase は成功応答禁止 |
| backup / restore / PITR | db、operation、size、checksum、commit/rollback result | backup body は記録禁止 |
| replication / HA | role、node id、frame_no、term、leader、result | token/frame bytes は記録禁止 |
| extension load / unload | extension name、version、sha256、state、result | absolute path は記録禁止 |
| internal adapter switch | flag、adapter name、mode、compat result、rollback result | silent fallback 禁止 |

audit 相当記録は、Phase 8 の `/v1/organizations/{org}/audit-logs` API を実装することを意味しない。audit logs API が unsupported の Phase では、内部 log / artifact として記録し、API は該当 unsupported 固定表に従う。

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| log snapshot | INFO/WARN/ERROR の代表 log、request id、status、duration、result |
| redaction sample | token、JWT、SQL args、backup body、extension path が redacted されること |
| secret scan result | log artifact / snapshot / fixture に secret がないこと |
| audit matrix | 対象操作、記録 field、禁止 field、対象外理由 |
| failure log | rollback、migration failure、auth denial、quota denial の sanitized log |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| secret / token / SQL args / backup body / absolute extension path が log または artifact に残る | merge 不可 |
| state-changing operation に request id 付き log / audit 相当記録がない | Phase 未完了 |
| auth denial / scope denial / quota denial の log redaction test がない | Phase 未完了 |
| unsupported/stub log に request body、token、upload binary が出る | merge 不可 |
| DEBUG/TRACE なら secret を出してよい扱いにする | merge 不可 |
| audit logs API 未実装を理由に内部記録も省略する | Phase 未完了 |

observability に関係する仕様変更は、manifest の `Security map`、`Regression set`、該当 SEC 契約 ID、log artifact、secret scan artifact を同時更新する。ログが「見やすい」だけで、秘匿・追跡・失敗調査の契約を満たさない場合は Phase 完了扱いにしない。

#### 9.1.18 Auth / permission / quota precedence 固定契約

各 Phase 実装 PR は、認証、認可、scope、ro/rw、organization/group、quota、usage、block policy、delete protection、replication/HA token の判定順を実装開始前に固定しなければならない。複数の拒否条件が同時に成立する場合でも、endpoint ごとに返す status/code が揺れてはならない。

**共通判定順：**

| Order | Gate | 失敗時の既定 error | 備考 |
|-------|------|--------------------|------|
| 1 | request parsing / size / method / content type | `400 INVALID_REQUEST` または `405 METHOD_NOT_ALLOWED` | 認証不要で公開してよい validation だけを先に行う |
| 2 | required auth presence | `401 AUTH_REQUIRED` | Authorization header なし、必要 token 未設定時 |
| 3 | auth format / signature / exact match / expiry / revoke | `401 AUTH_INVALID` または `401 AUTH_EXPIRED` | token 値の補正、trim、大小文字補正は禁止 |
| 4 | actor scope | `403 ORG_SCOPE_DENIED` または `403 PERMISSION_DENIED` | org/group/db/replica/HA scope 外 |
| 5 | resource existence | `404 DB_NOT_FOUND` 等 | scope 外 resource の存在を漏らす endpoint は §9.5 で例外明記 |
| 6 | operation permission | `403 PERMISSION_DENIED` | ro token write、admin 権限不足、block_reads/block_writes |
| 7 | protection policy | `403 ORG_SCOPE_DENIED` または `403 PERMISSION_DENIED` | delete_protection、allow_attach=false 等 |
| 8 | usage availability / quota | `503 USAGE_UNAVAILABLE` または `403/402 QUOTA_EXCEEDED` | usage 不明は quota 超過より先に返す |
| 9 | storage / lock / concurrency | `503 STORAGE_BUSY` | restore lock、DB busy、shutdown 中 |
| 10 | operation execution | endpoint 固有 error | SQL / replication / backup / extension / HA 固有 error |

上記順序は既定であり、Turso Cloud 互換 endpoint が異なる status を要求する場合だけ、該当 endpoint 節に差分理由と snapshot を明記して上書きできる。上書きしても `code` は §7.3 / §9.7 と一致させる。

**API 種別ごとの auth source：**

| API surface | Required auth | Scope / permission | Quota / block 適用 |
|-------------|---------------|--------------------|--------------------|
| hrana HTTP `/v2/pipeline` | JWT。Phase 4 前は auth disabled のみ | `a`、`dbs`、org/group/db scope、ro/rw | write/import 相当 SQL は quota/block_writes。read SQL は block_reads |
| hrana WebSocket `/v3/baton` | JWT | connection DB と stream operation ごとに判定 | transaction 内でも operation ごとに判定 |
| `/admin/v1/*` | Admin token | Admin token は管理 API 全体。ただし org/group scope 拡張が仕様化された Phase では scope を適用 | DB write、restore、branch、quota update に適用 |
| `/v1/*` Turso Platform API | Platform token または Phase 8 の Admin token 代用 | organizationSlug / groupName / databaseName の scope を先に判定 | `/v1/*` quota 超過は Turso 互換で 402 を優先 |
| replication API | replication token | primary/replica role、replica_id、frame range | apply/write 側のみ quota/block_writes |
| HA API | Admin token + HA token | node_id、role、term、leader state | quota は適用しない |
| health / metrics read | endpoint ごとの auth | read-only scope または admin scope | metrics read は quota 超過中も許可 |

**複数拒否条件時の優先 error：**

| 条件 | 返す error |
|------|------------|
| auth header なし + resource 不存在 | `401 AUTH_REQUIRED` |
| token 不正 + scope 外 | `401 AUTH_INVALID` |
| token 期限切れ + revoke 済み | `401 AUTH_EXPIRED` |
| org scope 外 + DB 不存在 | `403 ORG_SCOPE_DENIED`。存在漏洩を避ける |
| ro token write + quota 超過 | `403 PERMISSION_DENIED` |
| block_writes=true + quota 超過 | `403 PERMISSION_DENIED`。ただし `/v1/*` で quota status 互換が明記される場合は `402 QUOTA_EXCEEDED` |
| usage unavailable + quota 超過推定 | `503 USAGE_UNAVAILABLE` |
| delete_protection=true + DB not found | `404 DB_NOT_FOUND` |
| restore lock + auth failure | auth failure を優先 |
| shutdown 中 + auth failure | auth failure を優先。auth 通過後は shutdown/storage busy |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| auth matrix | no auth / bad format / invalid / expired / revoked / valid |
| scope matrix | global、dbs、org、group、platform、replication、HA の allow/deny |
| permission matrix | ro/rw、read/write SQL、admin operation、restore、branch、extension |
| quota matrix | allowed、quota exceeded、usage unavailable、Turso 402、admin/hrana 403 |
| block policy matrix | block_reads、block_writes、delete_protection、allow_attach=false |
| precedence snapshot | 複数拒否条件が同時に成立した時の status/code |
| redaction evidence | denied log に token、JWT、SQL args、quota internals が出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| endpoint ごとに auth / scope / quota の判定順が仕様なしに異なる | Phase 未完了 |
| scope 外 resource の存在有無が error で漏れる | merge 不可。ただし仕様本文に例外がある場合のみ可 |
| ro/rw、block、quota の優先順位を test していない | Phase 未完了 |
| `/admin/v1/*` と `/v1/*` の status 差分理由が仕様本文にない | review failure |
| replication / HA token を admin token と混同する | merge 不可 |
| auth denial log に token や JWT claim raw value を出す | merge 不可 |

認証・認可に関係する仕様変更は、manifest の `Security map`、§9.14 security boundary、該当 SEC 契約 ID、auth/scope/quota evidence、redaction artifact を同時更新する。正常系だけが通っても、拒否条件の precedence が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.19 Configuration resolution / validation 固定契約

各 Phase 実装 PR は、追加・変更する CLI flag、環境変数、TOML key、default、secret file、対象 Phase 前 config の扱いを実装開始前に固定しなければならない。設定解決が実行環境や実装者の判断で変わる場合、その Phase は未完了である。

**設定解決の必須項目：**

| 項目 | 必須内容 |
|------|----------|
| config key | CLI flag、env name、TOML section/key、内部 field 名 |
| type / unit | string、bool、integer、duration、bytes、path、enum。ms/sec/bytes など単位も固定 |
| default | 省略時の値。default なしの場合は必須扱いか対象 Phase 前無効を明記 |
| precedence | CLI > env > TOML > default など、全 source の順位 |
| invalid value | 範囲外、型違い、空文字、未知 enum、存在しない file/path の扱い |
| target Phase before behavior | 対象 Phase 前に指定された時、起動失敗か WARN 無効化か |
| redaction | log / stderr / artifact に値を出してよいか、redacted marker |
| evidence | config fixture、env fixture、stderr snapshot、redaction sample、priority test |

**既定 precedence：**

| 種別 | 既定順位 |
|------|----------|
| 通常設定 | CLI flag > env > TOML > default |
| `jwt_secret` | `--auth-jwt-secret-file` > `--auth-jwt-secret` > `ADLAIRE_JWT_SECRET` > TOML `[auth] jwt_secret_file` > TOML `[auth] jwt_secret` > auth disabled |
| admin token | `--admin-auth-token` > `ADLAIRE_ADMIN_TOKEN` > TOML `[admin] auth_token` > admin auth disabled |
| replication token | `--replication-auth-token` > `ADLAIRE_REPLICATION_TOKEN` > TOML `[replication] auth_token` > replication auth disabled only if Phase 節で許可 |
| HA token | `--ha-auth-token` > `ADLAIRE_HA_TOKEN` > TOML `[ha] auth_token` > HA API 起動不可 |
| data dir | `--data` > TOML `[storage] data_dir`。本番では CLI 指定を推奨し、未指定は起動失敗 |

既定順位と異なる設定は、§4、§8.4、§9.13、該当 Phase 節、manifest の `Config map` に差分理由を明記する。差分理由なしに順位を変更してはならない。

**secret / path validation：**

| 設定 | 必須 validation |
|------|-----------------|
| secret value | 最小長、空文字の扱い、前後空白禁止、log/stderr/artifact redaction |
| secret file | 存在、通常 file、readable、directory 禁止、symlink 方針、permission warning/error、内容末尾改行の扱い |
| data dir | create 可否、permission、absolute/canonical path、既存 file 衝突、lock 取得 |
| extension path | data-dir 配下固定、absolute path log 禁止、symlink 拒否 |
| primary URL | scheme、host、port、path、TLS 方針、secret を URL に含めることの禁止 |
| duration / timeout | 0、負数、上限、単位、省略時 default |
| bytes / size limit | suffix、整数 overflow、負数、0 の意味 |
| enum | 大小文字、未知値、deprecated value、fallback 禁止 |

**未実装 config の扱い：**

| 状態 | 必須挙動 |
|------|----------|
| 対象 Phase 前の key が指定された | §9.13.1 に従い、起動失敗または明示 WARN。silent ignore 禁止 |
| sample config に commented key がある | 実装許可ではない。対象 Phase 前に有効値として扱わない |
| unknown TOML key | 互換のため許可するか、起動失敗にするかを section 単位で固定。未定義なら起動失敗 |
| unknown env | 無視可。ただし Adlaire prefix の unknown env を warning するかは仕様化する |
| CLI unknown flag | clap 標準 error で非 0 終了 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| config priority fixture | CLI/env/TOML/default の上書き順を示す fixture |
| invalid config stderr | 型違い、範囲外、unknown enum、存在しない secret file の stderr snapshot |
| target Phase before fixture | 対象 Phase 前 config 指定時の起動失敗または WARN |
| redaction sample | secret、secret file contents、absolute path が log/stderr/artifact に出ないこと |
| restart fixture | config 変更後の起動成功/失敗が deterministic であること |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 未実装 config を silent ignore する | Phase 未完了 |
| invalid value を default に fallback する | merge 不可。ただし仕様本文に fallback 可と明記された場合のみ可 |
| secret value を stderr/log/artifact に出す | merge 不可 |
| CLI/env/TOML の優先順位 test がない | Phase 未完了 |
| sample config の comment だけで実装契約を代替する | review failure |
| Phase ごとに同じ key の意味や単位が変わる | merge 不可。migration / compatibility 契約がある場合のみ可 |

設定に関係する仕様変更は、§4、§8.4、§9.13、manifest の `Config map`、CFG 契約 ID、config fixture、stderr snapshot、redaction artifact を同時更新する。設定の正常系だけが通っても、不正値、優先順位、対象 Phase 前挙動、秘匿が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.20 API schema / validation / serialization 固定契約

各 Phase 実装 PR は、追加・変更する API の method、path、path parameter、query、header、Content-Type、Accept、body limit、request body schema、success response schema、error response schema、serialization、snapshot 正規化を実装開始前に固定しなければならない。schema が曖昧な API は route を公開してはならない。

**request validation 既定ルール：**

| 対象 | 既定仕様 | 例外条件 |
|------|----------|----------|
| method | §9.5 または該当 Phase 節に明記された method のみ許可。未対応 method は `405 METHOD_NOT_ALLOWED` | Turso 互換で異なる status が必要な場合は snapshot と差分理由を明記 |
| path parameter | URL decode 後に validation。空文字、`..`、`/`、NUL、未正規化 unicode は `INVALID_REQUEST` または専用 error | hrana path DB 名など既存互換がある場合のみ該当節で緩和 |
| query | 明記された key のみ許可。duplicate query key、空 query value、未知 query は `INVALID_REQUEST` | cursor など opaque value は型だけ固定 |
| Content-Type | JSON body は `application/json` 必須。`; charset=utf-8` は許可。octet-stream は `application/octet-stream` 必須 | body なし endpoint は Content-Type 不要 |
| Accept | response media type が JSON の場合は未指定、`*/*`、`application/json` を許可。それ以外は `406 NOT_ACCEPTABLE` | SDK 互換で Accept を無視する endpoint は明記 |
| body limit | endpoint ごとに byte 上限を明記。超過は `413 PAYLOAD_TOO_LARGE` | streaming response は response 側上限を別途固定 |
| JSON top-level | request body は object 必須。array/scalar/null は `INVALID_REQUEST` | hrana wire protocol で配列等が仕様化される場合のみ |
| unknown field | 管理 API、Turso Platform API、backup/restore/branch/extension/HA API は拒否 | hrana wire protocol は hrana 仕様に従い、該当節で許可 |
| omitted field | `?` または「省略可」と明記された field だけ省略可。明記なしは必須 | default が仕様化された field は省略可 |
| null | `null 可` と明記された field だけ許可。省略可能 field の `null` を省略扱いにしない | Turso 互換 DTO で null が必要な field |
| empty array / object | 空を許可する field だけ許可。filter、scope、batch、permission 配列の空は既定で `INVALID_REQUEST` | 空 object が Turso 互換上必要な場合 |

**response serialization 既定ルール：**

| 対象 | 固定仕様 |
|------|----------|
| success JSON | schema に定義された wrapper、field casing、必須 field、nullable field を厳密に返す |
| error JSON | `{"error": string, "code": string}` を基本形とし、追加 field は該当 error 契約に明記する |
| 204 response | body なし。`Content-Type` を付けない |
| hrana SQL error | HTTP 200 のまま `results[i].type="error"`。pipeline 全体の JSON 不正だけ HTTP 400 |
| timestamp | RFC3339 UTC 秒精度。ミリ秒、local time、timezone offset は返さない |
| numeric string | SQLite integer / Turso size など文字列指定の field は数値型へ勝手に変えない |
| field order | JSON object order は意味を持たない。snapshot 比較前に key を辞書順正規化する |
| dynamic field | UUID、timestamp、request id、JWT、host、path は placeholder へ正規化する |
| secret field | token / JWT / secret は作成時 1 回だけ返す endpoint を除き response に含めない |

**Turso / SDK 互換 schema：**

| Surface | 必須固定 |
|---------|----------|
| `/v1/*` Turso Platform API | wrapper 名、field casing、status、query mapping、unsupported response を snapshot 化 |
| `/admin/v1/*` | Adlaire 管理 API schema として `/v1/*` と混同しない。差分理由を明記 |
| hrana HTTP | libSQL SDK の request/response schema を優先。unknown field 方針は hrana 節に従う |
| hrana WebSocket | message type、request_id、stream_id、response_ok/error の対応を transcript 化 |
| binary endpoint | `Content-Type`、`Content-Disposition`、body size、checksum/header を固定 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| request fixture | method、path、query、header subset、body、Content-Type |
| response snapshot | status、header subset、body、error code、wrapper、field casing |
| validation error snapshot | unknown field、null、missing field、bad query、bad Content-Type、body limit |
| compatibility snapshot | Turso / SDK 互換 endpoint の正規化後 snapshot |
| serialization roundtrip | request deserialize、internal type、response serialize の roundtrip test |
| redaction snapshot | response / error に secret、token、raw path、SQL args が出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| success response schema が仕様本文にない | route 公開禁止 |
| unknown field を endpoint ごとに仕様なしで黙って無視する | Phase 未完了 |
| `null` と省略を同一扱いにする | review failure |
| 204 response に JSON body を返す | merge 不可 |
| Turso wrapper / field casing を実装都合で変更する | merge 不可 |
| snapshot 正規化なしに dynamic 値を保存する | Phase 未完了 |
| SDK 互換 endpoint を手動確認だけで完了扱いにする | Phase 未完了 |

API schema に関係する仕様変更は、§6、§7、§9.5、manifest の `Endpoint map`、API 契約 ID、request fixture、response snapshot、validation error snapshot を同時更新する。正常系だけが通っても、invalid / unknown / null / body limit / serialization の証跡がない場合は Phase 完了扱いにしない。

#### 9.1.21 Persistence atomicity / fsync / recovery 固定契約

各 Phase 実装 PR は、追加・変更する永続化 file、directory、metadata JSON、SQLite DB、WAL archive、backup artifact、branch artifact、lock file、manifest の書き込み順序、fsync 境界、atomic rename、破損検出、復旧可否を実装開始前に固定しなければならない。永続化成功の定義が曖昧な状態で client へ成功応答を返してはならない。

**atomic write 既定手順：**

| Step | 必須処理 | 失敗時の扱い |
|------|----------|--------------|
| 1 | 対象 directory を canonicalize し、data dir 配下であることを確認する | 起動時または request 時に失敗。path を secret/path redaction 付きで記録 |
| 2 | 同一 directory に一意な temporary file を作成する | 既存 file overwrite 禁止。失敗時は元 file を変更しない |
| 3 | 完全な新内容を書き込む。部分更新、in-place truncate、append で metadata JSON を更新しない | 書き込み失敗時は temp を削除し、元 file を維持 |
| 4 | temporary file を flush し、file fsync を行う | fsync 失敗時は成功応答禁止。元 file を維持 |
| 5 | temporary file を target file へ atomic rename する | rename 失敗時は成功応答禁止。元 file または temp の状態を recovery 対象にする |
| 6 | parent directory fsync を行う | fsync 失敗時は成功応答禁止。次回起動で recovery scan 必須 |
| 7 | lock release 前に in-memory state と disk state の version / checksum を照合する | 不一致なら panic ではなく内部 error とし、成功応答禁止 |

SQLite DB、WAL、backup binary のように library が内部で durability を管理する file は、library の commit 成功条件、checkpoint / sync mode、追加で必要な directory fsync、metadata 更新との順序を Phase 節に明記する。metadata だけ atomic でも、対応する DB/WAL/backup file が永続化されていない場合は成功扱いにしない。

**metadata JSON 既定 schema 契約：**

| 項目 | 固定仕様 |
|------|----------|
| schema_version | すべての metadata JSON に必須。初期値、増加条件、migration path を §9.6 に明記 |
| generation | atomic update ごとに単調増加。restart 後に逆行してはならない |
| checksum | file 内容または参照 artifact の checksum を保存する場合、対象 byte 範囲と algorithm を明記 |
| created_at / updated_at | RFC3339 UTC 秒精度。更新順序の証跡に使う場合は snapshot 正規化する |
| unknown field | 旧 version 互換で読み飛ばすか、破損扱いにするかを file ごとに固定 |
| missing required field | 起動失敗または該当 resource disabled。default 補完は禁止。ただし migration 契約がある場合のみ可 |
| duplicate logical key | DB 名、token id、backup id、branch id の重複は破損扱い。先勝ち/後勝ちは禁止 |

**lock / concurrency 既定ルール：**

| 対象 | 必須挙動 |
|------|----------|
| process lock | `--data` 単位で単一 writer を保証する。lock 取得不能時は起動失敗 |
| resource lock | DB create/delete、token revoke、backup restore、branch create は対象 resource の排他範囲を仕様化 |
| read during write | 読み取りが旧 state を見るか新 state を見るかを固定し、中間 temp state を見せない |
| concurrent update | generation / compare-and-swap / mutex のいずれで競合検出するかを明記 |
| shutdown during write | lock release、temp cleanup、未完了 operation の response 方針を固定 |
| retry | 同じ request を retry した時に idempotent か、重複 error かを API 契約と揃える |

**recovery 判定：**

| 状態 | 必須挙動 |
|------|----------|
| temp file のみ残存 | target file が正しければ temp を削除。target 不在なら operation 種別ごとの recovery 可否で判断 |
| target と temp が両方存在 | generation / checksum / manifest を比較し、勝者を deterministic に決める。判断不能なら起動失敗 |
| metadata JSON parse 失敗 | 自動上書き修復禁止。backup copy または manifest から復旧できる場合のみ復旧 |
| schema_version 未来値 | 起動失敗。downgrade / ignore 禁止 |
| schema_version 過去値 | migration 契約がある場合だけ migrate。なければ起動失敗 |
| checksum 不一致 | 対象 artifact を使用禁止にし、error code と recovery log を出す。成功応答禁止 |
| DB integrity_check 失敗 | 対象 DB を degraded / unavailable とし、通常 read/write を許可しない |
| recovery 失敗 | 部分的に成功したように見せず、起動失敗または対象 resource unavailable とする |

**Phase 別の追加固定対象：**

| Phase 範囲 | 対象 | 必須固定 |
|------------|------|----------|
| Phase 2 | data dir、default DB、lock、integrity marker | 初期化途中 crash、既存 file 衝突、integrity_check 失敗 |
| Phase 4-7 | tokens.json、databases.json、revoke state | token 作成/失効、DB create/delete の atomicity と restart 後状態 |
| Phase 8 | location / org / group / quota metadata | migration、quota usage 更新順序、Turso wrapper との整合 |
| Phase 11-13 | WAL archive、snapshot、replication manifest | frame number、CRC、retention cleanup、manifest/files 整合 |
| Phase 14-16 | backup、restore、PITR、branch | restore rollback、source snapshot 固定、branch metadata と DB file の commit 順序 |
| Phase 17-18 | extension、HA state | signed artifact、term/leader persistence、split-brain recovery |
| Phase 19+ | internal WAL/storage/executor state | libSQL 互換 path と内製 path の rollback、format marker |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| atomic trace | temp write、file fsync、rename、directory fsync、lock release の順序 |
| crash fixture | 各 Step 中断時の再起動後状態。旧 state / 新 state / 起動失敗のいずれかを固定 |
| corrupt fixture | JSON parse 失敗、missing field、future schema_version、checksum mismatch |
| restart fixture | 成功応答後の再起動で state が保持されること |
| concurrency fixture | 同一 resource への同時更新、reader during write、lock 取得不能 |
| recovery log snapshot | recovery 実施 / 失敗時の log。secret、absolute path、raw SQL args を出さない |
| manifest consistency | metadata が参照する artifact が存在し、checksum / size / generation と一致すること |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| metadata JSON を in-place truncate/write で更新する | merge 不可 |
| file fsync または directory fsync の扱いが仕様にない | Phase 未完了 |
| parse 失敗した metadata を空 default で再作成する | merge 不可 |
| checksum 不一致 artifact を警告だけで使用する | merge 不可 |
| success response 後の restart で state が消える | merge 不可 |
| recovery の勝者判定が実装依存または timestamp だけに依存する | review failure |
| temp file や absolute path を API response に出す | merge 不可 |
| crash / corrupt fixture なしに Phase 完了扱いにする | Phase 未完了 |

永続化に関係する仕様変更は、§9.6、§9.15、manifest の `Persistence map`、該当 Phase の storage 契約 ID、atomic trace、crash fixture、corrupt fixture、restart fixture を同時更新する。正常系の書き込みだけが通っても、fsync、rename、directory sync、破損検出、recovery 失敗時挙動が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.22 Error taxonomy / retry / client action 固定契約

各 Phase 実装 PR は、追加・変更する失敗条件について、error category、HTTP status、wire surface、`code`、message 粒度、retry 可否、client action、redaction、evidence を実装開始前に固定しなければならない。未分類の失敗を `INTERNAL_ERROR` に丸めて Phase 完了扱いにしてはならない。

**error category 既定表：**

| Category | 代表条件 | HTTP status / wire | Retry | Client action |
|----------|----------|--------------------|-------|---------------|
| request validation | JSON 不正、未知 field、型違い、body limit、bad query | 400 / 413 / 406 / 405 | No | request を修正 |
| auth required / invalid | token 欠落、不正形式、署名不一致、期限切れ、失効済み | 401 | No | token を再取得または設定修正 |
| permission denied | scope 不足、ro token write、block_reads / block_writes | 403 | No | 権限または設定を変更 |
| not found | DB、token、backup、branch、frame が存在しない | 404 | No | resource 名または selector を修正 |
| conflict | duplicate name、generation mismatch、同時更新競合、delete protection | 409 | Conditional | 最新 state を再取得して再試行 |
| locked / busy | SQLite busy、resource lock timeout、process lock 競合 | 423 または 503 | Yes | backoff retry。process lock は運用者対応 |
| quota / limit | quota 超過、usage unavailable、rate / size limit | 402 / 403 / 429 / 503 | Conditional | quota 調整、時間経過、usage 確認 |
| unavailable | replica lag、leader unavailable、primary 到達不能、maintenance | 503 / 307 | Yes | redirect follow または backoff retry |
| corruption / integrity | checksum 不一致、metadata parse 失敗、integrity_check 失敗 | 500 / 503 | No | recovery / restore / operator action |
| unsupported / not implemented | 未来 Phase、自己ホスト未対応、Turso 互換外 endpoint | 501 / 404 | No | feature availability を確認 |
| internal bug | 仕様上分類済みでない invariant violation | 500 | No | bug として扱い、redacted log を添付 |

上表と §9.7 が衝突する場合は、§9.7、該当 API 節、snapshot 期待値を同じ PR で更新し、差分理由を明記する。Turso Cloud 互換 endpoint が異なる status を必要とする場合でも、`code`、retry、client action は本節または該当 API 節で固定する。

**wire surface 別 error 変換：**

| Surface | 固定仕様 |
|---------|----------|
| admin HTTP | HTTP status と `{"error": string, "code": string}` を返す。debug field、stack trace、path は返さない |
| Turso Platform `/v1/*` | Turso 互換 status / wrapper / casing を優先し、差分理由と compatibility snapshot を残す |
| hrana HTTP `/v2/pipeline` | pipeline JSON 不正は HTTP 400。SQL 実行、permission、constraint は HTTP 200 の `results[i].type="error"` |
| hrana WebSocket | hello 前の認証失敗は `hello_error`。request 単位の失敗は `response_error`。connection close 条件を明記 |
| CLI / startup | HTTP error code に変換しない。stderr message、exit code、redaction snapshot を固定 |
| replication / HA | redirect、timeout、lag、term mismatch、split-brain rejection の status / code / retry を個別に固定 |

**`INTERNAL_ERROR` 使用条件：**

| 状態 | 判定 |
|------|------|
| OS / filesystem / library error でも仕様済み category に分類できる | 専用 code に写像する。`INTERNAL_ERROR` 禁止 |
| serialization、validation、auth、permission、quota、not found、conflict | `INTERNAL_ERROR` 禁止 |
| invariant violation、到達不能分岐、未分類 bug | `INTERNAL_ERROR` 可。ただし redacted log と bug evidence 必須 |
| secret scan failure、redaction failure | `INTERNAL_ERROR` にせず merge/blocking failure として扱う |
| panic / unwrap による process abort | Phase 未完了。HTTP 500 の代替として扱わない |

**複数 error 同時発生時の優先順位：**

| Order | 条件 | 理由 |
|-------|------|------|
| 1 | request body / route / method / content-type が不正 | resource 存在や auth 状態を推測させないため、構文不正を先に閉じる |
| 2 | auth required / invalid / expired / revoked | 未認証 caller に resource 状態を返さない |
| 3 | scope / permission / block policy / quota | 認証済み caller の操作可否を先に確定 |
| 4 | resource not found | 認可後に存在判定する |
| 5 | conflict / idempotency / generation mismatch | 対象 resource の現在 state と request の競合 |
| 6 | busy / unavailable / timeout | 操作可能だが一時的に完了できない |
| 7 | corruption / integrity / recovery failure | operator action が必要な状態 |
| 8 | internal bug | 上記分類に該当しない場合のみ |

§9.1.18 の認証・認可 precedence、§9.1.20 の validation 既定ルール、§9.1.21 の recovery 判定がより具体的な順序を定義する場合は、該当節を優先し、error matrix に差分を記録する。

**retry 既定ルール：**

| Retry | 条件 | Client behavior |
|-------|------|-----------------|
| No | validation、auth、permission、not found、unsupported、corruption | 同一 request をそのまま再送しない |
| Yes | storage busy、replication timeout、leader unavailable、temporary network / upstream failure | exponential backoff + jitter。idempotency 契約を確認 |
| Conditional | conflict、quota、rate limit、usage unavailable、redirect | state 再取得、quota/limit 確認、redirect follow 後に再試行 |
| Unknown | 仕様未定義 | 実装禁止。`INTERNAL_ERROR` で代用しない |

retry 可の error は、同一 request を再送した場合に二重作成、二重課金、二重 revoke、二重 restore、branch 重複が起きないよう、§9.1.16 の idempotency 契約または endpoint 固有契約を必ず参照する。

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| error matrix | 各 error code の category、status、wire surface、retry、client action |
| triggering fixture | 各 error code を発火させる最小 request / config / metadata |
| precedence snapshot | 複数 error 条件が同時に成立した時の status/code/body |
| retry matrix | retry 可 / 不可 / conditional の理由と idempotency 契約 ID |
| SDK/client action snapshot | libSQL SDK、Turso Platform 互換 client、CLI での観測結果 |
| redaction snapshot | error body、stderr、log に secret、JWT、raw SQL args、absolute path が出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 仕様済み error を `INTERNAL_ERROR` に丸める | merge 不可 |
| retry 可否が §9.7 または endpoint 契約にない | Phase 未完了 |
| client action が未定義のまま error code を追加する | review failure |
| hrana SQL error を HTTP 500 に変換する | merge 不可 |
| auth 前に resource not found を返して存在有無を漏らす | merge 不可 |
| error message に token、JWT claim raw value、SQL args、absolute path を出す | merge 不可 |
| 失敗条件の snapshot が正常系 snapshot だけで代替されている | Phase 未完了 |

error に関係する仕様変更は、§7.3、§9.5、§9.7、manifest の `Error map`、ERR 契約 ID、error matrix、triggering fixture、retry matrix、redaction snapshot を同時更新する。正常系だけが通っても、status、code、retry、client action、precedence、redaction が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.23 Turso Cloud tracking / compatibility diff / snapshot update 固定契約

各 Phase 実装 PR は、Turso Cloud 互換対象に関係する API、wire schema、metadata、auth、quota、location、organization/group、error/status、SDK 挙動、snapshot を追加・変更する場合、Turso Cloud 追従状態、差分分類、snapshot 更新理由、既存 Adlaire 後方互換への影響を実装開始前に固定しなければならない。Turso Cloud 互換差分を実装者判断で暗黙に作ってはならない。

**追従対象 surface：**

| Surface | 追従対象 | 必須 evidence |
|---------|----------|---------------|
| hrana HTTP | request/response schema、SQL error surface、baton、pipeline semantics | libSQL SDK transcript、wire snapshot |
| hrana WebSocket | message type、hello/open_stream/execute/sequence/store_sql、error message | WebSocket transcript、SDK regression |
| Turso Platform `/v1/*` | method、path、query、request body、wrapper、field casing、status、error body | Turso snapshot、compat diff |
| auth / JWT | claim 名、scope、ro/rw、expiration、revocation、token presentation | auth matrix、SDK token test |
| location / org / group | slug/name/id、membership、routing metadata、ownership boundary | metadata fixture、Platform snapshot |
| quota / usage | limit field、usage source、超過時 status/code、write denial | quota matrix、usage snapshot |
| metadata | schema version、field casing、default、legacy migration | old/new fixture、migration transcript |
| error/status | HTTP status、`code`、message 粒度、retry 可否 | error matrix、client action snapshot |
| SDK behavior | TypeScript/Rust/Go libSQL client の接続、CRUD、auth、error observation | SDK transcript、regression command |

**差分分類：**

| Classification | 意味 | 実装可否 |
|----------------|------|----------|
| `follow` | Turso Cloud と同一挙動へ追従する | 可。snapshot と regression を更新 |
| `intentional_diff` | 自己ホストの安全性・永続性・運用制約のため意図的に異なる | 可。ただし理由、SDK 影響、代替仕様、後方互換を明記 |
| `unsupported` | Adlaire DB の対象外または未来 Phase として成功応答しない | 可。§9.4 または該当 Phase 節に 501/404/400 等を固定 |
| `deferred` | 追従対象だが当該 PR では実装しない | 実装 PR では成功応答禁止。期限、Phase、stub error を固定 |
| `unknown` | Turso Cloud 挙動を確認できていない | 実装禁止。snapshot 更新禁止 |

**snapshot 更新許可条件：**

| 条件 | 判定 |
|------|------|
| Turso Cloud 追従により期待値が変わり、compat diff に source / date / endpoint / 差分理由がある | 更新可 |
| 仕様本文が先に更新され、変更対象 API / metadata / error の契約が明記されている | 更新可 |
| dynamic 値だけが異なり、placeholder 正規化規則の不足が原因 | 正規化規則を先に更新してから snapshot 更新可 |
| 実装都合で response wrapper、field casing、status、code が変わっただけ | 更新禁止 |
| Turso Cloud の確認結果がないのに `/v1/*` snapshot を変更する | 更新禁止 |
| secret、JWT、Bearer、absolute path、raw SQL args が snapshot に含まれる | 更新禁止。secret scan failure |

**compat diff 必須項目：**

| Field | 必須内容 |
|-------|----------|
| `surface` | hrana HTTP、hrana WebSocket、Platform API、metadata、auth、quota など |
| `reference` | Turso Cloud / libSQL SDK / hrana spec / legacy Adlaire のどれを参照したか |
| `observed_at` | 確認日。PR 作成日と大きく離れる場合は再確認する |
| `classification` | `follow` / `intentional_diff` / `unsupported` / `deferred` |
| `request` | method、path、query、header subset、body または SDK call |
| `reference_response` | status、header subset、body、error code、wire message |
| `adlaire_response` | 期待する Adlaire の status、header subset、body、error code、wire message |
| `diff_reason` | 差分理由。実装都合だけの理由は禁止 |
| `sdk_impact` | 既存 SDK / CLI / client がどう観測するか |
| `legacy_impact` | 既存 Adlaire metadata、config、API への影響 |
| `migration_or_rollback` | migration、互換維持策、rollback flag、または不要理由 |

**優先順位：**

| Order | 判断軸 | 備考 |
|-------|--------|------|
| 1 | security / data durability / secret redaction | Turso 追従より優先。差分理由を必ず書く |
| 2 | hrana wire / libSQL SDK 互換 | client 接続互換を最優先の互換面とする |
| 3 | Turso Platform `/v1/*` 互換 | wrapper、status、field casing を固定 |
| 4 | 既存 Adlaire 後方互換 | 既存 Phase の API、metadata、config を壊さない |
| 5 | 自己ホスト運用最適化 | 上位互換を壊さない範囲で採用 |
| 6 | 内製化都合 | API / wire / metadata 差分の理由にしてはならない |

上位の判断軸と下位の判断軸が衝突する場合は、上位を優先し、compat diff と仕様本文に衝突内容、採用理由、テスト証跡を残す。

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| Turso snapshot | `/v1/*` の status、header subset、body、wrapper、field casing |
| SDK transcript | libSQL SDK の接続、CRUD、auth、error 観測結果 |
| compat diff | 上記必須項目を満たす差分表 |
| legacy regression | 既存 Adlaire endpoint、metadata、config、DB 名の後方互換 |
| unsupported snapshot | 対象外 / deferred endpoint の 501/404/400 response |
| redaction scan | snapshot / transcript / diff に secret、JWT、Bearer、raw path、SQL args がないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| Turso Cloud 互換 endpoint の snapshot 差分理由が仕様本文にない | Phase 未完了 |
| `/v1/*` の wrapper / field casing / status を実装都合で変更する | merge 不可 |
| `unknown` 分類のまま成功応答を公開する | merge 不可 |
| 自己ホスト都合の差分に SDK 影響と代替仕様がない | review failure |
| snapshot を更新してから仕様本文を合わせる | review failure |
| secret を含む snapshot / transcript を証跡として採用する | merge 不可 |
| 内製化都合で API / wire / metadata / error 差分を作る | merge 不可 |

Turso Cloud 互換に関係する仕様変更は、§1.4、§3.5.3、§6、§7、§9.4、§9.5、§9.8、§9.15、該当 Phase 節、manifest の `Compatibility map`、COMPAT 契約 ID、Turso snapshot、SDK transcript、compat diff、legacy regression を同時更新する。Turso snapshot が更新されていても、差分分類、SDK 影響、後方互換、redaction が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.24 CI / release-check / environment reproducibility 固定契約

各 Phase 実装 PR は、完了判定に使う CI、Docker test environment、local verification、release-check、artifact 生成環境を実装開始前に固定しなければならない。実行環境によって結果が変わる検証、手順が口頭説明だけの検証、artifact と manifest が一致しない検証は Phase 完了根拠として扱わない。

**必須 gate：**

| Gate | 必須内容 | 適用条件 |
|------|----------|----------|
| format / lint | repository 標準 formatter / linter。存在しない場合は `N/A` 理由を manifest に記載 | all |
| unit | 対象 crate/module の unit test | all implementation phases |
| integration | HTTP、WebSocket、CLI、persistence の外部挙動 test | 外部 API または永続化を持つ Phase |
| SDK compatibility | TypeScript/Rust/Go libSQL SDK の該当 regression | hrana / SDK 互換面を変更する Phase |
| Turso compatibility | `/v1/*`、metadata、auth、quota、error の snapshot / compat diff | Turso 互換対象を変更する Phase |
| persistence / restart / rollback | fsync、rename、crash、corruption、migration、rollback | 永続化 schema または破壊的操作を変更する Phase |
| security / secret scan | auth denial、scope denial、artifact secret scan、log redaction | secret、auth、artifact を扱う Phase |
| release-check | repository 標準 release validation または同等 command | release、packaging、CI 完了判定を行う Phase |

**environment 固定項目：**

| 項目 | 必須固定 |
|------|----------|
| Rust toolchain | channel / version / target / components。`rust-toolchain.toml` がある場合はそれを正とする |
| Cargo lock | `Cargo.lock` を正とし、CI で lock drift を許可しない |
| Node.js | SDK compatibility test で使う Node.js version と package lock |
| Docker image | image 名、tag、digest または build context。`latest` のみは禁止 |
| OS / arch | Linux target、CPU architecture、glibc/musl 方針、unsupported OS |
| timezone / locale | `TZ=UTC`、UTF-8 locale を既定。timestamp snapshot は UTC 正規化 |
| env vars | test に必要な env 名、default、secret redaction。未定義時の挙動 |
| ports | 固定 port または ephemeral port の割当方法。衝突時 retry / failure 方針 |
| data dir | test ごとに isolated temporary directory。既存 user data dir 使用禁止 |
| network | 外部 network 依存の可否、mock / fixture、offline 時の扱い |

**local / Docker / CI 差分許容範囲：**

| 差分 | 許可 | 条件 |
|------|------|------|
| absolute path | Yes | artifact では `<normalized-path>` に置換 |
| host / port | Yes | `<normalized-host>` / `<normalized-port>` に置換 |
| execution time | Yes | performance gate 以外は pass/fail に使わない |
| timestamp | Yes | UTC 秒精度または `<normalized-time>` |
| dependency version | No | lock file / image tag / toolchain に一致させる |
| feature flag | No | manifest に明記された flags だけ使用 |
| test order | No | order 依存がある場合は test failure |
| network availability | No | external dependency は fixture 化または Manual exception |

**CI artifact 必須内容：**

| Artifact | 必須内容 |
|----------|----------|
| `environment.txt` | OS、arch、Rust、Cargo、Node、Docker image、timezone、locale、env subset |
| `ci.txt` | 実行 command、開始順序、exit code、pass/fail summary、対象 commit |
| `release-check.txt` | release-check command、exit code、検査項目、失敗時の理由 |
| `secret-scan.txt` | scan command、対象 path、検出 0 件または blocking failure |
| `toolchain.json` | machine-readable な toolchain / dependency / image version |
| `flaky-report.txt` | flaky なし、または検出時の原因と deterministic 化修正 |

artifact の保存先は §9.1.11 に従い、Contract ID を path に含める。CI artifact と manifest の `Regression set`、PR description、test name、snapshot path が 1 つでも不一致の場合は Phase 未完了とする。

**flaky / timeout / network 依存の扱い：**

| 状態 | 判定 |
|------|------|
| retry すれば通るが原因未特定 | Phase 未完了 |
| timeout が環境依存で、上限や待機条件が仕様化されていない | Phase 未完了 |
| 外部 network がないと失敗する test | fixture 化する。不可なら Manual exception と期限が必須 |
| sleep 固定待ちで成立する race test | review failure。状態待ち / event / timeout 契約へ置換 |
| CI では skip、local では実行 | skip 理由と代替 artifact がない限り Phase 未完了 |
| secret scan を通すために artifact を削除する | merge 不可。redaction 修正が必要 |

**release-check 必須検査：**

| Check | 必須判定 |
|-------|----------|
| worktree clean | release-check 開始時と終了時に clean |
| spec version | 仕様本文変更時に `V.{累積番号}` が増加している |
| lock drift | `Cargo.lock` / package lock / image pin に未承認差分がない |
| contract coverage | 対象 Phase の Contract ID に未カバーが 0 件 |
| artifact existence | manifest の Evidence path が存在する |
| snapshot strictness | snapshot 差分に仕様本文または compat diff の理由がある |
| secret scan | JWT、Bearer、admin/platform/replication/HA token、生 SQL args、absolute path がない |
| regression result | 対象 Phase 以前の regression が全件成功 |
| zero-bug gate | 既知不具合、未検証、未生成証跡、production TODO/FIXME が 0 件 |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| local だけ通して CI / Docker 再現性を固定しない | Phase 未完了 |
| `latest` tag、floating toolchain、unlocked dependency を完了 gate に使う | merge 不可 |
| CI artifact に実行 command、exit code、対象 commit がない | Phase 未完了 |
| failed gate を `known issue`、`flaky`、`後で検証` として残す | merge 不可 |
| release-check の対象を PR ごとに実装者判断で減らす | review failure |
| secret scan を未実行のまま artifact を採用する | merge 不可 |
| Docker と CI の差分理由が manifest にない | Phase 未完了 |

**canonical configuration / environment closure audit：**

configuration / environment は Phase 実装の再現性 gate であり、local で動いたこと、sample config の記載、または CI 成功ログだけで代替してはならない。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`configuration_environment_closure_result = pass` でなければ実装開始または Phase 完了に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `config_key_result` | 追加・変更する CLI flag、env name、TOML section/key、内部 field、type、unit、default、対象 Phase 前挙動が §9.1.19 / §9.13 / Phase 詳細節で一致している | 実装開始禁止 |
| `config_precedence_result` | CLI / env / TOML / default / secret file の優先順位が固定され、差分理由なしの順位変更が 0 件である | 実装開始禁止 |
| `invalid_config_result` | 範囲外、型違い、空文字、未知 enum、存在しない file/path、unknown key、unknown flag の期待 error / warning / exit code が fixture と stderr snapshot に接続されている | 実装開始禁止 |
| `secret_path_validation_result` | secret value、secret file、data dir、extension path、primary URL、duration、bytes、enum の validation と redaction が artifact に接続されている | merge 不可 |
| `target_phase_before_result` | 対象 Phase 前 key、sample config の commented key、unknown TOML / env / CLI の扱いが silent ignore なしで固定されている | Phase 未完了 |
| `config_redaction_result` | secret、secret file contents、absolute path、env secret、raw URL secret が stdout / stderr / log / artifact に残らない scan 結果がある | merge 不可 |
| `toolchain_result` | Rust、Cargo lock、Node、package lock、Docker image、OS / arch、timezone / locale、feature flag、dependency version が固定され floating が 0 件である | 実装開始禁止 |
| `docker_ci_local_result` | local / Docker / CI / release-check の command、working directory、env subset、ports、data dir、network 方針、差分許容範囲が一致または根拠付きである | 実装開始禁止 |
| `release_check_result` | release-check の worktree clean、spec version、lock drift、contract coverage、artifact existence、snapshot strictness、secret scan、regression、zero-bug gate が定義済みである | Phase 未完了 |
| `environment_artifact_result` | `environment.txt`、`ci.txt`、`release-check.txt`、`secret-scan.txt`、`toolchain.json`、`flaky-report.txt` が artifact manifest と review handoff に接続されている | Phase 未完了 |
| `configuration_environment_closure_result` | 上記 field がすべて `pass`。silent ignore、default fallback、secret leak、floating toolchain、Docker/CI/local 差分未説明、local only pass、release-check 欠落、environment artifact 未接続がすべて 0 件 | `pass` 以外は実装開始禁止 |

`configuration_environment_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。config / environment 影響がない Phase でも `configuration_environment_closure_result = pass` とし、`config_key:none`、`toolchain:unchanged`、`docker_ci_local:unchanged`、`release_check:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「設定・環境影響なし」の判断も、後続 Phase と reviewer が再現できる状態にする。

CI / release-check に関係する仕様変更は、§3.5.5、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.8、manifest の `Regression set`、該当 Contract ID、`environment.txt`、`ci.txt`、`release-check.txt`、`secret-scan.txt` を同時更新する。検証 command が成功していても、環境、toolchain、artifact、secret scan、release-check の再現性が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.25 Long-running operation / job lifecycle 固定契約

各 Phase 実装 PR は、restore、PITR、backup streaming、branch 作成/削除、replication catchup、extension load/unload、HA promote/demote、internal adapter 切替など、長時間・中断・再試行・部分失敗が起こり得る operation について、同期完了 API として扱うか、streaming operation として扱うか、永続 job として扱うかを実装開始前に固定しなければならない。完了していない operation に対して成功応答を返してはならない。

**operation 分類：**

| Classification | 定義 | 成功応答条件 |
|----------------|------|--------------|
| `sync` | request 処理中に全副作用が完了し、restart 後にも結果が確定している操作 | commit、fsync、metadata 更新、必要な検証が完了 |
| `streaming` | response body を返しながら処理するが、metadata や DB state を変更しない操作 | stream header 送信前に source snapshot が固定済み |
| `async_job` | request は job 作成だけを行い、後続 status endpoint で結果を確認する操作 | job metadata の atomic commit 完了。実処理完了ではない |
| `background_internal` | 外部 API の成功応答に直結しない内部 loop / maintenance | health / metrics / log に状態を出し、API 成功と混同しない |
| `stub` | 未来 Phase または unsupported として失敗応答だけ返す操作 | 501/404/400 等の仕様済み error のみ |

operation 分類を変更する場合は、該当 API 節、§9.5、§9.6、§9.7、manifest の Endpoint / Persistence / Error map、snapshot を同じ PR で更新する。

**job schema 必須 field：**

| Field | 必須仕様 |
|-------|----------|
| `job_id` | `job_` prefix の opaque id。request id や path を含めない |
| `type` | `restore`、`pitr_restore`、`branch_create`、`branch_delete`、`replication_catchup`、`extension_load`、`ha_promote`、`internal_adapter_switch` など |
| `resource` | 対象 DB / branch / node / extension の normalized id。secret や absolute path を含めない |
| `state` | `queued`、`running`、`verifying`、`committed`、`failed`、`rolled_back`、`cancelled`、`expired` のいずれか |
| `progress` | 0〜100 の整数または `null`。正確に出せない場合は `null` 固定 |
| `created_at` / `updated_at` | RFC3339 UTC 秒精度。snapshot では正規化 |
| `expires_at` | job result を保持する期限。期限なしは禁止 |
| `result` | 完了時の resource summary。secret、path、backup body は含めない |
| `error` | 失敗時の `{"error": string, "code": string}`。§9.1.22 に従う |
| `idempotency_key_hash` | idempotency key を受ける場合のみ保存。raw key は保存しない |

**共通 state machine：**

| From | To | 条件 |
|------|----|------|
| `queued` | `running` | worker が lock を取得し、対象 resource が存在する |
| `running` | `verifying` | 副作用の prepare が完了し、commit 前検証に入る |
| `verifying` | `committed` | integrity、checksum、quota、metadata consistency、fsync が完了 |
| `running` / `verifying` | `rolled_back` | 失敗後に旧 state へ戻せた |
| `running` / `verifying` | `failed` | rollback 不要または rollback 不能 marker を残した |
| `queued` / `running` | `cancelled` | cancel 可能な段階で operator が cancel した |
| terminal | `expired` | retention 期限を過ぎ、result summary だけ破棄した |

terminal state は `committed`、`failed`、`rolled_back`、`cancelled`、`expired` とする。terminal state から副作用を再開してはならない。再実行は新しい job または idempotency 契約に従う。

**Phase 別 job 化方針：**

| 対象 | 既定分類 | 例外条件 |
|------|----------|----------|
| DB create/delete | `sync` | 大容量 cleanup を後段に回す場合でも metadata commit は sync |
| backup download | `streaming` | backup body を metadata job として保存する仕様が追加された場合のみ `async_job` |
| restore / PITR | `sync` | Phase 節で status endpoint と job schema を定義した場合のみ `async_job` |
| branch create/delete | `sync` | source snapshot が大きく async 化する場合は branch status API を先に仕様化 |
| replication catchup | `background_internal` | operator-triggered catchup API を追加する場合は `async_job` |
| extension load/unload | `sync` | 外部検証が長時間化する場合は `async_job` とし、未検証 load 禁止 |
| HA promote/demote | `sync` | quorum / external consensus を導入する Phase までは `async_job` 禁止 |
| internal adapter switch | `sync` | shadow comparison を長時間 job にする場合は active 化と分離 |

**timeout / cancel / shutdown：**

| 状態 | 必須挙動 |
|------|----------|
| request timeout before commit | 成功応答禁止。rollback 可能なら rollback、不能なら recovery marker |
| request timeout after commit | commit 済みなら status / retry で完了を確認できるようにする |
| operator cancel before prepare | `cancelled`。副作用なし |
| operator cancel after prepare | rollback 成功後のみ `cancelled`。rollback 不能なら `failed` |
| graceful shutdown while running | lock release 前に rollback または resumable marker を fsync |
| forced shutdown | restart recovery で `running` / `verifying` job を deterministic に解決 |
| expired job cleanup | result summary を消してよいが、resource state と audit log は消さない |

cancel API を提供しない Phase では、cancel 不可を該当 API 節に明記し、shutdown/restart recovery だけを固定する。cancel 不可でも process shutdown による中断は必ず検証対象とする。

**retry / idempotency：**

| 状態 | 必須挙動 |
|------|----------|
| 同一 idempotency key + 同一 request body | 既存 job/result を返す。副作用を二重実行しない |
| 同一 idempotency key + 異なる request body | `409 IDEMPOTENCY_CONFLICT` または既存 error code を仕様化 |
| idempotency key なしの retry | endpoint 固有の重複 error または新規 job。曖昧な再実行禁止 |
| committed 後の retry | committed result を返すか、resource already exists / not found を返すかを固定 |
| failed / rolled_back 後の retry | 新規 job を許可するか、operator action を要求するかを固定 |

idempotency key を導入する場合は、header 名、body hash 範囲、retention、conflict response、secret redaction を §9.5 と API 節に明記する。raw idempotency key を metadata、log、artifact に保存してはならない。

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| job state fixture | queued/running/verifying/terminal の metadata sample |
| shutdown fixture | running/verifying 中 shutdown 後の recovery 結果 |
| retry matrix | idempotency key あり/なし、same/different body、terminal state retry |
| timeout snapshot | request timeout 前後の response、job state、resource state |
| recovery log | interrupted job の検出、rollback、commit 判定、operator action |
| redaction snapshot | job response/log/artifact に secret、absolute path、raw SQL args、backup body がないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| job metadata が committed でないのに 2xx success として扱う | merge 不可 |
| async job を作るが status endpoint / schema / retention がない | Phase 未完了 |
| retry で restore、branch create、quota charge、token revoke が二重適用される | merge 不可 |
| shutdown 中の running job が再起動後に silent success する | merge 不可 |
| cancel 後に副作用が残るのに `cancelled` と表示する | merge 不可 |
| job result に token、raw path、SQL args、backup body を含める | merge 不可 |
| background_internal の失敗を health / metrics / log に出さない | Phase 未完了 |

long-running operation に関係する仕様変更は、§9.1.16、§9.1.21、§9.1.22、§9.5、§9.6、§9.7、該当 Phase 節、manifest の Endpoint / Persistence / Error / Regression map、job state fixture、shutdown fixture、retry matrix、recovery log を同時更新する。正常系だけが通っても、timeout、cancel、shutdown、restart recovery、retry、idempotency が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.26 Resource identity / naming / path normalization 固定契約

各 Phase 実装 PR は、追加・変更する resource の external slug、internal id、filesystem name、display name、metadata key、URL path parameter、JWT scope value、quota key の関係を実装開始前に固定しなければならない。名前の validation、正規化、照合、path 変換が曖昧な resource は API、metadata、filesystem に公開してはならない。

**identity 種別の分離：**

| 種別 | 用途 | 変換可否 |
|------|------|----------|
| external slug | API path、query、Turso Platform 互換 field、SDK が見る名前 | validation 後に metadata lookup。勝手に内部 id へ表示変更しない |
| internal id | metadata 内部参照、stable primary key、audit / job reference | API path として直接受け付ける場合は専用 endpoint に明記 |
| filesystem name | data-dir 配下の directory / file 名 | external slug から直接 join せず、validated mapping を通す |
| display name | 将来 UI / description 用の任意文字列 | identity として使わない。path、scope、metadata key に使わない |
| legacy name | Phase 1〜7 で許可済みの既存 DB 名 | migration で保持し、新規作成規則とは分離 |

external slug、internal id、filesystem name を同一文字列として扱ってよいのは、該当 resource 節で validation、reserved word、collision、migration、path mapping が明記されている場合だけとする。

**resource 別 validation 既定表：**

| Resource | 形式 | 大小文字 | 予約 / 禁止 |
|----------|------|----------|-------------|
| DB slug Phase 8+ | `^[a-z0-9-]{1,64}$` | lowercase only | `admin`、`meta`、`.`、`..`、`___`、`/`、NUL |
| legacy DB name | 既存 metadata に存在する値だけ許可 | 既存値を保持 | 新規作成不可。rename 自動実行禁止 |
| branch name | `^[a-z0-9-]{1,64}$` | lowercase only | source DB と同名、`___`、`admin`、`meta`、`.`、`..` |
| internal branch DB name | `{source}___{branch}` | source / branch の規則に従う | external DB create では常に拒否 |
| organization slug | `^[a-z0-9-]{1,64}$` | lowercase only | `admin`、`default` の扱いは Phase 節で固定 |
| group slug | `^[a-z0-9-]{1,64}$` | lowercase only | organization 内 unique。global unique にしない |
| location code | `^[a-z0-9-]{1,32}$` | lowercase only | 空文字、unknown location |
| token id | `tok_[A-Za-z0-9_-]{16,80}` | case-sensitive | raw token、JWT、secret を id として保存しない |
| job id | `job_[A-Za-z0-9_-]{16,80}` | case-sensitive | request id、path、secret を含めない |
| extension name | `^[a-z0-9][a-z0-9_-]{0,63}$` | lowercase only | path separator、dot-prefix、SQL からの直接指定 |
| HA node id | `^[a-z0-9-]{1,64}$` | lowercase only | `standalone` は default 値専用。multi-node identity と混同しない |

上表と既存 Phase 詳細が衝突する場合は、本節、§9.5、§9.6、該当 Phase 節、migration 契約を同じ PR で更新する。互換維持のため緩和する場合は、緩和対象、期限、legacy fixture、Turso / SDK 影響を明記する。

**URL / Unicode / case normalization：**

| 対象 | 固定仕様 |
|------|----------|
| URL path parameter | percent decode 後に UTF-8 として validation。decode 不能は `INVALID_REQUEST` |
| Unicode | identity field は ASCII subset を原則とする。Unicode normalization に依存する identity は禁止 |
| case | lowercase only resource は入力が大文字を含む場合 `INVALID_*`。自動 lower-case 変換は禁止 |
| trailing slash | endpoint 節に明記がない限り、path canonicalization で同一扱いにしない |
| duplicate separator | `//`、`___`、`..`、`.` は identity 内で禁止 |
| whitespace | 前後空白、tab、newline、zero-width space は禁止。trim して受理しない |
| percent encoded slash | `%2F` は decode 後 `/` として拒否。path segment 分割前後で bypass させない |

**metadata / filesystem mapping：**

| 対象 | 必須仕様 |
|------|----------|
| metadata key | external slug ではなく、該当 resource の stable key を明記。legacy lookup がある場合は別 map を持つ |
| filesystem join | validated filesystem name だけを data-dir 配下へ join し、canonicalize 後に data-dir 配下であることを確認 |
| symlink | data-dir 内の resource directory / file に symlink を使う場合は Phase 節で明記。未定義なら拒否 |
| collision | case-insensitive filesystem でも衝突しないことを validation / fixture で確認 |
| rename | DB/org/group/branch/token/job/extension の rename は仕様化されるまで禁止。表示名変更と identity rename を混同しない |
| delete | metadata 削除と filesystem 削除 / trash / cleanup の順序を §9.1.21 と一致させる |
| backup/restore | backup file 内の DB 名や path を trust しない。restore target identity は request path を正とする |

filesystem path は API response、log、artifact では logical resource 名または `<normalized-path>` に正規化する。absolute path、local username、temp directory、trash directory を外部へ返してはならない。

**lookup precedence：**

| 状態 | 必須挙動 |
|------|----------|
| exact current slug が存在する | current resource を返す |
| exact legacy name が存在する | legacy resource として返し、response に legacy marker が必要か Phase 節で固定 |
| current slug と legacy name が衝突する | 起動失敗または migration failure。先勝ち/後勝ち禁止 |
| internal id と external slug の両方を受け付ける endpoint | 優先順位と ambiguity error を endpoint 節に明記 |
| branch internal DB name と通常 DB slug が衝突する | 通常 DB create を拒否。既存衝突は起動失敗 |
| org/group scoped lookup | organization slug → group slug → DB slug の順で scope を解決し、存在漏洩を §9.1.18 に従い抑制 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| name validation matrix | valid、uppercase、unicode、空、reserved、separator、長さ超過、percent encoded slash |
| path traversal fixture | `..`、`%2F`、symlink、absolute path、case collision、data-dir escape |
| legacy name fixture | Phase 1〜7 DB 名、legacy marker、new create rejection、lookup compatibility |
| metadata/path consistency snapshot | metadata key、filesystem path、canonical path、logical resource の対応 |
| scope lookup fixture | org/group/db、token scope、quota key の lookup precedence |
| redaction snapshot | path、local username、token id/raw token、job id が適切に秘匿 / 正規化されること |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| URL decode 前の文字列だけで validation する | merge 不可 |
| 大文字を自動 lowercase して別 resource として受理する | merge 不可 |
| external slug を未検証のまま filesystem path に join する | merge 不可 |
| legacy name を新規作成でも許可する | review failure。ただし仕様本文に例外がある場合のみ可 |
| internal id、external slug、display name を同じ field として混用する | Phase 未完了 |
| filesystem case collision の test がない | Phase 未完了 |
| resource not found の前に scope 外 resource の存在を漏らす | merge 不可 |

resource identity に関係する仕様変更は、§6、§8、§9.1.18、§9.1.20、§9.1.21、§9.5、§9.6、§9.14、該当 Phase 節、manifest の Endpoint / Persistence / Security / Compatibility map、name validation matrix、path traversal fixture、legacy fixture、metadata/path consistency snapshot を同時更新する。正常系だけが通っても、decode、case、legacy、collision、path traversal、scope lookup が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.27 List / pagination / cursor / filtering 固定契約

一覧 API を追加・変更する Phase は、実装開始前に並び順、limit、cursor、filter、snapshot 正規化、ページ跨ぎの mutation 挙動を固定しなければならない。`limit` と `cursor` の存在だけを endpoint 表に書くことは十分条件ではない。Turso Cloud 互換 endpoint と Adlaire 管理 endpoint が同じ resource を返す場合、wrapper / field casing を除き、対象集合、順序、cursor 境界、filter 結果は一致させる。

**共通 pagination 入力契約：**

| 項目 | 固定仕様 |
|------|----------|
| `limit` 未指定 | 100 |
| `limit` 最小 / 最大 | 1〜500。範囲外、非整数、浮動小数、指数表記、符号付き表記、空文字は `INVALID_REQUEST` |
| `cursor` 未指定 | 先頭ページ |
| `cursor` 空文字 | `INVALID_REQUEST`。空文字を先頭ページ扱いしない |
| duplicate query | 同一 key の複数指定は `INVALID_REQUEST` |
| unknown query | endpoint 固有表に明記されていない key は `INVALID_REQUEST` |
| offset pagination | `offset`、`page`、`per_page` は endpoint 固有表に明記しない限り禁止 |
| body | `GET` 一覧 API は body 禁止。body がある場合は `INVALID_REQUEST` |

**cursor 形式と検証：**

| 項目 | 固定仕様 |
|------|----------|
| wire format | base64url without padding の opaque string。client は内容を解釈してはならない |
| payload | `version`、`endpoint_id`、`resource_kind`、`filters_hash`、`sort_key`、`last_seen_key`、`issued_at` を含む |
| 署名 / 改ざん検知 | HMAC または同等の改ざん検知を必須とする。復号不能、署名不一致、必須 field 欠落は `INVALID_REQUEST` |
| scope binding | endpoint、resource kind、organization/group/database scope、filter、sort が一致しない cursor は `INVALID_REQUEST` |
| version | 実装が解釈できない cursor version は `INVALID_REQUEST`。silent reset 禁止 |
| expiry | endpoint が expiry を明記しない限り無期限。ただし metadata format 変更で解釈不能になった cursor は `INVALID_REQUEST` |
| secret | cursor payload に token、raw path、SQL、local username、quota 値の秘匿情報を入れない |

**共通 list 順序：**

| Resource 種別 | 既定順序 |
|---------------|----------|
| metadata resource | `created_at` 昇順、同値は stable identity 昇順 |
| usage / quota 集計 | scope 種別順 `organization` → `group` → `database`、scope identity 昇順 |
| location | `primary` true を先頭、その後 `created_at` 昇順、同値は location code 昇順 |
| token 一覧 | `created_at` 昇順、同値は token id 昇順。raw token は返さない |
| log / audit / event | `created_at` 昇順、同値は monotonic sequence 昇順。降順を採用する場合は endpoint 表に明記 |

endpoint が上表と異なる順序を必要とする場合は、endpoint 表、snapshot、cursor payload の `sort_key` を同じ PR で更新する。実装言語の map iteration order、filesystem order、JSON object key order、database engine の未指定順序に依存してはならない。

**filter 固定契約：**

| 項目 | 固定仕様 |
|------|----------|
| filter key | endpoint 表に明記された key のみ許可 |
| filter value | identity 系 value は §9.1.26 の validation 後に scope 解決する。trim / lowercase 補正は禁止 |
| 複数 filter | AND 条件。OR、部分一致、prefix match は endpoint 表に明記しない限り禁止 |
| 空結果 | 200 と空配列、`next_cursor:null`。対象 scope 自体が存在しない場合は対応する `*_NOT_FOUND` |
| 権限外 scope | §9.1.18 の precedence に従い、存在漏洩しない error を返す |
| cursor 併用 | cursor 発行時と同じ filter set だけ許可。filter 変更時は新規先頭ページとして cursor なしで要求する |

**ページ跨ぎ mutation 挙動：**

| 状態 | 必須挙動 |
|------|----------|
| 前ページ返却後に resource が追加された | 追加 resource が cursor の sort key より後なら後続ページに出てもよい。前なら今回の traversal には出なくてよい |
| 前ページ返却後に未返却 resource が削除された | 削除済み resource は返さない。穴埋めのために既返却 resource を重複返却しない |
| 前ページ返却後に既返却 resource が削除された | 後続ページで再返却しない |
| sort key が変更された | identity rename が禁止されている resource では発生させない。変更可能 resource は endpoint 節で cursor invalidation を明記 |
| metadata 破損 / migration 中 | partial list を返さず、該当 error と recovery evidence を残す |

`next_cursor` は次ページが存在する場合だけ non-null とする。最終ページ、空結果、`limit` より少ない結果しかない場合は `null` とする。`next_cursor` が non-null の場合でも、次 request までに resource が削除されて空ページになることは許容するが、その場合も 200 空配列 `next_cursor:null` とし、cursor を巻き戻してはならない。

**response / snapshot 正規化：**

| 項目 | 固定仕様 |
|------|----------|
| array order | API 契約の list 順序通りに比較する。snapshot 比較前に sort し直さない |
| object key order | snapshot 比較前に辞書順へ正規化する |
| timestamp | format、timezone、precision を endpoint / DTO 節に明記し、fixture では固定値を使う |
| wrapper | `/v1/*` は Turso 互換 wrapper / field casing、`/admin/v1/*` は Adlaire wrapper を維持する |
| empty list | 空配列と `next_cursor:null` を必ず含め、省略しない |
| redaction | cursor、token id、scope は snapshot に含めてよいが、raw token / raw path / local username は含めない |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| pagination matrix | limit 未指定、1、500、0、501、非整数、空、duplicate、unknown query |
| cursor fixture | valid、改ざん、別 endpoint、別 filter、別 scope、unknown version、空 cursor |
| order snapshot | created_at 同値、identity 同値不可、filesystem / map order 非依存 |
| mutation fixture | page 取得間の追加、削除、空ページ、最終ページ |
| filter matrix | scope 存在、scope 不存在、権限外、複数 filter、空結果 |
| Turso compatibility snapshot | `/v1/*` と `/admin/v1/*` の対象集合、順序、cursor 境界の差分分類 |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| cursor を単なる array index / offset として実装する | merge 不可 |
| filter 変更後も古い cursor を受理する | merge 不可 |
| map iteration / filesystem order に依存した list を返す | merge 不可 |
| duplicate query key を最後勝ち / 先勝ちで黙認する | merge 不可 |
| page 間削除で既返却 resource を再返却する | merge 不可 |
| snapshot 比較時に array を test 側で任意 sort する | Phase 未完了 |

list / pagination に関係する仕様変更は、§9.1.18、§9.1.20、§9.1.22、§9.1.23、§9.1.26、§9.5、該当 Phase 節、manifest の Endpoint / Compatibility / Regression map、pagination matrix、cursor fixture、order snapshot、mutation fixture、filter matrix を同時更新する。正常系だけが通っても、cursor scope、filter binding、stable order、page mutation、snapshot order が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.28 SQL execution / transaction / result mapping 固定契約

hrana-http、hrana-ws、ATTACH、replication redirect、backup consistency、internal executor 差し替えに関係する Phase は、SQL 実行単位、transaction 境界、result mapping、error surface、SDK 互換 transcript を実装開始前に固定しなければならない。SQL を libsql crate に渡すだけでは完了条件を満たさない。client が観測する wire format は Turso Cloud / libSQL SDK 互換を優先し、内部 executor の都合で status、result shape、transaction rollback 条件を変えてはならない。

**SQL request 種別別契約：**

| 種別 | 実行単位 | 成功応答 | 失敗応答 |
|------|----------|----------|----------|
| hrana-http `execute` | 単一 `stmt`。`want_rows` に従い `query` または `execute` | `results[i].type="ok"` + `response.type="execute"` | pipeline 全体は HTTP 200、当該 item は `results[i].type="error"` |
| hrana-http `sequence` | SQL 文字列全体を `execute_batch` へ渡す。自前 semicolon split 禁止 | 当該 item `ok`。個別 statement result は返さない | pipeline 全体は HTTP 200、当該 item は `error` |
| hrana-http `close` | 以降の request を処理しない | `close` の ok response | close 後の request は無視し、追加 result を返さない |
| hrana-ws `execute` | open stream 上の単一 `stmt` | `response_ok` + `execute` result | `response_error`。connection は維持 |
| hrana-ws `batch` | `batch` 配列順に `stmt` を逐次実行 | step ごとの result / error を順序維持で返す | request 自体の schema 不正は `response_error` |
| hrana-ws `sequence` | SQL 文字列全体を `execute_batch` へ渡す | `response_ok`。個別 statement result は返さない | `response_error` |
| hrana-ws `describe` | prepare / describe のみ。実行しない | parameter / column metadata | prepare error は `response_error` |

**`stmt` validation / parameter mapping：**

| 項目 | 固定仕様 |
|------|----------|
| `sql` | 非空文字列必須。空、空白のみ、NUL を含む値は `INVALID_REQUEST` |
| `args` | positional args。省略時は空配列。配列以外、変換不能 value は request 単位の SQL error surface に載せる |
| `named_args` | Phase 3〜8 の hrana-http では空配列または省略のみ許可。非空は当該 item を `SQLITE_ERROR` とする。Phase 9 以降で対応する場合は name 重複、positional 併用、SDK transcript を固定する |
| `want_rows` | boolean 必須。省略許可にする場合は endpoint / protocol 節で default を明記する |
| integer | hrana wire 上は文字列で返す。SQLite integer 範囲外または parse 不能は `SQLITE_ERROR` |
| float | IEEE 754 double として扱う。NaN / Infinity は JSON として受理しない |
| text | UTF-8 文字列。変換不能 byte列は blob としてのみ扱う |
| blob | base64 文字列。decode 不能は `SQLITE_ERROR` |
| null | SQLite NULL として bind し、response では hrana の null value として返す |

SQL text、args、row value は log / artifact / error message に生値で出してはならない。証跡では statement kind、arg count、type list、result column count までを許可し、値は `<redacted-arg>` または fixture 固定値だけにする。

**result mapping：**

| 項目 | 固定仕様 |
|------|----------|
| `cols` | `want_rows=true` の query では libsql / SQLite の column order を保持する。`want_rows=false` の execute では空配列 |
| `rows` | row order は SQLite が返した順序を保持する。test 側で sort しない。`want_rows=false` では空配列 |
| `rows_affected` | execute / write statement は libsql が返す affected rows。DDL や query で未定義の場合は 0 |
| `last_insert_rowid` | insert 成功後の値を文字列で返す。値が意味を持たない operation は `null`。数値型で返さない |
| multi result | `sequence` は個別 statement の `cols` / `rows` / `rows_affected` を返さない |
| column type | SQLite dynamic type を hrana value type に変換し、unsupported type は `SQLITE_ERROR` |
| response order | request order と results order は 1:1。parallel 実行で順序を入れ替えない |

**transaction / connection state：**

| 状態 | 固定仕様 |
|------|----------|
| HTTP pipeline | Phase 3〜8 では request 間 session を保持しない。SQLite 明示 transaction SQL は同一 pipeline 内の同一 connection で順序実行する場合のみ有効 |
| WebSocket stream | stream ごとに dedicated connection または同等の transaction isolation を持つ。別 stream の transaction state と混ぜない |
| `BEGIN` | stream state を `tx_active` にする。`BEGIN IMMEDIATE` / `BEGIN EXCLUSIVE` は write operation として認可・quota・block 判定する |
| `COMMIT` | commit が libsql / SQLite で成功し、response 送信前の状態更新が完了した場合だけ success |
| `ROLLBACK` | rollback 成功後に stream state を `open` へ戻す。rollback 失敗は `response_error` |
| close_stream | open transaction がある場合は rollback を試行してから close。rollback 失敗時は log / metric / evidence に残す |
| connection close | open transaction は rollback。COMMIT 成功応答前の切断は success と扱わない |
| shutdown | shutdown timeout 内に running SQL を完了または rollback する。timeout 後の abort は recovery evidence 必須 |

**permission / quota / block precedence：**

| 順位 | 判定 | 失敗時 |
|------|------|--------|
| 1 | request / wire schema validation | `INVALID_REQUEST` または protocol 固有 error |
| 2 | DB existence / resource state | `DB_NOT_FOUND`、`PERMISSION_DENIED` など |
| 3 | auth / scope / ro-rw | `AUTH_*`、`PERMISSION_DENIED` |
| 4 | block policy | `block_reads` / `block_writes` に応じて `PERMISSION_DENIED` |
| 5 | quota | write / import 相当 SQL は `QUOTA_EXCEEDED` |
| 6 | SQL prepare / execute | `SQLITE_ERROR`、`SQLITE_CONSTRAINT`、SQLite 固有 code |

read/write 判定は SQL text の単純な prefix だけに依存してはならない。Phase 10 の ATTACH、Phase 16 の extension、transaction control statement、DDL、PRAGMA を含む分類表を該当 Phase 節で更新する。分類できない statement は write 側に倒す。

**batch / sequence 途中失敗：**

| 種別 | 固定仕様 |
|------|----------|
| hrana-http request array | item ごとに順序実行する。ある item が SQL error でも後続 item は処理する。ただし `close` 後は処理しない |
| hrana-http `sequence` | `execute_batch` に委ねる。途中 statement の commit / rollback は SQLite の transaction semantics に従う |
| hrana-ws `batch` | step ごとの success / error を順序保持で返す。transaction 中の error 後に stream を強制 close しない |
| hrana-ws `sequence` | `execute_batch` に委ね、成功なら全体 success、失敗なら全体 `response_error` |
| explicit transaction | `BEGIN` 後の step error では自動 rollback しない。client の `ROLLBACK` または close / disconnect で rollback |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| SDK transcript | TypeScript / Rust / Go libSQL SDK の execute、batch、transaction、error 観測結果 |
| result mapping snapshot | cols、rows、integer string、blob、null、rows_affected、last_insert_rowid |
| args conversion matrix | integer、float、text、blob、null、invalid base64、integer overflow、named_args 非空 |
| SQL error matrix | syntax error、missing table、constraint、readonly、busy、permission、quota、block_reads/writes |
| transaction fixture | BEGIN/COMMIT/ROLLBACK、disconnect rollback、close_stream rollback、COMMIT 前切断 |
| batch / sequence matrix | 途中失敗、後続継続、sequence の execute_batch 境界、close 後無視 |
| redaction snapshot | SQL text、args、row value、absolute path、token が log / artifact / error に出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| SQL error を hrana-http の HTTP 500 / 400 に変換する | merge 不可 |
| `sequence` を ad hoc semicolon split する | merge 不可 |
| request order と results order を入れ替える | merge 不可 |
| `last_insert_rowid` を数値型で返す | SDK 互換 failure |
| open transaction を close / disconnect 後に残す | merge 不可 |
| SQL text、args、row value を log / artifact に出す | merge 不可 |
| read/write 判定不能 statement を read として許可する | merge 不可 |

SQL execution に関係する仕様変更は、§6.2、§6.3、§7.3、§9.1.16、§9.1.18、§9.1.20、§9.1.21、§9.1.22、§9.1.23、§9.5、§9.14、該当 Phase 節、manifest の Endpoint / Security / Compatibility / Regression map、SDK transcript、result mapping snapshot、args conversion matrix、transaction fixture、batch / sequence matrix を同時更新する。正常系だけが通っても、SQL error surface、result mapping、transaction rollback、permission precedence、SDK transcript が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.29 Specification consistency / cross-reference 固定契約

仕様書を更新する PR は、変更対象の本文だけでなく、関連する Phase 表、API 契約、error、永続化、テスト、evidence、manifest、実装前チェックリストの相互参照を同時に整合させなければならない。仕様本文に正しい内容を書いていても、別表や Phase 詳細が古いまま残る場合、その PR は仕様未確定として扱う。

**識別子整合ルール：**

| 識別子 | 固定仕様 |
|--------|----------|
| Phase 番号 | `Phase N` 表記を正とし、同一機能を別 Phase に移す場合は §9 の Phase 一覧、§9.2、§9.4、Phase 詳細節、§9.8、§9.11 を同時更新する |
| TC ID | テストケース番号は該当 Phase 詳細節、§9 Phase 一覧、§9.8、evidence artifact 名で一致させる。欠番を残す場合は理由を明記する |
| Task ID | 実装タスク番号は Phase 詳細節、§9 Phase 一覧、manifest の `Task map` で一致させる |
| API 契約 ID | endpoint、request/response snapshot、error snapshot、compatibility snapshot、manifest の `Endpoint map` で同一 ID を使う |
| Error code | §7.3、§9.1.22、endpoint 表、Phase 詳細節、error snapshot で同じ code / status / retry 方針を使う |
| Persistence key | §9.6、Phase 詳細節、migration fixture、recovery evidence、manifest の `Persistence map` で同じ file 名 / schema version を使う |
| Evidence 名 | §9.1.11、§9.1.24、Phase 詳細節、manifest の `Evidence map` で保存先と名前を一致させる |

**変更種別ごとの同時更新必須箇所：**

| 変更種別 | 同時更新必須箇所 |
|----------|------------------|
| 新 API / route | §6 または §9.5、§7.3、§9.1.20、§9.17、該当 Phase 詳細節、API 契約 ID、request/response/error snapshot |
| 新 error code | §7.3、§9.1.22、該当 endpoint 表、該当 Phase 詳細節、error matrix、client action snapshot |
| 新 metadata / file | §9.6、§9.1.21、該当 Phase 詳細節、migration plan、rollback / recovery fixture |
| 新 auth / scope | §9.1.18、§9.14、該当 endpoint 表、JWT / token fixture、permission matrix |
| 新 list API | §9.1.27、endpoint 表、pagination matrix、cursor fixture、order snapshot |
| SQL execution 変更 | §9.1.28、§6.2 / §6.3、SDK transcript、result mapping snapshot、transaction fixture |
| Phase スコープ変更 | §9 Phase 一覧、§9.2、§9.4、§9.8、§9.10、§9.11、該当 Phase 詳細節 |
| Turso 互換変更 | §9.1.23、Phase 8 または該当 Phase 詳細節、compatibility diff、Turso snapshot source |

**古い参照の扱い：**

| 状態 | 扱い |
|------|------|
| 存在しない節番号を参照している | merge 不可 |
| 古い Phase 名 / Phase 番号が残っている | 仕様未確定 |
| TC 範囲と実際の TC 定義数が一致しない | Phase 未完了 |
| Task 範囲と実装タスク表が一致しない | 実装開始禁止 |
| endpoint 表と Phase 詳細節の status / body / error が違う | endpoint 表を正とせず、同じ PR で解消するまで実装禁止 |
| error code が §7.3 にない | 新 code を使う実装禁止 |
| snapshot 名だけ存在し、生成条件がない | evidence 不足 |
| manifest だけ更新され本文がない | 仕様として扱わない |

**PR self-check 必須項目：**

| Check | 必須確認 |
|-------|----------|
| section reference scan | 追加・変更した `§` 参照が実在し、見出し番号と一致する |
| Phase table scan | §9 の Phase 一覧、§9.2、§9.4、§9.8、§9.11 の対象 Phase 行が同じスコープを表す |
| endpoint scan | method/path/auth/status/body/error が §6 / §9.5 / Phase 詳細節で一致する |
| evidence scan | manifest、artifact path、snapshot 名、test ID が一致する |
| compatibility scan | Turso / libSQL SDK 影響が §9.1.23 と該当 Phase 節に同じ分類で記録されている |
| stale text scan | 旧バージョン番号、旧 Phase 境界、旧対象外理由、旧 API 名が残っていない |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 本文だけ変更して Phase 表 / test matrix を更新しない | merge 不可 |
| Phase 詳細節だけ変更して §9.2 / §9.8 / §9.11 を更新しない | merge 不可 |
| error code を本文に書くが §7.3 に追加しない | merge 不可 |
| API response を変えるが snapshot / SDK transcript を更新しない | Phase 未完了 |
| 古い参照を「後で直す」として残す | 仕様未確定 |
| PR description だけで整合性を説明し、仕様本文に反映しない | 仕様として扱わない |

cross-reference に関係する仕様変更は、変更した節だけでなく、参照元、参照先、Phase 一覧、Phase 完了ゲート、test matrix、Definition of Ready / Done、manifest、evidence artifact を同時確認する。相互参照の機械確認が未整備の場合でも、PR 内で上記 self-check を完了し、未確認項目を `N/A` にしてはならない。

#### 9.1.30 Backup / restore / PITR / rollback 固定契約

backup、restore、PITR、branch seed、WAL archive replay に関係する Phase は、元 DB を破壊しない commit / rollback 境界、排他 lock、temporary layout、検証順序、失敗 marker、recovery evidence を実装開始前に固定しなければならない。破壊的操作は「成功応答を返した時点で、再起動後も新状態が一貫している」または「失敗応答後に旧状態へ戻っている」のどちらかだけを許可する。

**backup 契約：**

| 項目 | 固定仕様 |
|------|----------|
| snapshot | SQLite Online Backup API 相当の一貫 snapshot を返す。copy 中の filesystem 直読みは禁止 |
| concurrent write | backup 中の通常 write はブロックしない。backup snapshot に含まれるかどうかは snapshot 開始時点で固定する |
| response header | `Content-Type: application/octet-stream`、`Content-Disposition: attachment; filename="{db}.db"` |
| stream failure | client 切断時は backup を中断し、partial response を成功扱いにしない。DB 状態は変更しない |
| block / quota | `block_reads=true` は backup download を `403 PERMISSION_DENIED`。quota 超過中でも backup は許可 |
| redaction | backup body、SQLite page、absolute path は log / artifact に出さない。size、checksum、duration だけ許可 |

**restore / PITR 状態遷移：**

| 状態 | 必須条件 | 次状態 |
|------|----------|--------|
| `prepared` | request validation、auth、scope、quota/block 事前判定、exclusive lock 取得が完了 | `uploaded` または `replaying` |
| `uploaded` | upload body を temp に保存し fsync 済み | `verifying` |
| `replaying` | PITR snapshot と WAL frame を temp DB へ replay 中 | `verifying` |
| `verifying` | `PRAGMA integrity_check`、checksum、schema 互換確認を実行 | `committing` または `rolled_back` |
| `committing` | runtime DB close、old 退避、new rename、directory fsync、DB reopen を順に実行 | `committed` または `rolled_back` |
| `committed` | success response 可能。`204 No Content` 以外を返さない | terminal |
| `rolled_back` | old DB を復元し runtime DB reopen 済み | terminal |
| `failed_unrecoverable` | old DB 復元不能。`restore-failed.json` を fsync 済み | 起動失敗 / operator 対応 |

**temp layout / marker：**

| Path | 内容 |
|------|------|
| `{data-dir}/databases/{db}/restore-{request_id}/upload.db` | restore upload body。backup body を log に出さない |
| `{data-dir}/databases/{db}/restore-{request_id}/verified.db` | integrity_check 済み commit 候補 |
| `{data-dir}/databases/{db}/restore-{request_id}/old/data.db` | rollback 用旧 DB |
| `{data-dir}/databases/{db}/restore-{request_id}/restore-state.json` | state、request_id、target、started_at、phase、checksums |
| `{data-dir}/meta/restore-failed.json` | unrecoverable failure marker。存在する場合は対象 DB を起動時に open しない |

temp directory は data-dir 配下だけに作成する。request_id は §9.1.17 の秘匿規則に従い、path、token、SQL、user input body を含めない。cleanup は `committed` または `rolled_back` が fsync 済みであることを確認してから行い、cleanup 失敗は WARN として次回起動時に再試行する。

**restore / PITR request validation：**

| 項目 | 固定仕様 |
|------|----------|
| restore body | SQLite DB file binary。body size は設定値 `restore_max_bytes` が未定義の間、既存 DB size の 2 倍または 1 GiB の小さい方を上限 |
| PITR selector | `timestamp` または `frame_no` のどちらか 1 つだけ。両方あり、両方なし、型不正は `INVALID_REQUEST` |
| PITR disabled | `wal_retention_days = 0` または archive 未初期化は `503 PITR_NOT_ENABLED` |
| frame lookup | timestamp は manifest の frame timestamp へ決定的に解決する。同値境界は target 以下の最大 frame |
| checksum | snapshot と WAL frame は replay 前に checksum 検証。不一致は `RESTORE_FRAME_CORRUPT` |
| missing frame | target までの連続 frame が欠ける場合は `FRAME_NOT_FOUND` |
| integrity | replay 後または upload 後の DB は `PRAGMA integrity_check` が `ok` の場合だけ commit |

**拒否条件 precedence：**

| 順位 | 条件 | 失敗時 |
|------|------|--------|
| 1 | request / body / selector validation | `400 INVALID_REQUEST` または `413 PAYLOAD_TOO_LARGE` |
| 2 | auth / admin scope | `AUTH_REQUIRED`、`AUTH_INVALID`、`PERMISSION_DENIED` |
| 3 | DB existence / identity validation | `DB_NOT_FOUND`、`INVALID_DB_NAME` |
| 4 | `delete_protection=true` | `403 ORG_SCOPE_DENIED` |
| 5 | `block_reads=true` for backup | `403 PERMISSION_DENIED` |
| 6 | `block_writes=true` for restore / PITR | `403 PERMISSION_DENIED` |
| 7 | quota after restore/PITR candidate size calculation | `402 QUOTA_EXCEEDED` |
| 8 | restore lock / DB busy / shutdown | `503 STORAGE_BUSY` |
| 9 | PITR archive / checksum / integrity | `PITR_NOT_ENABLED`、`FRAME_NOT_FOUND`、`RESTORE_FRAME_CORRUPT`、`RESTORE_INTEGRITY_FAILED` |

**shutdown / restart recovery：**

| 状態 | 起動時挙動 |
|------|------------|
| temp directory exists, no committed marker | old DB がある場合は rollback して起動。old DB がなければ起動失敗 |
| `committing` marker exists | old/new の実ファイル状態を検査し、どちらか一方へ決定的に収束させる。silent success 禁止 |
| `committed` marker exists, cleanup 未完了 | DB を通常 open し、temp cleanup を WARN 付きで再試行 |
| `rolled_back` marker exists, cleanup 未完了 | old DB を通常 open し、temp cleanup を WARN 付きで再試行 |
| `restore-failed.json` exists | 対象 DB を open せず、health / admin detail で degraded を示す。operator が marker を解消するまで write 禁止 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| backup consistency artifact | backup 中 write、snapshot integrity、response header、client disconnect |
| restore rollback fixture | invalid upload、commit 前失敗、commit 中失敗、DB reopen 失敗、old 復元 |
| PITR replay fixture | timestamp selector、frame_no selector、missing frame、checksum corrupt、retention disabled |
| lock matrix | restore 中 read/write、backup 中 write、shutdown 中 restore、concurrent restore |
| precedence matrix | auth、delete_protection、block_reads、block_writes、quota、storage busy の優先順位 |
| recovery log | restart recovery、cleanup retry、unrecoverable marker、operator action |
| redaction snapshot | backup body、uploaded DB、WAL frame bytes、absolute path、token が出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| 元 DB を直接上書きしてから検証する | merge 不可 |
| integrity_check 前に runtime DB として公開する | merge 不可 |
| rollback 不能なのに 2xx success を返す | merge 不可 |
| PITR checksum 不一致 frame を skip して続行する | merge 不可 |
| restore 中 write を受け付ける | merge 不可 |
| backup / upload body を log / artifact に残す | merge 不可 |
| `restore-failed.json` を無視して起動する | merge 不可 |

backup / restore / PITR に関係する仕様変更は、§6.4、§7.3、§9.1.16、§9.1.18、§9.1.21、§9.1.22、§9.1.24、§9.1.29、§9.6、§9.14、Phase 13 / 14 / 15 詳細節、manifest の Endpoint / Persistence / Security / Regression map、backup consistency artifact、restore rollback fixture、PITR replay fixture、recovery log を同時更新する。正常系だけが通っても、rollback、restart recovery、checksum、lock、quota/block precedence、redaction が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.31 Branch lifecycle / seed / isolation 固定契約

branch 作成、branch 削除、Turso database seed、source DB 削除、branch routing に関係する Phase は、branch metadata、branch DB directory、runtime map、token scope、quota、source selector、restart recovery の関係を実装開始前に固定しなければならない。branch は通常 DB の別名ではなく、source から作成された独立 DB resource として扱う。

**branch create 状態遷移：**

| 状態 | 必須条件 | 次状態 |
|------|----------|--------|
| `creating` | name validation、source lookup、auth/scope、quota/block 判定、source lock 取得が完了 | `materializing` |
| `materializing` | current backup または PITR replay で temp DB を構築中 | `verifying` |
| `verifying` | integrity_check、source selector 記録、internal DB name 衝突確認 | `activating` または `rolled_back` |
| `activating` | branch DB directory finalize、runtime open、`branches.json` atomic commit | `active` または `rolled_back` |
| `active` | pipeline routing と admin list/detail に公開可能 | terminal |
| `rolled_back` | temp/partial directory を cleanup し、metadata 未公開 | terminal |

`branches.json` に `active` として commit する前に branch pipeline が成功してはならない。runtime map にだけ存在し metadata に存在しない branch は restart 後に消えるため、success response を返してはならない。

**branch delete 状態遷移：**

| 状態 | 必須条件 | 次状態 |
|------|----------|--------|
| `deleting` | branch lookup、auth/scope、branch exclusive lock 取得、new connection 拒否が完了 | `finalizing` |
| `finalizing` | runtime map から除去、active connection close、directory を trash へ rename、`branches.json` atomic commit | `deleted` または `delete_failed` |
| `deleted` | branch route は `DB_NOT_FOUND`。trash cleanup は完了または再試行可能 | terminal |
| `delete_failed` | metadata / directory の片方だけが残る可能性を recovery log に記録 | 起動時 recovery |

DELETE は Phase 15 では冪等 `204` とする。存在しない branch への DELETE は metadata と directory がどちらも存在しない場合だけ `204` とし、metadata 破損や directory だけ残る状態を silent success にしない。

**identity / routing / metadata：**

| 項目 | 固定仕様 |
|------|----------|
| branch name | §9.1.26 の `branch name` validation に従う。自動 lowercase / trim 禁止 |
| internal DB name | `{source_db}___{branch_name}`。external DB create では `___` を含む name を常に `DB_RESERVED_NAME` |
| metadata key | `source_db` + `branch_name` を logical key とし、internal DB name だけを primary key にしない |
| route | `/{source}___{branch}/v2/pipeline` は `branches.json` active entry と DB directory の両方がある場合だけ許可 |
| list order | §9.1.27 に従い、`created_at` 昇順、同値は `source_db`、`branch_name` 昇順 |
| response | branch API は `branch_name`、`source_db`、`db_name`、`from`、`created_at`、`state` を返す。filesystem path は返さない |

**source selector / Turso seed：**

| 入力 | 固定仕様 |
|------|----------|
| `from:"current"` | source DB の Online Backup API 相当で snapshot を作る。source write はブロックしない |
| `from:{timestamp}` | §9.1.30 の PITR timestamp selector と同じ解決規則を使う |
| `from:{frame_no}` | §9.1.30 の PITR frame selector と同じ checksum / missing frame 規則を使う |
| `/v1/* seed.type:"database"` | Phase 15 で有効化する場合、branch 名 field、source database field、group/org scope、Turso snapshot を同じ PR で固定する |
| branch 名なし seed | `INVALID_REQUEST`。source DB と同名の暗黙 branch 作成は禁止 |
| source not found | scope 判定後に `DB_NOT_FOUND`。存在漏洩は §9.1.18 に従う |

**isolation / permission / quota：**

| 項目 | 固定仕様 |
|------|----------|
| write isolation | branch write は source DB に反映しない。source write は既存 branch に反映しない |
| token scope | source DB token は branch DB へ自動拡張しない。branch DB 用 token は別途発行する |
| org/group/location | branch は source の organization、group、location を継承する。変更 API は Phase 15 対象外 |
| quota | branch 作成時に organization/group quota と branch DB candidate size を判定する。database quota は source の値を初期値として copy |
| source delete | active branch がある source DB delete は `403 ORG_SCOPE_DENIED`。cascade delete と orphan 化は禁止 |
| source block_reads | branch create は `403 PERMISSION_DENIED` |
| source block_writes | branch create は読み取り snapshot のため許可。ただし branch DB 作成先 quota / block policy は判定する |
| delete_protection | source DB の delete_protection は branch create を禁止しない。branch 自身の delete_protection は branch delete を禁止する |

**restart recovery：**

| 状態 | 起動時挙動 |
|------|------------|
| metadata active + directory missing | 起動失敗。silent cleanup 禁止 |
| metadata active + integrity_check failed | 起動失敗または branch disabled を Phase 節で明記。未定義なら起動失敗 |
| directory exists + metadata missing | partial create として WARN、接続不可、cleanup 対象 |
| trash exists + metadata deleted | cleanup 再試行。cleanup 失敗は WARN で起動継続 |
| metadata deleting + directory exists | delete recovery を実行し、完了まで branch route は `DB_NOT_FOUND` |
| duplicate branch key | metadata 破損として起動失敗。先勝ち/後勝ち禁止 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| branch lifecycle fixture | creating/materializing/verifying/active/deleting/deleted の metadata sample |
| branch race fixture | 同名 create、create 中 delete、source delete、concurrent route access |
| restart recovery fixture | metadata active + missing dir、dir only、trash cleanup、duplicate key |
| seed compatibility snapshot | `/admin/v1/*` branch API と `/v1/* seed.type:"database"` の成功/失敗差分 |
| isolation matrix | source write、branch write、token scope、quota、block_reads/block_writes |
| PITR branch fixture | timestamp/frame selector、missing frame、checksum corrupt、retention disabled |
| redaction snapshot | branch path、absolute path、token、source selector raw body が log / artifact に出ないこと |

**禁止事項：**

| 状態 | 判定 |
|------|------|
| metadata commit 前に branch route を成功させる | merge 不可 |
| source DB token を branch DB に自動適用する | merge 不可 |
| source DB delete で active branch を orphan 化する | merge 不可 |
| directory だけ存在する branch を active 扱いする | merge 不可 |
| branch create retry で二重 DB / 二重 quota charge を起こす | merge 不可 |
| branch internal DB name を通常 DB create で受理する | merge 不可 |
| branch path / absolute path を response、log、artifact に出す | merge 不可 |

branch に関係する仕様変更は、§6.4、§7.3、§9.1.16、§9.1.18、§9.1.21、§9.1.22、§9.1.26、§9.1.27、§9.1.29、§9.1.30、§9.5、§9.6、§9.14、Phase 8 / 14 / 15 詳細節、manifest の Endpoint / Persistence / Security / Compatibility / Regression map、branch lifecycle fixture、branch race fixture、restart recovery fixture、seed compatibility snapshot、isolation matrix を同時更新する。正常系だけが通っても、metadata/file commit 順序、restart recovery、source delete denial、token scope、quota、Turso seed 互換が固定されていない場合は Phase 完了扱いにしない。

#### 9.1.32 Phase implementation packet / per-phase execution contract 固定契約

各 Phase の実装 PR は、実装開始前に Phase implementation packet を 1 つ作成し、その Phase で読むべき仕様、変更してよい範囲、変更してはならない範囲、完了証跡を 1 箇所に固定しなければならない。Phase 詳細節、§9.2、§9.4、§9.8、§9.11、§9.17 を実装者が手作業で突き合わせないと判断できない状態は、実装開始不可とする。

**Phase packet 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `phase` | `Phase N`、対象 version、対象 commit |
| `scope` | 実装する機能と、同 Phase で実装しない対象外機能 |
| `entry_sections` | 実装前に読む節番号。最低でも §9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節 |
| `target_surface` | 変更対象 module、API、CLI、config、metadata、runtime state |
| `api_contracts` | method/path/auth/request/success/error/snapshot/SDK transcript |
| `persistence_contracts` | file path、schema、update order、fsync、rollback、recovery |
| `security_contracts` | auth、scope、ro/rw、quota、block policy、redaction |
| `concurrency_contracts` | lock、idempotency、retry、shutdown、long-running operation |
| `compatibility_contracts` | Turso / libSQL SDK / legacy metadata / previous Phase への影響 |
| `evidence_plan` | test、snapshot、fixture、log、artifact、secret scan の保存先 |
| `regression_set` | 当該 Phase と過去 Phase の実行必須コマンド |
| `unsupported_behavior` | 未来 Phase、stub、501/400/404/405 の固定挙動 |
| `dependency_graph` | API、persistence、security、compatibility、oracle、evidence、operational state の prerequisite |
| `invariant_ledger` | §9.1.39 の invariant ID、scope、before/after 条件、violation signal、regression guard |
| `scenario_matrix` | §9.1.40 の normal/error/auth/persistence/rollback/concurrency/compatibility/unsupported/redaction/operational scenario |
| `resource_lifecycle_matrix` | §9.1.41 の resource type、state、allowed/forbidden transition、commit order、recovery behavior |
| `schema_registry` | §9.1.42 の field-level schema、required/null/default/migration/compatibility/redaction 契約 |
| `decision_precedence_matrix` | §9.1.43 の複数条件同時成立時の優先順位、selected behavior、losing behavior、error/status/client action |
| `coverage_closure_matrix` | §9.1.44 の Contract ID / source matrix / test / artifact / oracle / N/A reason の網羅完了表 |
| `change_impact_matrix` | §9.1.45 の実装中変更に対する影響範囲、同時更新対象、承認状態、drift closure |
| `rollout_readiness_matrix` | §9.1.46 の起動、停止、再起動、rollback、health、operator action、release 可否 |
| `compatibility_baseline_matrix` | §9.1.47 の Turso Cloud / libSQL SDK 互換 baseline、refresh trigger、差分分類、証跡 |
| `security_abuse_matrix` | §9.1.48 の attack surface、untrusted input、bypass attempt、denial、redaction、audit、regression |
| `ambiguity_closure_matrix` | §9.1.49 の implementation question、推奨決定、却下案、根拠、影響範囲、証跡 |
| `atomic_task_ledger` | §9.1.50 の task ID、入力契約、変更対象、禁止変更、完了条件、検証、rollback |
| `review_handoff_packet` | §9.1.51 の読む順番、再現 command、期待 artifact、判断基準、失敗分類、レビュー禁止事項 |
| `operator_behavior_delta` | §9.1.52 の外部挙動、運用影響、互換差分、設定移行、rollback、release note |
| `completion_gate` | merge 前に満たす Done 条件と失敗時の扱い |

**canonical readiness packet format：**

Phase implementation packet は、実装開始前に以下の canonical key をこの順序で持たなければならない。Markdown table または JSON object のどちらでもよいが、key 名、順序、必須/任意、pass 条件を変えてはならない。

| key | 必須値 |
|-----|--------|
| `phase` | `Phase {N}`。N は 1〜19 の整数 |
| `spec_version` | 実装開始時点の仕様書 version。累積 version を使い、Phase 番号や日付で代替しない |
| `base_commit` | 実装開始前の commit SHA。PR branch の起点を示す |
| `entry_sections` | §9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節、必要な §9.1.x の一覧 |
| `scope_in` | この Phase で成功応答、永続化、運用証跡まで完成させる対象 |
| `scope_out` | 未来 Phase、stub、unsupported、明示対象外の対象と根拠 section |
| `contract_ids` | API / Persistence / Security / Compatibility / Regression / Precision closure の Contract ID 一覧 |
| `task_ids` | 対象 Phase の atomic task ID 一覧。各 task は入力契約、禁止変更、完了条件、verification command に接続する |
| `scenario_ids` | 対象 Phase の scenario ID 一覧。各 scenario は expected result、artifact path、oracle に接続する |
| `artifact_paths` | test、snapshot、fixture、log、manifest、review handoff、secret scan の保存先一覧 |
| `audit_results` | §9.17 の `phase_packet_result`、`acceptance_manifest_result`、`oracle_result`、`scenario_matrix_result`、`schema_registry_result`、`decision_precedence_result`、`coverage_closure_result`、`artifact_paths_result`、`failure_remediation_result`、`transition_ready_result`、`review_handoff_result`、`operator_delta_result`、`precision_closure_index_result`、`change_impact_closure_result`、`rollout_readiness_closure_result`、`compatibility_baseline_closure_result`、`security_abuse_closure_result`、`configuration_environment_closure_result`、`ambiguity_atomic_task_closure_result`、`review_operator_closure_result`、`merge_readiness_closure_result`、`dependency_provenance_closure_result`、`upgrade_data_compatibility_closure_result`、`performance_capacity_closure_result`、`incident_recovery_closure_result`、`contract_versioning_closure_result`、`machine_contract_artifact_closure_result`、`phase_execution_sequence_closure_result`、`verdict_normalization_closure_result`、`release_handoff_closure_result`、`defect_prevention_closure_result`、`artifact_layout_closure_result`、`upstream_observation_closure_result`、`ownership_approval_closure_result`、`operational_readiness_closure_result`、`ci_command_matrix_closure_result`、`state_invariant_closure_result`、`compatibility_delta_closure_result`、`artifact_contract_sync_closure_result`、`review_checklist_closure_result`、`post_merge_verification_closure_result`、`exception_deferral_closure_result`、`migration_compatibility_closure_result`、`configuration_secret_closure_result`、`data_retention_privacy_closure_result`、`documentation_runbook_closure_result`、`sbom_vulnerability_license_closure_result`、`telemetry_signal_contract_closure_result`、`phase_done_final_result`、`readiness_audit_result` を全て含む |
| `blocking_items` | 実装開始前に残っている blocker 一覧。実装開始可能な packet では空配列または `none` |
| `ready_to_implement` | 実装開始を許可する最終 boolean。`true` 以外は実装開始禁止 |

`ready_to_implement = true` にできるのは、実装開始時に評価可能な `audit_results` field がすべて `pass`、`blocking_items` が 0 件、根拠なし `N/A` が 0 件、未接続の Contract ID / Task ID / Scenario ID / artifact path が 0 件、かつ `scope_in` / `scope_out` が仕様本文と一致する場合だけである。`phase_done_final_result` は実装完了時の出口 gate として Done receipt で評価する。`ready_to_implement` が未記載、`false`、文字列、または pass 根拠なしの場合、その Phase は実装開始禁止とする。

canonical readiness packet と §9.17 の readiness audit は同じ入口 gate を表す。どちらか一方だけを更新してはならない。Phase packet、受入 manifest、Done receipt、artifact manifest、review handoff のいずれかで `phase`、`spec_version`、`contract_ids`、`task_ids`、`scenario_ids`、`artifact_paths`、`audit_results` が異なる場合は、仕様修正 PR に戻す。

**Phase group 粒度：**

| Phase group | Packet で特に固定すること |
|-------------|---------------------------|
| Phase 1〜5 | CLI、data-dir、default DB、hrana-http、JWT、ログ、SDK smoke、再起動永続性 |
| Phase 6〜8 | multi DB、admin API、Turso Platform API、metadata migration、organization/group/location/quota、legacy fallback |
| Phase 9〜10 | WebSocket stream/transaction、ATTACH policy、metrics counter 更新点、任意 path 拒否 |
| Phase 11〜13 | replication primary/replica、frame number、checksum、archive manifest、retention、health/redirect |
| Phase 14〜15 | backup/restore/PITR、branch lifecycle、destructive rollback、source snapshot、restart recovery |
| Phase 16〜18 | extension manifest/signature、metrics persistence、HA term/leader/split-brain、operator action |
| Phase 19 | internal adapter shadow/active/rollback、performance baseline、Phase 1〜18 regression、wire/API 差分ゼロ |

**実装開始禁止条件：**

| 状態 | 判定 |
|------|------|
| Phase packet がない | 実装開始禁止 |
| `scope` と §9.2 / §9.4 / Phase 詳細節が一致しない | 仕様修正 PR に戻す |
| `api_contracts` に snapshot / error case がない外部 API を追加する | route 追加禁止 |
| `persistence_contracts` に rollback / recovery がない状態変更を追加する | 書き込み処理禁止 |
| `security_contracts` に auth/scope/redaction がない管理 API を追加する | success response 禁止 |
| `regression_set` が固定されていない | 完了判定不可 |
| unsupported/stub の status と body が未定義 | future route 実装禁止 |

**完了扱い禁止条件：**

| 状態 | 判定 |
|------|------|
| Phase packet と実装差分が一致しない | Phase 未完了 |
| packet の `evidence_plan` にある artifact が生成されていない | Phase 未完了 |
| packet 外の API / config / metadata / dependency を追加している | merge 不可 |
| unsupported 対象が成功応答を返す | merge 不可 |
| regression set の一部を未実行にしている | Phase 未完了 |
| failure / flaky / TODO を後続 PR に持ち越す | Phase 未完了 |
| packet 更新が PR description のみで仕様本文にない | 仕様として扱わない |

**Phase packet と既存節の関係：**

| 既存節 | Packet への反映 |
|--------|-----------------|
| §9.2 | `scope`、`completion_gate`、対象外 |
| §9.4 | `target_surface`、Phase 別 API/永続化/error/test 契約 |
| §9.5 | `api_contracts`、API 契約 ID、snapshot |
| §9.6 | `persistence_contracts`、schema、recovery |
| §9.7 | `error code`、retry、client action |
| §9.8 | `regression_set`、test matrix |
| §9.11 | Definition of Ready / Done |
| §9.17 | 実装前チェックリスト |
| Phase 詳細節 | endpoint 固有仕様、TC、task、対象外 |

**必須 evidence：**

| Evidence | 必須内容 |
|----------|----------|
| phase packet artifact | `docs/phase-evidence/phase-{phase}/packet.md` または PR description の同等表 |
| contract mapping | API / Persistence / Security / Compatibility / Regression の各 map |
| unsupported snapshot | future route / config / body field の拒否結果 |
| regression transcript | fixed command、exit code、環境、artifact path |
| cross-reference scan | §9.1.29 の self-check 結果 |
| redaction scan | packet / artifact / log に secret、token、raw path、SQL args、backup body がないこと |

Phase packet に関係する仕様変更は、§9.1.9、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.29、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。Phase 単位で何を実装し、何を実装しないか、何をもって完了とするかが 1 箇所で読めない場合は、実装精度不足として Phase 未完了扱いにする。

#### 9.1.33 Phase completion gate / Done evidence / bug-zero acceptance 固定契約

各 Phase の完了判定は、コード差分の有無ではなく、Phase packet、受入 manifest、仕様本文、test、artifact、PR description、release check result が同じ契約集合を証明していることをもって行う。実装者、reviewer、後続 Phase 担当者が仕様本文だけで完了可否を再判定できない場合、その Phase は未完了とする。

**Done receipt 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `phase` | `Phase N`、対象仕様 version、対象 commit、対象 PR |
| `packet_ref` | §9.1.32 の Phase packet 参照と commit |
| `manifest_ref` | §9.1.10 の受入 manifest 参照と version |
| `implemented_scope` | §9.2 / §9.4 / Phase 詳細節で完了した項目 |
| `excluded_scope` | 同 Phase で意図的に実装しない対象と、その固定挙動 |
| `contract_map` | API、永続化、error、security、compatibility、concurrency、operation の契約 ID |
| `evidence_index` | artifact 名、保存先、生成コマンド、exit code、正規化内容 |
| `regression_result` | §9.8 の対象 TC、過去 Phase regression、SDK / CLI / HTTP / WS 実行結果 |
| `failure_closure` | 失敗、flaky、未検証、artifact 欠落、secret 混入が 0 件である根拠 |
| `compatibility_result` | Turso Cloud / libSQL SDK / legacy metadata / previous Phase への差分分類 |
| `redaction_result` | log、artifact、PR description、error body に secret / token / raw path / SQL args が残っていない根拠 |
| `release_check_result` | local / Docker / CI の実行コマンド、環境差分、再実行条件 |
| `oracle_result` | §9.1.35 の oracle 名、version、path、比較 command、exit code、差分理由 |
| `defect_classification_result` | §9.1.36 の defect 件数、分類別件数、`open:0`、evidence path |
| `operational_state_result` | §9.1.37 の state matrix、health snapshot、API behavior、recovery runbook |
| `dependency_graph_result` | §9.1.38 の prerequisite がすべて satisfied である証跡 |
| `invariant_result` | §9.1.39 の invariant ID ごとの pass/fail、violation 0 件、regression guard、evidence path |
| `scenario_matrix_result` | §9.1.40 の scenario ID ごとの pass/fail、not_applicable reason、evidence path、manual only 0 件 |
| `resource_lifecycle_result` | §9.1.41 の state / transition ごとの pass/fail、forbidden transition 0 件、recovery evidence path |
| `schema_registry_result` | §9.1.42 の schema ID ごとの field coverage、unknown/null/default/migration/redaction evidence |
| `decision_precedence_result` | §9.1.43 の decision ID ごとの precedence 実行結果、selected behavior、error/status snapshot、losing behavior 非発火証跡 |
| `coverage_closure_result` | §9.1.44 の coverage ID ごとの pass/fail/N/A、gap 0 件、test/artifact/oracle 実在証跡 |
| `change_impact_result` | §9.1.45 の change ID ごとの affected sections/matrices/tests/artifacts 更新完了、drift 0 件、承認証跡 |
| `rollout_readiness_result` | §9.1.46 の rollout ID ごとの startup/shutdown/restart/rollback/health/operator/compat/data safety 証跡 |
| `compatibility_baseline_result` | §9.1.47 の baseline ID ごとの upstream source、snapshot / SDK version、差分分類、refresh 可否、証跡 |
| `security_abuse_result` | §9.1.48 の security case ID ごとの bypass denial、redaction、audit/log、quota/rate/persistence、証跡 |
| `ambiguity_closure_result` | §9.1.49 の ambiguity ID ごとの採用決定、却下案、仕様反映、証跡、open ambiguity 0 件 |
| `atomic_task_result` | §9.1.50 の task ID ごとの完了条件、検証 command、証跡、rollback 可否、open task 0 件 |
| `review_handoff_result` | §9.1.51 の第三者再現 command、artifact、判断結果、失敗分類、口頭補足なし証跡 |
| `operator_behavior_delta_result` | §9.1.52 の external behavior、operator impact、compatibility delta、migration、rollback、release note 証跡 |
| `reviewer_decision` | `Done`、`Not Done`、`Spec correction required` のいずれか |

**canonical phase done receipt final audit：**

Phase Done receipt は、実装完了時に以下の final audit field をこの順序で持たなければならない。各 field は `pass`、`fail`、または仕様本文に根拠がある `not_applicable` のみ許可する。`manual_only`、`partial`、`not_run`、`accepted_risk`、空欄は Phase 完了根拠にしてはならない。

| final audit field | pass 条件 |
|-------------------|----------|
| `readiness_packet_match_result` | §9.1.32 の readiness packet と Done receipt の phase、spec_version、scope、Contract ID、Task ID、Scenario ID が一致 |
| `acceptance_manifest_match_result` | §9.1.10 の受入 manifest と Done receipt の Contract map、Evidence path、Status、Regression set が一致 |
| `artifact_manifest_match_result` | §9.1.11 の artifact path、producer command、expected hash、secret scan、reviewer command が Done receipt と一致 |
| `failure_remediation_match_result` | §9.1.14 の failure remediation が全件 `closure_result = pass` で、open failure、flaky、unverified、secret leak が 0 件 |
| `regression_chain_match_result` | 対象 Phase と過去 Phase の regression が仕様本文の除外根拠なしに skip / not run されていない |
| `review_handoff_match_result` | §9.1.51 の review handoff command、expected artifact、decision criteria が Done receipt と一致 |
| `operator_delta_match_result` | §9.1.52 の operator behavior delta、rollback、health/log/metric、release note が Done receipt と一致 |
| `precision_closure_match_result` | §9.11.1〜§9.11.13 の precision closure、artifact manifest、reviewer reproduction、final reconciliation が Done receipt と一致 |
| `zero_bug_final_result` | known bugs、unverified、missing evidence、open failure、unsupported success、rootless N/A、stale artifact がすべて 0 件 |
| `phase_done_final_result` | 上記 9 field がすべて `pass` で、`reviewer_decision = Done` |

`phase_done_final_result = pass` にできるのは、known bugs 0 件、unverified 0 件、missing evidence 0 件、open failure 0 件、unsupported success 0 件、rootless `N/A` 0 件、stale artifact 0 件、かつ `readiness_packet_match_result` から `zero_bug_final_result` までがすべて pass の場合だけである。機能が動作していても `phase_done_final_result` が pass でない Phase は未完了とする。

Phase Done receipt を修正する場合は、Done receipt だけを書き換えてはならない。readiness packet、受入 manifest、artifact manifest、failure remediation record、review handoff、operator delta、precision closure のうち影響を受けるものを同じ PR で更新し、final audit を再実行する。

**Phase group Done minimum：**

| Phase group | 最低証跡 |
|-------------|----------|
| Phase 1〜5 | build/test、CLI help、config precedence、data-dir 再起動、hrana-http transcript、JWT / log redaction、SDK smoke |
| Phase 6〜8 | admin API / Platform API snapshot、metadata migration、auth scope、organization/group/location/quota、legacy fallback、Phase 1〜7 regression |
| Phase 9〜10 | WebSocket transcript、transaction rollback、managed ATTACH 拒否、metrics counter 更新、任意 path 拒否 |
| Phase 11〜13 | replication frame、checksum、snapshot、archive manifest、retention cleanup、corruption detection、primary/replica restart |
| Phase 14〜15 | backup manifest、restore rollback、PITR selector、branch lifecycle、source snapshot、seed isolation、restart recovery |
| Phase 16〜18 | extension signature/load policy、metrics persistence、Prometheus output、HA term、leader transition、split-brain prevention |
| Phase 19 | adapter shadow/active/rollback、wire/API 差分、performance baseline、Phase 1〜18 full regression |

**完了扱い禁止条件：**

| 状態 | 判定 |
|------|------|
| Done receipt がない | Phase 未完了 |
| Done receipt と Phase packet / manifest / 仕様本文が一致しない | Phase 未完了 |
| evidence artifact が再生成不能、または生成コマンドが未記載 | Phase 未完了 |
| regression set の一部が skipped / manual only / not run のまま | Phase 未完了 |
| unsupported 対象が success response、部分成功、暗黙 fallback を返す | merge 不可 |
| failure / flaky / TODO / FIXME / unimplemented を後続 PR に持ち越す | merge 不可 |
| secret / token / raw path / SQL args / backup body の秘匿確認がない | merge 不可 |
| compatibility diff が仕様化されていない | 仕様修正 PR に戻す |
| destructive operation、auth、quota、restore、branch delete、extension load の検証が manual only | merge 不可 |

**bug-zero acceptance：**

| 項目 | 必須条件 |
|------|----------|
| known bug | 0 件。既知不具合、既知 flaky、既知未検証、既知 artifact 欠落を残さない |
| detected bug | 同一 PR 内で code fix + test、または仕様誤りとして spec fix + test/evidence に変換する |
| regression bug | 影響 Phase、契約 ID、TC ID、再発防止 evidence を Done receipt に記録する |
| accepted risk | 使用禁止。risk acceptance ではなく仕様変更、対象外明記、または実装修正で閉じる |
| follow-up | 完了条件の代替に使わない。follow-up は Done 後の改善のみ許可する |

**必須 evidence artifact：**

| Artifact | 必須内容 |
|----------|----------|
| Done receipt | `docs/phase-evidence/phase-{phase}/done.md` または PR description の同等表 |
| contract coverage matrix | Phase packet の全契約 ID と実行結果の対応 |
| regression transcript | command、exit code、環境、artifact path、正規化済み出力 |
| failure closure log | 発生した失敗、原因、修正、再実行結果、残件 0 の宣言 |
| unsupported snapshot | 未来 Phase、対象外 endpoint / field / config / operation の拒否結果 |
| redaction scan | secret、token、raw path、SQL args、backup body が残っていない確認 |
| compatibility diff | Turso Cloud、libSQL SDK、legacy metadata、previous Phase との差分分類 |
| cross-reference scan | §9.1.29 の参照整合 self-check 結果 |

Phase completion gate に関係する仕様変更は、§9.1.9、§9.1.10、§9.1.11、§9.1.14、§9.1.24、§9.1.29、§9.1.32、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。Done receipt を満たさない Phase は、機能が動作していても仕様上は完了扱いにしない。

#### 9.1.34 Phase execution sequence / handoff / interruption 固定契約

各 Phase の実装は、Phase packet と Done receipt の間に固定された execution sequence を持たなければならない。実装者は、どの順番で契約を確定し、どの順番でコードへ反映し、どの時点で API を公開してよいかを Phase 開始前に固定する。実装途中で担当者が変わる、中断する、失敗を検出する、未来 Phase の helper を先に作る場合でも、この節に従い現在位置と次の作業を判定する。

**Execution sequence 必須 step：**

| Step | 名称 | 完了条件 | 次 step へ進む条件 |
|------|------|----------|--------------------|
| 0 | intake | `AGENTS.md`、仕様 version、branch 整合性、対象 Phase を確認する | local / remote が一致し、対象 Phase が明記されている |
| 1 | packet freeze | §9.1.32 の Phase packet と §9.1.10 の manifest を作成する | scope、target surface、unsupported behavior、regression set が固定済み |
| 2 | contract freeze | schema、error、persistence、auth、compatibility、concurrency 契約を確定する | §9.5 / §9.6 / §9.7 / Phase 詳細節の参照が一致済み |
| 3 | persistence / recovery | metadata、migration、atomic update、rollback、recovery を実装する | 破損、再起動、rollback の証跡計画が存在する |
| 4 | core logic | API 非公開の service logic、validation、permission、state transition を実装する | internal tests または unit tests で契約境界が確認済み |
| 5 | external surface | HTTP / WebSocket / CLI / config / admin API を公開する | auth、error、unsupported、redaction、compatibility の snapshot が固定済み |
| 6 | evidence run | normal、error、auth、persistence、restart、rollback、regression、secret scan を実行する | §9.1.11 の artifact が生成済み |
| 7 | Done receipt | §9.1.33 の Done receipt を作成し、failure closure を 0 件にする | reviewer が `Done` と判定できる |

**Handoff state 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `phase` | 対象 Phase、仕様 version、branch、commit |
| `current_step` | 上記 execution sequence の step 番号 |
| `completed_contracts` | 完了済み API / persistence / error / security / compatibility / regression 契約 ID |
| `open_contracts` | 未完了契約 ID と未完了理由 |
| `last_successful_command` | 最後に成功した command、exit code、artifact path |
| `last_failed_command` | 最後に失敗した command、exit code、原因、再実行条件。失敗がない場合は `none` |
| `allowed_next_changes` | 次に変更してよい file / module / spec 節 |
| `forbidden_changes` | 現在 step で変更してはいけない API / metadata / config / behavior |
| `next_command` | 次に実行する verification command |
| `blocking_decision` | ユーザー承認、仕様変更、依存修正、環境復旧など必要な判断。不要な場合は `none` |

**前倒し実装の扱い：**

| 前倒し内容 | 許可条件 | 禁止条件 |
|------------|----------|----------|
| internal helper / type | 外部 API、永続化 schema、config default、wire format を変えない | success response や metadata commit に接続する |
| future route stub | 501 `NOT_IMPLEMENTED` と固定 body のみ返す | 200 / 201 / 204 / 307 / partial success を返す |
| future metadata file | §9.6 に初期値、破損時挙動、読み飛ばし可否が明記済み | migration なしで既存起動 path に必須化する |
| future config key | default、validation、unknown key handling、secret redaction が明記済み | default 挙動を変える、または未定義 key を有効化する |
| future dependency | feature flag off で既存挙動に影響しない | build/test/release-check を遅くするだけの未使用依存 |

**中断・再開ルール：**

| 状態 | 必須対応 |
|------|----------|
| Step 1 前に中断 | Phase 実装未開始として扱い、再開時に packet を作り直す |
| Step 2〜4 で中断 | Handoff state を残し、API success response を公開しない |
| Step 5 で中断 | 公開 surface を 501 / auth reject / feature flag off のいずれかに戻す |
| Step 6 で失敗 | §9.1.14 の failure closure に記録し、同一 PR で修正または仕様修正する |
| Step 7 前に未検証が残る | Done receipt 作成禁止 |
| 再開時に仕様 version が進んでいる | Phase packet、manifest、contract map を新 version に合わせて再確認する |

**実装順序違反の扱い：**

| 違反 | 判定 |
|------|------|
| persistence / rollback 未確定で external API を公開する | merge 不可 |
| error code 未確定で route を追加する | merge 不可 |
| auth / quota / scope 未確定で管理 API success response を返す | merge 不可 |
| migration / recovery 未確定で metadata schema を変更する | Phase 未完了 |
| test / artifact より先に Done receipt を作成する | Phase 未完了 |
| handoff state なしで途中作業を引き継ぐ | Phase 未完了 |

Phase execution sequence に関係する仕様変更は、§9.1.9、§9.1.10、§9.1.11、§9.1.14、§9.1.32、§9.1.33、§9.2、§9.4、§9.8、§9.11、§9.16、§9.17、該当 Phase 詳細節を同時更新する。実装者が現在 step、次 step、禁止変更を仕様本文だけで判断できない場合は、実装開始不可とする。

#### 9.1.35 Phase acceptance oracle / golden fixture 固定契約

各 Phase の完了判定には、実装結果を照合する acceptance oracle を持たなければならない。oracle は「実装が出した結果」ではなく「仕様が要求する正解」であり、snapshot、fixture、transcript、manifest、baseline のいずれかとして保存する。実装者が期待値を実装後に都合よく作る、snapshot 更新で差分を隠す、手元目視だけで一致扱いにすることは禁止する。

**Oracle 必須 artifact：**

| Artifact | 必須内容 | 適用 Phase |
|----------|----------|------------|
| API snapshot | method、path、status、headers subset、body schema、error body、content type | API を公開する全 Phase |
| error snapshot | error code、HTTP status、wire surface、retry、client action、precedence | 全 Phase |
| persistence fixture | metadata JSON、DB/file layout、atomic update 後状態、破損 fixture、recovery 後状態 | 永続化を変更する全 Phase |
| migration fixture | old format、new format、missing field、unknown field、corrupt file、rollback marker | metadata schema を変更する Phase |
| SDK transcript | TypeScript SDK / libSQL SDK / WebSocket client の request/response transcript | Phase 5 以降、WebSocket は Phase 9 以降 |
| unsupported snapshot | 未来 Phase、対象外 API、未対応 field、未対応 config、stub route の拒否結果 | 全 Phase |
| security fixture | auth failure、scope denial、quota denial、redaction、secret scan 結果 | auth / admin / quota / replication / HA / extension Phase |
| compatibility baseline | Turso Cloud、libSQL SDK、legacy metadata、previous Phase との差分分類 | Phase 5 以降 |
| regression baseline | 該当 Phase より前の TC / command / snapshot の再実行結果 | 全 Phase |

**Phase group oracle minimum：**

| Phase group | 最低 oracle |
|-------------|-------------|
| Phase 1〜5 | CLI help snapshot、config precedence fixture、hrana-http snapshot、JWT/auth error snapshot、log redaction fixture、TypeScript SDK transcript |
| Phase 6〜8 | admin API snapshot、Turso Platform API snapshot、metadata migration fixture、org/group/location/quota fixture、scope denial matrix、legacy fallback fixture |
| Phase 9〜10 | hrana-ws transcript、transaction commit/rollback fixture、store_sql cache snapshot、ATTACH allow/deny fixture、metrics counter baseline |
| Phase 11〜13 | replication log/snapshot transcript、frame checksum fixture、replica catchup fixture、archive manifest fixture、retention cleanup fixture |
| Phase 14〜15 | backup snapshot manifest、restore rollback fixture、PITR selector fixture、branch seed fixture、branch isolation/restart fixture |
| Phase 16〜18 | extension manifest/signature fixture、extension load failure snapshot、metrics snapshot/prometheus baseline、HA term/leader fixture、split-brain rejection transcript |
| Phase 19 | adapter shadow/active/rollback baseline、wire/API diff snapshot、performance baseline、Phase 1〜18 regression baseline |

**Snapshot / fixture 更新条件：**

| 状態 | 判定 |
|------|------|
| 仕様変更 commit なしに expected snapshot を更新する | merge 不可 |
| 実装差分だけを理由に oracle を更新する | merge 不可 |
| dynamic 値を placeholder 正規化せずに snapshot 化する | Phase 未完了 |
| secret、token、JWT signature、raw path、SQL args、backup body が oracle に残る | merge 不可 |
| failed / skipped / flaky の artifact を oracle として採用する | Phase 未完了 |
| body schema だけで status、header、error code を比較しない | Phase 未完了 |
| snapshot 差分理由が仕様本文にない | 仕様修正 PR に戻す |
| oracle 更新と実装修正を同じ PR で行う場合に差分理由がない | review failure |

**Oracle 正規化ルール：**

| 値 | 正規化 |
|----|--------|
| timestamp | `<timestamp:rfc3339>`。秒精度より細かい値は比較対象にしない |
| UUID / generated id | `<id:{prefix}>`。prefix と形式だけ比較する |
| request id | `<request_id>` |
| token / JWT / secret | `<redacted>`。生値が残る場合は failure |
| data-dir / absolute path | `<data_dir>` または `<tmp_dir>` |
| SQL args / backup body | `<redacted>`。値の有無と型だけ比較する |
| JSON object key order | 辞書順へ正規化する |
| array order | API 契約で定義した順序を保持する。test 側で sort し直さない |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `oracle_result` | 使用した oracle 名、version、path、生成/比較 command、exit code |
| `oracle_diff` | 差分なし、または仕様変更に基づく差分理由と参照 commit |
| `oracle_update_policy` | snapshot / fixture を更新した場合の仕様変更参照。更新なしなら `not updated` |

**Oracle 禁止事項：**

| 禁止事項 | 判定 |
|----------|------|
| production code の現在出力をそのまま expected として採用する | merge 不可 |
| regression failure を snapshot 更新で消す | merge 不可 |
| Turso Cloud / libSQL SDK 互換差分を自己ホスト都合だけで許容する | 仕様修正 PR に戻す |
| unsupported API の success response を snapshot として固定する | merge 不可 |
| human-readable log だけを正解 artifact にする | Phase 未完了 |
| oracle なしで Done receipt を作成する | Phase 未完了 |

Phase acceptance oracle に関係する仕様変更は、§9.1.3、§9.1.10、§9.1.11、§9.1.23、§9.1.24、§9.1.27、§9.1.28、§9.1.29、§9.1.33、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。oracle が固定されていない機能は、実装が動作していても Phase 完了扱いにしない。

#### 9.1.36 Phase defect classification / zero-bug triage 固定契約

各 Phase の実装中に見つかった不具合、仕様不足、互換差分、検証不足は、発見時点で defect classification に記録し、同一 PR 内で閉じなければならない。分類されていない失敗、分類済みだが処理方針が未確定の失敗、後続 PR に送られた失敗が 1 件でもある場合、その Phase は未完了とする。

**Defect classification 必須分類：**

| Classification | 定義 | 必須対応 | 完了判定 |
|----------------|------|----------|----------|
| `spec_gap` | 仕様本文に API、error、永続化、auth、test、oracle、対象外の記述がない | 実装を止め、仕様修正 PR として本文を先に更新する | 仕様更新後に packet / manifest / oracle を再固定するまで実装再開不可 |
| `implementation_bug` | 仕様は明確だが実装が満たしていない | 同一 PR 内で code fix、test、evidence を追加する | failure closure が 0 件になれば完了可 |
| `regression_bug` | 過去 Phase の契約、TC、snapshot、SDK 互換が壊れた | 対象 Phase 未完了として同一 PR 内で修正する | regression baseline が再度 pass するまで完了不可 |
| `compatibility_diff` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase と差分が出た | §9.1.23 と §9.1.35 に従い差分理由を仕様化する | 差分理由なしは merge 不可 |
| `oracle_gap` | 正解 snapshot / fixture / transcript / baseline が不足している | oracle を追加し、仕様変更根拠を明記する | oracle 追加前の実装完了は禁止 |
| `environment_gap` | local / Docker / CI / release-check の環境差で結果が揺れる | §9.1.24 の environment artifact と再現条件を更新する | 環境差分が再現可能になるまで完了不可 |
| `security_gap` | auth、scope、quota、redaction、secret scan、任意 path 拒否に不足がある | success response を禁止し、同一 PR 内で仕様または実装を修正する | merge 不可。manual only 禁止 |
| `persistence_gap` | atomic update、fsync、rollback、migration、recovery、破損時挙動が不足している | 書き込み処理を止め、§9.1.21 / §9.6 / Phase 詳細節を修正する | merge 不可。rollback evidence 必須 |

**Defect record 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `id` | `DEF-P{phase}-{number}` |
| `classification` | 上記 8 分類のいずれか |
| `detected_by` | test、review、oracle diff、CI、manual observation、Turso diff など |
| `contract_id` | 関連する API / persistence / error / security / compatibility / regression 契約 ID |
| `symptom` | 観測された失敗。推測ではなく artifact / command / snapshot path を含める |
| `root_cause` | 仕様不足、実装誤り、oracle 不足、環境差分などの確定原因 |
| `resolution` | spec fix、code fix、test fix、oracle追加、environment fix のいずれか |
| `evidence` | 修正後の command、exit code、artifact path |
| `status` | `open`、`fixed`、`spec_updated`、`not_a_defect`。Phase 完了時は `open` 禁止 |

**分類別禁止事項：**

| 状態 | 判定 |
|------|------|
| `spec_gap` を実装判断で埋める | merge 不可 |
| `implementation_bug` を既知課題として残す | Phase 未完了 |
| `regression_bug` を対象外扱いにする | merge 不可 |
| `compatibility_diff` を自己ホスト都合だけで許容する | 仕様修正 PR に戻す |
| `oracle_gap` を現行出力 snapshot 更新で隠す | merge 不可 |
| `environment_gap` を local pass だけで完了扱いにする | Phase 未完了 |
| `security_gap` / `persistence_gap` を follow-up に送る | merge 不可 |
| `not_a_defect` にした理由が仕様本文にない | review failure |

**zero-bug triage 処理順：**

| 順序 | 処理 | 失敗時 |
|------|------|--------|
| 1 | failure / diff / skipped / flaky / manual only をすべて列挙する | Phase 未完了 |
| 2 | 各項目に classification と contract_id を付与する | Phase 未完了 |
| 3 | `spec_gap` / `oracle_gap` / `compatibility_diff` は仕様本文を先に更新する | 実装継続禁止 |
| 4 | `implementation_bug` / `regression_bug` は同一 PR 内で修正し test を追加する | merge 不可 |
| 5 | `security_gap` / `persistence_gap` は success response と write path を停止する | merge 不可 |
| 6 | 修正後に oracle、regression、secret scan、release-check を再実行する | Done receipt 作成禁止 |
| 7 | defect record の `open` が 0 件であることを Done receipt に記録する | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `defect_classification_result` | defect 件数、分類別件数、`open:0`、各 defect の evidence path |
| `triage_result` | zero-bug triage の 1〜7 が完了したこと |
| `not_a_defect_decisions` | `not_a_defect` 判定がある場合の仕様本文参照。なければ `none` |

**完了根拠として禁止する表現：**

| 表現 | 扱い |
|------|------|
| `known issue` / `既知課題` | Phase 未完了 |
| `minor` / `軽微` | defect classification がない限り無効 |
| `later` / `follow-up` / `後続 Phase で対応` | 完了条件の代替に使えない |
| `manual only` / `手元確認済み` | §9.1.10 の Manual exception がない限り無効 |
| `accepted risk` | 使用禁止。仕様変更または修正で閉じる |
| `works for me` / `想定通り` | artifact と oracle がない限り無効 |

Phase defect classification に関係する仕様変更は、§9.1.1、§9.1.1a、§9.1.3、§9.1.10、§9.1.11、§9.1.14、§9.1.23、§9.1.24、§9.1.33、§9.1.35、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。defect classification がない失敗を残したまま Phase を完了扱いにしてはならない。

#### 9.1.37 Phase operational state / recovery runbook 固定契約

各 Phase は、実装対象 resource が正常時、劣化時、復旧中、rollback 必須時、operator 判断必須時にどの API 応答、health、log、write 可否、retry 可否を示すかを固定しなければならない。壊れた状態を `ok` と表示すること、operator 判断が必要な状態を自動成功扱いにすること、recovery 中に成功応答を返すことは禁止する。

**Operational state 固定表：**

| State | 定義 | API success | Write | Health | Log | Operator action |
|-------|------|-------------|-------|--------|-----|-----------------|
| `healthy` | 対象 resource が仕様通り利用可能 | 許可 | 許可 | `ok` | `INFO` | 不要 |
| `degraded` | read または一部機能は可能だが lag、retry、再取得、縮退がある | read は Phase 節で許可可。write は Phase 節で明記した場合のみ | 原則禁止。許可する場合は data loss なしを証明 | `degraded` | `WARN` | 状態確認または復旧判断 |
| `unavailable` | 対象 resource を安全に利用できない | 禁止。規定 error を返す | 禁止 | `unavailable` | `ERROR` | 必要 |
| `recovering` | 起動時または job recovery が進行中 | 対象 resource は成功応答禁止 | 禁止 | `recovering` | `INFO` / `WARN` | 原則不要。ただし timeout 時は必要 |
| `rollback_required` | 旧状態へ戻す必要があるが未完了 | 禁止 | 禁止 | `degraded` または `unavailable` | `ERROR` | 自動 rollback 可否を runbook で判定 |
| `operator_required` | 自動復旧が安全でない、または外部判断が必要 | 禁止 | 禁止 | `degraded` または `unavailable` | `ERROR` | 必須 |
| `blocked` | auth、quota、config、scope、policy により意図的に拒否 | 規定 error のみ | 禁止 | 原則 `ok`。system 健全性は壊れていない | `WARN` または audit 相当 | 設定変更または権限変更 |

**Runbook 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `state` | 上記 operational state のいずれか |
| `detection_condition` | state を検出する条件。file marker、checksum、health probe、job status、error code、fixture を含める |
| `affected_resource` | DB、branch、metadata file、replica、archive、extension、HA node、internal adapter など |
| `api_behavior` | 対象 API の status、body、error code、retry header、partial response 可否 |
| `write_policy` | write 許可 / 拒否、拒否 error、既存 transaction の扱い |
| `health_response` | `GET /v2/health` または該当 health/status endpoint の response field |
| `automatic_recovery` | 自動 recovery が許可される条件、禁止される条件、timeout |
| `operator_action` | 必要な手動操作、承認、復旧手順。不要な場合は `none` |
| `retry_condition` | client retry 可否、server retry 可否、backoff、再実行 idempotency |
| `evidence_artifact` | recovery log、health snapshot、error snapshot、restart fixture、operator marker |

**Phase group operational minimum：**

| Phase group | 必須 state / runbook |
|-------------|----------------------|
| Phase 1〜5 | 起動失敗、config invalid、DB open failure、lock held、health `ok` / `unavailable` |
| Phase 6〜8 | metadata migration failure、legacy fallback failure、quota blocked、scope denial、usage unavailable |
| Phase 9〜10 | WebSocket tx rollback、disconnect recovery、ATTACH blocked、metrics read degraded |
| Phase 11〜13 | replica lag degraded、primary unavailable、checksum mismatch、archive manifest corruption、retention cleanup failure |
| Phase 14〜15 | restore rollback、`restore-failed.json`、PITR archive missing、branch delete recovery、source unavailable |
| Phase 16〜18 | extension load failure、metrics snapshot corruption、HA no leader、candidate state、split-brain operator_required |
| Phase 19 | adapter shadow diff、active failure、rollback flag、performance blocked、internal path unavailable |

**自動 recovery 許可 / 禁止：**

| 状態 | 自動 recovery |
|------|---------------|
| temp file のみ残存し target が正常 | 許可。temp cleanup と WARN log |
| commit marker 前の interrupted migration | rollback または起動失敗。仕様にない推測 migration 禁止 |
| checksum mismatch | 対象 artifact を使用禁止にし、再取得または operator_required |
| `restore-failed.json` 存在 | 自動復旧禁止。対象 DB は write 禁止、operator_required |
| HA split-brain | 自動 primary 昇格禁止。operator_required |
| extension signature mismatch | 自動許可禁止。extension unavailable |
| internal adapter active failure | silent fallback 禁止。rollback flag または operator_required |

**API / health 禁止事項：**

| 状態 | 判定 |
|------|------|
| `degraded` / `unavailable` / `recovering` を health `ok` のみで返す | merge 不可 |
| rollback 未完了 resource に 2xx success を返す | merge 不可 |
| operator_required を自動修復して成功扱いにする | merge 不可 |
| blocked と degraded を同じ error code で返す | Phase 未完了 |
| recovery 中に partial list / partial metadata を返す | Phase 未完了 |
| recovery log に secret、token、raw path、SQL args、backup body を出す | merge 不可 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `operational_state_result` | state matrix、health snapshot、API behavior snapshot、write policy evidence |
| `recovery_runbook_result` | runbook fields、automatic recovery 可否、operator action、restart fixture |
| `operator_required_result` | operator_required がある場合の marker、health、manual action、解除条件。なければ `none` |

Phase operational state に関係する仕様変更は、§6.4、§7.3、§9.1.16、§9.1.21、§9.1.22、§9.1.25、§9.1.30、§9.1.31、§9.1.33、§9.1.36、§9.5、§9.6、§9.7、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。operational state と recovery runbook が固定されていない failure path は、実装が正常系で動いていても Phase 完了扱いにしない。

#### 9.1.38 Phase dependency graph / contract prerequisite 固定契約

各 Phase の実装 PR は、実装開始前に Phase 内 contract の dependency graph を固定しなければならない。dependency graph は「どの契約が満たされていなければ、次の契約を実装・公開・完了扱いにできないか」を示す実装順序の正本である。prerequisite が未完了の契約を実装済み、証跡済み、または Done として扱ってはならない。

**Contract prerequisite 固定表：**

| Contract type | prerequisite | prerequisite 未完了時の扱い |
|---------------|--------------|------------------------------|
| API contract | error contract、auth/security contract、persistence contract、oracle、unsupported behavior | route 追加禁止。stub は §9.1.5 / §9.1.34 に従う |
| Persistence contract | schema、atomic update、fsync、rollback、recovery、operational state | 書き込み処理禁止。metadata commit 禁止 |
| Security contract | auth source、scope、quota/block policy、denial snapshot、redaction artifact | success response 禁止 |
| Compatibility contract | Turso / libSQL SDK / legacy metadata 差分、oracle、migration / rollback 方針 | 互換差分を伴う実装禁止 |
| Concurrency contract | resource lock、idempotency、shutdown、retry、transaction boundary | 状態変更処理禁止 |
| Oracle contract | Contract ID、expected artifact、normalization、update policy | snapshot / fixture / expected 更新禁止 |
| Evidence contract | artifact path、generation command、secret scan、manifest linkage | Phase 完了扱い禁止 |
| Operational contract | state matrix、health response、write policy、operator action、recovery runbook | failure path 実装禁止 |
| Defect contract | classification、root cause、resolution、evidence、open:0 | Done receipt 作成禁止 |

**Dependency graph 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `contract_id` | §9.1.7 の Contract ID |
| `contract_type` | API / Persistence / Security / Compatibility / Concurrency / Oracle / Evidence / Operational / Defect |
| `requires` | prerequisite Contract ID の配列。不要な場合は `[]` |
| `blocks` | この契約が未完了の場合に止める契約 ID または operation |
| `status` | `planned`、`satisfied`、`blocked`、`not_applicable` のいずれか |
| `evidence` | satisfied 判定に使う artifact path / command / section |
| `not_applicable_reason` | `not_applicable` の場合のみ、仕様本文の根拠 |

**Phase group dependency minimum：**

| Phase group | 必須 dependency |
|-------------|-----------------|
| Phase 1〜5 | CLI/config prerequisite、DB open 前の data-dir contract、hrana API 前の SQL/error/oracle contract |
| Phase 6〜8 | DB route 前の naming/persistence/security contract、Turso API 前の org/group/location/quota/oracle contract |
| Phase 9〜10 | WebSocket execute 前の auth/stream/tx contract、ATTACH 前の parser/security/persistence contract |
| Phase 11〜13 | replication response 前の frame/checksum/auth contract、archive cleanup 前の manifest/recovery contract |
| Phase 14〜15 | restore/branch success 前の rollback/source/quota/security/operational contract |
| Phase 16〜18 | extension load 前の allowlist/signature/redaction contract、HA promotion 前の term/operator/split-brain contract |
| Phase 19 | adapter active 前の shadow/oracle/performance/rollback/full regression contract |

**未完了 prerequisite の禁止事項：**

| 状態 | 判定 |
|------|------|
| prerequisite が `planned` / `blocked` のまま dependent API を公開する | merge 不可 |
| persistence prerequisite 未完了で metadata を commit する | merge 不可 |
| security prerequisite 未完了で 2xx success を返す | merge 不可 |
| oracle prerequisite 未完了で snapshot を更新する | merge 不可 |
| operational prerequisite 未完了で recovery / degraded path を実装する | Phase 未完了 |
| defect prerequisite 未完了で Done receipt を作成する | Phase 未完了 |
| dependency graph を PR description だけに置き、仕様本文または artifact と対応しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `dependency_graph_result` | Contract ID ごとの status、blocked 0 件、not_applicable reason、evidence path |
| `blocked_contracts` | `blocked` がある場合は Phase 未完了。完了時は `none` |
| `prerequisite_closure` | dependency graph 上の全 dependent contract が prerequisite satisfied 後に検証済みであること |

Phase dependency graph に関係する仕様変更は、§9.1.7、§9.1.8、§9.1.10、§9.1.11、§9.1.32、§9.1.33、§9.1.34、§9.1.35、§9.1.36、§9.1.37、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。dependency graph がない Phase 実装 PR は、実装順序と完了判定が未確定として扱い、実装開始不可とする。

#### 9.1.39 Phase invariant ledger / non-regression invariant 固定契約

各 Phase の実装 PR は、実装開始前に Phase invariant ledger を固定しなければならない。invariant ledger は「その Phase と過去 Phase で、実装後も絶対に壊れてはならない条件」の正本である。単体の API、永続化、security、oracle、dependency graph が満たされていても、invariant が 1 件でも破れている場合、その Phase は完了扱いにしない。

**Invariant ledger 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `invariant_id` | `INV-P{phase}-{domain}-{number}` 形式の一意 ID |
| `scope` | API、persistence、security、compatibility、operation、redaction、dependency の対象範囲 |
| `phase` | invariant を導入する Phase と、継続して守る Phase 範囲 |
| `must_hold_before` | 実装前、migration 前、state transition 前に成立している必要条件 |
| `must_hold_after` | success response、commit、restart、rollback、operator action 後に成立している結果条件 |
| `violation_signal` | 破れた場合に返す HTTP status、error code、health state、log event、defect classification |
| `evidence` | invariant を証明する test、snapshot、fixture、log、manifest、artifact path |
| `regression_guard` | 過去 Phase で再実行する TC / command / oracle |
| `owner_contract` | invariant を所有する API / persistence / security / operational Contract ID |

**Invariant category 固定表：**

| Category | 必ず守る条件 | 違反時の扱い |
|----------|--------------|--------------|
| Durability invariant | success response は metadata / DB file / archive / branch state の fsync または明示された durable boundary 後にのみ返す | merge 不可。success-before-fsync は critical defect |
| Metadata/file consistency invariant | metadata と実ファイル、manifest、archive、branch source、replication frame の参照が相互に存在し、片側だけの commit を残さない | recovery / rollback 必須。未解消なら Phase 未完了 |
| Auth/scope/quota invariant | auth、scope、read-only/write、quota、block policy、organization/group/location boundary を bypass できない | 2xx success 禁止。security gap として open:0 まで完了不可 |
| Compatibility invariant | Turso Cloud API、libSQL SDK、legacy metadata、previous Phase の既存 response/error/snapshot を理由なしに変えない | compatibility diff の仕様化と oracle 更新まで merge 不可 |
| Error surface invariant | 同じ原因は同じ HTTP status、error code、body shape、client action、retry policy で返す | `INTERNAL_ERROR` への逃避禁止。error contract 修正まで未完了 |
| Operational state invariant | healthy、degraded、unavailable、recovering、rollback_required、operator_required、blocked の write policy と health response が矛盾しない | operator action または recovery runbook なしの実装禁止 |
| Redaction invariant | token、secret、raw filesystem path、SQL args、backup body、private metadata を log、artifact、PR description、error body に出さない | secret 混入は merge 不可。artifact 再生成必須 |
| Dependency/prerequisite invariant | prerequisite が satisfied になる前に dependent API、write、snapshot 更新、Done receipt を公開しない | dependent contract は未実装扱い |

**Phase group invariant minimum：**

| Phase group | 最低 invariant |
|-------------|----------------|
| Phase 1〜5 | CLI/config precedence、data-dir boundary、default DB identity、hrana-http error surface、JWT scope、log redaction、restart persistence |
| Phase 6〜8 | DB identity、admin API auth、Turso metadata shape、organization/group/location/quota boundary、legacy metadata migration、fallback 禁止条件 |
| Phase 9〜10 | WebSocket transaction atomicity、stream close semantics、managed ATTACH path isolation、metrics counter consistency、arbitrary path denial |
| Phase 11〜13 | replication frame monotonicity、checksum consistency、snapshot/archive manifest consistency、retention cleanup safety、primary/replica health transition |
| Phase 14〜15 | restore/PITR rollback safety、branch source selector、branch isolation、destructive operation lock、restart recovery marker、quota precedence |
| Phase 16〜18 | extension allowlist/signature boundary、metrics persistence、HA term monotonicity、leader election safety、split-brain prevention、operator-required transition |
| Phase 19 | internal adapter wire/API parity、shadow/active/rollback boundary、full regression non-regression、performance baseline、Turso Cloud compatibility gap closure |

**Invariant violation handling：**

| 状態 | 判定 |
|------|------|
| invariant violation が 1 件でも残る | Phase 未完了 |
| invariant violation 中に 2xx success を返す | merge 不可 |
| violation を `INTERNAL_ERROR`、generic 500、またはログのみで隠す | Phase 未完了。ただし未知の内部 bug は defect record と oracle gap を残した上で open:0 まで修正する |
| invariant の `evidence` がない | Done receipt 作成禁止 |
| `regression_guard` が未実行 | Phase 完了扱いにしない |
| 過去 Phase invariant を破る互換差分が仕様化されていない | merge 不可 |
| invariant ledger を PR description だけに置き、仕様本文または artifact と対応しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `invariant_result` | invariant ID ごとの pass/fail、violation 0 件、evidence path、regression guard command、owner Contract ID |
| `violated_invariants` | 完了時は `none`。1 件でもある場合は Phase 未完了 |
| `non_regression_closure` | 過去 Phase invariant が再実行され、互換差分が 0 件または仕様化済みであること |

Phase invariant ledger に関係する仕様変更は、§9.1.7、§9.1.10、§9.1.11、§9.1.14、§9.1.15、§9.1.23、§9.1.24、§9.1.32、§9.1.33、§9.1.35、§9.1.36、§9.1.37、§9.1.38、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。invariant ledger がない Phase 実装 PR は、バグ修正ゼロ判定に必要な非退行条件が未確定として扱い、実装開始不可とする。

#### 9.1.40 Phase scenario matrix / implementation blueprint 固定契約

各 Phase の実装 PR は、実装開始前に Phase scenario matrix を固定しなければならない。scenario matrix は、その Phase で実装・拒否・検証する全シナリオの実装設計図である。正常系だけを実装してから異常系、権限、永続化、互換、rollback、unsupported を後追いで補う進め方は禁止する。

**Scenario matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `scenario_id` | `SCN-P{phase}-{category}-{number}` 形式の一意 ID |
| `category` | normal / invalid_request / auth_scope_quota / persistence_restart / rollback_recovery / concurrency_idempotency / compatibility / unsupported / redaction / operational |
| `entrypoint` | API path、WebSocket message、CLI command、config key、startup path、background job、internal adapter |
| `precondition` | data-dir、metadata、token、config、DB state、Phase prerequisite、operator state |
| `input` | request body、query、header、SQL、config、fixture、failure injection |
| `expected_behavior` | HTTP status、error code、response body、metadata/file state、health、log、retry 可否 |
| `persistence_effect` | no-write、atomic write、fsync、rollback、migration、recovery marker、not_applicable のいずれか |
| `security_effect` | auth required、scope check、quota check、redaction、not_applicable のいずれか |
| `compatibility_effect` | Turso snapshot、libSQL SDK transcript、legacy fixture、previous Phase regression、not_applicable のいずれか |
| `evidence` | test ID、command、snapshot、fixture、artifact path |
| `owner_contract` | API / persistence / security / compatibility / operational Contract ID |
| `not_applicable_reason` | `not_applicable` の場合のみ、仕様本文の根拠 |

**Scenario category 固定表：**

| Category | 必ず固定すること | 未定義時の扱い |
|----------|------------------|----------------|
| `normal` | 成功条件、response、永続化後状態、idempotency | 実装開始禁止 |
| `invalid_request` | malformed body、unknown field、invalid name、bad type、size limit、unsupported query | validation 実装禁止 |
| `auth_scope_quota` | token なし、token 不正、scope 不一致、quota 超過、block policy | success response 禁止 |
| `persistence_restart` | file path、schema、fsync、再起動後復元、破損検出 | 書き込み処理禁止 |
| `rollback_recovery` | 途中失敗、crash、rollback marker、operator_required、retry | destructive operation 公開禁止 |
| `concurrency_idempotency` | 同時 create/delete/write、retry、duplicate request、lock timeout | 状態変更処理禁止 |
| `compatibility` | Turso Cloud snapshot、libSQL SDK transcript、legacy metadata、previous Phase 差分 | 互換対象 API 公開禁止 |
| `unsupported` | future Phase、対象外 field/config/route/operation の拒否 status/body | stub / route 追加禁止 |
| `redaction` | token、secret、raw path、SQL args、backup body、private metadata の非露出 | artifact / log 完了不可 |
| `operational` | health、degraded、unavailable、recovering、operator_required、write policy | failure path 実装禁止 |

**Phase group scenario minimum：**

| Phase group | 最低 scenario |
|-------------|----------------|
| Phase 1〜5 | CLI help/error、config precedence、data-dir lock/open、default DB restart、hrana normal/error、JWT allow/deny、log redaction、SDK smoke |
| Phase 6〜8 | DB create/delete/list/detail、invalid DB name、admin auth、Turso Platform wrapper、organization/group/location/quota、metadata migration、legacy fallback |
| Phase 9〜10 | WebSocket hello/auth/order、transaction commit/rollback/disconnect、managed ATTACH allow/deny、metrics counter/read、arbitrary path denial |
| Phase 11〜13 | replication token/range/checksum、primary/replica lag、redirect、archive manifest/snapshot/frame、retention cleanup、corrupt frame |
| Phase 14〜15 | backup stream、restore integrity、PITR selector、restore rollback、branch create/delete/routing/source isolation、seed compatibility、restart recovery |
| Phase 16〜18 | extension register/load/delete/signature、metrics persistence/Prometheus、HA leader/follower/failover/split-brain/operator_required |
| Phase 19 | internal adapter disabled/shadow/active/rollback、wire/API diff zero、metadata no-migration、performance baseline、crash recovery、Phase 1〜18 full regression |

**Scenario coverage 禁止事項：**

| 状態 | 判定 |
|------|------|
| scenario matrix がない | 実装開始禁止 |
| category が normal だけで error/auth/persistence/compatibility/unsupported がない | Phase 未完了 |
| scenario が `manual only`、`not run`、`todo`、`later`、空欄のまま | Phase 未完了 |
| `not_applicable` の仕様本文根拠がない | Phase 未完了 |
| unsupported / future feature の scenario が success response を期待する | merge 不可 |
| persistence effect があるのに rollback / restart scenario がない | merge 不可 |
| compatibility effect があるのに Turso / SDK / legacy / previous Phase evidence がない | merge 不可 |
| scenario matrix と Phase packet / manifest / Done receipt の Contract ID が一致しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `scenario_matrix_result` | scenario ID ごとの category、pass/fail、evidence path、not_applicable reason、manual only 0 件 |
| `scenario_coverage` | category 別件数、未実行 0 件、not_applicable の仕様本文参照 |
| `implementation_blueprint_closure` | scenario matrix の全 owner Contract ID が packet、manifest、oracle、invariant、test、artifact と一致すること |

Phase scenario matrix に関係する仕様変更は、§9.1.7、§9.1.10、§9.1.11、§9.1.14、§9.1.24、§9.1.32、§9.1.33、§9.1.35、§9.1.36、§9.1.37、§9.1.38、§9.1.39、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。scenario matrix がない Phase 実装 PR は、ケース漏れによる後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.41 Phase resource lifecycle / state transition matrix 固定契約

各 Phase の実装 PR は、実装開始前に Phase resource lifecycle matrix を固定しなければならない。resource lifecycle matrix は、対象 resource が持つ state、許可される transition、禁止される transition、API 応答、write policy、永続化 commit 順序、recovery behavior を 1 箇所に固定する正本である。metadata と実ファイル、runtime map、health、operator state が別々の state を示す場合、仕様本文の lifecycle matrix を正として復旧または起動失敗を選ぶ。

**Resource lifecycle matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `resource_type` | server / config / data_dir / database / token / organization / group / location / quota / websocket / transaction / replica / archive / restore_job / branch / extension / metrics_snapshot / ha_node / internal_adapter |
| `state` | `uninitialized`、`creating`、`active`、`blocked`、`degraded`、`recovering`、`deleting`、`deleted`、`rollback_required`、`operator_required`、`disabled`、`shadow`、`active_internal` など |
| `allowed_transition` | 許可される `from -> to` と発火条件 |
| `forbidden_transition` | 禁止される `from -> to` と検出時の判定 |
| `entry_condition` | state に入る条件。API、job、startup、failure、operator action、config flag を含める |
| `exit_condition` | state から出る条件。commit marker、fsync、health、oracle、operator action を含める |
| `api_behavior` | state ごとの HTTP/WebSocket/CLI success、error code、body、retry 可否 |
| `write_policy` | write 許可 / 拒否、既存 transaction の扱い、read 可否 |
| `persistence_commit_order` | file、metadata、marker、runtime map、directory fsync、cleanup の順序 |
| `recovery_behavior` | restart 時、crash 時、corruption 時、partial state 時の挙動 |
| `evidence` | state fixture、transition test、forbidden transition test、recovery log、health snapshot |

**Phase group lifecycle minimum：**

| Phase group | 最低 lifecycle |
|-------------|----------------|
| Phase 1〜5 | server startup/shutdown、config valid/invalid、data-dir locked/open、default DB open/error、token active/revoked、log redaction state |
| Phase 6〜8 | DB creating/active/deleting/deleted、admin token active/revoked、organization/group/location active/blocked、quota active/exceeded、usage available/unavailable |
| Phase 9〜10 | WebSocket connecting/hello/active/closing/closed、stream open/closed、transaction open/committed/rolled_back、ATTACH allowed/blocked、metrics active/degraded |
| Phase 11〜13 | primary active/degraded、replica syncing/caught_up/lagged/unavailable、frame available/missing/corrupt、archive active/retention_cleanup/corrupt |
| Phase 14〜15 | backup running/completed/failed、restore preparing/verifying/committed/rolled_back/rollback_required、PITR selected/missing/corrupt、branch creating/active/deleting/deleted |
| Phase 16〜18 | extension registered/loaded/failed/disabled、metrics snapshot active/corrupt/rebuilt、HA node follower/candidate/leader/demoted/operator_required |
| Phase 19 | internal adapter disabled/shadow/active_internal/rollback/operator_required、shadow diff clean/dirty、performance baseline pass/fail |

**State transition 固定表：**

| Transition type | 必須仕様 | 未定義時の扱い |
|-----------------|----------|----------------|
| create | file / runtime 準備後に metadata を commit する。success は durable boundary 後のみ | create API 実装禁止 |
| activate | integrity、auth、quota、compatibility、oracle が pass してから active にする | success response 禁止 |
| block | auth、quota、policy、operator action により意図的に blocked にする | 2xx success 禁止 |
| degrade | read-only / partial / lag など縮退時の health と write policy を固定する | degraded path 実装禁止 |
| recover | startup / job recovery の entry、exit、timeout、operator_required を固定する | recovery 実装禁止 |
| delete | route disable、runtime close、file cleanup、metadata commit の順序を固定する | delete API 実装禁止 |
| rollback | rollback marker、旧状態復元、失敗時 operator_required を固定する | destructive operation 禁止 |
| internal switch | shadow、active_internal、rollback flag、compatibility oracle を固定する | internal adapter active 禁止 |

**Lifecycle 禁止事項：**

| 状態 | 判定 |
|------|------|
| metadata が `active` だが file / directory が存在しない resource に success response を返す | merge 不可 |
| `creating` / `deleting` / `recovering` / `rollback_required` 中の resource に通常 write を許可する | merge 不可 |
| unknown state を silent fallback で `active` または `disabled` として扱う | merge 不可 |
| state transition evidence なしに Done receipt を作成する | Phase 未完了 |
| forbidden transition を検出しても error / health / log に出さない | Phase 未完了 |
| runtime map と metadata state が不一致なのに route を成功させる | merge 不可 |
| rollback_required / operator_required を自動成功扱いにする | merge 不可 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `resource_lifecycle_result` | resource type ごとの state、allowed transition、forbidden transition 0 件、recovery evidence path |
| `state_transition_coverage` | transition type 別 test、startup/restart fixture、forbidden transition test の実行結果 |
| `resource_state_closure` | metadata、file、runtime map、health、operator marker が同じ state 判定に収束していること |

Phase resource lifecycle に関係する仕様変更は、§9.1.16、§9.1.21、§9.1.25、§9.1.30、§9.1.31、§9.1.32、§9.1.33、§9.1.37、§9.1.39、§9.1.40、§9.2、§9.4、§9.5、§9.6、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。resource lifecycle matrix がない Phase 実装 PR は、状態不整合による後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.42 Phase schema registry / field-level contract 固定契約

各 Phase の実装 PR は、追加・変更する request、response、metadata、config、JWT claim、WebSocket message、artifact、fixture、log、metric の field-level schema を schema registry として固定しなければならない。field の必須/任意、null 可否、省略可否、default、validation、serialization、migration、compatibility、redaction が未定義のまま実装してはならない。

**Schema registry 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `schema_id` | `SCHEMA-P{phase}-{surface}-{name}` 形式の一意 ID |
| `owner_phase` | field を導入または変更する Phase |
| `surface` | request_body / response_body / metadata_json / config / jwt_claim / websocket_message / artifact_fixture / log_metric |
| `field_name` | 外部 field 名。内部名と異なる場合は mapping を明記 |
| `type` | string / integer / number / boolean / object / array / enum / null / bytes / timestamp / opaque string |
| `required` | 必須なら `true`。省略可能なら `false` と理由 |
| `nullable` | `null` 可否。`nullable:true` の場合は null の意味 |
| `omittable` | 省略可否。省略と null を同一扱いにする場合は明示 |
| `default` | 省略時、migration 時、旧 metadata 読み込み時の default |
| `validation` | length、regex、range、enum、unknown field、duplicate query、empty string/array の扱い |
| `serialization` | casing、timestamp 精度、sort order、redaction、response wrapper |
| `migration` | old field、new field、rename/delete、backfill、rollback、破損時挙動 |
| `compatibility` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase との差分 |
| `redaction` | secret / token / raw path / SQL args / backup body / private metadata の扱い |
| `evidence` | request fixture、response snapshot、metadata fixture、migration fixture、secret scan、compat diff |

**Schema surface 固定表：**

| Surface | 必ず固定する field |
|---------|--------------------|
| request_body | required、nullable、unknown field、empty string/array、duplicate key、content type |
| response_body | wrapper、field casing、nullable、omittable、timestamp、error body、secret 非露出 |
| metadata_json | schema version、required/default、migration、rollback、破損時挙動、unknown field |
| config | CLI/env/TOML/default、対象 Phase 前挙動、secret file、invalid value |
| jwt_claim | claim 名、scope、expiry、ro/rw、DB/org/group binding、legacy claim |
| websocket_message | message type、request_id、stream_id、unknown field、response_ok/error |
| artifact_fixture | 正規化 field、redaction、Contract ID、再生成 command |
| log_metric | field 名、label、cardinality、secret 非露出、request id / trace id |

**Field-level 禁止事項：**

| 状態 | 判定 |
|------|------|
| `required` / `nullable` / `omittable` が未定義の field を実装する | 実装開始禁止 |
| metadata field の rename / delete / required 化に migration と rollback がない | merge 不可 |
| response field casing、wrapper、null/省略を仕様化なしに変更する | merge 不可 |
| DTO と metadata で同名 field の意味が異なるのに mapping がない | Phase 未完了 |
| secret / token / raw path / SQL args / backup body field の redaction が未定義 | merge 不可 |
| unknown field を黙って無視する。ただし hrana wire 互換など該当節で明記された場合を除く | merge 不可 |
| `null` を省略扱いにする根拠がない | Phase 未完了 |
| schema registry と fixture / snapshot / migration test が一致しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `schema_registry_result` | schema ID ごとの field coverage、unknown/null/default/migration/redaction evidence |
| `field_compatibility_result` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase との差分分類 |
| `schema_migration_closure` | old/new/corrupt fixture、rollback、default backfill、破損時挙動が検証済みであること |

Phase schema registry に関係する仕様変更は、§9.1.10、§9.1.11、§9.1.12、§9.1.15、§9.1.19、§9.1.20、§9.1.23、§9.1.24、§9.1.27、§9.1.28、§9.1.29、§9.1.32、§9.1.33、§9.1.35、§9.1.40、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.13、§9.17、該当 Phase 詳細節を同時更新する。schema registry がない field 変更は、DTO / metadata / fixture の意味ズレによる後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.43 Phase decision precedence / conflict resolution matrix 固定契約

各 Phase の実装 PR は、複数の仕様条件が同時に成立した場合に、どの判定、error、HTTP status、state、client action を優先するかを decision precedence matrix として固定しなければならない。実装者が handler 内の if/else 順、既存 helper の都合、または実装しやすさで優先順位を決めることは禁止する。

**Decision precedence matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `decision_id` | `DEC-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 判定を導入または変更する Phase |
| `surface` | HTTP route / hrana request / WebSocket message / admin API / CLI / metadata migration / background job / health / log_metric |
| `conflicting_conditions` | 同時成立し得る条件。例: invalid body、auth missing、scope denied、resource missing、quota exceeded、block_writes、recovering、unsupported |
| `precedence_order` | 高い順の判定名。最低でも選択判定と退けられる判定を含める |
| `selected_behavior` | 優先される response、state transition、write policy、log、metric |
| `losing_behavior` | 優先されなかった条件をどう扱うか。隠す、audit のみ、内部 reason に残す、再評価する等 |
| `error_code` | §7.3 の error code。該当しない場合は `none` と理由 |
| `status` | HTTP status、WebSocket error、CLI exit code、health state、job state |
| `client_action` | retry、auth refresh、scope change、quota adjustment、operator action、request correction、none |
| `evidence` | conflict fixture、snapshot、SDK transcript、metadata fixture、job log、health snapshot |

**共通 precedence 固定表：**

| 同時成立条件 | 優先する判定 | 退ける判定 / 扱い |
|--------------|--------------|-------------------|
| body が安全に parse できない / content-type 不正 / request size 超過 と auth missing | parse / content validation | auth 判定は行わない。認証情報、resource 存在、scope 情報を露出しない |
| auth missing / invalid と resource missing | auth failure | resource の存在有無を返さない |
| scope denied / permission denied と resource missing | scope / permission denial | 存在漏洩を避ける endpoint では `not_found` を返さない。Turso 互換で `not_found` が必要な場合は decision に明記する |
| read-only token / write scope denied と quota exceeded / block_writes | scope / permission denial | quota / block policy は audit または internal reason に残すが client には優先しない |
| quota exceeded と block_writes | block_writes | operator または policy による明示停止を quota より優先する |
| recovering / rollback_required / operator_required と normal write | operational state refusal | write commit、metadata commit、success response を行わない |
| unsupported future feature と invalid field | route / feature ownership 確定後に unsupported | 未所有 route は 404/405、所有済み future feature は規定 501/400。invalid field で unsupported を隠さない |
| compatibility rule と self-host optimization | compatibility rule | security / durability / data loss を除き、Turso Cloud / libSQL SDK 互換を優先する |
| persistence conflict と response generation | persistence conflict | success response、snapshot 更新、Done receipt 作成を禁止する |
| secret / redaction violation と normal error body | redaction violation | 詳細 error を抑止し、artifact/log を再生成する |

**Phase group 最低 decision：**

| Phase group | 必須 decision |
|-------------|---------------|
| Phase 1〜5 | CLI/env/TOML priority、data-dir invalid vs DB open failure、JWT missing/invalid vs DB missing、hrana parse error vs auth error、log redaction vs detailed error |
| Phase 6〜8 | DB name validation vs auth、admin scope denial vs not found、organization/group/location mismatch、quota exceeded vs block_writes、legacy metadata migration failure vs API success |
| Phase 9〜10 | WebSocket message parse vs auth、stream/transaction rollback vs success frame、ATTACH unsupported vs path denial、metrics read degraded vs unavailable |
| Phase 11〜13 | replica lag vs primary unavailable、checksum mismatch vs retry、archive manifest missing vs retention cleanup、replication auth denial vs frame not found |
| Phase 14〜15 | restore rollback_required vs branch write、PITR archive missing vs invalid selector、branch source missing vs permission denied、destructive lock vs quota |
| Phase 16〜18 | extension signature failure vs allowlist denial、metrics corruption vs read degraded、HA no leader vs split-brain operator_required、promotion conflict vs retry |
| Phase 19 | adapter shadow diff vs active success、internal adapter failure vs compatibility fallback、rollback flag vs performance target、Turso parity diff vs self-host optimization |

**merge 不可条件：**

| 状態 | 判定 |
|------|------|
| conflict が起きる条件に decision ID がない | 実装開始禁止 |
| handler の最初に一致した if/else で判定し、precedence matrix と対応しない | merge 不可 |
| 同じ conflict を endpoint / protocol / CLI ごとに異なる error/status で返す | Phase 未完了。ただし Turso 互換差分として明記されている場合を除く |
| 存在漏洩を避けるべき endpoint で scope denial より `DB_NOT_FOUND` を優先する | merge 不可 |
| quota、block_writes、recovering、rollback_required、operator_required を write commit 後に判定する | merge 不可 |
| unsupported / compatibility conflict を `INTERNAL_ERROR`、generic 500、ログのみで隠す | Phase 未完了 |
| losing behavior の証跡がなく、退けられた判定が副作用を起こしていないことを確認できない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `decision_precedence_result` | decision ID ごとの conflict fixture、selected behavior、error/status snapshot、losing behavior 非発火証跡 |
| `decision_consistency_result` | HTTP / WebSocket / CLI / job / health 間で同じ conflict が同じ優先順位で処理された証跡 |
| `decision_compatibility_result` | Turso Cloud、libSQL SDK、previous Phase と異なる precedence がある場合の差分理由。なければ `none` |

Phase decision precedence に関係する仕様変更は、§7.3、§9.1.10、§9.1.11、§9.1.12、§9.1.14、§9.1.23、§9.1.24、§9.1.32、§9.1.33、§9.1.35、§9.1.36、§9.1.37、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.15、§9.17、該当 Phase 詳細節を同時更新する。decision precedence matrix がない conflict は、実装者ごとの分岐順差、存在漏洩、誤った retry、commit 後拒否による後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.44 Phase coverage closure / implementation completeness matrix 固定契約

各 Phase の実装 PR は、Phase packet に含まれる全契約が test、artifact、oracle、regression、Done receipt のいずれで証明されるかを coverage closure matrix として固定しなければならない。契約を定義しただけ、正常系だけ、PR description の説明だけ、または根拠のない `N/A` で Phase 完了扱いにしてはならない。

**Coverage closure matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `coverage_id` | `COV-P{phase}-{contract-id}` 形式の一意 ID |
| `phase` | coverage を閉じる Phase |
| `source_contract` | API / persistence / error / security / compatibility / concurrency / operation / regression の Contract ID |
| `source_matrix` | dependency_graph / invariant_ledger / scenario_matrix / resource_lifecycle_matrix / schema_registry / decision_precedence_matrix / phase_detail / api_table / persistence_table / error_table |
| `implementation_surface` | code module、route、CLI、config、metadata、job、health、log/metric、test fixture の対象 |
| `required_test` | 必須 test ID、command、または SDK / Turso snapshot compare。自動化不能な場合は manual exception ID |
| `required_evidence` | artifact path。snapshot、fixture、transcript、log、compat diff、secret scan、CI output のいずれか |
| `oracle` | §9.1.35 の oracle 名と version。oracle 不要の場合は理由 |
| `status` | `planned`、`pass`、`fail`、`blocked`、`not_applicable` のいずれか。Done 時は `pass` または根拠付き `not_applicable` のみ |
| `not_applicable_reason` | `not_applicable` の場合のみ、§9.2、§9.4、該当 Phase 詳細節、unsupported 固定表の本文参照 |
| `gap_class` | gap がある場合は `missing_test`、`missing_artifact`、`missing_oracle`、`missing_regression`、`manual_only`、`spec_gap`、`implementation_gap` |
| `blocker` | `status` が `pass` / 根拠付き `not_applicable` 以外の場合に止める operation。例: route publish、metadata commit、Done receipt、merge |

**coverage 対象固定表：**

| Source | 必ず coverage に含めるもの |
|--------|-----------------------------|
| dependency_graph | 全 prerequisite と dependent contract。`blocked`、`planned`、未証明 satisfied を残さない |
| invariant_ledger | invariant ID ごとの before/after、violation signal、regression guard |
| scenario_matrix | normal、invalid、auth/scope/quota、persistence、rollback、concurrency、compatibility、unsupported、redaction、operational |
| resource_lifecycle_matrix | state、allowed transition、forbidden transition、recovery、operator_required |
| schema_registry | field ごとの required/null/default/validation/migration/redaction/compatibility |
| decision_precedence_matrix | conflict ごとの selected behavior、losing behavior、error/status/client action |
| API / persistence / security contract | method/path/body/error、file/schema/fsync/rollback、auth/scope/quota/redaction |
| compatibility / regression contract | Turso Cloud、libSQL SDK、legacy metadata、previous Phase regression、Phase 1 から対象 Phase 直前までの影響 |

**Phase group coverage minimum：**

| Phase group | 最低 coverage |
|-------------|----------------|
| Phase 1〜5 | CLI/config/data-dir/default DB/hrana-http/JWT/log の normal/error/restart/redaction/SDK smoke |
| Phase 6〜8 | multi DB/admin API/Turso Platform/org/group/location/quota/legacy migration の auth/persistence/compat/regression |
| Phase 9〜10 | WebSocket/transaction/ATTACH/metrics の protocol、rollback、path denial、counter consistency、disconnect |
| Phase 11〜13 | replication/archive の frame/checksum/snapshot/retention/corruption/restart/role health |
| Phase 14〜15 | backup/restore/PITR/branch の destructive rollback、source selector、quota、seed isolation、restart recovery |
| Phase 16〜18 | extension/metrics/HA の signature、allowlist、snapshot corruption、term/leader/split-brain/operator_required |
| Phase 19 | internal adapter の shadow/active/rollback、wire/API parity、performance baseline、Phase 1〜18 full regression |

**coverage closure 禁止事項：**

| 状態 | 判定 |
|------|------|
| Contract ID、schema ID、scenario ID、decision ID が coverage matrix に存在しない | 実装開始禁止 |
| normal 系 test だけで error/auth/persistence/rollback/compatibility を pass 扱いにする | Phase 未完了 |
| `manual only` を coverage pass として扱う | merge 不可。ただし §9.1.10 の manual exception がある補助証跡を除く |
| `N/A` に本文参照と理由がない | Phase 未完了 |
| artifact path が存在しない、または Contract ID と対応しない | Phase 未完了 |
| oracle なしで snapshot / fixture / expected を更新する | merge 不可 |
| previous Phase regression を coverage 外にする | Phase 未完了 |
| failed / skipped / flaky / blocked を pass 扱いにする | merge 不可 |
| coverage matrix を PR description だけに置き、仕様本文、manifest、artifact と対応しない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `coverage_closure_result` | coverage ID ごとの status、gap 0 件、required_test、required_evidence、oracle、not_applicable reason |
| `coverage_gap_result` | `missing_test`、`missing_artifact`、`missing_oracle`、`missing_regression`、`manual_only`、`spec_gap`、`implementation_gap` が 0 件である証跡 |
| `coverage_regression_result` | previous Phase regression と互換 snapshot が coverage 対象に含まれ、pass していること |

Phase coverage closure に関係する仕様変更は、§9.1.7、§9.1.8、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.35、§9.1.36、§9.1.38、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.1.43、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。coverage closure matrix がない Phase 実装 PR は、契約定義と実装証跡の未対応、検証漏れ、根拠なし N/A、旧 Phase regression 漏れによる後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.45 Phase change impact / drift control matrix 固定契約

各 Phase の実装 PR は、実装中に scope、API、schema、metadata、error、auth、quota、lifecycle、test、oracle、artifact、compatibility のいずれかを変更する必要が生じた場合、変更前に change impact matrix を更新しなければならない。Phase packet freeze 後の暗黙変更、PR description だけの説明、snapshot だけの更新、または実装都合の仕様 drift を禁止する。

**Change impact matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `change_id` | `CHG-P{phase}-{number}` 形式の一意 ID |
| `phase` | 変更が発生した Phase |
| `change_type` | scope / api / request_response / error / persistence_schema / auth_scope_quota / lifecycle_state / compatibility / oracle_snapshot / test_artifact / dependency / operational |
| `changed_contract` | 変更対象の Contract ID、schema ID、scenario ID、decision ID、coverage ID |
| `affected_sections` | 同時更新する仕様節番号。最低でも変更対象節、§9.1.32、§9.1.33、§9.17 を含める |
| `affected_matrices` | dependency、invariant、scenario、resource lifecycle、schema registry、decision precedence、coverage closure のうち影響する表 |
| `affected_tests` | 追加・変更・再実行する TC、unit、integration、SDK、Turso snapshot、regression command |
| `affected_artifacts` | 更新する snapshot、fixture、transcript、compat diff、secret scan、CI output、Done receipt |
| `compatibility_impact` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase への影響。影響なしの場合も根拠を明記 |
| `migration_impact` | metadata / file / config / JWT claim / artifact schema migration の要否、old/new/corrupt fixture |
| `rollback_impact` | rollback 手順、rollback flag、復旧 marker、operator_required への影響 |
| `regression_expansion` | 追加する previous Phase regression、互換 snapshot、failure injection |
| `approval_state` | `not_required`、`approved`、`spec_pr_required`、`blocked`。仕様変更を伴う場合は承認済みでなければならない |
| `closure_evidence` | 更新後に drift 0 件を示す cross-reference scan、coverage result、artifact path |

**変更種別別の同時更新表：**

| Change type | 同時更新必須 |
|-------------|--------------|
| API route / method / path | §9.5、Phase 詳細節、API Contract ID、error snapshot、scenario matrix、coverage closure、SDK/Turso snapshot |
| request / response field | schema registry、API snapshot、validation rule、compatibility impact、redaction、coverage closure |
| error code / status / retry | §7.3、§9.7、decision precedence、error oracle、client action、regression test |
| metadata / persistence schema | §9.6、migration、rollback、resource lifecycle、invariant、old/new/corrupt fixture |
| auth / scope / quota / block policy | security contract、decision precedence、scenario matrix、denial snapshot、redaction、compatibility impact |
| lifecycle / state transition | resource lifecycle、operational state、recovery runbook、forbidden transition test、health snapshot |
| Turso Cloud / SDK compatibility | compatibility contract、mode boundary、snapshot source、diff reason、regression expansion、migration / rollback impact |
| test / oracle / snapshot | acceptance oracle、coverage closure、artifact path、normalization rule、snapshot update reason |
| Phase scope / target surface | Phase packet、manifest、§9.2、§9.4、dependency graph、coverage closure、Done receipt criteria |

**drift control 禁止事項：**

| 状態 | 判定 |
|------|------|
| Phase packet freeze 後に scope / API / schema / error / persistence を変更し、change ID がない | 実装開始禁止または Phase 未完了 |
| 実装都合で仕様外 field、metadata、error、config を追加する | merge 不可 |
| PR description だけで変更理由を説明し、仕様本文を更新しない | 仕様として扱わない |
| snapshot / fixture / expected だけを更新し、oracle と仕様本文を更新しない | merge 不可 |
| compatibility 影響なしと書くだけで SDK / Turso / previous Phase regression を増やさない | Phase 未完了 |
| migration / rollback 影響を未評価のまま metadata / file schema を変更する | merge 不可 |
| affected matrices のうち 1 つでも旧 Contract ID / 旧 field / 旧 error を参照する | Phase 未完了 |
| approval_state が `spec_pr_required` / `blocked` のまま実装を進める | 実装禁止 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `change_impact_result` | change ID ごとの affected sections/matrices/tests/artifacts、更新完了、承認状態、closure evidence |
| `drift_closure_result` | Phase packet、manifest、仕様本文、test、artifact、Done receipt の drift 0 件を示す scan 結果 |
| `compatibility_reassessment_result` | change によって再評価した Turso Cloud、libSQL SDK、previous Phase、migration、rollback の結果 |

**canonical change impact closure audit：**

仕様変更または実装中の scope、API、schema、metadata、error、auth、quota、lifecycle、test、oracle、artifact、compatibility 変更は、change impact matrix の作成だけで完了扱いにしてはならない。対象 Phase の readiness packet と Done receipt は、以下の final audit field を持ち、`change_impact_closure_result = pass` でなければ実装開始または Phase 完了に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `change_scope_result` | すべての change が `change_id`、`phase`、`change_type`、`changed_contract` を持ち、仕様変更か実装内補正かが一意に判定できる | 実装開始禁止 |
| `affected_sections_result` | `affected_sections` に変更対象節、§9.1.32、§9.1.33、§9.17、該当 Phase 詳細節が含まれ、各節が更新済みまたは根拠付き N/A である | 実装開始禁止 |
| `affected_matrices_result` | dependency、invariant、scenario、resource lifecycle、schema registry、decision precedence、coverage closure、artifact path、review handoff の該当表が旧値を参照しない | 実装開始禁止 |
| `affected_tests_result` | 追加・変更・再実行する TC、unit、integration、SDK、Turso snapshot、previous Phase regression が coverage closure に接続されている | 実装開始禁止 |
| `affected_artifacts_result` | snapshot、fixture、transcript、compat diff、secret scan、CI output、Done receipt の path と hash が artifact manifest と一致する | Phase 未完了 |
| `compatibility_impact_result` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase への影響が再評価され、差分理由または互換維持根拠が記録されている | 実装開始禁止 |
| `migration_rollback_result` | metadata / file / config / JWT claim / artifact schema の migration と rollback が old/new/corrupt fixture、rollback flag、復旧 marker へ接続されている | merge 不可 |
| `approval_trace_result` | 仕様変更を伴う change は承認済みで、承認日、PR、対象差分、未承認変更 0 件が追跡できる | merge 不可 |
| `cross_reference_scan_result` | 旧 Contract ID、旧 field、旧 error、旧 artifact path、旧 snapshot reason の残存が 0 件である scan 結果を持つ | Phase 未完了 |
| `change_impact_closure_result` | 上記 field がすべて `pass`。affected section update omission、old Contract ID residue、stale artifact、snapshot update without reason、migration / rollback unevaluated、approval unlinked がすべて 0 件 | `pass` 以外は実装開始禁止 |

`change_impact_closure_result` は change impact matrix の出口 gate である。`change_id` が 1 件以上ある Phase は、readiness packet の `audit_results` と Done receipt に `change_impact_closure_result` を含める。`change_id` が 0 件の場合も `change_impact_closure_result = pass` とし、`closure_evidence` に `no_change_detected` と scan command を記録する。これにより「変更なし」の主張も機械的に再現できる状態にする。

Phase change impact / drift control に関係する仕様変更は、§0、§7.3、§9.1.10、§9.1.11、§9.1.12、§9.1.13、§9.1.14、§9.1.15、§9.1.23、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.35、§9.1.38、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.1.43、§9.1.44、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.15、§9.17、該当 Phase 詳細節を同時更新する。change impact matrix がない Phase 実装 PR は、実装中の仕様 drift、古い Contract ID、古い snapshot、互換影響見落とし、migration / rollback 漏れによる後続バグ修正を防げないため、実装開始不可とする。

#### 9.1.46 Phase rollout readiness / operator acceptance matrix 固定契約

各 Phase の実装 PR は、Phase Done と rollout ready を分離して判定しなければならない。Phase Done は仕様・実装・test・artifact の完了判定であり、rollout ready は operator が起動、停止、再起動、rollback、health 監視、互換性確認、data safety 確認を行ったうえで運用投入してよい状態を指す。Done receipt があっても rollout readiness matrix が未完了なら release / deploy / production enable を行ってはならない。

**Rollout readiness matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `rollout_id` | `ROLL-P{phase}-{surface}` 形式の一意 ID |
| `phase` | rollout 判定対象 Phase |
| `release_surface` | binary / CLI / HTTP API / WebSocket / admin API / metadata migration / background job / Docker / CI / docs |
| `startup_condition` | 起動してよい前提条件、config、data-dir、metadata、lock、migration marker、secret の状態 |
| `shutdown_condition` | shutdown 時に flush / fsync / lock release / job cancel / transaction rollback が必要な条件 |
| `restart_condition` | restart 後に同じ state、health、metadata、runtime map、compat behavior に戻る条件 |
| `rollback_condition` | rollback flag、backup、old metadata、restore marker、operator 手順、rollback 不能時の扱い |
| `health_gate` | release 可否を判断する health endpoint / CLI status / log / metric の期待値 |
| `operator_action` | operator が実行する command、確認、承認、manual step。不要なら `none` |
| `observability_evidence` | log、metric、health snapshot、audit 相当記録、request id / trace id、redaction scan |
| `compatibility_gate` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase regression の pass 条件 |
| `data_safety_gate` | fsync、atomic write、backup、rollback、corruption handling、data loss なしの証跡 |
| `blocked_release_reason` | rollout 不可の場合の理由。不可でなければ `none` |

**Phase group rollout minimum：**

| Phase group | 最低 readiness |
|-------------|----------------|
| Phase 1〜5 | CLI help、config resolution、data-dir lock、default DB restart、hrana health、JWT secret、log redaction、SDK smoke |
| Phase 6〜8 | admin API auth、Turso Platform snapshot、metadata migration/restart、organization/group/location/quota、legacy fallback、quota block |
| Phase 9〜10 | WebSocket reconnect、open transaction rollback、ATTACH path denial、metrics counter persistence/visibility、disconnect behavior |
| Phase 11〜13 | primary/replica role health、replication lag、archive manifest integrity、retention cleanup safety、checksum mismatch handling |
| Phase 14〜15 | backup artifact readability、restore/PITR rollback、branch create/delete recovery、source snapshot、destructive operation lock |
| Phase 16〜18 | extension load failure handling、metrics snapshot rebuild、HA candidate/leader health、operator promote、split-brain rejection |
| Phase 19 | internal adapter shadow/active/rollback flag、compat snapshot diff zero、performance baseline、Phase 1〜18 regression、silent fallback 禁止 |

**rollout ready 禁止事項：**

| 状態 | 判定 |
|------|------|
| Phase Done receipt だけで rollout ready と扱う | release 不可 |
| rollback 手順、rollback flag、backup、または recovery marker が未検証 | release 不可 |
| health が `ok` でも `operator_required`、`rollback_required`、`degraded` を隠す | merge 不可 |
| Docker / CI / local の結果差分が未記録 | rollout ready 不可 |
| data loss、partial commit、success-before-fsync の可能性が残る | merge 不可 |
| Turso Cloud / libSQL SDK / previous Phase 互換差分が未分類 | rollout ready 不可 |
| startup / restart 後に metadata、file、runtime map、health が一致しない | release 不可 |
| operator action が必要なのに command、解除条件、確認 artifact がない | Phase 未完了 |
| rollout 不可理由があるのに `blocked_release_reason:none` とする | review failure |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `rollout_readiness_result` | rollout ID ごとの startup、shutdown、restart、rollback、health、operator、compatibility、data safety 証跡 |
| `operator_acceptance_result` | operator action、manual step、解除条件、command、artifact。不要な場合は `none` |
| `release_blocker_result` | blocked release reason が 0 件、または rollout 不可として明示されていること |

**canonical rollout readiness closure audit：**

rollout readiness は運用投入の出口 gate であり、Phase Done receipt の存在だけで代替してはならない。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`rollout_readiness_closure_result = pass` でなければ release / deploy / production enable に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `startup_closure_result` | startup condition、config、data-dir、metadata、lock、migration marker、secret の前提が明記され、起動成功と起動拒否の evidence がある | release 不可 |
| `shutdown_closure_result` | shutdown 時の flush、fsync、lock release、job cancel、transaction rollback、partial write 防止が artifact と command で再現できる | release 不可 |
| `restart_closure_result` | restart 後に state、health、metadata、runtime map、compat behavior が restart 前の期待値へ戻ることを snapshot で確認している | rollout ready 不可 |
| `rollback_closure_result` | rollback flag、backup、old metadata、restore marker、operator 手順、rollback 不能時の扱いが検証済みである | release 不可 |
| `health_gate_result` | health endpoint / CLI status / log / metric が `ok`、`degraded`、`operator_required`、`rollback_required`、`blocked` を隠さず区別する | merge 不可 |
| `operator_action_result` | operator action が必要な場合、command、実行順序、解除条件、確認 artifact、manual approval の要否が固定されている。不要なら `none` の根拠がある | Phase 未完了 |
| `observability_result` | log、metric、health snapshot、audit 相当記録、request id / trace id、redaction scan が artifact manifest に接続されている | Phase 未完了 |
| `compatibility_gate_result` | Turso Cloud、libSQL SDK、legacy metadata、previous Phase regression の pass 条件と差分理由が compatibility baseline と一致している | rollout ready 不可 |
| `data_safety_result` | data loss、partial commit、success-before-fsync、corruption handling、backup readability のリスクが 0 件である | merge 不可 |
| `blocked_release_result` | release blocker が 0 件、または `blocked_release_reason` として明示され release / deploy が禁止されている | review failure |
| `rollout_readiness_closure_result` | 上記 field がすべて `pass`。startup / shutdown / restart / rollback / health / operator / observability / compatibility / data safety / blocker の未評価が 0 件 | `pass` 以外は release / deploy / production enable 禁止 |

`rollout_readiness_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。Phase が release / deploy 対象でない場合も `rollout_readiness_closure_result = pass` とし、`release_surface:none`、`blocked_release_reason:none`、`operator_action:none`、および非対象理由を `closure_evidence` に記録する。これにより「運用投入なし」の判断も、後続 Phase と reviewer が再現できる状態にする。

Phase rollout readiness に関係する仕様変更は、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.15、§9.1.24、§9.1.32、§9.1.33、§9.1.37、§9.1.39、§9.1.41、§9.1.44、§9.1.45、§9.2、§9.4、§9.6、§9.8、§9.11、§9.15、§9.16、§9.17、該当 Phase 詳細節を同時更新する。rollout readiness matrix がない Phase 実装 PR は、実装完了後の起動・再起動・rollback・health・operator 判断の欠落による後続バグ修正を防げないため、release / deploy / production enable 不可とする。

#### 9.1.47 Phase compatibility baseline / upstream refresh matrix 固定契約

各 Phase の実装 PR は、Turso Cloud、libSQL SDK、hrana wire format、legacy metadata、previous Phase response を互換 baseline として固定しなければならない。互換 baseline は「現在の実装出力」ではなく「Adlaire DB が追従または明示差分化する外部基準」である。実装都合で snapshot、oracle、SDK transcript、expected を更新して互換差分を消すことは禁止する。

**Compatibility baseline matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `baseline_id` | `BASE-P{phase}-{surface}` 形式の一意 ID |
| `phase` | baseline を固定する Phase |
| `upstream_source` | Turso Cloud API、libSQL SDK、hrana spec、legacy Adlaire artifact、previous Phase snapshot のいずれか |
| `observed_behavior` | upstream または previous Phase で観測した path、method、request、response、error、metadata、SDK behavior |
| `adlaire_behavior` | Adlaire DB が採用する挙動。差分がある場合は mode、error mapping、client impact を明記 |
| `compatibility_class` | `match`、`intentional_self_host_diff`、`unsupported_until_phase`、`upstream_changed`、`sdk_regression`、`adlaire_extension_mode_only` |
| `snapshot_version` | snapshot / fixture / transcript の version、生成 commit、正規化 rule |
| `sdk_version` | 対象 SDK 名と version。SDK 対象外なら本文根拠付き `not_applicable` |
| `refresh_trigger` | upstream changelog、SDK update、Turso behavior diff、spec change、regression failure、security/persistence reason |
| `refresh_allowed_by` | refresh を許可する根拠。仕様変更 commit、Turso 追従記録、SDK changelog、承認済み change ID |
| `diff_reason` | 差分理由。差分なしなら `none` |
| `migration_impact` | metadata、config、JWT claim、artifact、client migration の要否 |
| `regression_scope` | refresh 後に再実行する Phase regression、SDK transcript、Turso snapshot、legacy fixture |
| `evidence` | upstream observation、snapshot diff、SDK transcript、compat diff、changelog reference、artifact path |

**compatibility class 固定表：**

| Class | 意味 | 完了条件 |
|-------|------|----------|
| `match` | upstream / SDK / previous Phase と client-visible behavior が一致 | snapshot / transcript 差分ゼロ |
| `intentional_self_host_diff` | security、persistence、operation の理由で自己ホスト差分を採用 | diff reason、client impact、代替仕様、regression が必須 |
| `unsupported_until_phase` | 将来 Phase まで明示的に未対応 | unsupported behavior、error/status、対象 Phase、snapshot が必須 |
| `upstream_changed` | Turso Cloud / SDK 側の変更に追従する必要がある | upstream 証跡、仕様更新、migration/rollback 評価が必須 |
| `sdk_regression` | SDK 互換が壊れた | Phase 未完了。修正または仕様差分化まで Done 禁止 |
| `adlaire_extension_mode_only` | Adlaire 独自 mode のみの挙動 | Turso 互換 mode への混入なし、mode boundary snapshot が必須 |

**baseline refresh 禁止事項：**

| 状態 | 判定 |
|------|------|
| 実装変更だけを理由に互換 snapshot / oracle / expected を更新する | merge 不可 |
| upstream 変更を確認せず `upstream_changed` とする | review failure |
| SDK transcript なしに SDK 互換を `match` とする | Phase 未完了 |
| Turso Cloud 差分を `INTERNAL_ERROR`、generic 500、ログだけで隠す | merge 不可 |
| 自己ホスト差分に diff reason、client impact、代替仕様がない | 実装開始禁止 |
| 古い baseline のまま Phase Done / rollout ready と扱う | Phase 未完了 |
| Adlaire extension mode の field / auth / metadata が Turso 互換 mode に混入する | merge 不可 |
| refresh 後の previous Phase regression を実行しない | rollout ready 不可 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `compatibility_baseline_result` | baseline ID ごとの compatibility class、snapshot version、SDK version、diff reason、refresh trigger、evidence |
| `upstream_refresh_result` | refresh がある場合の upstream observation、許可根拠、migration / rollback / regression 結果。ない場合は `none` |
| `compatibility_mode_boundary_result` | Turso 互換 mode と Adlaire extension mode の差分、混入なし、snapshot 証跡 |

**canonical compatibility baseline closure audit：**

compatibility baseline は Turso Cloud 互換追従の入口 gate であり、現在の Adlaire DB 実装出力、古い snapshot、または根拠なし expected 更新で代替してはならない。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`compatibility_baseline_closure_result = pass` でなければ実装開始または Phase 完了に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `upstream_source_result` | baseline ID ごとに Turso Cloud API、libSQL SDK、hrana spec、legacy metadata、previous Phase snapshot のどれを基準にするかが一意に固定されている | 実装開始禁止 |
| `observed_behavior_result` | upstream / SDK / previous Phase の path、method、request、response、error、metadata、SDK behavior が artifact と transcript で再現できる | 実装開始禁止 |
| `adlaire_behavior_result` | Adlaire DB が採用する behavior、mode、error mapping、client impact、unsupported response が baseline と比較可能な形で固定されている | 実装開始禁止 |
| `compatibility_class_result` | `match`、`intentional_self_host_diff`、`unsupported_until_phase`、`upstream_changed`、`sdk_regression`、`adlaire_extension_mode_only` のいずれかに分類され、分類根拠がある | 実装開始禁止 |
| `snapshot_version_result` | snapshot、fixture、transcript の version、生成 commit、正規化 rule、更新理由が artifact manifest と一致する | Phase 未完了 |
| `sdk_version_result` | 対象 SDK 名、version、実行 transcript が記録され、SDK 対象外の場合は本文根拠付き `not_applicable` である | Phase 未完了 |
| `refresh_trigger_result` | upstream changelog、SDK update、Turso behavior diff、spec change、regression failure、security/persistence reason のいずれが refresh trigger か固定されている | 実装開始禁止 |
| `refresh_authorization_result` | refresh が仕様変更 commit、Turso 追従記録、SDK changelog、承認済み change ID のいずれにより許可されたか追跡できる | merge 不可 |
| `diff_reason_result` | 差分がある場合は diff reason、client impact、代替仕様、error mapping、regression を持ち、差分なしの場合は `none` と差分ゼロ evidence がある | merge 不可 |
| `migration_regression_result` | metadata、config、JWT claim、artifact、client migration の要否と、refresh 後に再実行する Phase regression / SDK transcript / Turso snapshot / legacy fixture が接続されている | rollout ready 不可 |
| `mode_boundary_result` | Turso 互換 mode と Adlaire extension mode の field、auth、metadata、error、snapshot が分離され、混入が 0 件である | merge 不可 |
| `compatibility_baseline_closure_result` | 上記 field がすべて `pass`。古い baseline、根拠なし snapshot 更新、SDK transcript 欠落、upstream 未確認、自己ホスト差分理由なし、mode 混入、regression 未実行がすべて 0 件 | `pass` 以外は実装開始禁止 |

`compatibility_baseline_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。互換対象がない Phase でも `compatibility_baseline_closure_result = pass` とし、`baseline_id:none`、`sdk_version:not_applicable`、`refresh_trigger:none`、`diff_reason:none`、および非対象理由を `closure_evidence` に記録する。これにより「互換影響なし」の判断も、後続 Phase と reviewer が再現できる状態にする。

Phase compatibility baseline に関係する仕様変更は、§1.4、§1.5、§3.5.3、§7.3、§9.1.10、§9.1.11、§9.1.15、§9.1.23、§9.1.24、§9.1.32、§9.1.33、§9.1.35、§9.1.39、§9.1.40、§9.1.43、§9.1.44、§9.1.45、§9.1.46、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.15、§9.17、該当 Phase 詳細節を同時更新する。compatibility baseline matrix がない Phase 実装 PR は、互換基準、snapshot 更新条件、SDK transcript、upstream 追従差分、自己ホスト例外の根拠が未確定であるため、実装開始不可とする。

#### 9.1.48 Phase security abuse / bypass resistance matrix 固定契約

各 Phase の実装 PR は、攻撃面、信頼しない入力、必須制御、bypass attempt、拒否応答、redaction、audit/log、quota/rate、永続化副作用を security abuse matrix として固定しなければならない。正常系 auth が通ることだけでは security 完了扱いにしない。悪用された場合に何を拒否し、何を記録し、何を漏らさず、どの副作用を禁止するかを Phase 開始前に固定する。

**Security abuse matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `security_case_id` | `SEC-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | security case を導入または変更する Phase |
| `attack_surface` | CLI / config / HTTP API / WebSocket / admin API / Turso Platform API / replication / backup / extension / HA / internal adapter |
| `untrusted_input` | token、JWT claim、path/query/body、SQL args、backup body、replication frame、extension path、metadata file、env/config、header |
| `required_control` | auth、scope、quota、block policy、path normalization、signature/checksum、idempotency、redaction、rate/size limit |
| `bypass_attempt` | control をすり抜ける試行。例: missing token、wrong scope、path traversal、duplicate request、replay、tampered metadata、oversized body |
| `expected_denial` | HTTP status、error code、WebSocket error、CLI exit code、health state、client action |
| `redaction_rule` | response/log/artifact/metric に出してよい field と禁止 field |
| `audit_log_rule` | 記録する event、level、request id / trace id、secret 非露出。記録不要なら理由 |
| `rate_or_quota_effect` | quota、block_reads、block_writes、rate/size limit への影響。影響なしなら理由 |
| `persistence_effect` | 拒否時に metadata、DB file、archive、token、usage、counter を変更しないこと。変更が必要なら commit order |
| `regression_test` | bypass / denial / redaction / persistence no-op を検証する TC、command、fixture |
| `evidence` | denied response、log sample、secret scan、metadata before/after、replay fixture、path traversal fixture |

**Phase group security minimum：**

| Phase group | 最低 security case |
|-------------|--------------------|
| Phase 1〜5 | CLI secret、config precedence、data-dir permission/lock、JWT missing/invalid/revoked、log redaction、hrana malformed body |
| Phase 6〜8 | admin token、DB scope、organization/group/location boundary、quota exceeded、block_reads/block_writes、Platform token、legacy metadata |
| Phase 9〜10 | WebSocket auth/stream ownership、transaction close rollback、ATTACH path traversal、ro token write denial、metrics label injection |
| Phase 11〜13 | replication token、frame checksum tamper、snapshot access、archive manifest corruption、replica retry without token leak |
| Phase 14〜15 | backup body size/type、restore/PITR destructive lock、branch source scope、delete protection、quota before commit |
| Phase 16〜18 | extension path/symlink/signature、metrics snapshot poisoning、HA promote auth、term rollback、split-brain operator_required |
| Phase 19 | internal adapter shadow isolation、active switch flag、silent fallback denial、compat diff redaction、rollback flag abuse |

**bypass resistance 禁止事項：**

| 状態 | 判定 |
|------|------|
| security case がない attack surface を公開する | 実装開始禁止 |
| auth / scope / quota / block policy を bypass して success response を返す | merge 不可 |
| token、JWT、admin/platform/replication/HA token、SQL args、backup body、raw path を response/log/artifact/metric に出す | merge 不可 |
| path traversal、absolute path、symlink、reserved name を受理する | merge 不可 |
| quota、block_writes、rate/size limit、delete protection を commit 後に評価する | merge 不可 |
| replay / duplicate request で二重作成、二重 token、二重 quota charge、二重 branch を発生させる | merge 不可 |
| security failure を `INTERNAL_ERROR`、generic 500、ログのみで隠す | Phase 未完了 |
| security case を manual only で pass 扱いにする | merge 不可 |
| denial 時に metadata / file / runtime map が変わらない証跡がない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `security_abuse_result` | security case ID ごとの bypass attempt、expected denial、redaction、audit/log、persistence no-op、evidence |
| `security_redaction_result` | secret scan、log/artifact/response/metric redaction、漏洩 0 件 |
| `security_bypass_closure` | bypass、replay、path traversal、scope/quota/block policy 回避が 0 件である証跡 |

**canonical security abuse closure audit：**

security abuse / bypass resistance は成功系 auth の補助ではなく、Phase 実装開始前の入口 gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`security_abuse_closure_result = pass` でなければ attack surface を公開してはならない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `attack_surface_result` | CLI / config / HTTP API / WebSocket / admin API / Turso Platform API / replication / backup / extension / HA / internal adapter の公開面が Phase scope と一致し、security case ID に接続されている | 実装開始禁止 |
| `untrusted_input_result` | token、JWT claim、path/query/body、SQL args、backup body、replication frame、extension path、metadata file、env/config、header が列挙され、検証対象外が根拠付きである | 実装開始禁止 |
| `required_control_result` | auth、scope、quota、block policy、path normalization、signature/checksum、idempotency、redaction、rate/size limit の必要 control が attack surface ごとに固定されている | 実装開始禁止 |
| `bypass_attempt_result` | missing token、wrong scope、path traversal、duplicate request、replay、tampered metadata、oversized body などの bypass attempt が scenario / regression に接続されている | 実装開始禁止 |
| `expected_denial_result` | deny 時の HTTP status、error code、WebSocket error、CLI exit code、health state、client action が §7.3 / §9.7 / scenario matrix と一致している | merge 不可 |
| `redaction_rule_result` | response、log、artifact、metric に出してよい field と禁止 field が固定され、secret scan と redaction artifact に接続されている | merge 不可 |
| `audit_log_result` | audit/log event、level、request id / trace id、secret 非露出、記録不要理由が固定され、operator / review handoff で再現できる | Phase 未完了 |
| `quota_rate_result` | quota、block_reads、block_writes、rate/size limit、delete protection の評価順序が commit 前であり、拒否時の client-visible behavior が固定されている | merge 不可 |
| `persistence_noop_result` | denial 時に metadata、DB file、archive、token、usage、counter、runtime map が変わらないこと、または必要変更の commit order が evidence で示されている | Phase 未完了 |
| `security_regression_result` | bypass、denial、redaction、persistence no-op、replay、path traversal、scope/quota/block policy の regression test、command、fixture、artifact が coverage closure に接続されている | Phase 未完了 |
| `security_abuse_closure_result` | 上記 field がすべて `pass`。未接続 attack surface、未列挙 untrusted input、control 未定義、bypass 未検証、denial 未定義、secret leak、manual only security case、persistence side effect がすべて 0 件 | `pass` 以外は実装開始禁止 |

`security_abuse_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。security 対象外に見える Phase でも `security_abuse_closure_result = pass` とし、`attack_surface:none`、`untrusted_input:none`、`required_control:none`、`security_case_id:none`、および非対象理由を `closure_evidence` に記録する。これにより「security 影響なし」の判断も、後続 Phase と reviewer が再現できる状態にする。

Phase security abuse に関係する仕様変更は、§7.3、§9.1.10、§9.1.11、§9.1.14、§9.1.17、§9.1.18、§9.1.21、§9.1.23、§9.1.24、§9.1.26、§9.1.32、§9.1.33、§9.1.36、§9.1.37、§9.1.39、§9.1.40、§9.1.42、§9.1.43、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.14、§9.17、該当 Phase 詳細節を同時更新する。security abuse matrix がない Phase 実装 PR は、攻撃面、bypass、denial、redaction、persistence no-op の根拠が未確定であるため、実装開始不可とする。

#### 9.1.49 Phase ambiguity closure / implementation decision table 固定契約

各 Phase の実装 PR は、実装者が仕様本文を読んだ時点で迷う可能性がある判断を ambiguity closure matrix として実装開始前に閉じなければならない。曖昧なまま実装し、レビュー、テスト失敗、運用投入、後続 Phase で判断を補うことは禁止する。選択肢が複数ある場合は、推奨決定、却下した案、根拠、影響範囲、証跡、再検討条件を仕様本文に固定する。

**Ambiguity closure matrix 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `ambiguity_id` | `AMB-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | 判断を閉じる Phase |
| `implementation_question` | 実装者が迷う具体的な問い。例: auth と quota の優先、metadata 欠損時の扱い、retry 可否 |
| `candidate_options` | 検討した選択肢。最低 2 案。選択肢が 1 つしかない場合は理由 |
| `selected_decision` | 採用する挙動、status、error code、schema、commit order、log、client action |
| `rejected_options` | 却下した案と却下理由。Turso Cloud 互換、自己ホスト安全性、後方互換、運用性への影響を含める |
| `decision_basis` | 参照する仕様節、Turso Cloud / libSQL SDK baseline、既存 invariant、security / persistence 根拠 |
| `affected_contracts` | 同時更新が必要な API、schema、error、persistence、security、compatibility、test、artifact |
| `edge_cases` | null、missing、duplicate、race、restart、legacy、unsupported、malformed、large input、permission denied |
| `not_applicable_rule` | N/A を認める条件。根拠なし N/Aは禁止 |
| `reopen_trigger` | upstream change、仕様変更、regression、security finding、migration failure など再検討条件 |
| `evidence` | snapshot、fixture、oracle、decision log、spec diff、review checklist |

**Phase group ambiguity minimum：**

| Phase group | 最低 ambiguity closure |
|-------------|------------------------|
| Phase 1〜5 | CLI flag/env/TOML/default 優先、data-dir lock、default DB open failure、JWT missing/invalid/revoked、hrana error status |
| Phase 6〜8 | DB 名衝突、org/group/location scope、quota/block precedence、legacy metadata migration、Platform API 互換差分 |
| Phase 9〜10 | WebSocket close 時 tx、stream id 再利用、ATTACH 対象外、read-only token write、metrics counter 更新タイミング |
| Phase 11〜13 | replication frame ordering、checksum 不一致、snapshot lag、archive retention、replica retry/backoff |
| Phase 14〜15 | restore/PITR 失敗時 rollback、branch seed source、delete protection、quota before commit、source unavailable |
| Phase 16〜18 | extension allowlist/signature、Prometheus label、metrics snapshot 破損、HA promote/demote、split-brain handling |
| Phase 19 | shadow diff 許容範囲、active switch 条件、internal fallback、performance regression 閾値、rollback flag |

**曖昧表現禁止事項：**

| 表現 / 状態 | 判定 |
|-------------|------|
| `TBD`、`TODO`、`FIXME`、`未定`、`後で決める` が production path に残る | 実装開始禁止 |
| `実装判断`、`よしなに`、`必要に応じて`、`適宜`、`可能なら` を完了条件に使う | merge 不可 |
| 複数の valid behavior があるのに selected decision がない | 実装開始禁止 |
| 却下案と却下理由がない | review failure |
| N/A に仕様本文の根拠がない | Phase 未完了 |
| upstream / SDK / previous Phase と差分があるのに decision basis がない | merge 不可 |
| error code、status、client action、commit order、redaction のいずれかを実装者判断にする | merge 不可 |
| ambiguity closure が PR description のみで仕様本文にない | 仕様として扱わない |
| open ambiguity が 1 件以上ある状態で Done receipt を出す | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `ambiguity_closure_result` | ambiguity ID ごとの selected decision、rejected options、decision basis、affected contracts、evidence |
| `open_ambiguity_count` | `0` 固定。0 以外は Phase 未完了 |
| `decision_reopen_result` | reopen trigger 該当なし、または該当時の仕様更新と regression 結果 |

Phase ambiguity closure に関係する仕様変更は、§0、§7.3、§9.1.10、§9.1.11、§9.1.12、§9.1.14、§9.1.15、§9.1.18、§9.1.19、§9.1.20、§9.1.21、§9.1.22、§9.1.23、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.35、§9.1.38、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.1.43、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.1.48、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.14、§9.17、該当 Phase 詳細節を同時更新する。ambiguity closure matrix がない Phase 実装 PR は、実装判断、N/A、例外、error/status、commit order、互換差分の根拠が未確定であるため、実装開始不可とする。

#### 9.1.50 Phase atomic implementation task ledger 固定契約

各 Phase の実装 PR は、Phase scope を atomic task ledger に分解し、task ID ごとに入力契約、変更対象、禁止変更、完了条件、検証 command、証跡、rollback 条件を固定しなければならない。Phase 全体を大きな一括実装として扱うこと、task ID なしで差分を追加すること、検証がない task を Done 扱いにすることは禁止する。

**Atomic task ledger 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `task_id` | `TASK-P{phase}-{number}` 形式の一意 ID |
| `phase` | task が属する Phase |
| `task_goal` | 1 task で完了させる具体的な成果。API、persistence、security、test、artifact のいずれに効くかを明記 |
| `input_contracts` | 参照する Phase packet、manifest、schema、scenario、decision、security、compatibility、ambiguity ID |
| `change_targets` | 変更してよい module、API、config、metadata、test、artifact、docs。仕様書 PR では対象節番号 |
| `forbidden_changes` | 同 task で変更してはならない surface、future Phase、互換契約、schema、error、auth、secret、dependency |
| `completion_condition` | task が完了したと判定する observable result、snapshot、artifact、Done receipt field |
| `verification_command` | task 完了を確認する command、expected exit code、環境、artifact path。実行不能なら manual exception ID |
| `rollback_condition` | task 失敗時に戻す state、metadata/file、config、runtime map、artifact。rollback 不要なら理由 |
| `dependency` | 先行 task、block する task、並列可否 |
| `evidence` | diff、test log、snapshot、fixture、secret scan、metadata before/after、review checklist |

**Phase group atomic task minimum：**

| Phase group | 最低 task 分解 |
|-------------|----------------|
| Phase 1〜5 | CLI/config、data-dir、default DB、HTTP pipeline、JWT、log/redaction、SDK smoke、restart test を分離 |
| Phase 6〜8 | DB CRUD、admin token、scope、organization/group/location、quota/block、Platform API、legacy migration を分離 |
| Phase 9〜10 | WebSocket handshake、stream、transaction、ATTACH policy、metrics persistence、ro/rw enforcement を分離 |
| Phase 11〜13 | primary frame、replica apply、checksum、snapshot、archive manifest、retention、health/redirect を分離 |
| Phase 14〜15 | backup create、restore rollback、PITR select、branch create/delete、seed isolation、quota/delete protection を分離 |
| Phase 16〜18 | extension registry、signature/load、metrics snapshot、Prometheus、HA term、promote/demote、split-brain を分離 |
| Phase 19 | shadow adapter、diff capture、active switch、fallback denial、performance baseline、rollback flag を分離 |

**atomic task 禁止事項：**

| 状態 | 判定 |
|------|------|
| Phase 実装を 1 つの巨大 task として扱う | 実装開始禁止 |
| `TASK-P{phase}-{number}` がない差分を入れる | merge 不可 |
| task の input contracts が Phase packet / manifest / matrix に接続していない | 実装開始禁止 |
| task に verification command または manual exception ID がない | Phase 未完了 |
| task 外の API、metadata、config、dependency、error code、auth 境界を変更する | merge 不可 |
| rollback condition がない状態変更 task を完了扱いにする | Phase 未完了 |
| dependency 未完了の task を先に merge する | merge 不可 |
| open task が 1 件以上ある状態で Done receipt を出す | Phase 未完了 |
| task 完了を PR description の説明だけで代替する | 仕様として扱わない |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `atomic_task_result` | task ID ごとの input contracts、change targets、completion condition、verification command、evidence |
| `open_task_count` | `0` 固定。0 以外は Phase 未完了 |
| `task_dependency_result` | dependency 順序、並列実行可否、blocked task 0 件、rollback condition の証跡 |

**canonical ambiguity / atomic task closure audit：**

ambiguity closure と atomic task ledger は、Phase 実装者が迷わず、かつ巨大な一括実装に逃げずに進めるための入口 gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`ambiguity_atomic_task_closure_result = pass` でなければ実装開始または Phase 完了に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `implementation_question_result` | 実装者が迷う可能性のある question が `AMB-P{phase}-{surface}-{number}` として列挙され、対象外 question は根拠付きである | 実装開始禁止 |
| `selected_decision_result` | 各 ambiguity ID に採用する status、error code、schema、commit order、log、client action、redaction が一意に固定されている | 実装開始禁止 |
| `rejected_options_result` | 却下案と却下理由があり、Turso Cloud 互換、自己ホスト安全性、後方互換、運用性への影響が記録されている | review failure |
| `decision_basis_result` | 参照仕様節、Turso Cloud / libSQL SDK baseline、invariant、security / persistence 根拠が affected contracts と接続されている | merge 不可 |
| `edge_case_result` | null、missing、duplicate、race、restart、legacy、unsupported、malformed、large input、permission denied が scenario / oracle / regression に接続されている | Phase 未完了 |
| `reopen_trigger_result` | upstream change、仕様変更、regression、security finding、migration failure の再検討条件と再評価手順が固定されている | Phase 未完了 |
| `task_id_result` | Phase scope が `TASK-P{phase}-{number}` に分解され、open task 0 件、task ID なし差分 0 件である | 実装開始禁止 |
| `task_boundary_result` | input contracts、change targets、forbidden changes、future Phase 境界、互換契約、schema、error、auth、secret、dependency が task ごとに固定されている | 実装開始禁止 |
| `task_verification_result` | 各 task が completion condition、verification command、expected exit code、environment、artifact path、manual exception ID のいずれかに接続されている | Phase 未完了 |
| `task_dependency_rollback_result` | task dependency、並列可否、blocked task 0 件、rollback condition、rollback 不要理由が固定されている | Phase 未完了 |
| `ambiguity_atomic_task_closure_result` | 上記 field がすべて `pass`。open ambiguity、根拠なし N/A、実装者判断、task ID なし差分、巨大 task、task 外変更、verification 欠落、rollback 未定義がすべて 0 件 | `pass` 以外は実装開始禁止 |

`ambiguity_atomic_task_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。ambiguity がない Phase でも `ambiguity_atomic_task_closure_result = pass` とし、`ambiguity_id:none`、`open_ambiguity_count:0`、`open_task_count:0`、`task_id_result:pass`、および非対象理由を `closure_evidence` に記録する。これにより「迷う判断なし」の主張も、後続 Phase と reviewer が再現できる状態にする。

Phase atomic task ledger に関係する仕様変更は、§9.1.7、§9.1.8、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.16、§9.1.21、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.34、§9.1.35、§9.1.38、§9.1.39、§9.1.40、§9.1.41、§9.1.42、§9.1.43、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.1.48、§9.1.49、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。atomic task ledger がない Phase 実装 PR は、実装粒度、変更境界、検証単位、rollback 単位、完了判定が未確定であるため、実装開始不可とする。

#### 9.1.51 Phase review handoff / independent reproducibility 固定契約

各 Phase の実装 PR は、実装者以外のレビュアーが仕様本文、Phase packet、artifact だけを使って同じ判断と検証を再現できる review handoff packet を固定しなければならない。口頭説明、チャット履歴、PR description だけの補足、実装者のローカル環境だけに依存する再現手順を完了根拠にしてはならない。

**Review handoff packet 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `handoff_id` | `HANDOFF-P{phase}-{number}` 形式の一意 ID |
| `phase` | handoff 対象 Phase |
| `reading_order` | 第三者が読む順番。最低でも §9.2、§9.4、§9.8、§9.11、§9.17、Phase packet、Done receipt、該当 Phase 詳細節 |
| `reproduction_commands` | local / Docker / CI の再現 command、expected exit code、必要 env、timeout、artifact path |
| `expected_artifacts` | snapshot、fixture、log、secret scan、metadata before/after、compat transcript、release-check result |
| `decision_criteria` | Done / Not Done / Spec correction required を判定する条件、error/status 差分の扱い |
| `failure_classification` | 再現失敗時に defect、spec gap、oracle gap、environment gap、security gap、compatibility diff のどれに分類するか |
| `reviewer_scope` | レビューで確認する API、persistence、security、compatibility、rollback、redaction、task ledger |
| `out_of_scope` | レビュー対象外とする項目と仕様根拠。根拠なし N/Aは禁止 |
| `oral_context_free_evidence` | 口頭補足、チャット履歴、実装者記憶なしで判断できる artifact / spec section |
| `handoff_result_location` | review result、再現 log、差分メモ、未完了判定を保存する場所 |

**Phase group handoff minimum：**

| Phase group | 最低 handoff 対象 |
|-------------|-------------------|
| Phase 1〜5 | CLI/config、data-dir、HTTP pipeline、JWT、log redaction、SDK smoke、restart reproducibility |
| Phase 6〜8 | admin API、Platform API、metadata migration、scope/quota、organization/group/location、legacy fixture |
| Phase 9〜10 | WebSocket transcript、transaction rollback、ATTACH denial、metrics persistence、ro/rw enforcement |
| Phase 11〜13 | replication frame、checksum、snapshot、archive manifest、retention、primary/replica recovery |
| Phase 14〜15 | backup/restore/PITR、branch lifecycle、destructive rollback、seed isolation、quota/delete protection |
| Phase 16〜18 | extension signature/load、metrics snapshot、Prometheus output、HA promote/demote、split-brain |
| Phase 19 | shadow/active diff、fallback denial、performance baseline、rollback flag、Phase 1〜18 regression |

**review handoff 禁止事項：**

| 状態 | 判定 |
|------|------|
| レビュアーが PR description だけを読まないと判断できない | 仕様として扱わない |
| 口頭説明、チャット履歴、実装者の記憶を完了根拠にする | merge 不可 |
| 再現 command が local only で環境差分が固定されていない | Phase 未完了 |
| expected artifact の保存先、正規化、secret scan がない | Phase 未完了 |
| reviewer decision criteria が Done / Not Done / Spec correction required に写像されていない | review failure |
| 再現失敗時の分類先がない | Phase 未完了 |
| レビュアー判断任せで N/A、差分許容、snapshot 更新を決める | merge 不可 |
| handoff result が Done receipt と接続していない | Phase 未完了 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `review_handoff_result` | handoff ID ごとの reading order、reproduction command、expected artifact、decision criteria、review result |
| `independent_reproduction_result` | 実装者以外が再実行できる command、exit code、artifact、環境差分、再現可否 |
| `oral_context_free_result` | 口頭補足なしで Done / Not Done / Spec correction required を判定できる証跡 |

Phase review handoff に関係する仕様変更は、§9.1.3、§9.1.10、§9.1.11、§9.1.13、§9.1.14、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.34、§9.1.35、§9.1.36、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.1.48、§9.1.49、§9.1.50、§9.2、§9.4、§9.8、§9.11、§9.17、該当 Phase 詳細節を同時更新する。review handoff packet がない Phase 実装 PR は、第三者再現性、レビュー判断基準、artifact 完備性、失敗分類が未確定であるため、Phase 完了扱いにしない。

#### 9.1.52 Phase release note / operator-facing behavior delta 固定契約

各 Phase の実装 PR は、利用者・運用者から見える挙動差分を operator behavior delta として固定しなければならない。実装内部の完了、テスト成功、レビュアー再現性が満たされていても、外部 API、CLI、config、metadata、auth、logs、health、metrics、backup、replication、HA、rollback、非対応範囲の変化が運用者向けに分類されていない場合、その Phase は完了扱いにしない。

**Operator behavior delta 必須 fields：**

| Field | 必須内容 |
|-------|----------|
| `delta_id` | `DELTA-P{phase}-{surface}-{number}` 形式の一意 ID |
| `phase` | delta を導入する Phase |
| `audience` | user、operator、SDK client、admin、SRE、backup operator、HA operator、developer のいずれか |
| `external_surface` | CLI / HTTP API / WebSocket / config / metadata / auth / log / health / metrics / backup / replication / branch / HA / extension |
| `behavior_before` | 前 Phase または未対応時の挙動。新規の場合は `not_available` と明記 |
| `behavior_after` | 当該 Phase で観測される挙動、status、error code、log、metric、artifact、operator action |
| `compatibility_delta` | `none`、`turso_match`、`intentional_self_host_diff`、`breaking_change`、`new_feature`、`deprecated`、`unsupported_until_phase` |
| `operator_action` | 設定変更、migration、restart、backup、token rotation、monitoring 更新、runbook 更新。不要なら理由 |
| `migration_or_config_change` | config key、metadata migration、env、default 変更、旧形式対応、rollback 可否 |
| `rollback_note` | rollback 時の operator 手順、data safety、互換性、戻せない変更の有無 |
| `release_note_text` | そのまま release note に載せられる短文。内部実装名ではなく利用者視点で書く |
| `evidence` | snapshot、SDK transcript、health/log/metric sample、migration fixture、rollback artifact、compat diff |

**Phase group operator delta minimum：**

| Phase group | 最低 delta |
|-------------|------------|
| Phase 1〜5 | CLI 起動、config precedence、data-dir layout、health/pipeline、JWT/token、log redaction、SDK 接続方法 |
| Phase 6〜8 | admin API、Platform API、multi DB、organization/group/location、quota/block policy、metadata migration |
| Phase 9〜10 | WebSocket behavior、transaction rollback、ATTACH 非対応/拒否、metrics counter、ro/rw write denial |
| Phase 11〜13 | replication primary/replica、archive manifest、retention、health/redirect、lag/degraded 表示 |
| Phase 14〜15 | backup/restore/PITR、destructive lock、branch lifecycle、seed/source、delete protection、rollback 手順 |
| Phase 16〜18 | extension allowlist、metrics/Prometheus、HA status/promote/demote、split-brain operator action |
| Phase 19 | internal adapter switch、shadow/active mode、fallback denial、performance baseline、rollback flag、外部挙動差分ゼロ |

**operator delta 禁止事項：**

| 状態 | 判定 |
|------|------|
| 外部挙動が変わるのに release note / behavior delta がない | Phase 未完了 |
| breaking change または intentional self-host diff が互換差分として分類されていない | merge 不可 |
| operator action が必要なのに手順、restart 要否、rollback が未定義 | rollout ready 不可 |
| config / metadata / token / backup / HA の変更に migration_or_config_change がない | merge 不可 |
| logs / health / metrics の変化が sample artifact なしで記載される | Phase 未完了 |
| unsupported / deprecated / future Phase の挙動が release note で不明 | review failure |
| 内部実装名だけで利用者視点の release_note_text がない | Phase 未完了 |
| rollback 不可の変更を rollback 可として扱う | merge 不可 |

**Done receipt への反映：**

| Done receipt field | 必須内容 |
|--------------------|----------|
| `operator_behavior_delta_result` | delta ID ごとの audience、external surface、before/after、compatibility delta、operator action、evidence |
| `release_note_result` | release note text、breaking/new/deprecated/unsupported 分類、Turso/self-host 差分、公開可否 |
| `operator_rollout_result` | migration/config/restart/rollback/monitoring 更新の要否と証跡 |

**canonical review handoff / operator delta closure audit：**

review handoff と operator behavior delta は、実装者以外の第三者再現性と、利用者・運用者から見える挙動差分を同時に閉じるための出口 gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`review_operator_closure_result = pass` でなければ Phase 完了、rollout ready、PR merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `reading_order_result` | reviewer が読む仕様節、Phase packet、受入 manifest、Done receipt、artifact manifest、Phase 詳細節の順序が固定されている | Phase 未完了 |
| `reproduction_command_result` | local / Docker / CI の再現 command、expected exit code、必要 env、timeout、artifact path が固定されている | Phase 未完了 |
| `expected_artifact_result` | snapshot、fixture、log、secret scan、metadata before/after、compat transcript、release-check result の保存先、hash、正規化 rule が固定されている | Phase 未完了 |
| `decision_criteria_result` | Done / Not Done / Spec correction required の判定条件、許容差分、snapshot 更新条件、review failure 条件が一意に固定されている | review failure |
| `failure_classification_result` | 再現失敗時の defect、spec gap、oracle gap、environment gap、security gap、compatibility diff、operator gap の分類先と修正 PR 種別が固定されている | Phase 未完了 |
| `oral_context_free_result` | 口頭説明、チャット履歴、実装者の記憶なしで判断できる仕様節、artifact、command、log が接続されている | merge 不可 |
| `operator_surface_result` | user、operator、SDK client、admin、SRE、backup operator、HA operator、developer ごとの外部 surface が列挙され、対象外 surface は根拠付きである | Phase 未完了 |
| `behavior_delta_result` | before/after、compatibility delta、new/deprecated/unsupported/breaking/self-host diff の分類が delta ID ごとに固定されている | merge 不可 |
| `operator_action_result` | migration、config、restart、backup、token rotation、monitoring、runbook、rollback の必要有無と手順が artifact に接続されている | rollout ready 不可 |
| `release_note_rollback_result` | release note text、公開可否、rollback note、data safety、戻せない変更の有無が利用者視点で固定されている | Phase 未完了 |
| `review_operator_closure_result` | 上記 field がすべて `pass`。local only 再現、口頭説明依存、artifact 欠落、判断任せ N/A、release note 欠落、operator action 未定義、rollback 不明がすべて 0 件 | `pass` 以外は Phase 完了禁止 |

`review_operator_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。review / operator 影響がない Phase でも `review_operator_closure_result = pass` とし、`handoff_id:none`、`delta_id:none`、`operator_action:none`、`release_note_text:no_user_visible_change`、および非対象理由を `closure_evidence` に記録する。これにより「外部挙動差分なし」「レビュー追加手順なし」の主張も、後続 Phase と reviewer が再現できる状態にする。

**canonical merge readiness / stale artifact closure audit：**

merge readiness は Phase 実装 PR の最終 gate であり、実装開始時点の readiness、過去 CI 成功、または古い artifact だけで代替してはならない。対象 Phase の Done receipt と PR merge checklist は、以下の closure audit field を持ち、`merge_readiness_closure_result = pass` でなければ merge してはならない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `base_branch_freshness_result` | merge 前に base branch の最新 commit、PR head commit、rebase/merge commit 要否、conflict なしが記録されている | merge 不可 |
| `pr_diff_scope_result` | PR diff が Phase packet、atomic task ledger、change impact matrix の許可範囲内であり、task ID なし差分が 0 件である | merge 不可 |
| `artifact_commit_match_result` | Done receipt、artifact manifest、review handoff、CI log、snapshot の commit SHA が PR head commit と一致する | merge 不可 |
| `ci_rerun_result` | base 更新後または artifact 更新後に必須 CI / release-check が再実行され、latest run の exit code と artifact path が記録されている | merge 不可 |
| `regression_rerun_result` | Phase regression inheritance に従い、対象 Phase と過去 Phase regression が latest PR head で再実行されている | Phase 未完了 |
| `stale_snapshot_result` | snapshot、fixture、SDK transcript、compat baseline、golden artifact が古い commit、旧仕様 version、旧 Contract ID を参照していない | merge 不可 |
| `review_reapproval_result` | base 更新、scope 変更、artifact 再生成、snapshot 更新、operator delta 変更後に必要な reviewer 再承認が記録されている | review failure |
| `merge_blocker_result` | conflict、failing check、unknown merge state、unreviewed change、open failure、known flaky、rootless N/A、secret scan failure が 0 件である | merge 不可 |
| `merge_readiness_closure_result` | 上記 field がすべて `pass`。古い artifact、古い CI、base drift、unreviewed diff、scope 外変更、再承認漏れ、merge blocker がすべて 0 件 | `pass` 以外は merge 不可 |

`merge_readiness_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。仕様修正だけの PR でも `merge_readiness_closure_result = pass` とし、`base_branch_freshness_result`、`artifact_commit_match_result`、`ci_rerun_result`、`stale_snapshot_result`、`merge_blocker_result` の根拠を `closure_evidence` に記録する。これにより、merge 直前の「最後に通ったはず」「artifact は多分最新」という判断を禁止し、reviewer が PR head commit だけで merge 可否を再現できる状態にする。

**canonical dependency / supply-chain / provenance closure audit：**

dependency provenance は、Phase 実装で追加・更新・生成・実行される外部 crate、SDK、toolchain、binary、生成 artifact の由来を固定する入口 gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`dependency_provenance_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `dependency_inventory_result` | 追加・更新・削除する crate、SDK、CLI tool、Docker image、GitHub Action、binary artifact が Phase packet に列挙され、対象外は根拠付きである | 実装開始禁止 |
| `version_pin_result` | dependency version、lockfile、toolchain、Docker image digest、SDK version が固定され、floating latest、range-only、未記録 binary が 0 件である | merge 不可 |
| `license_policy_result` | 各 dependency の license、互換可否、配布影響、self-host 利用影響が記録され、禁止 license が 0 件である | merge 不可 |
| `security_advisory_result` | 既知脆弱性、yanked version、deprecated package、malicious package、unmaintained critical dependency が scan され、未評価 0 件である | merge 不可 |
| `build_toolchain_result` | Rust/PHP/Node/Docker/CI/local の toolchain version、platform、feature flag、build env が artifact と一致している | 実装開始禁止 |
| `generated_artifact_provenance_result` | 生成 code、snapshot、fixture、SDK transcript、OpenAPI/schema、migration artifact の producer command、input、commit SHA、hash が記録されている | Phase 未完了 |
| `binary_extension_provenance_result` | SQLite extension、native binary、container image、prebuilt artifact の source、sha256、signature/verification、allowlist、runtime path が固定されている | merge 不可 |
| `dependency_runtime_surface_result` | dependency が触れる network、filesystem、process、secret、DB file、extension load、SQL execution の runtime surface が security abuse matrix に接続されている | 実装開始禁止 |
| `rollback_removal_result` | dependency 追加・更新を戻す手順、lockfile rollback、generated artifact 再生成、runtime config 無効化、互換影響が固定されている | Phase 未完了 |
| `dependency_provenance_closure_result` | 上記 field がすべて `pass`。未固定 version、未評価 license、脆弱性未確認、由来不明 binary、生成物 producer 不明、runtime surface 未接続、rollback 不明がすべて 0 件 | `pass` 以外は実装開始禁止 |

`dependency_provenance_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。dependency / toolchain / generated artifact / binary extension に変更がない Phase でも `dependency_provenance_closure_result = pass` とし、`dependency_change:none`、`toolchain_change:none`、`generated_artifact_change:none`、`binary_extension_change:none`、および非対象理由を `closure_evidence` に記録する。これにより「依存変更なし」「生成物なし」の主張も、後続 Phase と reviewer が再現できる状態にする。

**canonical upgrade / downgrade / data compatibility closure audit：**

upgrade / downgrade / data compatibility は、旧 Phase、旧 metadata、旧 config、既存 DB file を持つ環境を新 Phase へ進めるための data-safety gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`upgrade_data_compatibility_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `source_version_inventory_result` | 対象 Phase が受け入れる旧 Phase、旧仕様 version、metadata schema version、config version、DB file state が列挙されている | 実装開始禁止 |
| `upgrade_path_result` | old -> new の migration trigger、順序、fsync、commit marker、idempotency、再実行時挙動が固定されている | 実装開始禁止 |
| `downgrade_boundary_result` | downgrade 可否、旧 binary 起動可否、不可の場合の拒否 error、operator action、data safety が固定されている | Phase 未完了 |
| `data_file_compatibility_result` | SQLite/libSQL DB file、WAL、archive、backup、branch、extension、adapter state の読み取り・書き込み互換が artifact に接続されている | merge 不可 |
| `metadata_schema_compatibility_result` | metadata field 追加・削除・rename・default・future schema・corrupt schema の挙動が fixture と rollback に接続されている | merge 不可 |
| `config_compatibility_result` | 旧 config/env/CLI/default の読み替え、非推奨、拒否、不正値、secret handling が operator delta と接続されている | 実装開始禁止 |
| `client_request_compatibility_result` | 旧 SDK/client request、旧 response snapshot、旧 error code、旧 pagination/cursor が Turso compatibility baseline と接続されている | merge 不可 |
| `failure_injection_upgrade_result` | migration 中断、fsync failure、partial write、crash、restart、concurrency、rollback 不能の injection が artifact に接続されている | Phase 未完了 |
| `rollback_after_upgrade_result` | upgrade 後の rollback 可否、rollback flag、backup restore、operator 手順、戻せない変更の明示が固定されている | Phase 未完了 |
| `upgrade_data_compatibility_closure_result` | 上記 field がすべて `pass`。旧 version 未列挙、片方向 migration、future schema 黙認、rollback 不明、partial upgrade 成功扱い、旧 client 差分未分類がすべて 0 件 | `pass` 以外は実装開始禁止 |

`upgrade_data_compatibility_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。upgrade / downgrade / data compatibility に影響しない Phase でも `upgrade_data_compatibility_closure_result = pass` とし、`source_version_inventory:unchanged`、`upgrade_path:none`、`downgrade_boundary:unchanged`、`data_file_compatibility:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「migration なし」「既存データ影響なし」の主張も、後続 Phase と reviewer が再現できる状態にする。

**canonical performance / capacity / resource limit closure audit：**

performance / capacity / resource limit は、Phase 実装が正常系だけでなく、負荷、容量、上限、backpressure の条件でも運用可能であることを固定する gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`performance_capacity_closure_result = pass` でなければ Phase 完了、rollout ready、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `workload_profile_result` | 対象 Phase の代表 workload、read/write 比率、request size、DB size、concurrency、duration、warmup、測定環境が固定されている | 実装開始禁止 |
| `latency_baseline_result` | p50/p95/p99 latency、timeout 閾値、baseline 比率、測定 command、artifact path が固定されている | Phase 未完了 |
| `memory_baseline_result` | RSS、heap/native memory、connection 数、stream/job 数、増加上限、測定 command が固定されている | Phase 未完了 |
| `storage_growth_result` | DB/WAL/archive/backup/branch/metadata/artifact の size 増加、cleanup、retention、上限超過時挙動が固定されている | merge 不可 |
| `large_input_limit_result` | request body、SQL batch、result rows、backup body、extension binary、snapshot artifact の size limit と error code が固定されている | 実装開始禁止 |
| `concurrency_capacity_result` | 同時 request、stream、transaction、job、replica、branch、extension load の許容量と拒否/queue policy が固定されている | Phase 未完了 |
| `backpressure_timeout_result` | resource lock timeout、busy timeout、queue full、replication lag、restore/backup/branch 中の backpressure と client action が固定されている | Phase 未完了 |
| `quota_capacity_result` | organization/group/database quota、usage 更新、quota 超過時の pre-commit denial、二重 charge 防止が artifact に接続されている | merge 不可 |
| `performance_regression_result` | 対象 Phase と継承 Phase の performance baseline 差分、許容閾値、差分理由、rollback 条件が固定されている | merge 不可 |
| `performance_capacity_closure_result` | 上記 field がすべて `pass`。測定条件なし baseline、large input 未定義、容量肥大化、backpressure 未定義、quota race、性能 regression 未分類がすべて 0 件 | `pass` 以外は Phase 完了禁止 |

`performance_capacity_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。performance / capacity に影響しない Phase でも `performance_capacity_closure_result = pass` とし、`workload_profile:unchanged`、`latency_baseline:unchanged`、`memory_baseline:unchanged`、`storage_growth:unchanged`、`quota_capacity:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「性能影響なし」「容量影響なし」の主張も、後続 Phase と reviewer が再現できる状態にする。

**canonical incident response / disaster recovery / operator runbook closure audit：**

incident response / disaster recovery / operator runbook は、障害検出後に運用者が何を確認し、どの手順で復旧し、いつ復旧完了と判断するかを固定する gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`incident_recovery_closure_result = pass` でなければ Phase 完了、rollout ready、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `incident_classification_result` | data loss risk、corruption、auth breach、quota exhaustion、replication lag、HA split-brain、backup/restore failure、adapter failure の分類と severity が固定されている | 実装開始禁止 |
| `detection_signal_result` | health、log、metric、exit code、artifact、admin status のどれで検出するか、検出不能の場合の禁止扱いが固定されている | Phase 未完了 |
| `rto_rpo_result` | 対象 Phase の RTO/RPO、許容 downtime、許容 data loss、backup/replication/PITR 依存が固定されている | rollout ready 不可 |
| `operator_runbook_result` | 初動確認、停止/隔離、safe mode、backup取得、restore/PITR、replica/HA 操作、再開判定、エスカレーションが手順化されている | Phase 未完了 |
| `safe_mode_result` | read-only、write stop、degraded health、maintenance mode、traffic drain、rollback flag の発動条件と解除条件が固定されている | Phase 未完了 |
| `backup_restore_drill_result` | backup 取得、restore、PITR、restore-failed marker、rollback、integrity_check の drill command と artifact が固定されている | merge 不可 |
| `replication_ha_recovery_result` | primary/replica lag、archive gap、promotion failure、split-brain、demote failure、operator_required の復旧手順が固定されている | Phase 未完了 |
| `post_incident_evidence_result` | incident timeline、root cause、affected resources、recovery command、artifact hash、secret scan、regression rerun の保存先が固定されている | Phase 未完了 |
| `customer_impact_result` | client visible error、retry guidance、data safety statement、release note、operator communication が固定されている | rollout ready 不可 |
| `incident_recovery_closure_result` | 上記 field がすべて `pass`。検出不能 incident、RTO/RPO 未定義、runbook 欠落、safe mode 不明、DR drill 未実施、post-incident evidence 欠落がすべて 0 件 | `pass` 以外は Phase 完了禁止 |

`incident_recovery_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。incident / DR / operator runbook に影響しない Phase でも `incident_recovery_closure_result = pass` とし、`incident_surface:unchanged`、`detection_signal:unchanged`、`rto_rpo:unchanged`、`operator_runbook:unchanged`、`dr_drill:not_applicable_with_reason`、および非対象理由を `closure_evidence` に記録する。これにより「運用復旧影響なし」の主張も、後続 Phase と reviewer が再現できる状態にする。

**canonical external contract versioning / deprecation / sunset closure audit：**

external contract versioning / deprecation / sunset は、Turso Cloud 互換 mode の API、wire schema、metadata、error、config、SDK client behavior を実装都合で破壊しないための compatibility gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`contract_versioning_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `contract_surface_inventory_result` | HTTP API、WebSocket wire、Platform API、Admin API、metadata schema、JWT claim、error code、config key、SDK transcript の変更有無が列挙されている | 実装開始禁止 |
| `versioning_policy_result` | 既存 version 維持、新 version 追加、mode 分離、feature flag、compat shim のどれで扱うかが固定されている | 実装開始禁止 |
| `deprecation_policy_result` | deprecated field / endpoint / config / error の有無、警告方法、継続期間、代替手段、Turso Cloud 追従理由が固定されている | merge 不可 |
| `sunset_policy_result` | sunset する場合の対象、条件、最短時期、operator notice、SDK/client impact、rollback 条件が固定されている | merge 不可 |
| `breaking_change_result` | Turso 互換 mode で breaking change が 0 件である。例外は security / durability 上必須で、自己ホスト差分として mode 分離されている | merge 不可 |
| `compat_mode_result` | 既定 mode が Turso 互換であり、Adlaire 拡張 mode、experimental flag、internal adapter 差分が既定 response に混入しない | 実装開始禁止 |
| `sdk_client_impact_result` | TypeScript/Rust/Go SDK、curl、existing client request / response snapshot への影響、migration notice、compat transcript が固定されている | Phase 未完了 |
| `metadata_error_config_contract_result` | metadata schema、error code、HTTP status、config key/default/precedence の変更有無と互換維持策が固定されている | 実装開始禁止 |
| `contract_migration_notice_result` | 利用者・運用者向け release note、migration guide、rollback note、deprecated/sunset notice が operator delta と接続されている | rollout ready 不可 |
| `contract_versioning_closure_result` | 上記 field がすべて `pass`。暗黙 breaking change、deprecated/sunset 未通知、SDK impact 未評価、default compat mode 破壊、metadata/error/config drift がすべて 0 件 | `pass` 以外は実装開始禁止 |

`contract_versioning_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。external contract に影響しない Phase でも `contract_versioning_closure_result = pass` とし、`contract_surface:unchanged`、`versioning_policy:unchanged`、`deprecation:none`、`sunset:none`、`breaking_change:none`、および非対象理由を `closure_evidence` に記録する。これにより「外部 contract 影響なし」の主張も、後続 Phase と reviewer が再現できる状態にする。

**canonical machine-readable contract / snapshot / artifact template closure audit：**

machine-readable contract / snapshot / artifact template は、本文の仕様判断を CI、reviewer、実装者が同じ input/output で再現するための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`machine_contract_artifact_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `openapi_contract_result` | HTTP / Admin / Platform / HA API の method、path、parameter、request、response、status、auth、error が OpenAPI または同等 machine-readable artifact に固定されている | 実装開始禁止 |
| `json_schema_contract_result` | metadata、config、JWT claim、manifest、Done receipt、artifact manifest の JSON Schema または同等 schema が保存先と hash 付きで固定されている | 実装開始禁止 |
| `wire_snapshot_contract_result` | hrana HTTP / WebSocket、SDK transcript、error body、legacy compatibility response の正規化 snapshot が artifact path と更新条件付きで固定されている | 実装開始禁止 |
| `turso_observed_snapshot_result` | Turso Cloud 互換を主張する surface は、観測元、観測日時、request、response、正規化 rule、差分理由が artifact として固定されている | 実装開始禁止 |
| `sdk_transcript_contract_result` | TypeScript/Rust/Go SDK または対象外理由が固定され、対象 SDK は command、version、request/response transcript、expected diff が保存されている | Phase 未完了 |
| `artifact_template_result` | Phase packet、受入 manifest、Done receipt、artifact manifest、review handoff の必須 field、未記入禁止 field、hash、secret scan、review command が template 化されている | 実装開始禁止 |
| `ci_machine_verification_result` | OpenAPI/schema/snapshot/transcript/template を検証する command が `not_run` ではなく、CI または reviewer replay で再実行できる | merge 不可 |
| `contract_drift_detection_result` | 本文、machine-readable contract、snapshot、artifact template の差分検出方法、許容差分、更新手順、drift fail 条件が固定されている | 実装開始禁止 |
| `reviewer_replay_contract_result` | clean checkout から reviewer が artifact を再生成または照合でき、local path、時刻、hostname、secret に依存しない | Phase 未完了 |
| `machine_contract_artifact_closure_result` | 上記 field がすべて `pass`。本文だけの契約、手作業だけの検証、snapshot 未固定、SDK 影響未評価、template 未接続、drift 検出不能がすべて 0 件 | `pass` 以外は実装開始禁止 |

`machine_contract_artifact_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。machine-readable artifact に影響しない Phase でも `machine_contract_artifact_closure_result = pass` とし、`openapi:unchanged`、`json_schema:unchanged`、`wire_snapshot:unchanged`、`turso_snapshot:not_applicable_with_reason`、`artifact_template:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「本文だけでは正しいが reviewer/CI が再現できない」状態を Phase 未完了として扱える。

**canonical phase execution sequence / stop condition / evidence handoff closure audit：**

phase execution sequence / stop condition / evidence handoff は、Phase 実装 PR が入口 gate、実装順序、途中変更、停止判断、証跡作成、出口判定を同じ順序で通過するための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`phase_execution_sequence_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `phase_entry_sequence_result` | `AGENTS.md` 読了、branch 整合、対象 Phase、spec version、readiness packet、blocking items 0 件、`ready_to_implement = true` の順序が固定されている | 実装開始禁止 |
| `pre_implementation_freeze_result` | 実装開始前に scope、Contract ID、Task ID、Scenario ID、artifact path、machine-readable contract、snapshot、review command が freeze されている | 実装開始禁止 |
| `task_order_result` | 原子タスク台帳の実行順、並列可能 task、先行 task、rollback point、検証 command が固定され、順序未定義 task が 0 件である | 実装開始禁止 |
| `stop_condition_result` | 仕様未定義、contract drift、test failure、artifact 欠落、security/compatibility 影響、unexpected success、unreviewed dependency が発生した場合の停止条件が固定されている | merge 不可 |
| `mid_phase_change_control_result` | 実装中に scope/API/schema/error/persistence/auth/config/operator behavior が変わる場合、先に仕様 PR へ戻す条件と再評価 field が固定されている | 実装継続禁止 |
| `evidence_handoff_sequence_result` | test、snapshot、fixture、log、manifest、secret scan、review handoff、Done receipt の生成順序と hash 接続順序が固定されている | Phase 未完了 |
| `phase_exit_sequence_result` | Done receipt、artifact manifest、review handoff、regression、operator delta、zero-bug declaration、merge readiness の出口順序が固定されている | Phase 完了扱い禁止 |
| `blocked_state_result` | blocked の定義、記録場所、再開に必要な仕様差分、残してよい artifact、破棄すべき artifact、merge 禁止条件が固定されている | merge 不可 |
| `resume_state_result` | 中断・再開時に再実行する fetch、spec version 確認、artifact freshness、test rerun、snapshot drift、approval 再確認が固定されている | 実装再開禁止 |
| `phase_execution_sequence_closure_result` | 上記 field がすべて `pass`。入口順序未定義、freeze なし実装、停止条件なし継続、証跡後付け、blocked/resume 不明、出口 gate 飛ばしがすべて 0 件 | `pass` 以外は実装開始禁止 |

`phase_execution_sequence_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。実装順序に影響しない仕様修正だけの Phase でも `phase_execution_sequence_closure_result = pass` とし、`entry_sequence:unchanged`、`task_order:unchanged`、`stop_condition:unchanged`、`handoff_sequence:unchanged`、`resume_state:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「最後にまとめて証跡を作る」「失敗したが続行する」「再開時に古い artifact を使う」状態を Phase 未完了として扱える。

**canonical verdict vocabulary / pass-fail normalization / reviewer decision closure audit：**

verdict vocabulary / pass-fail normalization / reviewer decision は、Phase の Done / Not Done / Blocked / N/A / pass / fail 判定を実装者、reviewer、CI、operator が同じ語彙で扱うための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`verdict_normalization_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `verdict_vocabulary_result` | 使用可能な判定語彙が `pass`、`fail`、`blocked`、`not_applicable_with_reason`、`not_run`、`manual_review_required`、`done`、`not_done` に限定されている | 実装開始禁止 |
| `pass_condition_result` | `pass` にできる条件が、実行済み command、保存済み artifact、仕様本文との一致、未接続 ID 0 件、根拠なし N/A 0 件として固定されている | 実装開始禁止 |
| `fail_condition_result` | `fail` にする条件が、test failure、contract drift、unexpected success、artifact mismatch、secret leak、unsupported success、schema/error/status 不一致として固定されている | merge 不可 |
| `blocked_condition_result` | `blocked` にできる条件、必要な blocking item、再開条件、merge 禁止、artifact freshness 再検証が固定されている | 実装継続禁止 |
| `not_applicable_condition_result` | `not_applicable_with_reason` は本文で対象外が明示された場合だけ許可し、理由、参照節、影響なし証跡、再評価 trigger が固定されている | 実装開始禁止 |
| `manual_review_condition_result` | `manual_review_required` は自動判定不能な operator/security/legal/release 判断だけに限定し、reviewer、判断基準、artifact、期限が固定されている | Phase 未完了 |
| `partial_result_prohibition_result` | `partial`、`mostly_pass`、`best_effort`、`works_locally`、`assumed_pass`、`later` を完了根拠として使わないことが固定されている | merge 不可 |
| `reviewer_decision_trace_result` | reviewer が Done / Not Done を選んだ根拠、参照 field、command、artifact hash、差戻し理由が review handoff に保存されている | Phase 未完了 |
| `done_not_done_mapping_result` | `done` は全 closure field pass、zero-bug 0 件、artifact fresh、merge readiness pass の場合だけ許可し、それ以外は `not_done` に写像される | Phase 完了扱い禁止 |
| `verdict_normalization_closure_result` | 上記 field がすべて `pass`。曖昧 verdict、根拠なし N/A、manual 判断の放置、partial 完了、reviewer 判断根拠欠落、Done/Not Done 写像漏れがすべて 0 件 | `pass` 以外は実装開始禁止 |

`verdict_normalization_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。判定語彙に影響しない Phase でも `verdict_normalization_closure_result = pass` とし、`verdict_vocabulary:unchanged`、`pass_condition:unchanged`、`fail_condition:unchanged`、`not_applicable:none_without_reason`、`partial_result:none`、および非対象理由を `closure_evidence` に記録する。これにより「ほぼ完了」「手元では pass」「あとで確認」「対象外のはず」といった曖昧な完了判定を禁止する。

**canonical release handoff / rollout decision / rollback evidence closure audit：**

release handoff / rollout decision / rollback evidence は、Phase 完了後に何を release candidate とし、どの条件で rollout し、どの証跡で rollback できるかを固定する closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`release_handoff_closure_result = pass` でなければ Phase 完了、rollout ready、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `release_candidate_inventory_result` | release 対象の binary、config、schema、artifact、snapshot、migration、operator document、対象外 artifact が列挙され、commit SHA と一致している | rollout ready 不可 |
| `rollout_decision_result` | rollout 可否、段階的 rollout 条件、停止条件、承認者、対象環境、rollout しない Phase の理由が固定されている | merge 不可 |
| `rollback_evidence_result` | rollback command、rollback artifact、復旧確認 command、data safety、config revert、feature flag revert、失敗時 escalation が固定されている | Phase 完了扱い禁止 |
| `operator_release_note_result` | operator 向け release note に user-visible change、compatibility delta、config/migration、monitoring、rollback、known blocker 0 件が記録されている | rollout ready 不可 |
| `compatibility_release_delta_result` | Turso Cloud 互換、SDK transcript、wire snapshot、legacy metadata、error/status、default mode の release 差分が 0 件または理由付きで固定されている | merge 不可 |
| `data_safety_release_result` | 永続化、migration、backup、restore、PITR、replication、HA、internal adapter の data safety 判定と必要 artifact が固定されている | Phase 完了扱い禁止 |
| `post_release_monitoring_result` | release 後に見る health、log、metric、error rate、replication lag、quota/storage、operator alert、rollback trigger が固定されている | rollout ready 不可 |
| `release_blocker_result` | 未解決 failure、flaky、manual review、security issue、compatibility drift、artifact stale、approval missing、PR conflict が 0 件である | merge 不可 |
| `release_handoff_trace_result` | Done receipt、artifact manifest、review handoff、operator note、release note、rollback guide の相互参照と hash が一致している | Phase 未完了 |
| `release_handoff_closure_result` | 上記 field がすべて `pass`。release candidate 不明、rollout 判断未定義、rollback 証跡欠落、operator note 欠落、post-release monitoring 未定義、release blocker 残存がすべて 0 件 | `pass` 以外は Phase 完了禁止 |

`release_handoff_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。release / rollout 対象外の Phase でも `release_handoff_closure_result = pass` とし、`release_candidate:none`、`rollout_decision:not_applicable_with_reason`、`rollback_evidence:unchanged`、`operator_release_note:no_user_visible_change`、`release_blocker:0`、および非対象理由を `closure_evidence` に記録する。これにより「merge はできたが運用投入・rollback 判断ができない」状態を Phase 未完了として扱える。

**canonical defect taxonomy / root cause / recurrence prevention closure audit：**

defect taxonomy / root cause / recurrence prevention は、実装中、review 中、release 後に見つかった defect を分類し、根本原因、修正範囲、再発防止、仕様へのフィードバックを閉じるための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`defect_prevention_closure_result = pass` でなければ Phase 完了、rollout ready、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `defect_taxonomy_result` | defect が spec gap、implementation bug、test gap、artifact drift、compatibility drift、security issue、operator issue、environment issue、release issue のいずれかに分類されている | Phase 未完了 |
| `severity_priority_result` | severity、priority、user impact、data safety impact、security impact、Turso compatibility impact、release blocker 判定が固定されている | merge 不可 |
| `root_cause_result` | 直接原因、根本原因、混入 Phase、検出できなかった理由、関連 Contract ID / Task ID / Scenario ID が記録されている | Phase 未完了 |
| `fix_scope_result` | 修正対象、修正しない対象、仕様改訂要否、rollback 要否、affected artifact、regression 範囲が固定されている | 実装継続禁止 |
| `regression_prevention_result` | defect を再発させない regression、snapshot、schema check、negative test、CI gate、reviewer command が追加または既存 artifact に接続されている | merge 不可 |
| `recurrence_test_result` | defect 再現手順、修正後 pass command、同種 defect の横展開 scan、future Phase への継承条件が固定されている | Phase 未完了 |
| `spec_feedback_result` | defect が仕様不足なら本文、Phase 表、oracle、scenario matrix、schema registry、readiness audit、Done receipt に反映されている | 実装開始禁止 |
| `defect_escape_analysis_result` | なぜ既存 gate を抜けたか、抜けた gate、追加する closure field、review handoff 更新、operator note 更新が記録されている | rollout ready 不可 |
| `known_defect_zero_result` | known defect、known flaky、unverified fix、unclassified failure、deferred defect、root cause unknown が 0 件である | merge 不可 |
| `defect_prevention_closure_result` | 上記 field がすべて `pass`。未分類 defect、原因不明 defect、再発防止なし修正、仕様 feedback なし修正、known defect 残存がすべて 0 件 | `pass` 以外は Phase 完了禁止 |

`defect_prevention_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。defect が発生していない Phase でも `defect_prevention_closure_result = pass` とし、`defect_count:0`、`known_defect_zero:true`、`regression_prevention:unchanged`、`spec_feedback:not_required_with_reason`、`escape_analysis:not_applicable_with_reason`、および非対象理由を `closure_evidence` に記録する。これにより「バグは直したが分類・再発防止・仕様反映がない」状態を Phase 未完了として扱える。

**canonical artifact layout / schema file / snapshot storage closure audit：**

artifact layout / schema file / snapshot storage は、Phase ごとの schema、snapshot、transcript、manifest、review handoff を同じ保存先と命名で扱い、CI と reviewer が迷わず再生できるようにする closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`artifact_layout_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `artifact_root_layout_result` | `docs/phase-evidence/phase-{phase}/` と `tests/artifacts/phase-{phase}/` の責務、必須 directory、禁止 local path が固定されている | 実装開始禁止 |
| `phase_artifact_naming_result` | packet、manifest、Done receipt、review handoff、schema、snapshot、transcript、log、secret scan の file name が Phase 番号と Contract ID に接続されている | 実装開始禁止 |
| `schema_file_location_result` | OpenAPI、JSON Schema、metadata/config/JWT/artifact schema の保存先、version、hash、更新理由が artifact manifest に接続されている | 実装開始禁止 |
| `snapshot_storage_result` | HTTP/WS/wire/error/metadata/compatibility snapshot の保存先、正規化 rule、更新条件、expected diff が固定されている | Phase 未完了 |
| `transcript_storage_result` | SDK、curl、CLI、operator command transcript の保存先、command、tool version、env redaction、再実行方法が固定されている | Phase 未完了 |
| `hash_manifest_result` | artifact manifest が全 artifact の relative path、sha256、生成 command、生成 commit、secret scan result、staleness 判定を持つ | merge 不可 |
| `artifact_update_policy_result` | artifact 更新時の許可条件、同時更新すべき本文、reviewer reapproval、snapshot drift 判定、古い artifact の扱いが固定されている | 実装開始禁止 |
| `artifact_retention_result` | 保持する artifact、再生成可能 artifact、削除禁止 artifact、release 後保持期間、private/secret artifact の扱いが固定されている | merge 不可 |
| `artifact_replay_path_result` | clean checkout から artifact を再生成または照合する command、相対 path、出力先、失敗時分類が固定されている | Phase 未完了 |
| `artifact_layout_closure_result` | 上記 field がすべて `pass`。保存先不明、命名揺れ、hash 欠落、snapshot 更新条件なし、local path 依存、再生不能 artifact がすべて 0 件 | `pass` 以外は実装開始禁止 |

`artifact_layout_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。artifact layout に影響しない Phase でも `artifact_layout_closure_result = pass` とし、`artifact_root:unchanged`、`schema_location:unchanged`、`snapshot_storage:unchanged`、`hash_manifest:unchanged`、`replay_path:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「artifact はあるが保存先・命名・hash・再生方法が揺れて reviewer が判定できない」状態を Phase 未完了として扱える。

**canonical upstream observation / Turso snapshot refresh / compatibility drift closure audit：**

upstream observation / Turso snapshot refresh / compatibility drift は、Turso Cloud 互換を主張する surface について、観測対象、取得 command、取得周期、snapshot 更新判断、自己ホスト差分を固定する closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`upstream_observation_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `upstream_surface_inventory_result` | 観測対象の Turso Cloud API、hrana behavior、Platform API、metadata、error/status、SDK behavior、対象外 surface が列挙されている | 実装開始禁止 |
| `observation_command_result` | curl/SDK/CLI などの観測 command、request body、auth redaction、tool version、保存先、失敗時分類が固定されている | 実装開始禁止 |
| `observation_cadence_result` | 観測周期、Phase 実装前の再観測条件、release 前の再観測条件、upstream changelog 追従条件が固定されている | merge 不可 |
| `snapshot_normalization_result` | timestamp、request id、region、account id、token、hostname、順序揺れ、rate-limit header などの正規化 rule が固定されている | Phase 未完了 |
| `compatibility_drift_result` | Adlaire と Turso Cloud の差分検出方法、許容差分、非許容差分、snapshot diff artifact、reviewer command が固定されている | 実装開始禁止 |
| `drift_classification_result` | drift を upstream change、self-host intentional delta、Adlaire bug、test artifact drift、unknown に分類し、unknown drift を merge 禁止にする | merge 不可 |
| `refresh_decision_result` | snapshot 更新可否、仕様本文更新要否、artifact 更新要否、reviewer reapproval、旧 snapshot 保持理由が固定されている | 実装開始禁止 |
| `self_host_delta_result` | 自己ホスト固有差分の理由、mode boundary、default Turso compatibility への非混入、operator notice、test artifact が固定されている | Phase 未完了 |
| `upstream_reference_trace_result` | 観測日時、upstream endpoint、SDK version、snapshot path、hash、差分理由、関連 Contract ID が artifact manifest に接続されている | Phase 未完了 |
| `upstream_observation_closure_result` | 上記 field がすべて `pass`。観測対象漏れ、command 不明、取得周期未定義、正規化なし snapshot、unknown drift、自己ホスト差分混入がすべて 0 件 | `pass` 以外は実装開始禁止 |

`upstream_observation_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。Turso Cloud 観測に影響しない Phase でも `upstream_observation_closure_result = pass` とし、`upstream_surface:unchanged`、`observation_command:unchanged`、`cadence:unchanged`、`compatibility_drift:none`、`self_host_delta:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「Turso Cloud 互換のはずだが、いつ何を観測したか分からない」状態を Phase 未完了として扱える。

**canonical ownership / approval authority / review escalation closure audit：**

ownership / approval authority / review escalation は、Phase の責任者、reviewer、承認権限、差戻し条件、再承認 trigger を固定し、仕様・実装・証跡・release 判断の責任境界を曖昧にしないための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`ownership_approval_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `phase_owner_result` | 対象 Phase の owner、責任範囲、代理条件、owner 不在時の blocked 条件が固定されている | 実装開始禁止 |
| `reviewer_role_result` | spec reviewer、implementation reviewer、security reviewer、operator reviewer、compatibility reviewer の要否と判定範囲が固定されている | 実装開始禁止 |
| `approval_authority_result` | `承認`、spec approval、artifact approval、release approval、operator acceptance のどれが必要か、誰が承認できるかが固定されている | merge 不可 |
| `escalation_path_result` | scope conflict、security issue、Turso drift、data safety、release blocker、owner 不在、review disagreement の escalate 先と停止条件が固定されている | 実装継続禁止 |
| `reapproval_trigger_result` | scope/API/schema/error/persistence/auth/config/artifact/snapshot/rollback/release note が変わった時の再承認条件が固定されている | merge 不可 |
| `change_authority_boundary_result` | 実装者が変更できる範囲、仕様改訂 PR に戻す範囲、reviewer が差戻す範囲、operator 判断が必要な範囲が固定されている | 実装開始禁止 |
| `merge_authority_result` | merge 可否の判断主体、必須 pass field、CLEAN branch 条件、unreviewed change 禁止、unknown merge state 禁止が固定されている | merge 不可 |
| `operator_acceptance_authority_result` | operator-visible change、migration/config、rollback、release note、monitoring の受入判断と差戻し条件が固定されている | rollout ready 不可 |
| `audit_signoff_trace_result` | 承認者、承認対象、commit SHA、artifact hash、review command、差戻し履歴が review handoff または Done receipt に接続されている | Phase 未完了 |
| `ownership_approval_closure_result` | 上記 field がすべて `pass`。owner 不明、reviewer 不明、承認権限不明、再承認漏れ、差戻し先不明、signoff trace 欠落がすべて 0 件 | `pass` 以外は実装開始禁止 |

`ownership_approval_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。ownership / approval に影響しない Phase でも `ownership_approval_closure_result = pass` とし、`phase_owner:unchanged`、`reviewer_role:unchanged`、`approval_authority:unchanged`、`reapproval_trigger:unchanged`、`audit_signoff_trace:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「誰が承認したか分からない」「差戻し権限が曖昧」「変更後の再承認がない」状態を Phase 未完了として扱える。

**canonical operational readiness / SLO / alert threshold / capacity acceptance closure audit：**

operational readiness / SLO / alert threshold / capacity acceptance は、Phase 実装後に運用可能と判定できる水準、監視信号、容量上限、劣化時挙動、runbook を固定し、動作はするが運用基準が未確定の状態を防ぐための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`operational_readiness_closure_result = pass` でなければ実装開始、Phase 完了、または rollout ready に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `slo_target_result` | 対象 Phase の latency、availability、error rate、replication lag、backup freshness、recovery time のうち該当する SLO target と非対象理由が固定されている | 実装開始禁止 |
| `alert_threshold_result` | warn / critical threshold、連続発生回数、通知先、suppression 条件、alert 解除条件が固定されている | 実装開始禁止 |
| `health_signal_result` | health endpoint、metric、log field、trace、artifact check のどれで正常性を判定するかが固定され、成功/失敗判定が oracle に接続されている | 実装開始禁止 |
| `capacity_acceptance_result` | DB 数、接続数、同時 query、WAL/backup size、snapshot 数、branch 数、storage 使用量の該当上限と受入 command が固定されている | 実装開始禁止 |
| `monitoring_dashboard_result` | operator が確認する dashboard、metric 名、表示単位、refresh 間隔、欠損時の扱いが固定されている | rollout ready 不可 |
| `degradation_policy_result` | overload、upstream drift、replication delay、backup failure、storage pressure、auth failure 時の degraded response、retry、read-only、503/429、rollback 条件が固定されている | Phase 未完了 |
| `oncall_runbook_result` | alert 受信後の一次切り分け、確認 command、artifact path、rollback 手順、escalation 先、復旧完了条件が固定されている | rollout ready 不可 |
| `operator_acknowledgement_result` | operator が SLO、alert、capacity、degradation、runbook、release note を確認した証跡が review handoff または Done receipt に接続されている | Phase 未完了 |
| `operational_exception_result` | 運用対象外にする signal、SLO、capacity 項目がある場合、対象外理由、代替検知、期限、再評価 trigger が固定されている | 実装開始禁止 |
| `operational_readiness_closure_result` | 上記 field がすべて `pass`。SLO 未定義、alert 未定義、health signal 不明、capacity 未測定、dashboard 未接続、runbook 欠落、operator 未確認、根拠なし対象外がすべて 0 件 | `pass` 以外は実装開始禁止 |

`operational_readiness_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。運用影響がない Phase でも `operational_readiness_closure_result = pass` とし、`slo_target:unchanged`、`alert_threshold:unchanged`、`health_signal:unchanged`、`capacity_acceptance:unchanged`、`degradation_policy:unchanged`、`operator_acknowledgement:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「監視していない」「容量上限が分からない」「劣化時に何を返すか未定」「runbook がない」状態を Phase 未完了として扱える。

**canonical CI command matrix / required verification command / shard determinism closure audit：**

CI command matrix / required verification command / shard determinism は、Phase ごとに実行必須 command、順序、CI job、local replay、timeout、cache、failure artifact、rerun 条件を固定し、実装者や CI 環境ごとの検証差分を防ぐための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`ci_command_matrix_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `required_command_inventory_result` | 対象 Phase の format、lint、unit、integration、contract、snapshot、compatibility、security、performance、migration、release check のうち必須 command と非対象理由が固定されている | 実装開始禁止 |
| `command_execution_order_result` | command の実行順、前提 artifact、停止条件、後続 command の skip 条件が固定されている | 実装開始禁止 |
| `ci_shard_mapping_result` | CI job 名、shard 名、対象 command、対象 OS/runtime、必須/任意区分、parallel 実行可否が固定されている | merge 不可 |
| `local_replay_command_result` | reviewer がローカルで再実行できる command、環境変数、fixture、seed、artifact 出力先、期待 exit code が固定されている | Phase 未完了 |
| `timeout_budget_result` | command ごとの timeout、slow test 条件、timeout 時の判定、再実行上限、性能劣化時の fail 条件が固定されている | merge 不可 |
| `dependency_cache_result` | dependency cache key、lockfile 依存、cache miss 時の扱い、cache 更新 trigger、poisoned cache 検出条件が固定されている | 実装開始禁止 |
| `failure_artifact_result` | 失敗時に保存する log、snapshot diff、coverage、trace、test report、secret scan report、CI URL の artifact path が固定されている | Phase 未完了 |
| `rerun_policy_result` | flaky 判定、rerun 許可回数、rerun 後も必要な artifact、manual rerun 禁止条件、known failure 化の禁止条件が固定されている | merge 不可 |
| `command_drift_detection_result` | readiness packet、CI workflow、review handoff、Done receipt の command list 差分を検出する方法と差分時の停止条件が固定されている | 実装開始禁止 |
| `ci_command_matrix_closure_result` | 上記 field がすべて `pass`。必須 command 漏れ、順序不明、CI job 未接続、local replay 不可、timeout 未定義、cache drift、failure artifact 欠落、rerun 根拠なし、command drift がすべて 0 件 | `pass` 以外は実装開始禁止 |

`ci_command_matrix_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。CI または検証 command に影響しない Phase でも `ci_command_matrix_closure_result = pass` とし、`required_command_inventory:unchanged`、`command_execution_order:unchanged`、`ci_shard_mapping:unchanged`、`local_replay_command:unchanged`、`failure_artifact:unchanged`、`command_drift_detection:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「どの command を通せば完了か分からない」「CI とローカル検証が違う」「失敗 artifact が残らない」状態を Phase 未完了として扱える。

**canonical lifecycle state / transition matrix / data invariant closure audit：**

lifecycle state / transition matrix / data invariant は、Phase ごとの状態、許可遷移、禁止遷移、永続化・冪等性・並行性・復旧・rollback の不変条件を固定し、DB lifecycle、branch、backup、restore、replication、adapter 切替で状態整合性バグを出さないための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`state_invariant_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `lifecycle_state_inventory_result` | 対象 Phase の resource state、DB state、branch state、backup state、replication state、adapter state、failure state のうち該当状態と非対象理由が固定されている | 実装開始禁止 |
| `allowed_transition_matrix_result` | 作成、更新、削除、停止、再開、branch、restore、promote、rollback、adapter 切替のうち許可される遷移、前提条件、成功後 state が固定されている | 実装開始禁止 |
| `forbidden_transition_matrix_result` | 禁止遷移、禁止理由、返す error code、永続化変更の有無、audit log の有無が固定されている | 実装開始禁止 |
| `persistence_invariant_result` | disk metadata、SQLite/libSQL file、WAL、backup、snapshot、branch metadata、tenant/location metadata の整合条件と破損検出条件が固定されている | merge 不可 |
| `idempotency_invariant_result` | retry、duplicate request、partial success、network timeout、client reconnect 時に同一結果または安全な conflict を返す条件が固定されている | Phase 未完了 |
| `concurrency_invariant_result` | concurrent create/delete/query/backup/restore/replication/adapter switch の lock、serialization、conflict、lost update 防止条件が固定されている | merge 不可 |
| `recovery_invariant_result` | crash、process restart、disk full、partial write、corrupt metadata、replication interruption 後に維持すべき state と復旧 command が固定されている | Phase 未完了 |
| `rollback_invariant_result` | rollback 後に残してよい artifact、戻す state、戻してはならない external effect、再実行可否、operator confirmation が固定されている | rollout ready 不可 |
| `invariant_violation_handling_result` | invariant 違反検出時の error、log、metric、quarantine、read-only 化、repair 禁止/許可条件、escalation が固定されている | 実装開始禁止 |
| `state_invariant_closure_result` | 上記 field がすべて `pass`。状態未定義、許可/禁止遷移不明、永続化 invariant 欠落、冪等性不明、並行性不明、復旧/rollback invariant 欠落、違反時処理未定義がすべて 0 件 | `pass` 以外は実装開始禁止 |

`state_invariant_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。状態遷移または永続化 invariant に影響しない Phase でも `state_invariant_closure_result = pass` とし、`lifecycle_state_inventory:unchanged`、`allowed_transition_matrix:unchanged`、`forbidden_transition_matrix:unchanged`、`persistence_invariant:unchanged`、`concurrency_invariant:unchanged`、`recovery_invariant:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「この状態から進めてよいか分からない」「禁止遷移なのに永続化された」「rollback 後の state が不明」な状態を Phase 未完了として扱える。

**canonical compatibility delta / self-host variance / rebaseline closure audit：**

compatibility delta / self-host variance / rebaseline は、Turso Cloud 互換を優先する方針のもと、観測した upstream behavior と自己ホスト固有差分を分離し、どの差分を互換バグ、どの差分を承認済み自己ホスト仕様として扱うかを固定するための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`compatibility_delta_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `upstream_behavior_baseline_result` | 対象 API / CLI / SDK / persistence / operator behavior の Turso Cloud 観測 snapshot、観測日、commit、SDK version、normalized transcript が固定されている | 実装開始禁止 |
| `self_host_delta_inventory_result` | 自己ホストで差分が出る API、error、persistence、side effect、latency、operator behavior、resource limit、location/organization/quota behavior が列挙されている | 実装開始禁止 |
| `compatible_delta_classification_result` | Turso Cloud と完全一致、許容差分、自己ホスト固有仕様、未来追従、互換バグの分類と判定理由が固定されている | merge 不可 |
| `api_response_delta_result` | status code、header、body schema、pagination、ordering、default value、nullability、id format の差分が snapshot と schema に接続されている | 実装開始禁止 |
| `error_delta_result` | error code、message、HTTP status、retryability、SDK exception mapping、operator log の差分と許容可否が固定されている | merge 不可 |
| `persistence_side_effect_delta_result` | file layout、metadata、WAL、backup、branch、replication、audit log、副作用なし失敗の差分と互換判定が固定されている | Phase 未完了 |
| `sdk_compatibility_delta_result` | TypeScript / Rust / Go など対象 SDK の observed transcript、client-visible behavior、unsupported mapping、retry behavior の差分が固定されている | merge 不可 |
| `operator_visible_delta_result` | CLI/API 表示、health、metric、log、dashboard、release note、migration/config に現れる差分と運用者説明が固定されている | rollout ready 不可 |
| `delta_approval_rebaseline_result` | 許容差分の承認者、再観測 trigger、Turso Cloud 変更時の rebaseline 条件、互換バグへの格上げ条件が固定されている | 実装開始禁止 |
| `compatibility_delta_closure_result` | 上記 field がすべて `pass`。upstream baseline 不明、自己ホスト差分未列挙、互換/非互換分類不明、API/error/persistence/SDK/operator 差分未接続、rebaseline 条件欠落がすべて 0 件 | `pass` 以外は実装開始禁止 |

`compatibility_delta_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。Turso Cloud 互換差分に影響しない Phase でも `compatibility_delta_closure_result = pass` とし、`upstream_behavior_baseline:unchanged`、`self_host_delta_inventory:unchanged`、`compatible_delta_classification:unchanged`、`api_response_delta:unchanged`、`error_delta:unchanged`、`sdk_compatibility_delta:unchanged`、`delta_approval_rebaseline:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「Turso Cloud と違うが許容差分か不明」「自己ホスト仕様なのか互換バグなのか不明」「upstream 変更時に再基準化されない」状態を Phase 未完了として扱える。

**canonical artifact / contract / schema / snapshot synchronization closure audit：**

artifact / contract / schema / snapshot synchronization は、仕様本文で定義した contract、schema、snapshot、readiness packet、Done receipt が実物 artifact と一致し、古い生成物や未再生成 snapshot を Phase 実装へ持ち込まないための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`artifact_contract_sync_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `contract_artifact_inventory_result` | 対象 Phase の OpenAPI、wire contract、CLI contract、SDK contract、persistence contract、operator contract の artifact path、owner、更新 trigger が固定されている | 実装開始禁止 |
| `schema_artifact_inventory_result` | JSON Schema、manifest schema、readiness packet schema、Done receipt schema、snapshot schema、artifact manifest schema の path、version、hash が固定されている | 実装開始禁止 |
| `snapshot_artifact_inventory_result` | Turso observed snapshot、self-host snapshot、SDK transcript、error snapshot、wire transcript、migration snapshot の path、生成 command、生成 commit が固定されている | 実装開始禁止 |
| `readiness_packet_template_sync_result` | readiness packet template の required key、audit_results、Contract ID、Task ID、Scenario ID、artifact path が仕様本文と一致している | merge 不可 |
| `done_receipt_template_sync_result` | Done receipt template の required key、version_result、verification result、artifact hash、known defect zero、reviewer decision が仕様本文と一致している | Phase 未完了 |
| `artifact_hash_commit_sync_result` | 各 artifact の hash、生成 commit、生成 tool version、入力 snapshot、生成日時、更新理由が artifact manifest に接続されている | merge 不可 |
| `stale_artifact_detection_result` | 仕様 version、Contract ID、Task ID、Scenario ID、schema version、snapshot version が古い artifact を検出し、検出時に実装開始を止める条件が固定されている | 実装開始禁止 |
| `machine_validation_command_result` | artifact と仕様本文の一致を検証する machine-readable command、期待 exit code、出力 artifact、reviewer replay 方法が固定されている | 実装開始禁止 |
| `artifact_regeneration_trigger_result` | API/schema/error/persistence/auth/compatibility/Phase gate/CI command/ownership/operation/SLO/invariant が変わった時の artifact 再生成 trigger が固定されている | merge 不可 |
| `artifact_contract_sync_closure_result` | 上記 field がすべて `pass`。contract artifact 未接続、schema path 未定義、snapshot 未生成、template drift、hash/commit 不一致、stale artifact、validation command 欠落、再生成 trigger 欠落がすべて 0 件 | `pass` 以外は実装開始禁止 |

`artifact_contract_sync_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。artifact / contract / schema / snapshot に影響しない Phase でも `artifact_contract_sync_closure_result = pass` とし、`contract_artifact_inventory:unchanged`、`schema_artifact_inventory:unchanged`、`snapshot_artifact_inventory:unchanged`、`readiness_packet_template_sync:unchanged`、`done_receipt_template_sync:unchanged`、`stale_artifact_detection:unchanged`、`machine_validation_command:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「仕様本文は更新済みだが artifact が古い」「schema と template がズレている」「snapshot の生成 commit が不明」な状態を Phase 未完了として扱える。

**canonical reviewer checklist / evidence order / approval-rejection closure audit：**

reviewer checklist / evidence order / approval-rejection は、Phase reviewer が何を、どの順番で、どの command と artifact に基づいて合否判定するかを固定し、レビュー判断が属人化して defect escape を許す状態を防ぐための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`review_checklist_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `reviewer_checklist_inventory_result` | 対象 Phase の spec、API、schema、persistence、security、compatibility、operator、artifact、CI、release の reviewer checklist が固定されている | 実装開始禁止 |
| `review_evidence_order_result` | reviewer が確認する順序、前提 artifact、停止条件、再確認条件が fixed order として固定されている | 実装開始禁止 |
| `required_reviewer_command_result` | reviewer が再実行する command、期待 exit code、必要な環境変数、artifact output、実行不能時の blocked 条件が固定されている | merge 不可 |
| `blocking_finding_taxonomy_result` | merge blocker、Phase incomplete、implementation start forbidden、security blocker、compatibility blocker、data safety blocker の分類と例が固定されている | merge 不可 |
| `non_blocking_finding_taxonomy_result` | follow-up 可、documentation-only、future phase、operator note、known limitation の分類、期限、昇格条件が固定されている | Phase 未完了 |
| `reviewer_replay_scope_result` | reviewer が再現すべき scenario、snapshot、SDK transcript、failure artifact、manual review の範囲が固定されている | merge 不可 |
| `approval_checklist_result` | approve 可能な最小条件、必要な pass field、artifact hash、PR state、branch cleanliness、signoff trace が固定されている | merge 不可 |
| `rejection_checklist_result` | reject / request changes / blocked にする条件、差戻し先、再承認 trigger、再提出時の required evidence が固定されている | merge 不可 |
| `checklist_drift_detection_result` | readiness packet、review handoff、Done receipt、PR description、CI workflow の checklist 差分を検出する方法と差分時の停止条件が固定されている | 実装開始禁止 |
| `review_checklist_closure_result` | 上記 field がすべて `pass`。checklist 欠落、evidence order 不明、reviewer command 欠落、blocking/non-blocking 分類不明、replay scope 不明、approve/reject 条件不明、checklist drift がすべて 0 件 | `pass` 以外は実装開始禁止 |

`review_checklist_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。review checklist に影響しない Phase でも `review_checklist_closure_result = pass` とし、`reviewer_checklist_inventory:unchanged`、`review_evidence_order:unchanged`、`required_reviewer_command:unchanged`、`blocking_finding_taxonomy:unchanged`、`reviewer_replay_scope:unchanged`、`approval_checklist:unchanged`、`checklist_drift_detection:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「何を見れば承認できるか不明」「差戻し条件が reviewer ごとに違う」「review handoff と Done receipt の checklist がズレている」状態を Phase 未完了として扱える。

**canonical post-merge / post-release verification / defect watch closure audit：**

post-merge / post-release verification / defect watch は、Phase が merge または release された後も Done 判定を維持できるかを確認し、実装直後だけ pass して後から artifact、CI、互換観測、運用 signal が崩れる状態を防ぐための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`post_merge_verification_closure_result = pass` でなければ Phase 完了、release ready、または Done 維持に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `post_merge_verification_scope_result` | merge 後に再確認する API、schema、artifact、CI、compatibility、operator、release note、rollback の範囲と非対象理由が固定されている | Phase 完了扱い禁止 |
| `post_release_verification_command_result` | release 後に実行する smoke、contract、compatibility、artifact availability、monitoring check の command、期待 exit code、artifact path が固定されている | release ready 不可 |
| `deployment_smoke_test_result` | deploy 後の health、admin API、hrana endpoint、auth、DB lifecycle、backup/restore、replication、internal adapter の smoke 判定が固定されている | release ready 不可 |
| `compatibility_recheck_result` | Turso Cloud baseline、SDK transcript、wire snapshot、error snapshot、self-host delta の再確認条件と drift 時の停止条件が固定されている | Phase 未完了 |
| `artifact_availability_check_result` | Done receipt、readiness packet、snapshot、schema、CI log、release note、rollback evidence が merge 後も参照可能で hash 一致する | Phase 未完了 |
| `monitoring_signal_check_result` | post-release の health、metric、log、alert、dashboard、SLO signal の確認期間、pass 条件、missing signal 時の扱いが固定されている | rollout ready 不可 |
| `rollback_readiness_recheck_result` | rollback command、rollback artifact、operator confirmation、data safety、compatibility fallback の再確認条件が固定されている | release ready 不可 |
| `defect_watch_window_result` | merge/release 後の watch window、known defect 判定、defect escalation、hotfix 禁止/許可条件、Done 取り消し条件が固定されている | Phase 完了扱い禁止 |
| `post_merge_failure_escalation_result` | post-merge verification 失敗時の owner、escalation、revert/rollback、spec feedback、artifact regeneration、reapproval trigger が固定されている | merge 後 blocked |
| `post_merge_verification_closure_result` | 上記 field がすべて `pass`。post-merge scope 不明、release command 欠落、smoke 未定義、compatibility recheck 欠落、artifact 参照不能、monitoring signal 欠落、rollback 再確認なし、watch window 不明、失敗時 escalation 不明がすべて 0 件 | `pass` 以外は Phase 完了扱い禁止 |

`post_merge_verification_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。post-merge / post-release verification に影響しない Phase でも `post_merge_verification_closure_result = pass` とし、`post_merge_verification_scope:unchanged`、`post_release_verification_command:unchanged`、`deployment_smoke_test:unchanged`、`compatibility_recheck:unchanged`、`artifact_availability_check:unchanged`、`monitoring_signal_check:unchanged`、`rollback_readiness_recheck:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「merge 後に確認されない」「release 後に監視 signal が見られていない」「post-merge failure の差戻し先が不明」な状態を Phase 未完了として扱える。

**canonical exception / deferral / known limitation closure audit：**

exception / deferral / known limitation は、Phase ごとの仕様例外、延期、既知制限を Done と混同せず、許可条件、期限、owner、再評価 trigger、削除条件を固定するための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`exception_deferral_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `exception_inventory_result` | 対象 Phase の仕様例外、明示対象外、既知制限、延期項目、manual review 項目が Contract ID / Task ID / Scenario ID と接続されている | 実装開始禁止 |
| `deferral_reason_result` | 各延期の理由、依存 Phase、依存 artifact、依存 upstream behavior、代替検証、未対応時のリスクが固定されている | 実装開始禁止 |
| `deferral_owner_result` | 各例外/延期/既知制限の owner、reviewer、承認者、代理条件、owner 不在時の blocked 条件が固定されている | merge 不可 |
| `expiry_revisit_trigger_result` | 期限、再評価 trigger、Turso Cloud drift、Phase transition、artifact regeneration、release、incident 時の再確認条件が固定されている | merge 不可 |
| `known_limitation_classification_result` | known limitation、unsupported、future phase、temporary deferral、self-host accepted variance、compatibility bug の分類が固定されている | Phase 未完了 |
| `done_impact_classification_result` | 各例外が readiness、implementation start、Phase Done、release ready、rollout、post-merge verification に与える影響が固定されている | Phase 完了扱い禁止 |
| `unsupported_future_phase_distinction_result` | unsupported と future phase の差、成功応答禁止、501/409/422 などの扱い、future success response 禁止条件が固定されている | 実装開始禁止 |
| `exception_approval_result` | 例外/延期の承認者、承認対象、承認 commit、artifact hash、review handoff、再承認 trigger が固定されている | merge 不可 |
| `exception_removal_condition_result` | 例外を削除できる条件、必要 artifact、regression、snapshot、release note、Done receipt 更新、削除漏れ検出条件が固定されている | Phase 未完了 |
| `exception_deferral_closure_result` | 上記 field がすべて `pass`。例外未列挙、延期理由不明、owner 不明、期限/再評価なし、分類不明、Done 影響不明、unsupported/future 混同、承認欠落、削除条件欠落がすべて 0 件 | `pass` 以外は実装開始禁止 |

`exception_deferral_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。例外、延期、既知制限がない Phase でも `exception_deferral_closure_result = pass` とし、`exception_inventory:none`、`deferral_reason:none`、`known_limitation_classification:none`、`done_impact_classification:none`、`exception_approval:none_required`、`exception_removal_condition:none`、および非対象理由を `closure_evidence` に記録する。これにより「未実装を延期扱いにしただけ」「既知制限なのに Done として扱った」「unsupported と future phase が混ざった」状態を Phase 未完了として扱える。

**canonical migration / upgrade / downgrade compatibility closure audit：**

migration / upgrade / downgrade compatibility は、metadata、snapshot、backup、WAL、internal adapter、schema version の変更時に forward migration、downgrade policy、復旧可能性、互換性を固定し、DB 破損、復旧不可、古い artifact との非互換を防ぐための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`migration_compatibility_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `migration_inventory_result` | 対象 Phase の metadata、schema、snapshot、backup、WAL、branch、replication、adapter、config の migration 有無と非対象理由が固定されている | 実装開始禁止 |
| `forward_migration_contract_result` | 旧 version から新 version への入力、出力、idempotency、partial success、再実行、成功 artifact が固定されている | 実装開始禁止 |
| `backward_downgrade_policy_result` | downgrade 可否、禁止理由、compatibility fallback、read-only fallback、operator warning、unsupported downgrade error が固定されている | merge 不可 |
| `metadata_version_mapping_result` | metadata version、schema version、manifest version、snapshot version、adapter version、変換 rule、default value が固定されている | merge 不可 |
| `backup_restore_compatibility_result` | 旧 backup の restore、新 backup の restore、cross-version restore、restore 後 integrity check、restore 失敗時の rollback が固定されている | Phase 未完了 |
| `wal_snapshot_compatibility_result` | WAL、snapshot、checkpoint、replication cursor、branch snapshot の互換条件、再生成条件、破損検出条件が固定されている | Phase 未完了 |
| `zero_downtime_maintenance_policy_result` | zero-downtime 可否、maintenance mode、write freeze、read-only、connection drain、operator notice の条件が固定されている | rollout ready 不可 |
| `migration_failure_handling_result` | migration 中断、disk full、partial write、lock timeout、corrupt input、version mismatch 時の停止、repair 禁止/許可、escalation が固定されている | 実装開始禁止 |
| `migration_replay_rollback_evidence_result` | migration replay command、rollback evidence、before/after artifact hash、integrity check、audit log、reviewer replay 方法が固定されている | Phase 未完了 |
| `migration_compatibility_closure_result` | 上記 field がすべて `pass`。migration 未列挙、forward contract 不明、downgrade 方針不明、version mapping 欠落、backup/restore 非互換、WAL/snapshot 非互換、maintenance 方針欠落、失敗時処理不明、rollback evidence 欠落がすべて 0 件 | `pass` 以外は実装開始禁止 |

`migration_compatibility_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。migration、upgrade、downgrade に影響しない Phase でも `migration_compatibility_closure_result = pass` とし、`migration_inventory:none`、`forward_migration_contract:none`、`backward_downgrade_policy:unchanged`、`metadata_version_mapping:unchanged`、`backup_restore_compatibility:unchanged`、`wal_snapshot_compatibility:unchanged`、`migration_replay_rollback_evidence:none_required`、および非対象理由を `closure_evidence` に記録する。これにより「metadata version が変わったが移行仕様がない」「backup は取れるが restore 互換が不明」「downgrade 不可なのに operator notice がない」状態を Phase 未完了として扱える。

**canonical configuration / secret / credential / token closure audit：**

configuration / secret / credential / token は、Phase ごとの設定 key、default、必須値、禁止値、環境 override、secret、credential、token scope、rotation、redaction、drift 検出を固定し、環境依存バグ、secret leak、不安全 default、権限過大 token を防ぐための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`configuration_secret_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `configuration_key_inventory_result` | 対象 Phase の env var、config file key、CLI flag、admin API setting、runtime default、operator override が列挙されている | 実装開始禁止 |
| `default_required_forbidden_value_result` | 各設定の default、required、forbidden、empty/null 扱い、型、範囲、単位、invalid value error が固定されている | 実装開始禁止 |
| `environment_override_policy_result` | dev/test/prod/self-host/CI/Docker/systemd での override 優先順位、禁止 override、drift 時の停止条件が固定されている | merge 不可 |
| `secret_inventory_result` | JWT secret、API token、database credential、backup credential、replication credential、TLS key、webhook secret の有無と保存場所が固定されている | 実装開始禁止 |
| `secret_redaction_policy_result` | log、metric、artifact、snapshot、panic、error response、review handoff、Done receipt で redaction する field と検証 command が固定されている | merge 不可 |
| `credential_rotation_policy_result` | secret / credential の rotation 手順、dual-read/write 可否、失効条件、rollback、operator notice、artifact evidence が固定されている | rollout ready 不可 |
| `token_scope_expiry_policy_result` | token scope、audience、issuer、expiry、refresh、revocation、least privilege、SDK-visible error が固定されている | 実装開始禁止 |
| `configuration_drift_detection_result` | readiness packet、config schema、runtime default、CI env、Docker env、release note の設定差分を検出する方法と停止条件が固定されている | 実装開始禁止 |
| `insecure_configuration_failure_handling_result` | insecure default、missing secret、weak token、overbroad scope、forbidden override、unredacted secret 検出時の error、log、metric、startup refusal が固定されている | merge 不可 |
| `configuration_secret_closure_result` | 上記 field がすべて `pass`。設定 key 未列挙、default 不明、override 優先順位不明、secret 未分類、redaction 欠落、rotation 方針なし、token scope/expiry 不明、config drift、insecure config 処理不明がすべて 0 件 | `pass` 以外は実装開始禁止 |

`configuration_secret_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。configuration / secret / credential / token に影響しない Phase でも `configuration_secret_closure_result = pass` とし、`configuration_key_inventory:unchanged`、`default_required_forbidden_value:unchanged`、`environment_override_policy:unchanged`、`secret_inventory:unchanged`、`secret_redaction_policy:unchanged`、`credential_rotation_policy:unchanged`、`token_scope_expiry_policy:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「環境変数で動作が変わるが仕様にない」「secret が artifact に残る」「token scope が広すぎる」状態を Phase 未完了として扱える。

**canonical data retention / privacy / deletion closure audit：**

data retention / privacy / deletion は、Phase ごとの DB data、metadata、WAL、backup、snapshot、log、metric、artifact、review handoff に残る情報の分類、保持期間、削除条件、例外、機密/個人情報混入時の扱いを固定し、削除漏れ、保持過多、restore 不可能、privacy leak を防ぐための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`data_retention_privacy_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `data_classification_result` | 対象 Phase が扱う user data、database file、WAL、metadata、token-derived data、log、metric、artifact、fixture、snapshot の分類と保存場所が固定されている | 実装開始禁止 |
| `retention_policy_result` | 各 data class の保持期間、起算時刻、期限切れ判定、延長条件、Turso Cloud 互換差分、自己ホスト既定値が固定されている | 実装開始禁止 |
| `delete_policy_result` | API delete、operator delete、cleanup worker、retention expiry、rollback、partial failure、idempotency、delete protection の削除条件と禁止削除条件が固定されている | merge 不可 |
| `backup_retention_result` | backup / restore / PITR / branch の保持、削除、参照中 backup 保護、restore 可能範囲、削除後の operator-visible error が固定されている | Phase 未完了 |
| `snapshot_retention_result` | snapshot、fixture、compatibility baseline、Turso observation snapshot、test artifact の保存先、保持期間、再生成条件、削除禁止条件が固定されている | 実装開始禁止 |
| `log_retention_result` | access log、audit log、error log、metric snapshot、incident evidence の保持期間、redaction、rotation、削除、監査用保持例外が固定されている | rollout ready 不可 |
| `artifact_retention_privacy_result` | CI artifact、review handoff、Done receipt、transcript、hash manifest に含めてよい情報、secret / personal data 検出、削除または再生成手順が固定されている | merge 不可 |
| `personal_sensitive_data_result` | personal data、tenant data、credential、token、DB row sample、path、hostname、IP、email の扱い、masking、保存禁止 field、例外承認が固定されている | 実装開始禁止 |
| `retention_exception_result` | 法務/監査/incident/互換検証/rollback の保持例外、owner、期限、再評価 trigger、削除復帰条件が固定されている | Phase 未完了 |
| `data_retention_privacy_closure_result` | 上記 field がすべて `pass`。data 分類未列挙、保持期間不明、削除条件不明、backup/snapshot/log/artifact 保持不明、personal/sensitive data 方針なし、保持例外未承認がすべて 0 件 | `pass` 以外は実装開始禁止 |

`data_retention_privacy_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。data retention、privacy、delete semantics に影響しない Phase でも `data_retention_privacy_closure_result = pass` とし、`data_classification:unchanged`、`retention_policy:unchanged`、`delete_policy:unchanged`、`backup_retention:unchanged`、`snapshot_retention:unchanged`、`log_retention:unchanged`、`artifact_retention_privacy:unchanged`、`personal_sensitive_data:none_added`、および非対象理由を `closure_evidence` に記録する。これにより「DB は削除したが backup に残る」「artifact に個人/機密情報が残る」「retention cleanup と restore 可能範囲が矛盾する」「Turso Cloud 互換 API の delete と自己ホスト保持方針が衝突する」状態を Phase 未完了として扱える。

**canonical documentation / runbook / guide synchronization closure audit：**

documentation / runbook / guide synchronization は、仕様本文、Phase packet、Done receipt、API docs、operator runbook、migration guide、rollback guide、release note、troubleshooting が同じ挙動・制約・手順を指すことを固定し、古い文書、手順漏れ、利用者向け説明不足、運用誤操作を防ぐための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`documentation_runbook_closure_result = pass` でなければ実装開始、Phase 完了、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `documentation_inventory_result` | 対象 Phase に関係する仕様節、API docs、README、operator runbook、migration guide、rollback guide、release note、troubleshooting、schema comment が列挙されている | 実装開始禁止 |
| `api_documentation_sync_result` | method、path、request、response、status、error code、auth、quota、pagination、compatibility delta が仕様本文と API docs で一致している | merge 不可 |
| `operator_runbook_sync_result` | startup、shutdown、backup、restore、replication、HA、token rotation、incident、degraded state の operator 手順が仕様本文と一致している | rollout ready 不可 |
| `migration_guide_sync_result` | migration / upgrade / downgrade の前提、手順、停止条件、rollback、互換 window、operator notice が migration closure と一致している | Phase 未完了 |
| `rollback_guide_sync_result` | rollback trigger、手順、不可条件、data safety、metadata / backup / WAL 影響、検証 command が rollback evidence と一致している | Phase 未完了 |
| `release_note_sync_result` | release note の user-visible change、breaking / deprecated / unsupported、Turso Cloud 互換差分、self-host 固有差分、known blocker 0 件が operator delta と一致している | rollout ready 不可 |
| `configuration_documentation_sync_result` | config key、env var、CLI flag、default、required、forbidden、secret handling、rotation、redaction が configuration closure と一致している | 実装開始禁止 |
| `troubleshooting_documentation_result` | 代表的な error、health state、log event、metric、recovery command、escalation、再現 artifact が troubleshooting と runbook に接続されている | Phase 未完了 |
| `documentation_stale_detection_result` | 旧 version、旧 Phase 境界、旧 API、旧 error、旧 config、旧 runbook 手順、未参照 artifact を検出する command と fail 条件が固定されている | merge 不可 |
| `documentation_runbook_closure_result` | 上記 field がすべて `pass`。文書 inventory 欠落、API docs 不一致、runbook 古さ、migration / rollback guide 未同期、release note 欠落、config docs 不一致、troubleshooting 不足、stale text 検出なしがすべて 0 件 | `pass` 以外は実装開始禁止 |

`documentation_runbook_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。documentation、operator runbook、guide、release note に影響しない Phase でも `documentation_runbook_closure_result = pass` とし、`documentation_inventory:unchanged`、`api_documentation_sync:unchanged`、`operator_runbook_sync:unchanged`、`migration_guide_sync:unchanged`、`rollback_guide_sync:unchanged`、`release_note_sync:unchanged`、`configuration_documentation_sync:unchanged`、`troubleshooting_documentation:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「仕様は変えたが README/API docs/runbook が古い」「rollback 手順だけ旧仕様」「release note に互換差分が出ない」状態を Phase 未完了として扱える。

**canonical SBOM / vulnerability / license release closure audit：**

SBOM / vulnerability / license release closure は、Phase ごとの crate、SDK、toolchain、Docker image、GitHub Action、binary、generated artifact、extension artifact の構成、脆弱性、license、配布可否、例外承認、再スキャン条件を固定し、lockfile と SBOM の乖離、未評価 CVE、配布不可 license、由来不明 binary を release / merge 前に遮断する closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`sbom_vulnerability_license_closure_result = pass` でなければ実装開始、Phase 完了、rollout ready、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `sbom_inventory_result` | crate、SDK、CLI tool、Docker image、GitHub Action、binary、generated artifact、extension artifact が SBOM に列挙され、Phase packet と一致している | 実装開始禁止 |
| `lockfile_sbom_sync_result` | Cargo.lock、package lock、toolchain file、Docker digest、GitHub Action ref、generated artifact manifest と SBOM の version / digest が一致している | merge 不可 |
| `vulnerability_scan_result` | CVE、GHSA、RustSec、yanked version、deprecated package、malicious package、critical unmaintained dependency の scan command、日時、DB version、結果、未評価 0 件が固定されている | merge 不可 |
| `license_distribution_result` | license、notice、attribution、static/dynamic link、Docker image 配布、self-host 配布、SaaS 利用、禁止 license 0 件、例外承認が固定されている | rollout ready 不可 |
| `container_image_attestation_result` | Docker image base、tag、digest、build context、provenance、root/non-root、OS package scan、rebuild command が固定されている | 実装開始禁止 |
| `github_action_supply_chain_result` | GitHub Action の owner、ref、pin、permissions、third-party action、更新条件、禁止 floating ref が固定されている | merge 不可 |
| `binary_artifact_origin_result` | 事前配置 binary、extension `.so`、generated binary、downloaded artifact の producer、hash、signature、再生成可否、保存場所が固定されている | 実装開始禁止 |
| `exception_waiver_result` | 脆弱性、license、unmaintained dependency、署名欠落、SBOM 差分の waiver 条件、owner、期限、再評価 trigger、削除条件が固定されている | Phase 未完了 |
| `rescan_trigger_result` | dependency change、lockfile change、Docker base change、GitHub Action change、release、incident、upstream advisory、Turso Cloud 追従時の再スキャン条件が固定されている | merge 不可 |
| `sbom_vulnerability_license_closure_result` | 上記 field がすべて `pass`。SBOM 欠落、lockfile/SBOM 不一致、未評価脆弱性、license 未判断、image attestation 欠落、Action pin 不明、binary origin 不明、waiver 未承認、rescan trigger 欠落がすべて 0 件 | `pass` 以外は実装開始禁止 |

`sbom_vulnerability_license_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。dependency、image、GitHub Action、binary、generated artifact に影響しない Phase でも `sbom_vulnerability_license_closure_result = pass` とし、`sbom_inventory:unchanged`、`lockfile_sbom_sync:unchanged`、`vulnerability_scan:unchanged`、`license_distribution:unchanged`、`container_image_attestation:unchanged`、`github_action_supply_chain:unchanged`、`binary_artifact_origin:unchanged`、`exception_waiver:none_required`、および非対象理由を `closure_evidence` に記録する。これにより「依存は固定したが SBOM がない」「CVE が未評価」「license は見たが配布可否が未判断」「Action が floating ref のまま」な状態を Phase 未完了として扱える。

**canonical telemetry / signal / observability contract closure audit：**

telemetry / signal / observability contract は、Phase ごとの成功、失敗、拒否、劣化、復旧、rollback、operator_required を、log、metric、health/status、request_id / trace_id、artifact、operator-visible signal で追跡できるように固定し、動作はしても観測不能、失敗分類不能、秘匿漏れ、相関不能になる実装を防ぐための closure gate である。対象 Phase の readiness packet と Done receipt は、以下の closure audit field を持ち、`telemetry_signal_contract_closure_result = pass` でなければ実装開始、Phase 完了、rollout ready、または merge に進めない。

| Audit field | pass 条件 | fail 時の扱い |
|-------------|----------|---------------|
| `signal_inventory_result` | 対象 Phase の API、background job、storage、auth、quota、replication、backup、branch、extension、metrics、HA、internal adapter が出す log / metric / health / artifact signal が列挙されている | 実装開始禁止 |
| `log_field_contract_result` | log level、event name、required field、forbidden field、request_id、resource id、error code、state、redaction が固定されている | merge 不可 |
| `metric_name_contract_result` | metric name、type、label、unit、cardinality、増減点、reset / persistence、Prometheus 公開可否が固定されている | 実装開始禁止 |
| `health_status_contract_result` | health/status endpoint の state、degraded / unavailable / recovering / operator_required 表示、write policy、retry guidance が固定されている | Phase 未完了 |
| `request_trace_correlation_result` | request_id、trace_id、connection id、job id、artifact id の生成、伝播、正規化、snapshot 上の置換 rule が固定されている | 実装開始禁止 |
| `redaction_signal_contract_result` | token、JWT、SQL args、raw body、absolute path、secret、credential、personal data が log / metric / health / artifact に出ない検証 command が固定されている | merge 不可 |
| `failure_signal_mapping_result` | validation error、auth denial、quota exceeded、storage busy、corruption、rollback failure、upstream drift、internal adapter diff の signal と defect classification が接続されている | Phase 未完了 |
| `operator_visible_signal_result` | operator が見る dashboard、health、log、metric、release note、runbook の signal 名、確認 command、判断基準が固定されている | rollout ready 不可 |
| `telemetry_artifact_contract_result` | telemetry snapshot、log sample、metric snapshot、health snapshot、redaction scan、failure trace の artifact path、hash、再生成 command が固定されている | Phase 未完了 |
| `telemetry_signal_contract_closure_result` | 上記 field がすべて `pass`。signal 未列挙、log field 不明、metric 名不明、health state 不明、request 相関不能、redaction 欠落、failure signal 未接続、operator signal 不明、artifact 欠落がすべて 0 件 | `pass` 以外は実装開始禁止 |

`telemetry_signal_contract_closure_result` は readiness packet の `audit_results` と Done receipt に必ず含める。telemetry、signal、observability に影響しない Phase でも `telemetry_signal_contract_closure_result = pass` とし、`signal_inventory:unchanged`、`log_field_contract:unchanged`、`metric_name_contract:unchanged`、`health_status_contract:unchanged`、`request_trace_correlation:unchanged`、`redaction_signal_contract:unchanged`、`failure_signal_mapping:unchanged`、`operator_visible_signal:unchanged`、および非対象理由を `closure_evidence` に記録する。これにより「エラーは返るが運用者が検出できない」「log と artifact が request に紐づかない」「metric label に secret が出る」「health が劣化を隠す」状態を Phase 未完了として扱える。

Phase operator behavior delta に関係する仕様変更は、§1.4、§1.5、§3.5.3、§7.3、§9.1.10、§9.1.11、§9.1.13、§9.1.15、§9.1.17、§9.1.19、§9.1.20、§9.1.22、§9.1.23、§9.1.24、§9.1.29、§9.1.32、§9.1.33、§9.1.37、§9.1.44、§9.1.45、§9.1.46、§9.1.47、§9.1.49、§9.1.50、§9.1.51、§9.2、§9.4、§9.5、§9.6、§9.7、§9.8、§9.11、§9.14、§9.17、該当 Phase 詳細節を同時更新する。operator behavior delta がない Phase 実装 PR は、利用者・運用者から見える変更、互換差分、運用手順、release note、rollback が未確定であるため、Phase 完了扱いにしない。
