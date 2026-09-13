# Adlaire DB プロジェクト憲章 / 仕様正本入口

**バージョン：** V.217
**ステータス：** 設計中
**最終更新：** 2026-09-13

---

## 0. プロジェクト憲章 / 仕様正本入口の責務

`docs/PROJECT_CHARTER.md` は、プロジェクト憲章であり、仕様正本群の入口も兼ねる。本ファイルは Adlaire DB の概要、方針、ポリシー、仕様正本群への入口、仕様駆動開発方針をまとめる。実装・テスト・レビュー・完了判定に使う確定契約は `docs/spec/` 配下の仕様正本群へ切り離して管理する。仕様正本内で矛盾がある場合は、実装判断ではなく仕様改訂で解消する。

### 0.1 仕様バージョン管理責務

本仕様書のバージョンは `V.{累積番号}` 形式で表記する。現在の仕様書バージョンは `V.217` である。

仕様書バージョンは累積単調増加とし、リセットしてはならない。大規模改訂、Phase 再編、リポジトリ移行、仕様書構成変更、実装方針変更、Turso Cloud 互換方針の更新があっても、`V.1`、`0.x`、日付ベース、Phase 番号ベースへ戻してはならない。

仕様書を更新する PR は、変更内容が仕様本文に影響する場合、必ず現在値より大きい次の累積番号へ進める。`V.99` の次は `V.100` とし、以後 `V.101`、`V.102` のように 1 ずつ増加させる。

**禁止事項：**

| 状態 | 判定 |
|------|------|
| `V.{累積番号}` 以外の仕様書バージョン表記へ変更する | review failure |
| 仕様書バージョンを過去番号へ戻す | merge 不可 |
| Phase 番号に合わせて仕様書バージョンをリセットする | merge 不可 |
| リポジトリ移行や仕様再編を理由に `V.1` から再開する | merge 不可 |
| 仕様本文を変更したのに仕様書バージョンを上げない | review failure |

### 0.2 プロジェクト憲章・方針・ポリシー・仕様の区別

本リポジトリでは、プロジェクト憲章、方針、ポリシー、仕様を以下の意味で区別し、本ファイル内に分けて記載する。実装対象は仕様だけである。

| 種別 | 意味 | 実装時の扱い |
|------|------|--------------|
| プロジェクト憲章 | 目的、価値、目標、優先順位、判断原則を示す文書 | 単独では実装根拠にしない。実装根拠は `docs/spec/` 配下の仕様正本群に固定された契約だけとする |
| 方針 | 目指す方向、優先順位、進め方、将来の進化方向を示す記述 | 仕様ではない。実装根拠にしてはならない |
| ポリシー | 条件に応じた判断ルール、運用ルール、禁止・許可の考え方 | 仕様ではない。実装根拠にしてはならない |
| 仕様 | 実装者がそのまま実装、テスト、レビューできる確定契約 | 実装根拠にできる |

#### 0.2.1 プロジェクト憲章として記載する内容

プロジェクト憲章には、Adlaire DB の目的、価値、優先順位、判断原則、最終目標を記載する。プロジェクト憲章は「何を目指すか」を固定するが、それ単独では API、error、metadata、永続化、test、Done 判定を確定しない。

#### 0.2.2 方針として記載する内容

方針には、Turso Cloud 追従、自己ホスト活用、段階的内製化、開発・保守の進め方、将来の進化方向を記載する。方針は仕様ではない。たとえば「内部は段階的に内製化する」は方針であり、そのままでは実装根拠にしてはならない。

方針を実装する場合は、対象 API、adapter 境界、config、metadata、永続化、error、rollback、test、artifact、Done 判定を仕様として固定してから実装する。

#### 0.2.3 ポリシーとして記載する内容

ポリシーには、判断ルール、運用ルール、禁止事項、許可条件、レビュー判定基準を記載する。ポリシーは仕様ではない。たとえば「secret をログに出さない」はポリシーであり、そのままでは実装対象ではない。

ポリシーを実装する場合は、先に仕様へ変換しなければならない。例えば「secret をログに出さない」というポリシーは、そのままでは実装対象ではない。仕様にするには、secret field、対象ログ、redaction 形式、error、artifact、検証 command、test を `docs/spec/` 配下の仕様正本へ固定する。

**ポリシーの例：**

