#!/usr/bin/env python3
"""Adlaire DB spec HTML builder — v3 (ADS tokens, blue variant).

Design tokens conform to Adlaire Design System:
https://github.com/fqwink/Adlaire-Design-System (Tokens/)
--adlaire-* トークンはデザインシステム定義値のみを使用。スクリプト側での独自追加・変更は行わない。
"""

import re, html, unicodedata
from pathlib import Path
from typing import Optional

SRC = "docs/spec/index.md"
OUT = "docs/Adlaire-db-spec.html"

INCLUDE_RE = re.compile(r'^<!--\s*include:\s*([^>]+?)\s*-->\s*$')

# ─── helpers ─────────────────────────────────────────────────────────────────
_seen: dict[str, int] = {}

def slugify(text: str) -> str:
    t = re.sub(r'[`*_~\[\]]', '', text).strip()
    out = []
    for ch in t:
        cat = unicodedata.category(ch)
        if ch in ' -_.':
            out.append('-')
        elif ch.isalnum() or cat.startswith('L') or cat.startswith('N'):
            out.append(ch)
    s = re.sub(r'-+', '-', ''.join(out)).strip('-') or 'section'
    n = _seen.get(s, 0)
    _seen[s] = n + 1
    return s if n == 0 else f"{s}-{n}"

def esc(s: str) -> str:
    return html.escape(s, quote=True)

# ─── inline MD → HTML ────────────────────────────────────────────────────────
def inline(text: str) -> str:
    segments: list[str] = []
    i = 0
    while i < len(text):
        bt = text.find('`', i)
        if bt == -1:
            segments.append(esc(text[i:]))
            break
        segments.append(esc(text[i:bt]))
        if text[bt:bt+2] == '``':
            end = text.find('``', bt + 2)
            if end == -1:
                segments.append(esc(text[bt:]))
                break
            segments.append(f'<code class="ic">{esc(text[bt+2:end])}</code>')
            i = end + 2
        else:
            end = text.find('`', bt + 1)
            if end == -1:
                segments.append(esc(text[bt:]))
                break
            segments.append(f'<code class="ic">{esc(text[bt+1:end])}</code>')
            i = end + 1
    t = ''.join(segments)
    t = re.sub(r'\*\*\*(.+?)\*\*\*', r'<strong><em>\1</em></strong>', t)
    t = re.sub(r'\*\*(.+?)\*\*',     r'<strong>\1</strong>', t)
    t = re.sub(r'(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)', r'<em>\1</em>', t)
    t = re.sub(r'__(.+?)__', r'<strong>\1</strong>', t)
    t = re.sub(r'(?<!_)_(?!_)(.+?)(?<!_)_(?!_)', r'<em>\1</em>', t)
    t = re.sub(r'~~(.+?)~~', r'<del>\1</del>', t)
    t = re.sub(r'\[([^\]]+)\]\(([^)]+)\)', r'<a href="\2">\1</a>', t)
    return t

# ─── source loading ──────────────────────────────────────────────────────────
def load_markdown_lines(path: str, seen: Optional[set[str]] = None) -> list[str]:
    seen = seen or set()
    source = Path(path)
    key = str(source)
    if key in seen:
        raise RuntimeError(f'Circular include detected: {path}')
    seen.add(key)

    expanded: list[str] = []
    with source.open(encoding='utf-8') as f:
        for line in f:
            m = INCLUDE_RE.match(line.rstrip('\n'))
            if m:
                include_path = m.group(1).strip()
                expanded.extend(load_markdown_lines(include_path, seen.copy()))
            else:
                expanded.append(line)
    return expanded

# ─── parse headings ──────────────────────────────────────────────────────────
raw_lines = load_markdown_lines(SRC)

headings: list[tuple[int, str, str, int]] = []
_fence = False
for li, line in enumerate(raw_lines):
    s = line.rstrip('\n')
    if re.match(r'^```|^~~~', s):
        _fence = not _fence
        continue
    if _fence:
        continue
    m = re.match(r'^(#{1,4})\s+(.*)', s)
    if m:
        lv = len(m.group(1))
        tx = m.group(2).strip()
        headings.append((lv, tx, slugify(tx), li))

slug_by_line = {li: sl for _, _, sl, li in headings}

# ─── TOC HTML ────────────────────────────────────────────────────────────────
def build_toc(headings):
    lines: list[str] = []
    vis = [(lv, tx, sl) for lv, tx, sl, _ in headings if lv <= 3]
    stack: list[tuple[int, str]] = []

    def close_to(target_lv: int):
        while stack and stack[-1][0] >= target_lv:
            lv, kind = stack.pop()
            if kind == 'group':
                lines.append('</ul></li>')

    for i, (lv, tx, sl) in enumerate(vis):
        tx_safe = esc(tx)
        link = f'<a href="#{sl}" class="tl lv{lv}" data-slug="{sl}">{tx_safe}</a>'
        next_lv = vis[i+1][0] if i+1 < len(vis) else 0
        is_parent = next_lv > lv
        close_to(lv)
        if is_parent:
            lines.append(
                f'<li class="tg">'
                f'<div class="tg-row">'
                f'{link}'
                f'<button class="tg-btn" aria-expanded="false" data-target="tg-{sl}" aria-label="展開">'
                f'<svg viewBox="0 0 10 10" width="10" height="10"><polyline points="2,3 5,7 8,3"'
                f' stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/></svg>'
                f'</button>'
                f'</div>'
                f'<ul id="tg-{sl}" class="tc" hidden>'
            )
            stack.append((lv, 'group'))
        else:
            lines.append(f'<li class="ti">{link}</li>')
            stack.append((lv, 'item'))

    while stack:
        _, kind = stack.pop()
        if kind == 'group':
            lines.append('</ul></li>')

    return '\n'.join(lines)

