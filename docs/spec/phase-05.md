### Phase 5：ログ・統合テスト

**目標**：構造化ログを整備し Phase 1〜5 全テストを完走させる

**スコープ：**
- tracing + JSON Lines 出力
- HTTP リクエストログ
- TC-4（TypeScript SDK）・TC-5（データ永続性）を含む全件完走

**完了条件（テストケース）：**

```
TC-4: TypeScript SDK 互換性
  const client = createClient({
    url: "http://localhost:8080",
    authToken: "<JWT>",          // 認証なしモードなら省略可
  });
  await client.execute("CREATE TABLE IF NOT EXISTS users (id INT, name TEXT)");
  await client.execute("INSERT INTO users VALUES (1, 'Alice')");
  const result = await client.execute("SELECT * FROM users");
  期待: result.rows[0] = { id: 1, name: "Alice" }

TC-5: データ永続性（I-4 検証）
  （a）INSERT 後にサーバーを Ctrl+C で停止
  （b）同じ --data で再起動
  （c）SELECT で挿入したデータが返ること
```

**実装タスク：**

```
T-10: ログ実装
  [ ] tracing crate + tracing-subscriber（JSON Lines 出力）
  [ ] --log-level フラグ対応
  [ ] HTTP リクエストごとの INFO ログ（method, path, status, duration_ms）
  [ ] 起動・停止の INFO ログ
  参照: §12

T-11: 統合テスト・TC 完走確認
  [ ] TC-1: curl で SQL 実行（CREATE / INSERT / SELECT）
  [ ] TC-2: GET /v2/health → {"status":"ok"}
  [ ] TC-3: JWT 認証（有効・ヘッダなし・不正トークン）
  [ ] TC-4: TypeScript SDK (@libsql/client) による CRUD
  [ ] TC-5: データ永続性（再起動後に SELECT）
  [ ] TC-6: 起動・停止・.lock 解放
```

**タスク依存グラフ（Phase 1〜5）：**

```
T-1 → T-2 → T-3 → T-4 ─┐
                          ├→ T-5 → T-6 → T-7 → T-8 → T-9 → T-10 → T-11
                          └→ T-5（並行可）
```

T-4〜T-5 は並行実装可。T-6（hrana 変換）は T-4・T-5 の両方が揃った後。

**対象外（Phase 6 以降）：**
- マルチ DB・管理 API・WebSocket・レプリケーション

---


#### 実装詳細

#### 14.11 テスト実装パターン

**ユニットテスト例（JWT 検証・全ケース）：**

```rust
// auth/mod.rs — #[cfg(test)] mod tests
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn secret() -> Vec<u8> { "a".repeat(32).into_bytes() }

    fn make_auth() -> AuthState {
        AuthState::load_with(&secret())
    }

    #[tokio::test]
    async fn valid_rw_token_passes() {
        let auth  = make_auth();
        let token = auth.issue_test_token(AccessLevel::Rw);
        let c = auth.verify(&token).await.unwrap();
        assert_eq!(c.a, AccessLevel::Rw);
    }

    #[tokio::test]
    async fn expired_token_is_rejected() {
        let auth  = make_auth();
        let token = auth.issue_test_token_exp(AccessLevel::Rw, Some(Utc::now() - Duration::seconds(1)));
        assert!(matches!(auth.verify(&token).await, Err(AppError::AuthExpired)));
    }

    #[tokio::test]
    async fn wrong_signature_is_rejected() {
        let auth = make_auth();
        assert!(matches!(auth.verify("eyJ.eyJ.badsig").await, Err(AppError::AuthInvalid)));
    }

    #[tokio::test]
    async fn revoked_token_is_rejected() {
        let auth  = make_auth();
        let token = auth.issue_test_token(AccessLevel::Rw);
        let sub   = auth.verify(&token).await.unwrap().sub;
        auth.revoke_sync(&sub);
        assert!(matches!(auth.verify(&token).await, Err(AppError::AuthInvalid)));
    }

    #[test]
    fn db_scope_rw_overrides_global_ro() {
        let claims = Claims {
            sub: "tok_x".into(),
            a:   AccessLevel::Ro,
            dbs: Some([("db_a".into(), AccessLevel::Rw)].into()),
            iss: None, iat: 0, exp: None,
        };
        assert_eq!(claims.resolve_access("db_a"), AccessLevel::Rw);
        assert_eq!(claims.resolve_access("db_b"), AccessLevel::Ro);
    }
}
```

**テストヘルパー（`tests/helpers.rs`）：**