| ポリシー | 意味 | 仕様化する場合に必要な内容 |
|----------|------|----------------------------|
| 互換差分明示ポリシー | Turso Cloud と異なる挙動は暗黙にしない | 対象 API、差分内容、error、SDK 影響、test |
| secret redaction policy | secret をログや artifact に出さない | secret field、redaction 形式、対象ログ、検証 command |
| write policy | 状態に応じて write を許可または拒否する | 状態、判定順、error、永続化影響、test |
| retention policy | artifact や backup の保持ルールを定める | 対象 data、保持期間、削除条件、例外、test |

**ポリシーと仕様の関係：**

- ポリシーは実装対象ではない
- ポリシーは仕様正本ではない
- ポリシーだけでは API、error、metadata、永続化、test は確定しない
- ポリシーを実装する場合は、仕様に変換してから実装する
- ポリシーと仕様が矛盾した場合は、仕様を優先する

**ポリシー運用の禁止事項：**

| 禁止事項 | 理由 |
|----------|------|
| ポリシーだけを根拠に実装する | 実装対象が未確定であるため |
| ポリシーだけを根拠に Phase 完了扱いにする | 完了条件と証跡が仕様化されていないため |
| ポリシーを仕様の代わりに PR description で補う | 正本に残らず後続実装が迷うため |

#### 0.2.4 仕様駆動開発方針

Adlaire DB は仕様駆動開発を採用する。実装、テスト、レビュー、完了判定は、`docs/spec/` 配下の仕様正本群に固定された契約に基づいて行う。

方針およびポリシーは、そのまま実装根拠にしてはならない。実装する場合は、先に仕様正本群へ落とし込み、対象、入力、出力、状態、error、永続化、検証、Done 条件を固定してから実装する。

実装中に仕様不足、仕様矛盾、未定義の挙動を見つけた場合は、実装判断で補完してはならない。先に仕様変更 PR で仕様正本群を更新し、必要な契約を固定してから実装を進める。

#### 0.2.5 仕様として記載する内容

仕様には、実装者がそのまま実装、テスト、レビュー、完了判定できる確定契約を記載する。実装根拠にできるのは、`docs/spec/` 配下で仕様正本として固定された契約だけである。

仕様として扱うには、少なくとも対象、入力、出力、状態遷移、永続化、error、認証認可、互換差分、rollback、検証 command、artifact、Done 判定のうち該当項目が明示されていなければならない。

#### 0.2.6 混在時の扱い

実装時にプロジェクト憲章、方針、ポリシーと仕様が矛盾する場合は、仕様を優先する。仕様が不足している場合は、実装で補完してはならない。先に仕様変更 PR を作成し、API 契約、状態遷移、永続化契約、エラー契約、テスト条件、完了条件を追加してから実装する。

## 1. プロダクト責務

Adlaire DB が何を提供し、何を優先し、どの方向へ進化するかを固定する責務である。Turso Cloud 互換、自己ホスト運用、将来的な内製化方針は、この責務に従って判断する。

### 1.1 概要・位置づけ・進化方針

### 1.1 プロジェクト概要

Adlaire DB は **libSQL ワイヤプロトコル（hrana-http v2 / hrana-ws v3）互換のサーバー特化 DB サーバー**である。

libSQL クライアント SDK（TypeScript・Rust・Go 等）から接続 URL を差し替えるだけで動作する。クライアント側の埋め込みレプリカ機能は対象外とし、サーバー側の HTTP/WebSocket API・マルチDB管理・レプリケーション・バックアップに特化する。

libSQL フォークの内部コンポーネント（WAL・ページストレージ・SQL エンジン等）の内製化は Phase 19 から開始する。Phase 18 以前は Turso Cloud 互換レイヤー、管理 API、レプリケーション、バックアップ、ブランチ、HA の完成を優先し、production path の内部差し替えは行わない。

### 1.2 ポジション

| 比較対象 | Adlaire DB との関係 |
|----------|---------------------|
| Turso Cloud | ワイヤプロトコル（hrana）互換の参照実装。埋め込みレプリカは対象外 |
| libSQL / libsql crate | ワイヤプロトコルと embedded SQLite の実装参照。libsql 0.6（crates.io）を組み込み利用 |
| SQLite | libsql crate 経由で互換性を維持 |

### 1.3 固定制約

