# Adlaire DB プロジェクト憲章 / 仕様正本入口

**バージョン：** V.218
**ステータス：** 設計中
**最終更新：** 2026-09-13

---

## 0. 文書の役割

`docs/PROJECT_CHARTER.md` は、Adlaire DB のプロジェクト憲章であり、仕様正本群への入口も兼ねる。

本ファイルに記載する内容は、プロジェクトの目的、価値、方針、ポリシー、仕様正本群の所在である。実装、テスト、レビュー、完了判定に使う確定契約は `docs/spec/` 配下の仕様正本群を正とする。

### 0.1 版管理

本仕様書群のバージョンは `V.{累積番号}` 形式で表記する。現在の仕様書群バージョンは `V.218` である。

仕様書群バージョンは累積単調増加とし、リセットしてはならない。大規模改訂、Phase 再編、リポジトリ移行、文書構成変更、実装方針変更、Turso Cloud 互換方針の更新があっても、`V.1`、`0.x`、日付ベース、Phase 番号ベースへ戻してはならない。

仕様本文に影響する変更を行う PR は、必ず現在値より大きい次の累積番号へ進める。`V.99` の次は `V.100` とし、以後 `V.101`、`V.102` のように 1 ずつ増加させる。

| 禁止事項 | 判定 |
|----------|------|
| `V.{累積番号}` 以外のバージョン表記へ変更する | review failure |
| バージョンを過去番号へ戻す | merge 不可 |
| Phase 番号に合わせてバージョンをリセットする | merge 不可 |
| リポジトリ移行や仕様再編を理由に `V.1` から再開する | merge 不可 |
| 仕様本文を変更したのにバージョンを上げない | review failure |

### 0.2 文書種別の区別

本リポジトリでは、文書内の記述を次の 4 種類に分ける。実装根拠にできるのは仕様だけである。

| 種別 | 内容 | 実装時の扱い |
|------|------|--------------|
| プロジェクト憲章 | 目的、価値、優先順位、判断原則、最終目標 | 単独では実装根拠にしない |
| 方針 | 目指す方向、進め方、将来の進化方向 | 仕様ではない。実装根拠にしない |
| ポリシー | 判断ルール、運用ルール、禁止事項、許可条件 | 仕様ではない。実装根拠にしない |
| 仕様 | 入力、出力、状態、error、永続化、検証、Done 条件を固定した契約 | 実装根拠にできる |

方針またはポリシーを実装する場合は、先に `docs/spec/` 配下の仕様正本群へ落とし込み、対象、入力、出力、状態、error、永続化、検証、Done 条件を固定してから実装する。

---

## 1. プロジェクト憲章

### 1.1 プロジェクト概要

Adlaire DB は、**libSQL ワイヤプロトコル（hrana-http v2 / hrana-ws v3）互換のサーバー特化 DB サーバー**である。

libSQL クライアント SDK（TypeScript、Rust、Go 等）から接続 URL を差し替えるだけで動作することを目指す。クライアント側の埋め込みレプリカ機能は対象外とし、サーバー側の HTTP/WebSocket API、マルチ DB 管理、レプリケーション、バックアップ、復旧、運用管理に特化する。

### 1.2 価値

Adlaire DB の価値は、SQLite / libSQL 系の軽量さを保ちながら、Turso Cloud と同等の管理体験を自己ホスト環境へ持ち込むことである。

単なる SQLite wrapper ではなく、Turso Cloud 互換の API、SDK 接続、認証、metadata、organization / group / location / quota、backup、restore、PITR、branch、replication、metrics、HA、運用証跡を段階的に備える DB 管理基盤として実装する。

### 1.3 最終目標

Adlaire DB の最終目標は、Turso Cloud 互換の自己ホスト DB 管理基盤として実用化したうえで、外部 contract を固定したまま内部実装を Adlaire 独自基盤へ段階移行することである。

外部 contract とは、HTTP/WebSocket wire format、Platform API、admin API、SDK 互換挙動、JWT claim、metadata schema、error code、response wrapper、persistence compatibility、operator-facing behavior を指す。

内部実装とは、WAL、checkpoint、storage、query executor adapter、archive、branch engine、quota engine、scheduler、metrics、HA coordination、recovery engine を指す。

### 1.4 優先順位

実装判断で迷う場合は、次の順で優先する。

1. Turso Cloud 互換
2. libSQL SDK 互換
3. 既存 Adlaire 後方互換
4. 自己ホスト運用性
5. 内製化都合
6. Adlaire 独自拡張

内部実装を育てることは目的であるが、外部 contract を壊してまで内製化を進めてはならない。

---

## 2. 方針

### 2.1 Turso Cloud 互換優先

Adlaire DB は Turso Cloud 互換を継続追従する。Turso Cloud の API、metadata、認証、権限、エラー、管理モデル、SDK 互換挙動に変更が確認された場合は互換性レビュー対象とし、Adlaire DB 側の仕様差分を明示する。

自己ホスト都合で Turso Cloud と異なる仕様を採用する場合は、差分理由、代替仕様、既存 SDK への影響、後方互換性を仕様正本群へ明記してから実装する。

### 2.2 サーバー特化