toc_html = build_toc(headings)

# ─── MD → HTML converter ─────────────────────────────────────────────────────
def convert(lines, slug_by_line):
    out: list[str] = []
    i = 0
    n = len(lines)

    fence_active = False
    fence_lang = ''
    fence_buf: list[str] = []
    fence_marker = ''

    para_buf: list[str] = []
    list_stack: list[tuple[str, int]] = []
    table_buf: list[str] = []

    def flush_para():
        if para_buf:
            out.append(f'<p class="mp">{inline(" ".join(para_buf))}</p>')
            para_buf.clear()

    def flush_list():
        while list_stack:
            tag, _ = list_stack.pop()
            out.append(f'</{tag}>')

    def flush_table():
        if not table_buf:
            return
        rows = table_buf[:]
        table_buf.clear()
        sep_idx = next((j for j, r in enumerate(rows)
                        if all(re.match(r'^\s*:?-+:?\s*$', c.strip())
                               for c in r.strip('|').split('|') if c.strip())), None)
        out.append('<div class="tw"><table class="mt">')
        for j, row in enumerate(rows):
            if j == sep_idx:
                continue
            is_head = sep_idx is not None and j < sep_idx
            tag = 'th' if is_head else 'td'
            cells = [c.strip() for c in row.strip().strip('|').split('|')]
            out.append('<tr>' + ''.join(f'<{tag}>{inline(c)}</{tag}>' for c in cells) + '</tr>')
        out.append('</table></div>')

    def emit_code():
        code = esc('\n'.join(fence_buf))
        lang = esc(fence_lang) if fence_lang else ''
        lang_label = f'<span class="cl">{lang}</span>' if lang else ''
        copy_btn = '<button class="cb-copy" aria-label="コピー">コピー</button>'
        meta = f'<div class="cb-meta">{lang_label}{copy_btn}</div>'
        out.append(f'<div class="cb-wrap" data-lang="{lang}">{meta}<pre class="cb"><code>{code}</code></pre></div>')

    while i < n:
        raw = lines[i].rstrip('\n')
        stripped = raw.strip()

        # fenced code block
        fence_m = re.match(r'^(`{3,}|~{3,})(.*)', raw)
        if not fence_active and fence_m:
            flush_para()
            flush_list()
            flush_table()
            fence_active = True
            fence_marker = fence_m.group(1)
            fence_lang = fence_m.group(2).strip()
            i += 1
            continue
        if fence_active:
            if raw.startswith(fence_marker) and raw.strip() == fence_marker:
                emit_code()
                fence_active = False
                fence_buf = []
                fence_lang = ''
                fence_marker = ''
            else:
                fence_buf.append(raw)
            i += 1
            continue

        # heading
        hm = re.match(r'^(#{1,4})\s+(.*)', raw)
        if hm:
            flush_para()
            flush_list()
            flush_table()
            lv = len(hm.group(1))
            tx = hm.group(2).strip()
            sl = slug_by_line.get(i, '')
            id_attr = f' id="{sl}"' if sl else ''
            out.append(f'<h{lv}{id_attr} class="mh h{lv}">{inline(tx)}</h{lv}>')
            i += 1
            continue

        # horizontal rule
        if re.match(r'^(---+|\*\*\*+|___+)$', stripped):
            flush_para()
            flush_list()
            flush_table()
            out.append('<hr class="mr">')
            i += 1
            continue

        # table
        if stripped.startswith('|'):
            flush_para()
            flush_list()
            table_buf.append(stripped)
            j = i + 1
            while j < n and lines[j].strip().startswith('|'):
                table_buf.append(lines[j].strip())
                j += 1
            flush_table()
            i = j
            continue

        # blockquote
        if stripped.startswith('>'):
            flush_para()
            flush_list()
            content = re.sub(r'^>\s?', '', stripped)
            out.append(f'<blockquote class="mbq">{inline(content)}</blockquote>')
            i += 1
            continue

        # list item
        ul_m = re.match(r'^(\s*)([-*+])\s+(.*)', raw)
        ol_m = re.match(r'^(\s*)(\d+)[.)]\s+(.*)', raw)
        lm = ul_m or ol_m
        if lm:
            flush_para()
            flush_table()
            indent = len(lm.group(1))
            tag = 'ul' if ul_m else 'ol'
            content = lm.group(3)
            if not list_stack:
                out.append(f'<{tag} class="ml">')
                list_stack.append((tag, indent))
            else:
                cur_tag, cur_indent = list_stack[-1]
                if indent > cur_indent:
                    out.append(f'<{tag} class="ml">')
                    list_stack.append((tag, indent))
                elif indent < cur_indent:
                    while list_stack and list_stack[-1][1] > indent:
                        t, _ = list_stack.pop()
                        out.append(f'</{t}>')
            out.append(f'<li>{inline(content)}</li>')
            i += 1
            continue

        # blank line
        if not stripped:
            flush_para()
            if list_stack:
                flush_list()
            i += 1
            continue

        # paragraph
        flush_list()
        flush_table()
        para_buf.append(stripped)
        j = i + 1
        while j < n:
            nxt = lines[j].strip()
            if (nxt
                    and not nxt.startswith('#')
                    and not nxt.startswith('|')
                    and not nxt.startswith('```')
                    and not nxt.startswith('~~~')
                    and not nxt.startswith('>')
                    and not re.match(r'^\s*[-*+]\s', lines[j])
                    and not re.match(r'^\s*\d+[.)]\s', lines[j])
                    and not re.match(r'^(---+|\*\*\*+|___+)$', nxt)):
                para_buf.append(nxt)
                j += 1
            else:
                break
        flush_para()
        i = j

    flush_para()
    flush_list()
    flush_table()
    if fence_active and fence_buf:
        emit_code()

    return '\n'.join(out)

