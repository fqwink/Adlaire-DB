# 内製化仕様

## 1. 定義

内製化仕様とは、libSQL などの外部実装に依存している内部コンポーネントを、Adlaire DB の内製コンポーネントへ差し替えるための実装契約である。

内製化仕様は、外部 API や SDK 互換を変更するための仕様ではない。内製化は、外部契約を維持したまま内部実装を置き換えるために行う。

## 2. 対象

内製化仕様には、以下を必ず固定する。

| 項目 | 固定内容 |
|------|----------|
| 差し替え対象 | どの crate、adapter、engine、scheduler、storage 境界を差し替えるか |
| 差し替え前実装 | 既存の外部 crate、libSQL 統合、または現行 adapter |
| 差し替え後実装 | Adlaire 内製 crate、adapter、または internal module |
| adapter 境界 | 外部 contract と内部実装を分離する入出力、error、metadata、log、metrics 境界 |
| 外部 API 不変条件 | API path、method、request、response、status code、response wrapper を変更しない条件 |
| wire format 不変条件 | HTTP / WebSocket / hrana wire bytes、field casing、unknown field 扱いを変更しない条件 |
| metadata schema 不変条件 | 既存 metadata field、migration、後方互換、rollback を壊さない条件 |
| error code 不変条件 | 既存 error code、HTTP status、response body、redaction を壊さない条件 |
| JWT claim 不変条件 | token claim、scope、permission、拒否条件を壊さない条件 |
| SDK 挙動不変条件 | libSQL SDK から見える connect、query、transaction、error、retry 挙動を壊さない条件 |
| shadow mode | 既存実装を正として client-visible response を返し、内製実装との差分を artifact に記録する条件 |
| active mode | 内製実装を production path へ切り替えられる条件 |
| rollback | config flag または同等の仕様化済み手段で旧経路へ戻せる条件 |
| error mapping | 内製実装内部の error を既存 error code へ写像する規則 |
| performance baseline | latency、RSS、WAL size、checkpoint time、throughput の許容差分 |
| crash recovery | crash、partial write、restart、metadata/file 不一致時の復旧条件 |
| compatibility test | Turso Cloud / libSQL SDK / hrana 互換を証明する test |
| regression test | 対象 Phase 以前の contract を壊していないことを証明する test |
| Done 判定 | Done receipt に必要な pass 条件、artifact、未解決 0 条件 |

## 3. 絶対条件

内製化仕様では、以下を必ず守る。

| 条件 | 判定 |
|------|------|
| Turso Cloud 互換 mode の API を変えない | 破った場合は merge 不可 |
| libSQL SDK の既存挙動を壊さない | 破った場合は merge 不可 |
| metadata schema を仕様変更なしに変えない | 破った場合は merge 不可 |
| error code を仕様変更なしに変えない | 破った場合は merge 不可 |
| JWT claim を仕様変更なしに変えない | 破った場合は merge 不可 |
| 内製化を理由に response wrapper を変えない | 破った場合は merge 不可 |
| rollback できない切り替えをしない | 破った場合は merge 不可 |
| shadow mode の検証なしに active 化しない | 破った場合は Phase 未完了 |
| compatibility test が失敗した状態で完了にしない | 破った場合は Phase 未完了 |

## 4. Phase 19 内製化仕様

Phase 19 は内部差し替えを始める Phase であり、外部契約を変える Phase ではない。

### 4.1 対象

Phase 19 で対象にできるのは以下のみである。

| 対象 | 扱い |
|------|------|
| WAL checkpoint control | 実装可 |
| executor adapter boundary | 実装可 |
| readonly storage adapter | 実装可 |

Phase 19 では以下を対象外とする。

| 対象外 | 判定 |
|--------|------|
| storage write path の active 化 | Phase 20 以降の仕様変更 PR が承認されるまで禁止 |
| SQL parser の差し替え | Phase 20 以降の仕様変更 PR が承認されるまで禁止 |
| metadata migration を伴う変更 | Phase 20 以降の仕様変更 PR が承認されるまで禁止 |
| API path / method / request / response の変更 | merge 不可 |
| error code の新設 | 先に仕様変更 PR が必要 |
| JWT claim の変更 | merge 不可 |
| SDK 挙動の変更 | merge 不可 |

### 4.2 外部契約

Phase 19 では、以下を変更してはならない。

| 外部契約 | 固定仕様 |
|----------|----------|
| HTTP API | path、method、request、response、status code、response wrapper を変更しない |
| WebSocket wire format | hrana wire format、message type、field casing、unknown field 扱いを変更しない |
| 管理 API | endpoint、metadata、pagination、error、permission を変更しない |
| metadata schema | 既存 field の rename、delete、required 化を行わない |
| error code | 既存 error code と HTTP status を保つ |
| JWT claim | 既存 claim、scope、permission を保つ |
| SDK 挙動 | libSQL SDK から見える connect、query、transaction、error、retry 挙動を保つ |
| 既定 mode | 直前 Phase と同じ挙動を保つ |

### 4.3 adapter 境界

内製化は adapter 境界の内側に限定する。

adapter の外側に見える入力、出力、error、metadata、ログ、metrics 名を変更してはならない。新しい公開 metrics 名が必要な場合は、先に仕様変更 PR で固定する。

### 4.4 shadow mode

内製実装は、まず shadow mode で動作させる。

shadow mode では、既存実装を正として実行し、内製実装との差分を記録する。client に返す response は既存実装の結果とする。

shadow mode の artifact には、対象 request、既存実装 result、内製実装 result、diff、許容理由、timestamp、commit SHA を含める。

### 4.5 active mode

active mode に切り替えられるのは、以下をすべて満たした場合のみである。

| 条件 | 判定 |
|------|------|
| shadow diff が許容範囲内 | 必須 |
| compatibility test が pass | 必須 |
| regression test が pass | 必須 |
| crash recovery test が pass | 必須 |
| performance baseline を満たす | 必須 |
| rollback flag が動作する | 必須 |

### 4.6 rollback

active mode に切り替える場合は、必ず rollback flag を用意する。

rollback flag により、旧実装へ戻せなければならない。rollback 後に metadata schema、DB file、WAL、response、error、SDK 挙動が破損または不整合を起こす場合は active mode へ切り替えてはならない。

### 4.7 error mapping

内製実装内部の error は、既存 error code に写像する。

新しい error code が必要な場合は、内製化実装前に仕様変更 PR で追加する。`INTERNAL_ERROR` で既存 error を隠してはならない。

### 4.8 完了条件

Phase 19 の内製化は、以下がすべて満たされた場合のみ完了とする。

| 条件 | 必須結果 |
|------|----------|
| 外部 API 差分 | 0 |
| SDK 互換差分 | 0 |
| metadata schema 差分 | 0 |
| error code 差分 | 0 |
| JWT claim 差分 | 0 |
| shadow diff | 記録済み |
| rollback test | pass |
| compatibility test | pass |
| regression test | pass |
| crash recovery test | pass |
| Done receipt | 作成済み |

Done receipt には、coverage gap、未解決判断、仕様未確定、既知 flaky が 0 件であることを明記する。
