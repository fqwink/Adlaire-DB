### Phase 4：JWT 認証・token コマンド

**目標**：JWT HS256 認証と `adlaire-db token create` が動作する

**スコープ：**
- Authorization: Bearer ヘッダ抽出・6 ステップ検証フロー
- tokens.json 読み込み・revoke リスト照合
- `token create` サブコマンド（JWT 生成・stdout 出力）

**完了条件（テストケース）：**

```
TC-3: JWT 認証ありモード（Phase 4 CLI 手動確認）
  $ SECRET="test-secret-at-least-32bytes-long"
  $ TOKEN=$(./adlaire-db token create --secret "$SECRET")
  $ ./adlaire-db serve --data ./testdb --port 8080 --auth-jwt-secret "$SECRET"
  （a）有効なトークンで SQL 実行 → 200 OK
  （b）Authorization ヘッダなし → 401 AUTH_REQUIRED
  （c）不正なトークン → 401 AUTH_INVALID
```

> **注意**：TC-3 の自動化（tokens.json CRUD + revoke フローを含む完全検証）は Phase 7 の TC-2-4〜TC-2-5 で行う。Phase 4 では上記の CLI ベース手動確認が完了条件。自動テストは `tests/integration/auth.rs` として Phase 7 で追加する。

**実装タスク：**

```
T-7: JWT 認証ミドルウェア
  [ ] jsonwebtoken crate で HS256 検証
  [ ] Authorization: Bearer <JWT> ヘッダ抽出
  [ ] 6 ステップ検証フロー実装（§5.6）
      ①ヘッダ有無, ②署名, ③exp, ④revoke リスト照合,
      ⑤アクセスレベル, ⑥通過
  [ ] --auth-jwt-secret 未設定時は認証をスキップ（WARN ログ）
  [ ] 各エラーの HTTP ステータス・コード返却（§7.3）
  参照: §5.1〜5.5, §7.3
  検証: TC-3（JWT 認証）, ETC-1（認証エラー）, ETC-2（権限エラー）

T-8: tokens.json 読み込み・revoke リスト
  [ ] 起動時に meta/tokens.json をメモリに展開（§8.1 Step 5）
  [ ] なければ空リストで初期化・書き出し
  [ ] JWT 検証ステップ④での revoke 照合
  [ ] Phase 1〜5 では tokens.json の更新は CLI のみ（管理 API は Phase 7）
  参照: §5.6

T-9: `token create` サブコマンド
  [ ] --secret <VALUE>, --expiry <DURATION>, --access ro|rw フラグ
  [ ] JWT を HS256 で署名して stdout に出力
  [ ] tok_<random> 形式の token_id を生成（sub クレーム）
  [ ] tokens.json に新規トークンを追記
  参照: §4.1, §5.2, §5.5
  検証: TC-3（TOKEN=$(./adlaire-db token create ...)）
```

---


#### 実装詳細

#### 14.3 JWT 認証関数

```rust
// auth/middleware.rs

use hyper::{Request, body::Incoming};

/// Authorization ヘッダから JWT を検証し Claims を返す
/// 認証無効モード（jwt_secret 未設定）は Claims::unauthenticated() を返す
pub async fn extract_claims<B>(
    req: &Request<B>,
    state: &SharedState,
) -> Result<Claims, AppError> {
    if !state.auth.is_auth_enabled() {
        tracing::debug!("auth disabled — passing unauthenticated claims");
        return Ok(Claims::unauthenticated());
    }

    let raw = req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(AppError::AuthRequired)?;

    state.auth.verify(raw).await
}
```

#### 14.3b 管理 API 認証

管理 API の認証は `http/mod.rs` の `admin_route()` 内で行う（§14.4 参照）。固定トークンの Bearer 文字列完全一致。


#### 14.16 AuthState 実装（Phase 3 スタブ）

Phase 4 で JWT 検証・revoke リスト・token 発行を実装する。