| 項目 | 内容 |
|------|------|
| 実装言語 | Rust + 標準ライブラリ |
| ストレージ・SQL 基盤 | libsql crate 0.6（embedded SQLite / WAL モード）|
| 外部フレームワーク | 使用禁止。外部クレートは使用可能だが、Web フレームワーク（axum・actix-web・rocket 等）は採用しない |
| 目標機能 | Turso Cloud 互換・hrana プロトコル互換・サーバー特化機能 |
| Phase 19 方針 | Turso Cloud 追従を継続し、互換レイヤーを維持したまま libSQL 内部を段階的に内製化する |
| デプロイ形態 | シングルバイナリ起動 |
| 対象 OS | Linux |

### 1.4 設計不変条件

実装のあらゆる判断においてこれらを最優先する。

**I-1：hrana プロトコル互換**
既存の libSQL クライアント SDK（TypeScript・Rust・Go 等）が、Turso Cloud の URL を Adlaire DB の URL に差し替えるだけで動作しなければならない。ただし埋め込みレプリカ（`syncUrl` 指定）は対象外とし、通常の HTTP/WebSocket 接続のみを対象とする。

**I-1a：Turso Cloud 互換の継続追従**
Adlaire DB は Turso Cloud 互換を継続追従する。Turso Cloud の API、metadata、認証、権限、エラー、管理モデル、SDK 互換挙動に変更が確認された場合は互換性レビュー対象とし、Adlaire DB 側の仕様差分を明示する。自己ホスト都合で Turso Cloud と異なる仕様を採用する場合は、差分理由、代替仕様、既存 SDK への影響、後方互換性を本仕様書に明記してから実装する。

**I-2：外部 DB 依存は libSQL フォーク一本**
SQLite・libSQL フォーク以外の外部 DB ライブラリ（PostgreSQL・MySQL ドライバ等）に依存しない。

**I-3：シングルバイナリ**
サーバー起動は `./adlaire-db <flags>` 一コマンドで完結する。外部デーモン・サイドカーを必要としない（Phase 1）。

**I-4：データ永続化の先行保証**
クライアントへ成功応答を返す前に、書き込みデータが永続化（fsync）されていることを保証する。

**I-5：内製化は段階的・計画的に**
libSQL 内部コンポーネントの内製化はフェーズ完了後に計画・判断する。内製化は Turso Cloud 互換を捨てるためではなく、互換レイヤーを保ったまま内部実装を段階的に置き換えるために行う。「実装が大変だから」という理由で無計画に外部依存を追加することは認めない。

**I-6：外部 Web フレームワーク不使用**
HTTP サーバー層に Web フレームワーク（axum・actix-web・rocket 等）を使用しない。hyper 等の低レベル HTTP ライブラリ（クレート）は使用可能だが、ルーティング・ミドルウェア・リクエスト解析の構造はフレームワークに依存せず自前で実装する。

### 1.5 活用・進化方針

Adlaire DB の活用目的は、SQLite / libSQL 系の軽量さを保ちながら、Turso Cloud と同等の管理体験を自己ホスト環境へ持ち込むことである。単なる SQLite wrapper ではなく、Turso Cloud 互換の API、SDK 接続、認証、metadata、organization / group / location / quota、backup、restore、PITR、branch、replication、metrics、HA、運用証跡を段階的に備える DB 管理基盤として実装する。

**活用対象：**

| 活用対象 | 目的 | 必須方針 |
|----------|------|----------|
| 小規模 SaaS / 個人開発 | user / organization / project ごとの DB を軽量に管理する | Turso Cloud 互換 API と libSQL SDK 互換を優先する |
| 社内・オンプレ環境 | 外部クラウドへデータを出せない環境で DB 管理 API を提供する | 自己ホスト運用でも organization / group / location / quota を正式対象にする |
| 開発・検証環境 | Turso Cloud を使わずに互換 API、SDK、migration、backup、branch を検証する | snapshot / oracle / compatibility test を仕様化する |
| アプリ別独立 DB | CMS、業務ツール、管理画面、tenant ごとに DB を分離する | DB identity、path boundary、auth scope、quota を破らない |
| 運用復旧基盤 | backup、restore、PITR、branch、rollback により戻せる運用を実現する | success-before-fsync、partial commit、metadata/file 不一致を禁止する |

**進化方針：Turso Cloud 互換優先、内部は段階的に内製化**

