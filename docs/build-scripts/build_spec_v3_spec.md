# build_spec_v3.py — ビルドスクリプト仕様ドキュメント

**対象スクリプト：** `docs/build-scripts/build_spec_v3.py`
**出力ファイル：** `docs/Adlaire-db-spec.html`
**バージョン：** v3（Adlaire Design System ブルートークン正式採用）
**最終更新：** 2026-09-13
**開発方針：** 仕様駆動開発（Spec-Driven Development）
**デザインシステム：** [Adlaire Design System](https://github.com/fqwink/Adlaire-Design-System)

---

## 1. 概要

`build_spec_v3.py` は、Adlaire DB 仕様憲章の Markdown ソースを単一の自己完結型 HTML ドキュメントへ変換する Python スクリプトである。外部ライブラリに依存せず、標準ライブラリ（`re`、`html`、`unicodedata`、`pathlib`、`typing`）のみで動作する。

### 1.1 目的

- 仕様憲章と Phase 分割ファイルを、快適に閲覧できる単一 HTML ドキュメントサイトへ変換する
- CSS・JS をすべてインラインに埋め込み、単一 HTML ファイルとして配布可能にする
- Adlaire Design System（ADS）のトークンを採用し、一貫したデザイン言語を維持する

### 1.2 開発方針

本プロジェクトは **仕様駆動開発（Spec-Driven Development）** を採用する。

- **仕様書が唯一の真実（Single Source of Truth）**：実装の追加・変更はすべて本ドキュメントへの反映を先行させる
- **仕様→実装の順序**：設計上の決定は本ドキュメントに記録してから `build_spec_v3.py` に反映する
- **仕様との乖離は不整合**：スクリプトの動作が本仕様書と食い違う場合、どちらかに誤りがある
- **仕様書のバージョン管理**：スクリプトの変更履歴と仕様書の改訂履歴は同期して管理する
- **デザイントークンの準拠**：CSS カスタムプロパティ（`--adlaire-*`）はすべて [Adlaire Design System](https://github.com/fqwink/Adlaire-Design-System) の `Tokens/` で定義された値のみを使用する。スクリプト側での独自トークンの追加・変更は行わない

### 1.3 要件

| 項目 | 内容 |
|------|------|
| Python バージョン | 3.9 以上（型ヒント `dict[str, int]`、`list[tuple]` を使用） |
| 外部依存 | **なし** — `re`・`html`・`unicodedata`・`pathlib`・`typing` の標準ライブラリのみ使用。`pip install` 不要 |
| 入力 | UTF-8 エンコードの Markdown ファイル。`docs/spec/spec.md` から `docs/spec/phase-*.md` を include 展開する |
| 出力 | UTF-8 エンコードの単一 HTML ファイル |

---

## 2. ファイルパス設定

スクリプト冒頭の定数で入出力パスを管理する。

```python
SRC = "docs/spec/spec.md"      # 入力 Markdown 仕様憲章
OUT = "docs/Adlaire-db-spec.html"    # 出力 HTML
```

別の環境で実行する場合はこの 2 変数を書き換える。

仕様憲章内の `<!-- include: docs/spec/phase-01.md -->` 形式の行は、HTML 生成前に指定 Markdown ファイルの本文へ展開する。include はリポジトリルートからの相対パスで記述する。

---

## 3. 処理パイプライン

```
┌────────────────────────────────────────────────────────────────┐
│ 1. MDファイル読み込み・include 展開（raw_lines）                 │
├────────────────────────────────────────────────────────────────┤
│ 2. 見出し抽出パス（フェンス内を除外）                           │
│    → headings: list[(level, text, slug, line_number)]          │
│    → slug_by_line: dict[line_number → slug]                    │
├────────────────────────────────────────────────────────────────┤
│ 3. TOC HTML 生成（build_toc()）                                 │
│    → toc_html: str                                             │
├────────────────────────────────────────────────────────────────┤
│ 4. MD → HTML 変換（convert()）                                  │
│    → body_html: str                                            │
├────────────────────────────────────────────────────────────────┤
│ 5. HTML テンプレート合成（PAGE f-string）                        │
│    CSS トークン・レイアウト・JS をすべてインライン埋め込み        │
├────────────────────────────────────────────────────────────────┤
│ 6. ファイル書き出し（OUT）                                       │
└────────────────────────────────────────────────────────────────┘
```

---

## 4. 関数リファレンス

### 4.1 `slugify(text: str) → str`

見出しテキストから HTML アンカー用のスラグ（`id` 属性値）を生成する。

**処理手順：**
1. Markdown 記法文字（`` ` * _ ~ [ ] ``）を除去
2. 文字を 1 文字ずつ走査し、空白・ハイフン・ドット・アンダースコアは `-` に変換、英数字・Unicode 文字（カテゴリ `L*`、`N*`）はそのまま保持、それ以外は除去
3. 連続する `-` を 1 つに正規化、前後の `-` をトリム
4. 空文字列になった場合は `section` にフォールバック
5. 同一スラグが複数回出現した場合、2 回目以降は `-1`、`-2` と連番サフィックスを付与

**グローバル状態：** `_seen: dict[str, int]` — 同一スラグの出現回数を追跡

**注意：** `_seen` はモジュールレベルのグローバル変数であり、スクリプト実行中に状態が蓄積される。スクリプトを複数回 `import` して使用する場合、`_seen` をリセットする必要がある。

---

### 4.2 `esc(s: str) → str`

`html.escape(s, quote=True)` のラッパー。HTML 特殊文字（`<`、`>`、`&`、`"`、`'`）をエスケープする。

---

### 4.3 `inline(text: str) → str`

インライン Markdown 記法を HTML に変換する。コードスパンを先にキャラクターレベルのスキャンで処理することで、後続の正規表現がコードスパン内の記法を誤って変換するのを防ぐ。

**処理手順：**

1. **コードスパン抽出（文字列スキャン）**
   - バッククォート（`` ` ``）の位置を順次検索
   - `` `` `` で始まる場合はダブルバッククォートコードスパンとして処理
   - 単一バッククォートの場合はシングルバッククォートコードスパンとして処理
   - コードスパン内テキストは `esc()` した上で `<code class="ic">` でラップ
   - コードスパン外テキストは `esc()` してそのままセグメントに追加

2. **インライン記法の正規表現置換**（コードスパン処理後のテキストに適用）

| パターン | 出力 |
|---------|------|
| `***text***` | `<strong><em>text</em></strong>` |
| `**text**` | `<strong>text</strong>` |
| `*text*`（単独 `*`） | `<em>text</em>` |
| `__text__` | `<strong>text</strong>` |
| `_text_`（単独 `_`） | `<em>text</em>` |
| `~~text~~` | `<del>text</del>` |
| `[label](url)` | `<a href="url">label</a>` |

**制約：** ネストしたインライン記法（`**_text_**` など）は限定的にサポート。画像記法（`![alt](url)`）は未対応。

---

### 4.4 `build_toc(headings) → str`

見出しリストから TOC（目次）の HTML を生成する。h1〜h3 のみを対象とし（h4 は除外）、子見出しを持つ見出しはグループとしてアコーディオン形式に構築する。

**引数：** `headings: list[tuple[int, str, str, int]]` — `(level, text, slug, line_number)` のリスト

**出力：** TOC の `<li>` 要素群の HTML 文字列（`<ul>` ルートは HTML テンプレート側で定義）

**グループ（`.tg`）とリーフ（`.ti`）の判定：**
現在の見出しの次の見出しレベルが現在より深い場合、グループとして扱い、展開ボタン（`.tg-btn`）付きの `<ul>` をネストする。それ以外はリーフ（`.ti`）として `<li>` 1 個を出力する。

**スタック管理：** 内部スタック `stack: list[tuple[int, str]]` でグループの開閉を追跡し、`close_to(target_lv)` で不要になった `</ul></li>` を閉じる。

**生成 HTML 構造（グループの場合）：**
```html
<li class="tg">
  <div class="tg-row">
    <a href="#slug" class="tl lv1" data-slug="slug">見出しテキスト</a>
    <button class="tg-btn" aria-expanded="false" data-target="tg-slug" aria-label="展開">
      <svg>…</svg>
    </button>
  </div>
  <ul id="tg-slug" class="tc" hidden>
    …子アイテム…
  </ul>
</li>
```

**生成 HTML 構造（リーフの場合）：**
```html
<li class="ti">
  <a href="#slug" class="tl lv1" data-slug="slug">見出しテキスト</a>
</li>
```

---

### 4.5 `convert(lines, slug_by_line) → str`

Markdown の行リストを走査し、HTML コンテンツ文字列を生成するメインコンバーター。

**引数：**
- `lines: list[str]` — Markdown の全行
- `slug_by_line: dict[int, str]` — 行番号から見出しスラグへのマッピング

**内部バッファと状態変数：**

| 変数 | 型 | 用途 |
|------|-----|------|
| `out` | `list[str]` | 出力 HTML 断片の蓄積 |
| `fence_active` | `bool` | コードフェンス内かどうか |
| `fence_lang` | `str` | コードフェンスの言語識別子 |
| `fence_buf` | `list[str]` | フェンス内の行バッファ |
| `fence_marker` | `str` | フェンス開始マーカー（`` ``` `` or `~~~`） |
| `para_buf` | `list[str]` | 段落テキストの行バッファ |
| `list_stack` | `list[tuple[str, int]]` | リストのネスト状態スタック |
| `table_buf` | `list[str]` | テーブル行のバッファ |

**内部ヘルパー関数（`convert` のクロージャ）：**

- `flush_para()` — `para_buf` を `<p class="mp">` として出力し、バッファをクリア
- `flush_list()` — `list_stack` を巻き戻し、すべての `</ul>` / `</ol>` を閉じる
- `flush_table()` — `table_buf` をパースし `<div class="tw"><table class="mt">` として出力
- `emit_code()` — `fence_buf` を `<div class="cb-wrap">` 構造として出力

**ブロック要素の検出優先順位（1 行ずつ処理）：**

1. フェンスコードブロック開始・終了（`` ``` `` / `~~~` で始まる行）
2. 見出し（`#` で始まる行）
3. 水平線（`---`、`***`、`___`）
4. テーブル（`|` で始まる行、連続する `|` 行をまとめて処理）
5. 引用（`>` で始まる行）
6. リスト項目（`- * +` または `1.` 形式）
7. 空行（バッファのフラッシュトリガー）
8. 段落（上記以外の非空行、連続行を 1 つの `<p>` にまとめる）

**テーブル変換の詳細：**
セパレーター行（`:---:`、`---` などで構成された行）のインデックスを自動検出し、セパレーター行より前の行をヘッダー（`<th>`）、それ以降を本文（`<td>`）として出力する。セパレーター行自体は出力しない。

**フェンスコードブロックの未閉鎖フォールバック：**
ファイル末尾まで読んだ時点で `fence_active` が `True` のままの場合（閉じる `` ``` `` がない場合）、`emit_code()` を呼び出して強制出力する。

---

### 4.6 `emit_code()` （`convert` 内クロージャ）

フェンスコードブロックを HTML に変換して `out` に追記する。

**出力 HTML 構造：**
```html
<div class="cb-wrap" data-lang="rust">
  <div class="cb-meta">
    <span class="cl">rust</span>
    <button class="cb-copy" aria-label="コピー">コピー</button>
  </div>
  <pre class="cb"><code>…エスケープ済みコード…</code></pre>
</div>
```

- `data-lang` 属性：言語識別子（なければ空文字列）
- `.cl`（言語ラベル）：言語識別子がない場合は出力しない
- コード内容：`\n`.join(fence_buf) を `esc()` でエスケープしてから出力

---

## 5. HTML 出力構造

```html
<!DOCTYPE html>
<html lang="ja">
<head>
  <!-- インライン CSS（ADS トークン + レイアウト + コンポーネント） -->
</head>
<body>
  <header id="hdr">        <!-- 固定ヘッダー（高さ 52px、背景 --adlaire-surface-accent） -->
  <div id="lay">           <!-- フレックスコンテナ -->
    <nav id="sb">          <!-- サイドバー（幅 260px、固定） -->
      <div class="sb-search-wrap">
        <input id="sb-search">  <!-- TOC 検索 -->
      </div>
      <div id="sb-toc">
        <ul class="tr" id="toc-root">  <!-- TOC リスト -->
      </div>
    </nav>
    <main id="ct">         <!-- コンテンツエリア -->
      <div class="ci">     <!-- 最大幅 760px センタリングコンテナ -->
        {body_html}
      </div>
    </main>
  </div>
  <button id="btt">        <!-- トップへ戻るボタン -->
  <script>…インライン JS…</script>
</body>
</html>
```

---

## 6. CSS クラス一覧

### コンテンツ要素

| クラス | 要素 | 説明 |
|--------|------|------|
| `.mh` | `h1`〜`h4` | 見出し共通スタイル |
| `.h1`〜`.h4` | `h1`〜`h4` | 見出しレベル別スタイル |
| `.mp` | `p` | 本文段落（`max-width: 68ch`） |
| `.mr` | `hr` | 水平線 |
| `.mbq` | `blockquote` | 引用ブロック |
| `.ic` | `code` | インラインコード |
| `.ml` | `ul`/`ol` | リスト |

### コードブロック

| クラス | 説明 |
|--------|------|
| `.cb-wrap` | コードブロック外枠（`position: relative`） |
| `.cb-meta` | 言語ラベル＋コピーボタンのオーバーレイ（`position: absolute; top: 8px; right: 10px`） |
| `.cl` | 言語ラベル（`var(--adlaire-font-family-mono)`、大文字、`var(--adlaire-font-size-xs)`） |
| `.cb-copy` | コピーボタン（ホバーまで非表示） |
| `.cb-copy.copied` | コピー完了状態（1.8 秒間） |
| `.cb` | `<pre>` 要素（背景 `var(--adlaire-surface-soft-strong)`） |

### テーブル

| クラス | 説明 |
|--------|------|
| `.tw` | テーブルラッパー（`overflow-x: auto`） |
| `.mt` | `<table>` 要素 |

### TOC（サイドバー）

| クラス | 説明 |
|--------|------|
| `.tr` | TOC ルート `<ul>` |
| `.tg` | グループ（子を持つ見出し） |
| `.tg-row` | グループの行（リンク＋展開ボタン） |
| `.tg-btn` | グループ展開トグルボタン |
| `.tc` | グループの子 `<ul>`（`hidden` で折りたたみ） |
| `.ti` | リーフ（子を持たない見出し） |
| `.tl` | TOC リンク（`<a>` 要素） |
| `.tl.lv1`〜`.tl.lv3` | TOC リンクのレベル別スタイル |
| `.tl.active` | アクティブな TOC リンク |
| `.sb-none` | 検索結果なしメッセージ |

---

## 7. JavaScript 機能

### 7.1 テーマ切り替え（廃止）

ADS 採用により、ダークモードおよびテーマトグルボタンは廃止。
出力 HTML はライトモード固定（`prefers-color-scheme` 非対応）。

### 7.2 サイドバー開閉

`localStorage` キー `adb-sb` に `"1"`（開）または `"0"`（閉）を保存する。

- デスクトップ（`> 768px`）：`#sb.closed` / CSS `margin-left` で幅を制御
- モバイル（`≤ 768px`）：`#sb.open` / `transform: translateX` で画面外から引き出す
- モバイルでは TOC リンククリック時に自動的にサイドバーを閉じる

### 7.3 TOC グループ展開

`.tg-btn` クリックで対応する `<ul id="tg-{slug}">` の `hidden` 属性をトグルし、`aria-expanded` 属性を更新する。`data-target` 属性で対象 `<ul>` の ID を指定する。

### 7.4 TOC 検索フィルター

`#sb-search` の `input` イベントで TOC 項目をリアルタイムフィルタリングする。

- 各 `.tl` のテキストコンテンツと検索クエリ（小文字化）を照合
- 非一致の `<li>` は `hidden = true` で非表示
- グループは子に一致項目があれば表示を維持し、`<ul>` を強制展開
- 入力がクリアされた場合は全項目を復元し、展開状態を `aria-expanded` に従って復元
- 一致なしの場合は `#sb-none` メッセージを表示

**状態変数 `searchActive`：** 検索中はアクティブ TOC リンクの自動スクロールを抑制する。

### 7.5 アクティブ見出し追跡

`IntersectionObserver` で `.mh[id]` 要素のビューポート内への進入を監視する。

- `rootMargin: "-8% 0px -78% 0px"` — 画面上部 8% 〜 22% の帯域内の見出しを「アクティブ」とみなす
- 進入した見出しの `id` に対応する TOC リンクに `.active` クラスを付与
- 親グループが折りたたまれている場合は自動展開し `aria-expanded="true"` を設定
- 検索中でない場合は対応 TOC リンクを `scrollIntoView` で可視範囲にスクロール

### 7.6 コピーボタン

各 `.cb-copy` ボタンのクリックで、親 `.cb-wrap` 内の `<code>` の `innerText` をクリップボードにコピーする。

```
navigator.clipboard.writeText() → 成功: done()
                                 → 失敗: fallbackCopy() → done()
fallbackCopy(): <textarea> を一時生成して execCommand('copy') を実行
done(): ボタンテキストを "✓ 完了" に変更、.copied クラス付与、1.8 秒後に "コピー" に戻す
```

### 7.7 トップへ戻るボタン

`scroll` イベント（`passive: true`）で `scrollY > 400` の場合に `#btt.visible` クラスを付与する。クリックで `window.scrollTo({ top: 0, behavior: 'smooth' })` を実行する。

---

## 8. カスタマイズポイント

### 8.1 デザイントークンの変更

> ⚠️ **準拠義務：** `--adlaire-*` トークンの値は [Adlaire Design System](https://github.com/fqwink/Adlaire-Design-System)（`Tokens/` ディレクトリ）で定義された値に準拠すること。スクリプト側での独自値への変更・追加は行わない。

スクリプト内 `PAGE` f-string の `:root { }` 内 `--adlaire-*` トークンブロックを編集して再実行する。
ライトモード固定のため、ダークモード用ブロックは不要。

主なトークンの例：

```css
--adlaire-surface-accent:  #0066cc;  /* ヘッダー背景色 */
--adlaire-surface-page:    #f5f5f5;  /* ページ背景色 */
--adlaire-surface-card:    #ffffff;  /* カード背景色 */
```

### 8.2 入出力パスの変更

```python
SRC = "path/to/input.md"
OUT  = "path/to/output.html"
```

### 8.3 ドキュメントタイトル・バージョンの変更

HTML テンプレートの `PAGE` f-string 内を直接編集する。

```python
<title>Adlaire DB 仕様憲章</title>          # ← ブラウザタブタイトル
<span class="hdr-title">Adlaire DB</span>  # ← ヘッダー表示名
<span class="hdr-ver">V.205</span>         # ← バージョンバッジ
```

### 8.4 TOC の対象見出しレベル

`build_toc()` 内の以下の行でフィルタリングレベルを変更できる（現在は h1〜h3）。

```python
vis = [(lv, tx, sl) for lv, tx, sl, _ in headings if lv <= 3]
```

---

## 9. 実行方法

```bash
python3 docs/build-scripts/build_spec_v3.py
```

**標準出力：**
```
Converting MD...
Building TOC...
Assembling HTML...
Done -> docs/Adlaire-db-spec.html  (1,713,731 bytes / 1,673 KB)
```

**再実行時の注意：** `slugify()` が参照するグローバル変数 `_seen` はスクリプト起動時にリセットされるため、複数回実行しても出力は同一になる。

---

## 10. 既知の制限

| 項目 | 内容 |
|------|------|
| 画像記法 | `![alt](url)` は未対応（リンクとして処理される） |
| ネスト引用 | `>> 二重引用` は未対応 |
| タスクリスト | `- [ ] チェックボックス` は通常のリストとして処理 |
| 脚注 | `[^1]` 形式の脚注は未対応 |
| 定義リスト | `:` 形式の定義リストは未対応 |
| シンタックスハイライト | 意図的に非実装（コードは単色テキスト） |
| フェンス検出パターンの差異 | 見出し抽出フェーズは `` ^```\|^~~~ ``（バッククォート数不問）、`convert()` は `` ^(`{3,}\|~{3,}) ``（3 個以上）を使用。前者はバッククォート 2 個以下の行にも誤マッチする可能性あり |
