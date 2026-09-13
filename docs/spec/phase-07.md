### Phase 7：管理 API・トークン CRUD・DB スコープ JWT

**目標**：管理 API と DB スコープアクセス制御を実装する

- 管理 API（DB CRUD・トークン CRUD）
- DB 単位のアクセス制御（JWT クレーム拡張）

**完了条件（テストケース）：**

```
TC-2-1: マルチ DB SQL 実行
  （a）POST /admin/v1/databases {"name":"db_a"} → 201
  （b）POST /admin/v1/databases {"name":"db_b"} → 201
  （c）POST /db_a/v2/pipeline で db_a に CREATE TABLE t(v TEXT); INSERT
  （d）POST /db_b/v2/pipeline で db_b に CREATE TABLE t(v TEXT); 別データ INSERT
  （e）db_a の SELECT → db_a のデータのみ返る
  （f）db_b の SELECT → db_b のデータのみ返る（db_a のデータは見えない）

TC-2-2: 管理 API — DB CRUD
  （a）GET /admin/v1/databases → [] （初期は空リスト）
  （b）POST /admin/v1/databases {"name":"testdb"} → 201, {id,name,created_at}
  （c）GET /admin/v1/databases → [testdb] がリストに含まれる
  （d）GET /admin/v1/databases/testdb → 200, DB の詳細情報
  （e）DELETE /admin/v1/databases/testdb → 204
  （f）GET /admin/v1/databases/testdb → 404
  （g）POST /testdb/v2/pipeline（削除後） → 404 DB_NOT_FOUND

TC-2-3: DB 名バリデーション
  （a）POST /admin/v1/databases {"name":""} → 400 INVALID_DB_NAME
  （b）POST /admin/v1/databases {"name":"a b"} → 400 INVALID_DB_NAME（スペース不可）
  （c）POST /admin/v1/databases {"name":"../evil"} → 400 INVALID_DB_NAME（パストラバーサル不可）
  （d）POST /admin/v1/databases {"name":"validname"} → 201（英数字・ハイフン・アンダースコアは有効）
  （e）同名 DB を再作成 → 409 DB_ALREADY_EXISTS

TC-2-4: トークン CRUD
  （a）POST /admin/v1/tokens {"access":"rw","expiry":"30d"} → 201, {id,token,access,expires_at}
  （b）GET /admin/v1/tokens → 発行済みトークン一覧（secret は含まない）
  （c）GET /admin/v1/tokens/{id} → トークン詳細（revoked フラグ含む）
  （d）DELETE /admin/v1/tokens/{id} → 204（revoke 実行）
  （e）GET /admin/v1/tokens/{id} → revoked:true になっている

TC-2-5: トークン失効の即時反映
  （a）有効トークン T で POST /v2/pipeline → 200
  （b）DELETE /admin/v1/tokens/{T.id} で T を失効
  （c）同じトークン T で POST /v2/pipeline → 401 AUTH_INVALID（失効反映が即時であること）
  （d）新規トークン T2 で POST /v2/pipeline → 200（他のトークンは影響なし）

TC-2-5b: DB スコープトークン
  （a）POST /admin/v1/tokens {"access":"ro","dbs":{"db_a":"rw"}} → 201
  （b）発行トークンで POST /db_a/v2/pipeline INSERT → 200（db_a は rw 許可）
  （c）発行トークンで POST /db_b/v2/pipeline INSERT → 403 PERMISSION_DENIED
       （db_b は dbs に含まれないため a="ro" が適用）
  （d）発行トークンで POST /db_b/v2/pipeline SELECT → 200（読み取りは ro で許可）
  （e）GET /admin/v1/tokens/{id} → dbs フィールドに {"db_a":"rw"} が含まれる

TC-2-6: データディレクトリ永続化（マルチ DB）
  （a）db_a / db_b を作成し各テーブルにデータ投入
  （b）サーバーを停止・再起動（同じ --data ディレクトリ）
  （c）db_a・db_b 両方のデータが復元されること
  （d）{data-dir}/databases/ 以下に db_a/ db_b/ ディレクトリが存在すること
  （e）{data-dir}/meta/databases.json に両 DB が記録されていること
```

**実装タスク：**

```
T2-3: 管理 API — DB CRUD
  [ ] GET /admin/v1/databases → databases.json の一覧を返す
  [ ] POST /admin/v1/databases — DB 名バリデーション・ディレクトリ作成・libsql オープン
  [ ] GET /admin/v1/databases/{name} → 個別情報（name・created_at・size_bytes）
  [ ] DELETE /admin/v1/databases/{name} — Arc<libsql::Database> drop・ディレクトリ削除
  [ ] size_bytes は data.db のファイルサイズを返す
  参照: §6.4（DB 管理）

T2-4: 管理 API — トークン CRUD
  [ ] POST /admin/v1/tokens — JWT 生成・tokens.json への追記・201 返却
  [ ] GET /admin/v1/tokens / GET /admin/v1/tokens/{id} — tokens.json から読み込み
  [ ] DELETE /admin/v1/tokens/{id} — tokens.json の revoked を true に更新（冪等）
  [ ] expiry パース（30d / 24h / 3600s 等）→ JWT exp クレームへの変換
  参照: §6.4（トークン管理）, §5.5, §5.6

T2-5: DB スコープ JWT（dbs クレーム）
  [ ] Phase 4 の JWT 検証を拡張（7 ステップフロー §5.4）
  [ ] dbs クレームが存在する場合、対象 DB 名でアクセス権を解決
  [ ] POST /admin/v1/tokens に dbs フィールドを追加
  [ ] tokens.json の dbs フィールドを保存
  参照: §5.4

T2-6: 統合テスト TC-2-1〜TC-2-6（TC-2-5b 含む）
  [ ] 各テストケースを実行し全て PASS することを確認
  [ ] Phase 1〜6 の TC がリグレッションしないことを確認
```

#### 実装詳細

```rust
// http/admin/mod.rs
// Phase 6〜7 で各ハンドラを実装する。現時点はすべて 501 を返す stub。

use hyper::{Request, Response, body::Incoming};
use http_body_util::Full;
use bytes::Bytes;
use std::convert::Infallible;
use crate::state::SharedState;

fn not_implemented() -> Response<Full<Bytes>> {
    Response::builder()
        .status(http::StatusCode::NOT_IMPLEMENTED)
        .body(Full::default())
        .unwrap()
}

pub mod databases {
    use super::*;
    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn delete(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}

pub mod tokens {
    use super::*;
    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn revoke(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}

pub mod metrics {
    use super::*;
    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}

pub mod backup {
    use super::*;
    pub async fn backup(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn restore(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn pitr(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}

pub mod branches {
    use super::*;
    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
    pub async fn delete(_req: Request<Incoming>, _state: SharedState) -> Result<Response<Full<Bytes>>, Infallible> { Ok(not_implemented()) }
}
```

---
