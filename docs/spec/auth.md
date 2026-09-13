## 5. 認証・認可仕様

JWT、claim、scope、DB scope、token lifecycle を固定する責務である。認証・認可の境界は API 実装と管理操作の前提であり、実装都合で拡張または緩和してはならない。

### 5.1 認証契約

### 5.1 方式

**JWT（JSON Web Token）HS256** を採用する。

- サーバー起動時に `--auth-jwt-secret` で共有秘密鍵を指定する
- クライアントは `Authorization: Bearer <JWT>` ヘッダでトークンを送信する
- 秘密鍵未指定時は認証を無効化する（開発・テスト用）

Turso Cloud の認証トークンと同じ JWT クレーム構造を採用し、既存ツールとの互換性を維持する。

### 5.2 JWT クレーム

```json
{
  "iss": "adlaire-db",
  "sub": "<token-id>",
  "iat": 1700000000,
  "exp": 1800000000,        // 省略時 = 無期限
  "a":  "rw"               // "rw" = 読み書き / "ro" = 読み取り専用
}
```

### 5.3 スコープ（Phase 1）

| `a` 値 | 許可操作 |
|--------|---------|
| `rw` | 全 DB への読み書き |
| `ro` | 全 DB への読み取りのみ |

Phase 1〜5 ではトークンのスコープは全体一律。DB 単位の制御は Phase 7 で追加する。

### 5.4 DB スコープ（Phase 7）

Phase 7 から JWT に省略可能な `dbs` クレームを追加する。

**グローバルトークン（Phase 1〜5 互換・Phase 6 以降も有効）：**

```json
{
  "iss": "adlaire-db",
  "sub": "tok_abc123",
  "iat": 1700000000,
  "exp": 1800000000,
  "a":  "rw"
}
```

`dbs` が存在しない場合は全 DB に `a` クレームのアクセスを適用する（Phase 1〜5 挙動と同じ）。

**DB スコープトークン（Phase 7〜）：**

```json
{
  "iss": "adlaire-db",
  "sub": "tok_def456",
  "iat": 1700000000,
  "exp": 1800000000,
  "a":  "ro",
  "dbs": {
    "analytics": "rw",
    "reports":   "ro"
  }
}
```

`dbs` クレームが存在する場合の権限解決ルール：

| 条件 | 適用アクセス |
|------|-------------|
| `dbs[db_name]` が存在する | `dbs[db_name]` の値を使用 |
| `dbs[db_name]` が存在しない | `a` クレームを使用 |

つまり `dbs` は個別 DB のデフォルト（`a`）を上書きする。全 DB を拒否するには `"a": "ro"` のうえ書き込みが必要な DB のみ `"dbs": {"target": "rw"}` で許可するパターンを使う。

**POST /admin/v1/tokens の DB スコープ指定：**

```json
{
  "access": "ro",
  "expiry": "30d",
  "dbs": {
    "analytics": "rw"
  }
}
```

`dbs` 省略時はグローバルトークン（`dbs` クレームなし）を発行する。

**Phase 7 JWT 検証フロー（DB スコープ対応版）：**

> **§5.4 vs §5.6 の使い分け**：§5.6 の 6 ステップフローは Phase 1〜6 の実装基準（`dbs` クレーム不使用）。§5.4 の 7 ステップフローは Phase 7 以降の拡張版（`dbs` クレームによる DB スコープ権限を追加）。実装フェーズに応じて参照するセクションを切り替える。

```
1. Authorization: Bearer <JWT> ヘッダを取得
   → なし → 401 AUTH_REQUIRED

2. JWT 署名を HS256 で検証
   → 失敗 → 401 AUTH_INVALID

3. exp クレームを確認
   → 期限切れ → 401 AUTH_EXPIRED

4. sub クレーム（token_id）を tokens.json と照合
   → revoked=true → 401 AUTH_INVALID

5. リクエスト対象 DB のアクセスレベルを解決
   dbs[db_name] が存在する → その値を使用
   存在しない              → a クレームを使用

6. 解決したアクセスレベルと要求操作を照合
   → ro で書き込み操作 → 403 PERMISSION_DENIED

7. 検証通過 → リクエスト処理へ
```

### 5.5 トークン生成

```bash
# グローバル rw トークン（Phase 1 と同じ）
adlaire-db token create --secret "my-secret" --expiry 30d

# グローバル ro トークン
adlaire-db token create --secret "my-secret" --access ro

# DB スコープトークン（Phase 7〜）
adlaire-db token create --secret "my-secret" \
  --access ro \
  --db analytics:rw \
  --db reports:ro
# → eyJ...（標準出力）
```

### 5.6 トークン失効管理

**tokens.json の構造：**

```json
{
  "tokens": [
    {
      "id":         "tok_abc123",
      "access":     "rw",
      "dbs":        null,
      "created_at": "2026-09-10T12:00:00Z",
      "expires_at": "2026-10-10T12:00:00Z",
      "revoked":    false,
      "revoked_at": null
    },
    {
      "id":         "tok_def456",
      "access":     "ro",
      "dbs":        {"analytics": "rw", "reports": "ro"},
      "created_at": "2026-09-01T00:00:00Z",
      "expires_at": null,
      "revoked":    false,
      "revoked_at": null
    }
  ]
}
```

`dbs` フィールド：`null` = グローバルトークン、オブジェクト = DB スコープトークン。

**JWT 検証フロー（リクエストごと）：**

```
1. Authorization: Bearer <JWT> ヘッダを取得
   → なし → 401 AUTH_REQUIRED

2. JWT 署名を HS256 で検証
   → 失敗 → 401 AUTH_INVALID

3. exp クレームを確認
   → 期限切れ → 401 AUTH_EXPIRED

4. sub クレーム（token_id）を tokens.json と照合
   → revoked=true → 401 AUTH_INVALID

5. resolve_access(db_name) でアクセスレベルを解決
     dbs[db_name] が存在する → その値を使用（Phase 7〜）
     存在しない / dbs なし → a クレームを使用
   → Ro かつ書き込み操作 → 403 PERMISSION_DENIED

6. 検証通過 → リクエスト処理へ
```

**パフォーマンス：**
- サーバー起動時に `tokens.json` をメモリへロードする
- 失効済みトークン ID の集合はメモリ上に `RwLock<HashSet<String>>` で保持する（`AuthState` 内）
- `DELETE /admin/v1/tokens/{id}` 受信時にメモリ上の失効リストを更新し `tokens.json` を書き直す
- メモリロード後は `tokens.json` の再読み込みは行わない（サーバー再起動で反映）

---