| 項目 | 固定方針 |
|------|----------|
| 外部契約 | API、wire format、SDK 挙動、認証、metadata、error、管理モデルは Turso Cloud 互換を優先する |
| 自己ホスト差分 | 単一サーバーやオンプレ都合の差分は許可するが、差分理由、代替仕様、SDK 影響、後方互換性を仕様本文へ明記してから実装する |
| 内部実装 | 初期は libsql crate と外部 crate を利用し、運用機能を満たす。成熟後に adapter 境界の内側から段階的に内製 crate へ置き換える |
| 内製化禁止線 | 内製化を理由に API、response wrapper、JWT claim、metadata schema、error code、SDK 互換挙動を破ってはならない |
| モード分離 | Adlaire 独自拡張が必要な場合は、Turso 互換 mode と Adlaire 拡張 mode を仕様上分離し、既定は Turso 互換 mode とする |
| 完了判定 | 内製化 PR は Turso Cloud / libSQL SDK compatibility、Phase regression、oracle、invariant ledger が通るまで完了扱いにしない |

**最終目標：**

Adlaire DB の最終目標は、Turso Cloud 互換の自己ホスト DB 管理基盤として実用化したうえで、外部 contract を固定したまま内部実装を Adlaire 独自基盤へ段階移行することである。外部 contract とは、HTTP/WebSocket wire format、Platform API、admin API、SDK 互換挙動、JWT claim、metadata schema、error code、response wrapper、persistence compatibility、operator-facing behavior を指す。内部実装とは、WAL、checkpoint、storage、query executor adapter、archive、branch engine、quota engine、scheduler、metrics、HA coordination、recovery engine を指す。

**絶対ルール：**

| Rule | 内容 | 違反時の扱い |
|------|------|--------------|
| External contract freeze | Turso Cloud 互換 mode の外部 contract は、内製化都合で変更しない | merge 不可 |
| Internal replacement only | 内製化 PR は外部 contract ではなく adapter 境界の内側だけを置き換える | Phase 未完了 |
| Compatibility first | Turso Cloud / libSQL SDK 互換 snapshot が壊れる変更は、先に仕様差分、mode 分離、migration、rollback を定義する | 実装開始禁止 |
| Default compatibility mode | 既定挙動は常に Turso 互換 mode とする | Adlaire 拡張 mode を既定にした場合は merge 不可 |
| No silent divergence | 自己ホスト都合の差分を暗黙仕様にしない。差分理由、代替仕様、client impact、evidence を本文に残す | review failure |
| Evidence before Done | compatibility、oracle、invariant、regression、rollback evidence が揃うまで Done receipt を作成しない | Phase 未完了 |

**禁止事項：**

| 禁止事項 | 例 | 判定 |
|----------|----|------|
| 内製化都合で API path / method / request / response を変更する | adapter 差し替えのため `/v1/*` wrapper を変える | merge 不可 |
| SDK 互換を壊す | `@libsql/client` が既存 URL 差し替えで動かない | merge 不可 |
| metadata schema を理由なく破壊する | migration / rollback なしに field rename / delete を行う | merge 不可 |
| error code を差し替える | 既存 `DB_NOT_FOUND` を別 code に変える | merge 不可 |
| Turso 互換 mode に Adlaire 独自挙動を混ぜる | 既定 mode で独自 field 必須、独自 auth 必須にする | merge 不可 |
| `INTERNAL_ERROR` で互換差分を隠す | 本来 `INVALID_REQUEST` / `QUOTA_EXCEEDED` の差分を 500 にする | Phase 未完了 |
| production path を rollback flag なしで内製 crate へ切り替える | shadow 検証なしに write path を置換する | merge 不可 |

**許可事項：**

| 許可事項 | 条件 | 必須 evidence |
|----------|------|---------------|
| 内部 adapter の追加 | 既定 path の外部 contract に差分を出さない | shadow diff、compat snapshot |
| libsql crate 内側の置換準備 | production write path を変えない、または rollback flag を持つ | rollback test、crash recovery test |
| shadow mode 実装 | client-visible response に影響しない | shadow/active diff、performance baseline |
| Adlaire 拡張 mode 追加 | Turso 互換 mode と config / API / metadata を仕様上分離する | mode matrix、SDK regression |
| 自己ホスト差分の採用 | Turso Cloud と同一にできない理由が security / persistence / operation 上明確 | compatibility diff、client impact |
| 内製 crate の新規開発 | 外部 contract を固定し、adapter 境界に閉じる | unit coverage、oracle、invariant result |