```rust
// auth/mod.rs

impl AuthState {
    pub fn new(config: &Config) -> Self {
        Self { secret_bytes: config.jwt_secret_bytes.clone() }
    }

    pub fn is_auth_enabled(&self) -> bool { self.secret_bytes.is_some() }

    /// Phase 4 で実装。Phase 3 では常に AuthInvalid を返す。
    pub async fn verify(&self, _raw_token: &str) -> Result<Claims, AppError> {
        Err(AppError::AuthInvalid)
    }
}

// NOTE: `revoked` フィールドおよび `load_with(secret, revoked_subs)` の完全版は Phase 4 で追加される。
#[cfg(test)]
impl AuthState {
    /// テスト用: 秘密鍵を直接指定して AuthState を構築する（Phase 3 版）
    pub fn load_with(secret: &[u8]) -> Self {
        Self { secret_bytes: Some(secret.to_vec()) }
    }

    /// テスト用: 有効期限なしのテストトークンを発行する
    pub fn issue_test_token(&self, access: AccessLevel) -> String {
        // Phase 4 で実装
        let _ = access;
        unimplemented!("issue_test_token は Phase 4 で実装")
    }

    /// テスト用: 任意の exp を持つテストトークンを発行する
    pub fn issue_test_token_exp(&self, access: AccessLevel, exp: Option<chrono::DateTime<chrono::Utc>>) -> String {
        let _ = (access, exp);
        unimplemented!("issue_test_token_exp は Phase 4 で実装")
    }

    /// テスト用: 指定 sub を同期的に失効リストへ追加する
    pub fn revoke_sync(&self, sub: &str) {
        // Phase 4 で実装（tokio::sync::RwLock は非同期。テスト用に try_write を使用）
        let _ = sub;
        unimplemented!("revoke_sync は Phase 4 で実装")
    }
}

/// Phase 4 スタブ。tokens.json の読み込みは Phase 4 で実装する。
pub fn load_tokens(_config: &Config) -> anyhow::Result<Vec<TokenRecord>> {
    Ok(vec![])
}
```

---


#### 14.18 Expiry パース・トークン ID 生成

```rust
// token/util.rs

/// "30d" / "24h" / "3600s" / "90m" 形式の期限文字列を chrono::Duration に変換する
pub fn parse_expiry(s: &str) -> anyhow::Result<chrono::Duration> {
    // 最初のアルファベット文字の位置で数値部分とサフィックスを分割
    let split_pos = s.find(|c: char| c.is_alphabetic())
        .ok_or_else(|| anyhow::anyhow!("expiry must end with a unit (s/m/h/d): {s}"))?;
    let (num_str, unit) = s.split_at(split_pos);
    let n: i64 = num_str.parse()
        .map_err(|_| anyhow::anyhow!("invalid expiry number: {num_str}"))?;
    let dur = match unit {
        "s" => chrono::Duration::seconds(n),
        "m" => chrono::Duration::minutes(n),
        "h" => chrono::Duration::hours(n),
        "d" => chrono::Duration::days(n),
        _   => anyhow::bail!("unknown expiry unit '{}' (use s/m/h/d)", unit),
    };
    Ok(dur)
}

/// /dev/urandom から 8 バイト読み取り "tok_{hex16}" 形式の ID を生成する
pub fn generate_token_id() -> String {
    let mut buf = [0u8; 8];
    let mut f = std::fs::File::open("/dev/urandom").expect("cannot open /dev/urandom");
    use std::io::Read;
    f.read_exact(&mut buf).expect("cannot read /dev/urandom");
    let n = u64::from_le_bytes(buf);
    format!("tok_{:016x}", n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_expiry_days() {
        assert_eq!(parse_expiry("30d").unwrap(), chrono::Duration::days(30));
    }

    #[test]
    fn parse_expiry_hours() {
        assert_eq!(parse_expiry("24h").unwrap(), chrono::Duration::hours(24));
    }

    #[test]
    fn parse_expiry_seconds() {
        assert_eq!(parse_expiry("3600s").unwrap(), chrono::Duration::seconds(3600));
    }

    #[test]
    fn parse_expiry_invalid_unit() {
        assert!(parse_expiry("10y").is_err());
    }

    #[test]
    fn parse_expiry_no_unit() {
        assert!(parse_expiry("3600").is_err());
    }
}
```

---