print("Converting MD...")
body_html = convert(raw_lines, slug_by_line)
print("Building TOC...")
print("Assembling HTML...")

PAGE = f'''<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Adlaire DB 仕様正本</title>
<style>
/* ══ ADLAIRE DESIGN SYSTEM TOKENS ══════════════════════════════════════════
   Conforms to: https://github.com/fqwink/Adlaire-Design-System  (Tokens/)
   --adlaire-* トークンはデザインシステム定義値のみを使用すること
   ══════════════════════════════════════════════════════════════════════════ */
:root {{
  /* colors */
  --adlaire-color-agws-blue-primary:   #0066cc;
  --adlaire-color-agws-blue-secondary: #0055aa;
  --adlaire-color-agws-blue-accent:    #004499;
  --adlaire-color-agws-green-primary:  #00a968;
  --adlaire-color-primary:   var(--adlaire-color-agws-blue-primary);
  --adlaire-color-secondary: var(--adlaire-color-agws-blue-secondary);
  --adlaire-color-accent:    var(--adlaire-color-agws-blue-accent);

  /* surface */
  --adlaire-surface-accent:       #0066cc;
  --adlaire-surface-accent-mid:   #0055aa;
  --adlaire-surface-accent-strong:#004499;
  --adlaire-surface-page:         #f5f5f5;
  --adlaire-surface-card:         #ffffff;
  --adlaire-surface-soft:         #f0f7ff;
  --adlaire-surface-soft-strong:  #e8f2ff;
  --adlaire-surface-border:       #e0e0e0;
  --adlaire-surface-text:         #333333;
  --adlaire-surface-text-muted:   #555555;
  --adlaire-surface-text-subtle:  #666666;
  --adlaire-surface-notice:       #ff9800;
  --adlaire-surface-notice-soft:  #fff3cd;
  --adlaire-surface-notice-text:  #856404;

  /* typography */
  --adlaire-font-family-base: "Helvetica Neue", Helvetica, Arial, sans-serif;
  --adlaire-font-family-mono: "JetBrains Mono", "Courier New", Courier, monospace;
  --adlaire-font-size-xs:  0.75rem;
  --adlaire-font-size-sm:  0.875rem;
  --adlaire-font-size-md:  1rem;
  --adlaire-font-size-lg:  1.125rem;
  --adlaire-font-size-xl:  1.5rem;
  --adlaire-font-size-2xl: 2rem;
  --adlaire-line-height-tight:   1.25;
  --adlaire-line-height-base:    1.6;
  --adlaire-line-height-relaxed: 1.8;
  --adlaire-font-weight-normal:   400;
  --adlaire-font-weight-medium:   500;
  --adlaire-font-weight-semibold: 600;
  --adlaire-font-weight-bold:     700;

  /* spacing */
  --adlaire-space-0:  0;
  --adlaire-space-1:  0.25rem;
  --adlaire-space-2:  0.5rem;
  --adlaire-space-3:  0.75rem;
  --adlaire-space-4:  1rem;
  --adlaire-space-5:  1.25rem;
  --adlaire-space-6:  1.5rem;
  --adlaire-space-8:  2rem;
  --adlaire-space-10: 2.5rem;
  --adlaire-space-12: 3rem;

  /* effects */
  --adlaire-radius-sm:    4px;
  --adlaire-radius-md:    6px;
  --adlaire-radius-lg:    8px;
  --adlaire-radius-round: 50%;
  --adlaire-shadow-card:        0 2px 8px rgba(0,0,0,.1);
  --adlaire-shadow-card-hover:  0 4px 16px rgba(0,0,0,.15);
  --adlaire-shadow-button:      0 2px 8px rgba(0,0,0,.15);
  --adlaire-shadow-header:      0 2px 10px rgba(0,0,0,.1);
  --adlaire-shadow-focus-ring:  0 0 0 3px rgba(0,102,204,.1);
  --adlaire-shadow-blue-soft:   0 2px 8px rgba(0,102,204,.15);
  --adlaire-shadow-blue:        0 2px 8px rgba(0,102,204,.3);
  --adlaire-shadow-blue-hover:  0 4px 12px rgba(0,102,204,.5);
  --adlaire-transition-fast:    0.15s ease-in-out;
  --adlaire-transition-button:  0.2s ease-in-out;
  --adlaire-transition-base:    0.3s ease;
  --adlaire-z-sticky:    100;
  --adlaire-z-page-top: 1000;

  /* layout */
  --adlaire-layout-container:        1200px;
  --adlaire-layout-container-narrow: 760px;
  --adlaire-layout-sidebar:          300px;
  --adlaire-layout-sidebar-compact:  260px;
  --adlaire-layout-gap:       2rem;
  --adlaire-layout-gutter:    1.5rem;

  /* page structure */
  --sw: var(--adlaire-layout-sidebar-compact);
  --hh: 52px;
}}

/* ══ RESET ════════════════════════════════════════════════════════════════ */
*,*::before,*::after {{ box-sizing: border-box; margin: 0; padding: 0 }}
html {{ font-size: 16px; scroll-behavior: smooth }}
body {{
  background: var(--adlaire-surface-page);
  color: var(--adlaire-surface-text);
  font-family: var(--adlaire-font-family-base);
  font-size: var(--adlaire-font-size-md);
  line-height: var(--adlaire-line-height-relaxed);
}}
a {{ color: var(--adlaire-color-primary); text-decoration: none }}
a:hover {{ text-decoration: underline }}
a:focus-visible {{
  outline: 2px solid var(--adlaire-color-primary);
  outline-offset: 2px;
  border-radius: var(--adlaire-radius-sm);
}}

/* ══ HEADER ══════════════════════════════════════════════════════════════ */
#hdr {{
  position: fixed; top: 0; left: 0; right: 0; height: var(--hh);
  background: var(--adlaire-surface-accent);
  box-shadow: var(--adlaire-shadow-header);
  display: flex; align-items: center; gap: var(--adlaire-space-3);
  padding: 0 var(--adlaire-space-5);
  z-index: var(--adlaire-z-page-top);
}}
#sb-btn {{
  background: none; border: none; cursor: pointer;
  color: rgba(255,255,255,.85);
  padding: var(--adlaire-space-2) var(--adlaire-space-3);
  border-radius: var(--adlaire-radius-md);
  display: flex; align-items: center; gap: var(--adlaire-space-2);
  font: var(--adlaire-font-weight-medium) var(--adlaire-font-size-sm) var(--adlaire-font-family-base);
  transition: background var(--adlaire-transition-fast); flex-shrink: 0;
}}
#sb-btn:hover {{ background: rgba(255,255,255,.12); color: #fff }}
#sb-btn svg {{ width: 16px; height: 16px }}
.hdr-title {{
  font-family: var(--adlaire-font-family-base);
  font-size: var(--adlaire-font-size-sm);
  font-weight: var(--adlaire-font-weight-semibold);
  color: rgba(255,255,255,.95);
  letter-spacing: .01em;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}}
.hdr-sep {{ color: rgba(255,255,255,.4); font-size: var(--adlaire-font-size-xs) }}
.hdr-ver {{
  font-family: var(--adlaire-font-family-mono);
  font-size: var(--adlaire-font-size-xs);
  font-weight: var(--adlaire-font-weight-normal);
  background: rgba(255,255,255,.2);
  color: #fff;
  border-radius: var(--adlaire-radius-md);
  padding: 2px var(--adlaire-space-3);
  border: 1px solid rgba(255,255,255,.28);
  white-space: nowrap;
}}

/* ══ LAYOUT ══════════════════════════════════════════════════════════════ */
#lay {{ display: flex; margin-top: var(--hh); min-height: calc(100vh - var(--hh)) }}

/* ══ SIDEBAR ══════════════════════════════════════════════════════════════ */
#sb {{
  width: var(--sw); flex-shrink: 0;
  position: fixed; top: var(--hh); left: 0; bottom: 0;
  background: var(--adlaire-surface-card);
  border-right: 1px solid var(--adlaire-surface-border);
  box-shadow: var(--adlaire-shadow-blue-soft);
  display: flex; flex-direction: column;
  z-index: var(--adlaire-z-sticky);
  transition: transform var(--adlaire-transition-base);
}}
#sb.closed {{ transform: translateX(calc(-1 * var(--sw))) }}

.sb-search-wrap {{
  padding: var(--adlaire-space-3) var(--adlaire-space-3) var(--adlaire-space-2);
  border-bottom: 1px solid var(--adlaire-surface-border);
  flex-shrink: 0;
}}
#sb-search {{
  width: 100%;
  padding: var(--adlaire-space-2) var(--adlaire-space-3) var(--adlaire-space-2) 30px;
  background: var(--adlaire-surface-page);
  border: 1px solid var(--adlaire-surface-border);
  border-radius: var(--adlaire-radius-md);
  color: var(--adlaire-surface-text);
  font: var(--adlaire-font-weight-normal) var(--adlaire-font-size-sm) var(--adlaire-font-family-base);
  outline: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='14' height='14' fill='none' stroke='%23999' stroke-width='1.5' viewBox='0 0 24 24'%3E%3Ccircle cx='11' cy='11' r='7'/%3E%3Cpath d='M21 21l-4-4'/%3E%3C/svg%3E");
  background-repeat: no-repeat; background-position: 9px center; background-size: 14px;
  transition: border-color var(--adlaire-transition-fast);
}}
#sb-search:focus {{ border-color: var(--adlaire-color-primary); box-shadow: var(--adlaire-shadow-focus-ring) }}
#sb-search::placeholder {{ color: var(--adlaire-surface-text-subtle) }}

#sb-toc {{
  flex: 1; overflow-y: auto; overflow-x: hidden;
  padding: var(--adlaire-space-2) 0 var(--adlaire-space-10);
  scrollbar-width: thin; scrollbar-color: var(--adlaire-surface-border) transparent;
}}

.tr {{ list-style: none; margin: 0; padding: 0 }}
.tg, .ti {{ list-style: none }}
.tg-row {{
  display: flex; align-items: stretch;
  border-left: 2px solid transparent;
  transition: border-color var(--adlaire-transition-fast);
}}
.tg-row:hover {{ border-left-color: var(--adlaire-color-primary) }}
.tg-btn {{
  flex-shrink: 0; width: 28px;
  background: none; border: none; cursor: pointer;
  color: var(--adlaire-surface-text-subtle);
  display: flex; align-items: center; justify-content: center;
  padding: 0; transition: color var(--adlaire-transition-fast);
}}
.tg-btn:hover {{ color: var(--adlaire-color-primary) }}
.tg-btn svg {{ transform: rotate(-90deg); transition: transform .18s }}
.tg-btn[aria-expanded="true"] svg {{ transform: rotate(0deg) }}
.tc {{ list-style: none; margin: 0; padding: 0 }}
.tc[hidden] {{ display: none }}

.tl {{
  display: block; flex: 1;
  padding: 3px var(--adlaire-space-3) 3px 14px;
  font-size: var(--adlaire-font-size-sm); line-height: 1.5;
  color: var(--adlaire-surface-text-muted);
  border-left: 2px solid transparent;
  transition: color var(--adlaire-transition-fast), background var(--adlaire-transition-fast), border-color var(--adlaire-transition-fast);
  text-decoration: none; word-break: break-all;
}}
.tl.lv1 {{ font-weight: var(--adlaire-font-weight-semibold); color: var(--adlaire-surface-text); padding-left: 14px }}
.tl.lv2 {{ padding-left: 20px; font-size: 0.8125rem }}
.tl.lv3 {{ padding-left: 34px; font-size: var(--adlaire-font-size-xs); color: var(--adlaire-surface-text-subtle) }}
.tl:hover, .tg-row:hover .tl {{
  color: var(--adlaire-color-primary);
  background: var(--adlaire-surface-soft);
}}
.tl.active {{
  color: var(--adlaire-color-secondary);
  background: var(--adlaire-surface-soft);
  border-left-color: var(--adlaire-color-primary);
  font-weight: var(--adlaire-font-weight-medium);
}}
.sb-none {{ padding: var(--adlaire-space-4) 14px; font-size: var(--adlaire-font-size-sm); color: var(--adlaire-surface-text-subtle); display: none }}

/* ══ CONTENT ══════════════════════════════════════════════════════════════ */
#ct {{
  flex: 1; min-width: 0; margin-left: var(--sw);
  padding: var(--adlaire-space-12) var(--adlaire-space-12) 96px;
  transition: margin-left var(--adlaire-transition-base);
}}
#sb.closed ~ #ct {{ margin-left: 0 }}
.ci {{ max-width: var(--adlaire-layout-container-narrow); margin: 0 auto }}

/* ── 見出し ── */
.mh {{ font-family: var(--adlaire-font-family-base); line-height: var(--adlaire-line-height-tight) }}
.h1 {{
  font-size: var(--adlaire-font-size-2xl);
  font-weight: var(--adlaire-font-weight-bold);
  color: var(--adlaire-surface-text);
  border-bottom: 2px solid var(--adlaire-color-primary);
  padding-bottom: var(--adlaire-space-3); margin: 0 0 var(--adlaire-space-6);
}}
.h2 {{
  font-size: var(--adlaire-font-size-xl);
  font-weight: var(--adlaire-font-weight-semibold);
  color: var(--adlaire-surface-text);
  border-bottom: 1px solid var(--adlaire-surface-border);
  padding-bottom: var(--adlaire-space-2); margin: var(--adlaire-space-12) 0 var(--adlaire-space-4);
}}
.h3 {{
  font-size: var(--adlaire-font-size-lg);
  font-weight: var(--adlaire-font-weight-semibold);
  color: var(--adlaire-surface-text);
  margin: var(--adlaire-space-8) 0 var(--adlaire-space-3);
}}
.h4 {{
  font-size: var(--adlaire-font-size-sm);
  font-weight: var(--adlaire-font-weight-medium);
  font-family: var(--adlaire-font-family-mono);
  color: var(--adlaire-surface-text-muted);
  margin: var(--adlaire-space-6) 0 var(--adlaire-space-2);
  padding: var(--adlaire-space-2) var(--adlaire-space-3);
  background: var(--adlaire-surface-soft);
  border-left: 3px solid var(--adlaire-color-primary);
  border-radius: 0 var(--adlaire-radius-md) var(--adlaire-radius-md) 0;
}}

/* ── 本文 ── */
.mp {{ max-width: 68ch; margin-bottom: var(--adlaire-space-4) }}
.mr {{ border: none; border-top: 1px solid var(--adlaire-surface-border); margin: var(--adlaire-space-8) 0 }}
.mbq {{
  border-left: 3px solid var(--adlaire-color-primary);
  background: var(--adlaire-surface-soft);
  padding: var(--adlaire-space-3) var(--adlaire-space-5);
  margin: var(--adlaire-space-5) 0;
  color: var(--adlaire-surface-text-muted);
  font-style: italic;
  border-radius: 0 var(--adlaire-radius-lg) var(--adlaire-radius-lg) 0;
  max-width: 68ch;
}}

/* ── インラインコード ── */
.ic {{
  font-family: var(--adlaire-font-family-mono);
  font-size: .83em;
  background: var(--adlaire-surface-soft-strong);
  color: var(--adlaire-surface-accent-strong);
  padding: .1em .4em;
  border-radius: var(--adlaire-radius-sm);
  border: 1px solid var(--adlaire-surface-border);
}}

/* ── コードブロック ── */
.cb-wrap {{
  position: relative; margin: var(--adlaire-space-5) 0;
  border-radius: var(--adlaire-radius-lg);
  border: 1px solid var(--adlaire-surface-border);
  box-shadow: var(--adlaire-shadow-card);
  overflow: hidden;
}}
.cb-meta {{
  position: absolute; top: 8px; right: 10px;
  display: flex; align-items: center; gap: var(--adlaire-space-2);
  pointer-events: none;
}}
.cb-meta > * {{ pointer-events: auto }}
.cl {{
  font-family: var(--adlaire-font-family-mono);
  font-size: var(--adlaire-font-size-xs);
  font-weight: var(--adlaire-font-weight-normal);
  color: var(--adlaire-surface-text-subtle);
  letter-spacing: .08em; text-transform: uppercase;
}}
.cb-copy {{
  background: var(--adlaire-surface-card);
  border: 1px solid var(--adlaire-surface-border);
  cursor: pointer;
  color: var(--adlaire-surface-text-subtle);
  font: var(--adlaire-font-weight-normal) var(--adlaire-font-size-xs) var(--adlaire-font-family-base);
  padding: 2px var(--adlaire-space-3);
  border-radius: var(--adlaire-radius-sm);
  opacity: 0;
  transition: opacity var(--adlaire-transition-fast), background var(--adlaire-transition-fast), color var(--adlaire-transition-fast);
  white-space: nowrap;
}}
.cb-wrap:hover .cb-copy {{ opacity: 1 }}
.cb-copy:hover {{
  background: var(--adlaire-surface-soft);
  color: var(--adlaire-color-secondary);
  border-color: var(--adlaire-color-primary);
}}
.cb-copy.copied {{ opacity: 1; color: var(--adlaire-color-secondary); border-color: var(--adlaire-color-primary) }}
.cb {{
  background: var(--adlaire-surface-soft-strong);
  color: var(--adlaire-surface-accent-strong);
  font-family: var(--adlaire-font-family-mono);
  font-size: var(--adlaire-font-size-sm);
  font-weight: var(--adlaire-font-weight-normal);
  line-height: 1.65;
  padding: var(--adlaire-space-5) var(--adlaire-space-5);
  overflow-x: auto; tab-size: 4;
  margin: 0; border: none; border-radius: 0; display: block;
}}
.cb code {{ font: inherit; color: inherit; background: none; padding: 0; border: none; white-space: pre }}

/* ── テーブル ── */
.tw {{
  overflow-x: auto; margin: var(--adlaire-space-5) 0 var(--adlaire-space-8);
  border-radius: var(--adlaire-radius-lg);
  border: 1px solid var(--adlaire-surface-border);
  box-shadow: var(--adlaire-shadow-card);
}}
.mt {{
  border-collapse: collapse;
  font-size: var(--adlaire-font-size-sm);
  line-height: var(--adlaire-line-height-base);
  width: 100%; min-width: 360px;
}}
.mt th {{
  background: var(--adlaire-surface-soft);
  color: var(--adlaire-surface-text);
  font-family: var(--adlaire-font-family-mono);
  font-size: var(--adlaire-font-size-xs);
  font-weight: var(--adlaire-font-weight-semibold);
  letter-spacing: .04em; text-align: left;
  padding: var(--adlaire-space-3) var(--adlaire-space-4);
  border-bottom: 1px solid var(--adlaire-surface-border);
  white-space: nowrap;
}}
.mt td {{
  padding: var(--adlaire-space-3) var(--adlaire-space-4);
  border-bottom: 1px solid var(--adlaire-surface-border);
  color: var(--adlaire-surface-text); vertical-align: top;
}}
.mt tr:last-child td {{ border-bottom: none }}
.mt tr:nth-child(even) td {{ background: var(--adlaire-surface-soft) }}
.mt tr:hover td {{ background: var(--adlaire-surface-soft-strong) }}

/* ── リスト ── */
.ml {{ padding-left: var(--adlaire-space-6); margin: var(--adlaire-space-2) 0 var(--adlaire-space-4) }}
.ml li {{ margin-bottom: var(--adlaire-space-2); max-width: 68ch }}

/* ══ BACK TO TOP ══════════════════════════════════════════════════════════ */
#btt {{
  position: fixed; bottom: 28px; right: 24px;
  width: 38px; height: 38px; border-radius: var(--adlaire-radius-round);
  background: var(--adlaire-surface-accent); color: #fff;
  border: none; cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  box-shadow: var(--adlaire-shadow-blue);
  opacity: 0; pointer-events: none;
  transition: opacity var(--adlaire-transition-fast), transform var(--adlaire-transition-fast);
  z-index: var(--adlaire-z-sticky);
}}
#btt.visible {{ opacity: 1; pointer-events: auto }}
#btt:hover {{ transform: translateY(-2px); box-shadow: var(--adlaire-shadow-blue-hover) }}

/* ══ RESPONSIVE ══════════════════════════════════════════════════════════ */
@media (max-width: 768px) {{
  :root {{ --sw: 86vw }}
  #ct {{ padding: var(--adlaire-space-8) var(--adlaire-space-5) 64px; margin-left: 0 }}
  #sb {{ transform: translateX(calc(-1 * var(--sw))) }}
  #sb.open {{ transform: translateX(0) }}
  #sb.closed {{ transform: translateX(calc(-1 * var(--sw))) }}
  .h1 {{ font-size: var(--adlaire-font-size-xl) }}
  .h2 {{ font-size: var(--adlaire-font-size-lg) }}
}}
@media (max-width: 400px) {{
  #ct {{ padding: var(--adlaire-space-6) var(--adlaire-space-4) var(--adlaire-space-12) }}
}}
</style>
</head>
<body>

<header id="hdr">
  <button id="sb-btn" aria-label="目次を開閉" aria-expanded="true">
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round">
      <line x1="2" y1="4" x2="14" y2="4"/>
      <line x1="2" y1="8" x2="14" y2="8"/>
      <line x1="2" y1="12" x2="14" y2="12"/>
    </svg>
    目次
  </button>
  <span class="hdr-title">Adlaire DB</span>
  <span class="hdr-sep">/</span>
  <span class="hdr-title" style="color:rgba(255,255,255,.7);font-weight:400">仕様正本</span>
  <span class="hdr-ver">V.212</span>
</header>

<div id="lay">
  <nav id="sb" aria-label="目次">
    <div class="sb-search-wrap">
      <input id="sb-search" type="search" placeholder="セクションを検索…" autocomplete="off" aria-label="目次検索">
    </div>
    <div id="sb-toc">
      <ul class="tr" id="toc-root">
        {toc_html}
      </ul>
      <p class="sb-none" id="sb-none">一致するセクションがありません</p>
    </div>
  </nav>

  <main id="ct">
    <div class="ci">
      {body_html}
    </div>
  </main>
</div>

<button id="btt" aria-label="先頭に戻る">
  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"
       stroke-linecap="round" stroke-linejoin="round">
    <polyline points="3,10 8,5 13,10"/>
  </svg>
</button>

<script>
(function () {{
"use strict";

// ── sidebar ──────────────────────────────────────────────────────────────
const sb    = document.getElementById('sb');
const ct    = document.getElementById('ct');
const sbBtn = document.getElementById('sb-btn');
const mobile = () => window.innerWidth <= 768;

let open = !mobile();
try {{ const s = localStorage.getItem('adb-sb'); if (s !== null) open = s === '1' }} catch (e) {{}}

function setSb(v) {{
  open = v;
  if (mobile()) {{
    sb.classList.toggle('open', v);
    sb.classList.remove('closed');
  }} else {{
    sb.classList.toggle('closed', !v);
    ct.style.marginLeft = v ? 'var(--sw)' : '0';
  }}
  sbBtn.setAttribute('aria-expanded', String(v));
  try {{ localStorage.setItem('adb-sb', v ? '1' : '0') }} catch (e) {{}}
}}
setSb(open);
sbBtn.addEventListener('click', () => setSb(!open));
sb.querySelectorAll('.tl').forEach(a => {{
  a.addEventListener('click', () => {{ if (mobile()) setSb(false) }});
}});

// ── TOC group toggles ─────────────────────────────────────────────────
sb.querySelectorAll('.tg-btn').forEach(btn => {{
  btn.addEventListener('click', e => {{
    e.preventDefault(); e.stopPropagation();
    const ul = document.getElementById(btn.dataset.target);
    if (!ul) return;
    const exp = btn.getAttribute('aria-expanded') === 'true';
    btn.setAttribute('aria-expanded', String(!exp));
    ul.hidden = exp;
  }});
}});

// ── TOC search ────────────────────────────────────────────────────────
const inp     = document.getElementById('sb-search');
const noneMsg = document.getElementById('sb-none');
const allLinks  = Array.from(sb.querySelectorAll('.tl'));
const allLi     = Array.from(sb.querySelectorAll('li'));
const allGroups = Array.from(sb.querySelectorAll('.tg'));

let searchActive = false;

inp.addEventListener('input', () => {{
  const q = inp.value.trim().toLowerCase();
  if (!q) {{
    searchActive = false;
    allLi.forEach(li => li.hidden = false);
    allGroups.forEach(g => {{
      const ul  = g.querySelector('.tc');
      const btn = g.querySelector('.tg-btn');
      if (ul) ul.hidden = btn.getAttribute('aria-expanded') !== 'true';
    }});
    noneMsg.style.display = 'none';
    return;
  }}
  searchActive = true;
  let any = false;
  allLinks.forEach(a => {{
    const match = a.textContent.toLowerCase().includes(q);
    const li = a.closest('li');
    if (li) li.hidden = !match;
    if (match) any = true;
  }});
  allGroups.forEach(g => {{
    const hasVis = g.querySelector('li:not([hidden]) .tl');
    g.hidden = !hasVis;
    const ul = g.querySelector('.tc');
    if (ul && hasVis) ul.hidden = false;
  }});
  noneMsg.style.display = any ? 'none' : 'block';
}});

// ── active heading tracking ───────────────────────────────────────────
const heads = Array.from(document.querySelectorAll('.mh[id]'));

function setActive(id) {{
  allLinks.forEach(a => a.classList.remove('active'));
  const target = sb.querySelector('.tl[href="#' + id + '"]');
  if (!target) return;
  target.classList.add('active');
  let el = target.parentElement;
  while (el && el !== sb) {{
    if (el.classList.contains('tc')) {{
      el.hidden = false;
      const row = el.previousElementSibling;
      if (row) {{
        const b = row.querySelector('.tg-btn');
        if (b) b.setAttribute('aria-expanded', 'true');
      }}
    }}
    el = el.parentElement;
  }}
  if (!searchActive) target.scrollIntoView({{ block: 'nearest', behavior: 'smooth' }});
}}

let cur = '';
const io = new IntersectionObserver(entries => {{
  entries.forEach(e => {{
    if (e.isIntersecting && e.target.id !== cur) {{
      cur = e.target.id;
      setActive(cur);
    }}
  }});
}}, {{ rootMargin: '-8% 0px -78% 0px', threshold: 0 }});
heads.forEach(h => io.observe(h));

// ── copy buttons ──────────────────────────────────────────────────────
document.querySelectorAll('.cb-copy').forEach(btn => {{
  btn.addEventListener('click', () => {{
    const code = btn.closest('.cb-wrap')?.querySelector('code');
    if (!code) return;
    const text = code.innerText;
    const done = () => {{
      btn.textContent = '✓ 完了';
      btn.classList.add('copied');
      setTimeout(() => {{ btn.textContent = 'コピー'; btn.classList.remove('copied') }}, 1800);
    }};
    if (navigator.clipboard) {{
      navigator.clipboard.writeText(text).then(done).catch(() => {{ fallbackCopy(text); done(); }});
    }} else {{ fallbackCopy(text); done(); }}
  }});
}});
function fallbackCopy(text) {{
  const ta = document.createElement('textarea');
  ta.value = text; ta.style.cssText = 'position:fixed;opacity:0';
  document.body.appendChild(ta); ta.select();
  try {{ document.execCommand('copy') }} catch (e) {{}}
  document.body.removeChild(ta);
}}

// ── back to top ──────────────────────────────────────────────────────
const btt = document.getElementById('btt');
addEventListener('scroll', () => btt.classList.toggle('visible', scrollY > 400), {{ passive: true }});
btt.addEventListener('click', () => window.scrollTo({{ top: 0, behavior: 'smooth' }}));

}})();
</script>
</body>
</html>'''

with open(OUT, 'w', encoding='utf-8') as f:
    f.write(PAGE)

size = len(PAGE.encode('utf-8'))
print(f"Done → {OUT}  ({size:,} bytes / {size//1024} KB)")
