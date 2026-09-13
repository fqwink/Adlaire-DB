# Adlaire DB — 作業ルール

## 仕様書の位置づけ

- `docs/PROJECT_CHARTER.md` は**プロジェクト憲章正本**であり、仕様正本群の入口も兼ねる
- `docs/spec/core.md`、`docs/spec/auth.md`、`docs/spec/api.md`、`docs/spec/internalization.md`、`docs/spec/testing.md`、`docs/spec/phase-*.md` は**仕様正本群**（single source of truth）である
- 中核仕様、認証・認可仕様、API契約仕様、内製化仕様、テスト仕様、Phase 1〜19 の詳細は `docs/spec/` に分割して管理する
- ポリシーは `docs/PROJECT_CHARTER.md` に記載し、仕様ではない
- 方針、ポリシーを実装根拠にする場合は、先に仕様正本へ落とし込むこと
- `docs/Adlaire-db-spec.html` は `docs/PROJECT_CHARTER.md` から生成された閲覧用 HTML であり、正本ではない
- 実装は仕様書に基づいて行うこと
- 仕様書と実装が乖離している場合は、**仕様書を優先する**
- 実装の都合で仕様書を変更する場合は、下記「変更承認フロー」に従うこと

## 仕様書 HTML 更新ルール

`docs/PROJECT_CHARTER.md`、`docs/spec/core.md`、`docs/spec/auth.md`、`docs/spec/api.md`、`docs/spec/internalization.md`、`docs/spec/testing.md`、`docs/spec/phase-*.md` を改訂した場合は、`docs/build-scripts/build_spec_v3.py` 経由で `docs/Adlaire-db-spec.html` を更新すること。

- 仕様書改訂 PR では、Markdown 正本と HTML 生成物の整合性を確認する
- HTML 生成物だけを正として仕様判断してはならない
- HTML 生成物と Markdown 正本が乖離した場合は、Markdown 正本を優先し、HTML を再生成して整合させる

## ドキュメント生成ツールの位置づけ

以下は別途リポジトリで開発したドキュメント生成ツールおよび関連文書である。

- `docs/build-scripts/build_spec_v3.py`
- `docs/build-scripts/build_spec_v3_spec.md`
- `docs/build-scripts/DESIGN.md`

Adlaire DB のプロジェクト憲章正本は `docs/PROJECT_CHARTER.md` である。仕様正本群は、上記ツールではなく `docs/spec/core.md`、`docs/spec/auth.md`、`docs/spec/api.md`、`docs/spec/internalization.md`、`docs/spec/testing.md`、`docs/spec/phase-*.md` である。

## 作業開始時の確認

作業開始時は、以下の手順を**必ず**守ること。

1. `AGENTS.md` を読む
2. 読了後に作業を開始してよい
3. `AGENTS.md` の読了と同時に、ローカルブランチとリモートブランチの整合性を確認する
4. 不整合がある場合は、作業開始前に整合性を完了させる
5. 整合性の完了ができない場合は、理由と必要な判断を提示して作業を止める

## 変更承認フロー

仕様書（docs/PROJECT_CHARTER.md）およびその他ファイルへの変更は、
以下の手順を**必ず**守ること。

1. 変更内容（差分・理由）をチャットに提示する
2. ユーザーが「承認」と返答するまで、ファイルを編集しない
3. 「承認」を得てから編集・コミット・プッシュを行う
4. 変更作業の提示には、対象ファイル、変更理由、変更予定内容を含める
5. ユーザーが「承認」と返答した場合、その変更作業は承認済みとして扱う

## PR / マージ運用

`main` への反映は、必ず PR 経由にすること。

- Codex は main へ直接 push しない
- すべての変更は作業ブランチへ commit し、その作業ブランチを push する
- 作業完了時は PR を作成する
- Codex は PR 作成までを担当する
- main へのマージはユーザーが行う
- GitHub remote 設定では、PR merge 後の head branch 自動削除を有効化する

## PR 報告形式

PR を作成・報告する場合は、必ず `PR #番号: タイトル` の形式を主表記にすること。

- 例: `PR #4: Add Docker test environment`
- URL を併記する場合も、主表記は `PR #番号: タイトル` とする
- PR 作成用 URL だけを最終報告にしてはならない