Adlaire DB はサーバー側機能に特化する。対象は通常の HTTP/WebSocket 接続、管理 API、認証、マルチ DB、レプリケーション、バックアップ、復旧、branch、metrics、HA である。

クライアント側の埋め込みレプリカ同期は対象外とする。

### 2.3 シングルバイナリ運用

サーバー起動は `./adlaire-db <flags>` 一コマンドで完結する。外部デーモンやサイドカーを必須にしない。

### 2.4 段階的内製化

libSQL 内部コンポーネントの内製化は Phase 19 から開始する。Phase 18 以前は Turso Cloud 互換レイヤー、管理 API、レプリケーション、バックアップ、ブランチ、HA の完成を優先し、production path の内部差し替えは行わない。

内製化は Turso Cloud 互換を捨てるためではなく、互換レイヤーを保ったまま内部実装を段階的に置き換えるために行う。

内製化の方針は次の通りである。

| 項目 | 方針 |
|------|------|
| 単位 | 内製化の単位はクレートとする |
| 進め方 | 外部クレートと内製クレートの併用パターンを初期段階から採用する |
| 置換対象 | adapter 境界の内側から段階的に置き換える |
| 既定 mode | Turso 互換 mode を常に既定とする |
| 独自拡張 | Adlaire 拡張 mode を追加する場合は Turso 互換 mode と仕様上分離する |
| 完了判定 | Turso Cloud / libSQL SDK compatibility、Phase regression、oracle、invariant ledger が通るまで完了扱いにしない |

内製化の実装契約は `docs/spec/internalization.md` を正とする。

### 2.5 仕様駆動開発

Adlaire DB は仕様駆動開発を採用する。実装、テスト、レビュー、完了判定は、`docs/spec/` 配下の仕様正本群に固定された契約に基づいて行う。

実装中に仕様不足、仕様矛盾、未定義の挙動を見つけた場合は、実装判断で補完してはならない。先に仕様変更 PR で仕様正本群を更新し、必要な契約を固定してから実装する。

---

## 3. ポリシー

### 3.1 ポリシーの扱い

ポリシーは判断ルール、運用ルール、禁止事項、許可条件、レビュー判定基準である。ポリシーは仕様ではない。

ポリシーだけでは API、error、metadata、永続化、test、Done 条件は確定しない。ポリシーを実装する場合は、先に仕様正本群へ変換する。

### 3.2 互換差分明示ポリシー

Turso Cloud と異なる挙動は暗黙にしない。差分を採用する場合は、対象 API、差分内容、error、SDK 影響、後方互換性、検証方法を仕様正本群へ固定する。

### 3.3 secret redaction policy

secret をログ、snapshot、artifact、error response、debug output に出してはならない。

このポリシーを実装する場合は、secret field、対象ログ、redaction 形式、error、artifact、検証 command、test を仕様正本群へ固定する。

### 3.4 write policy

状態に応じて write を許可または拒否する場合は、状態、判定順、error、永続化影響、rollback、test を仕様正本群へ固定する。

### 3.5 retention policy

artifact、backup、snapshot、WAL archive の保持ルールを扱う場合は、対象 data、保持期間、削除条件、例外、復旧影響、test を仕様正本群へ固定する。

### 3.6 ポリシー運用の禁止事項

| 禁止事項 | 理由 |
|----------|------|
| ポリシーだけを根拠に実装する | 実装対象が未確定であるため |
| ポリシーだけを根拠に Phase 完了扱いにする | 完了条件と証跡が仕様化されていないため |
| ポリシーを PR description だけで補う | 正本に残らず後続実装が迷うため |
| ポリシーと仕様が矛盾したまま実装する | 実装判断が分岐するため |

---

## 4. 仕様正本群への入口

実装、テスト、レビュー、完了判定に使う仕様本文は、`docs/spec/` 配下の仕様正本群へ分けて記載する。

`docs/PROJECT_CHARTER.md` は仕様正本群への入口を兼ねるが、仕様本文そのものではない。仕様本文を改訂する場合は、該当する `docs/spec/` 配下の正本を更新し、`docs/build-scripts/build_spec_v3.py` 経由で `docs/Adlaire-db-spec.html` を再生成する。

### 4.1 仕様正本群

| 正本 | 役割 |
|------|------|
| `docs/spec/core.md` | 互換性、境界、実行基盤、データ保全、運用、実装統制、Phase 仕様入口 |
| `docs/spec/auth.md` | 認証・認可仕様 |
| `docs/spec/api.md` | API 契約仕様 |
| `docs/spec/internalization.md` | 内製化の実装契約 |
| `docs/spec/testing.md` | テスト仕様 |
| `docs/spec/phase-*.md` | Phase 単位の詳細仕様 |

### 4.2 仕様不足時の扱い

仕様正本群に不足、矛盾、未定義の挙動がある場合は、実装で補完してはならない。先に仕様変更 PR を作成し、API 契約、状態遷移、永続化契約、エラー契約、テスト条件、完了条件を追加してから実装する。

---

<!-- include: docs/spec/core.md -->
<!-- include: docs/spec/internalization.md -->
<!-- include: docs/spec/testing.md -->