**実装判断表：**

| 変更種別 | 判断 | 仕様に必要な固定事項 |
|----------|------|----------------------|
| Turso Cloud 互換に影響する変更 | 原則 Turso Cloud に追従。差分は例外扱い | snapshot source、差分理由、SDK impact、error mapping |
| libSQL SDK 互換に影響する変更 | SDK 互換を壊さない。壊す場合は Turso 互換 mode では不可 | SDK transcript、regression command、migration note |
| 自己ホスト差分 | security、persistence、operation の理由がある場合のみ可 | 代替仕様、operator impact、compatibility diff |
| 内部最適化 | 外部 contract に差分がなければ可 | performance baseline、rollback condition |
| Adlaire 独自拡張 | Turso 互換 mode と分離する場合のみ可 | mode flag、API boundary、metadata boundary |
| Phase 19 以降の内製化 | external contract freeze / internal replacement only を満たす場合のみ可 | shadow diff、oracle、invariant、full regression |

実装判断で迷う場合は、`Turso Cloud 互換 > libSQL SDK 互換 > 既存 Adlaire 後方互換 > 自己ホスト最適化 > 内製化都合 > Adlaire 独自拡張` の順で優先する。内部実装を育てることは目的であるが、外部契約を壊してまで内製化を進めてはならない。

---

## 2. 仕様正本群

実装、テスト、レビュー、完了判定に使う仕様本文は、プロジェクト憲章本文から切り離し、`docs/spec/` 配下の仕様正本群へ分けて記載する。

`docs/PROJECT_CHARTER.md` はプロジェクト憲章であり、仕様正本群への入口も兼ねる。仕様本文を改訂する場合は、該当する `docs/spec/` 配下の正本を更新し、HTML を再生成する。

<!-- include: docs/spec/core.md -->

## 11. 内製化方針

本章は、内製化の目的、方向性、優先順位だけを記載する。内製化の adapter 境界、shadow / active mode、rollback、error mapping、performance baseline、crash recovery、compatibility test、regression test、Done 判定は本章には置かず、内製化仕様へ切り離す。

内製化方針は仕様ではない。内製化を実装する場合は、内製化仕様または Phase 詳細に固定された契約だけを実装根拠にする。

- 内製化の単位はクレートとする
- 外部クレートを内製クレートに段階的に差し替えることで内製化を進める
- 内製化の目的は、Turso Cloud 互換の自己ホスト DB 管理基盤を維持したまま、内部実装を Adlaire 独自基盤へ育てることである
- 内製化は Turso Cloud 互換を維持するための内部実装差し替えであり、Turso Cloud 追従を止める理由にしてはならない
- API、認証、metadata、エラー、SDK 互換挙動は互換レイヤーとして固定し、その内側の実装から段階的に置き換える
- Turso 互換 mode は常に既定 mode とし、Adlaire 独自拡張 mode を追加する場合も互換 mode の snapshot と SDK regression に差分を出してはならない
- 既存の外部クレートで要件を満たせる場合は積極的に採用する
- 既存クレートで不足する機能は、最初から内製クレートとして開発する
- **外部クレートと内製クレートの併用パターンを初期段階から採用する**
- 内製化の順序は実装難易度が低いものから優先する

### 11.1 併用パターン

初期段階から外部クレートと内製クレートを併用する。
外部クレートで不足する機能を内製クレートで補い、
成熟次第に外部クレートを内製クレートへ差し替える。
差し替え中も Turso Cloud 互換テストと既存 libSQL SDK 互換テストを必須とし、互換性が落ちる差し替えは完了扱いにしない。

### 11.2 内製化順序（難易度低い順）

| 優先度 | 対象 | 現行クレート | 備考 |
|--------|------|------------|------|
| 1 | WAL チェックポイント制御 | libSQL | Phase 11 と直結 |
| 2 | ストレージ層 | libSQL（SQLite ページャー）| WAL 内製後に着手 |
| 3 | SQL パーサ | libSQL（SQLite）| 最難関・最後 |

### 11.3 実施時期

内製化の実装開始は Phase 19 とする。Phase 18 以前は、内製 crate の設計メモ、ベンチマーク、互換テスト追加のみ許可し、production path の切り替えは行わない。

---

<!-- include: docs/spec/internalization.md -->
<!-- include: docs/spec/testing.md -->