```rust
// tests/helpers.rs

pub struct TestConfig {
    pub jwt_secret:  Option<String>,
    pub admin_token: Option<String>,
}

impl Default for TestConfig {
    fn default() -> Self { Self { jwt_secret: None, admin_token: None } }
}

pub struct TestServer {
    pub base_url:  String,
    pub admin_url: String,
    client:        reqwest::Client,
    _dir:          tempfile::TempDir,
    _handle:       tokio::task::AbortHandle,
}

impl Drop for TestServer {
    fn drop(&mut self) { self._handle.abort(); }
}

impl TestServer {
    pub async fn spawn(cfg: TestConfig) -> Self {
        let dir   = tempfile::TempDir::new().unwrap();
        let port  = pick_unused_port();
        let aport = pick_unused_port();
        let args = ServeArgs {
            data:              dir.path().to_path_buf(),
            port:              Some(port),
            admin_port:        Some(aport),
            auth_jwt_secret:   cfg.jwt_secret,
            admin_auth_token:  cfg.admin_token,
            ..Default::default()
        };
        let handle = tokio::spawn(async move { run_serve(args).await.unwrap() });
        wait_for_ready(&format!("http://127.0.0.1:{port}/v2/health")).await;
        Self {
            base_url:  format!("http://127.0.0.1:{port}"),
            admin_url: format!("http://127.0.0.1:{aport}"),
            client:    reqwest::Client::new(),
            _dir:      dir,
            _handle:   handle.abort_handle(),
        }
    }

    pub async fn pipeline(&self, body: serde_json::Value) -> reqwest::Response {
        self.client.post(format!("{}/v2/pipeline", self.base_url))
            .json(&body).send().await.unwrap()
    }

    pub async fn get(&self, path: &str) -> reqwest::Response {
        self.client.get(format!("{}{path}", self.base_url)).send().await.unwrap()
    }

    pub async fn admin_post(&self, path: &str, body: serde_json::Value) -> reqwest::Response {
        self.client.post(format!("{}{path}", self.admin_url))
            .json(&body).send().await.unwrap()
    }

    pub async fn admin_delete(&self, path: &str) -> reqwest::Response {
        self.client.delete(format!("{}{path}", self.admin_url)).send().await.unwrap()
    }

    pub async fn admin_get(&self, path: &str) -> reqwest::Response {
        self.client.get(format!("{}{path}", self.admin_url)).send().await.unwrap()
    }

    /// 管理トークンを発行して JWT 文字列を返す
    pub async fn create_token(&self, access: &str) -> String {
        let resp = self.admin_post("/admin/v1/tokens",
            serde_json::json!({"access": access})).await;
        resp.json::<serde_json::Value>().await.unwrap()["token"]
            .as_str().unwrap().to_string()
    }

    /// Bearer トークン付きで /v2/pipeline を呼ぶ
    pub async fn pipeline_with_token(&self, token: &str, body: serde_json::Value) -> reqwest::Response {
        self.client.post(format!("{}/v2/pipeline", self.base_url))
            .bearer_auth(token)
            .json(&body).send().await.unwrap()
    }
}

fn minimal_select() -> serde_json::Value {
    serde_json::json!({"baton":null,"requests":[
        {"type":"execute","stmt":{"sql":"SELECT 1","args":[],"want_rows":true}},
        {"type":"close"}
    ]})
}

fn pick_unused_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap()
        .local_addr().unwrap().port()
}

async fn wait_for_ready(url: &str) {
    let client = reqwest::Client::new();
    for _ in 0..100 {
        if client.get(url).send().await.map(|r| r.status().is_success()).unwrap_or(false) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("TestServer did not become ready at {url}");
}
```

**統合テスト例（TC-1）：**

```rust
// tests/integration/phase1.rs
//! cargo test --test integration

mod helpers;
use helpers::TestServer;

#[tokio::test]
async fn tc1_sql_execution() {
    let srv = TestServer::spawn(Default::default()).await;

    let resp = srv.pipeline(serde_json::json!({
        "baton": null,
        "requests": [
            {"type":"execute","stmt":{"sql":"CREATE TABLE t(id INT, v TEXT)","args":[],"want_rows":false}},
            {"type":"execute","stmt":{"sql":"INSERT INTO t VALUES(1,'hello')","args":[],"want_rows":false}},
            {"type":"execute","stmt":{"sql":"SELECT * FROM t","args":[],"want_rows":true}},
            {"type":"close"}
        ]
    })).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let row = &body["results"][2]["response"]["result"]["rows"][0];
    assert_eq!(row[0], serde_json::json!({"type":"integer","value":"1"}));
    assert_eq!(row[1], serde_json::json!({"type":"text","value":"hello"}));
}

#[tokio::test]
async fn tc2_health_check() {
    let srv = TestServer::spawn(Default::default()).await;
    let resp = srv.get("/v2/health").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<serde_json::Value>().await.unwrap(), serde_json::json!({"status":"ok"}));
}

#[tokio::test]
async fn tc3_jwt_auth() {
    let secret = "test-secret-32bytes-minimum-len!";
    let srv = TestServer::spawn(TestConfig {
        jwt_secret:  Some(secret.to_string()),
        admin_token: Some("admin-tok".to_string()),
    }).await;

    // 管理 API でトークン発行（Phase 7 以降で有効）
    let valid_token = srv.create_token("rw").await;

    // (a) 有効トークン → 200
    assert_eq!(srv.pipeline_with_token(&valid_token, minimal_select()).await.status(), 200);

    // (b) Authorization ヘッダなし → 401 AUTH_REQUIRED
    // pipeline() は Bearer ヘッダを付けないので認証エラーになる
    let no_auth = srv.pipeline(minimal_select()).await;
    assert_eq!(no_auth.status(), 401);
    assert_eq!(no_auth.json::<serde_json::Value>().await.unwrap()["code"], "AUTH_REQUIRED");

    // (c) 不正トークン → 401 AUTH_INVALID
    let bad = srv.pipeline_with_token("eyJ.eyJ.badsig", minimal_select()).await;
    assert_eq!(bad.status(), 401);
    assert_eq!(bad.json::<serde_json::Value>().await.unwrap()["code"], "AUTH_INVALID");
}
```

---
