### Phase 6：マルチ DB ルーター・DB マネージャ

**目標**：1インスタンスで複数 DB をルーティングできる

- パスベース DB ルーティング（`/{db-name}/v2/pipeline`）
- DB ごとのデータ分離

**実装タスク：**

```
T2-1: パスベース DB ルーター
  [ ] route() を /{db-name}/v2/pipeline にマッチするように拡張
  [ ] パスセグメントから db-name を抽出し、DB 名バリデーションを適用
  [ ] 存在しない db-name → 404 DB_NOT_FOUND
  [ ] Phase 1〜5 の単一 DB ルート（/v2/pipeline）との共存（後方互換）
  参照: §6.1, §3.4

T2-2: マルチ DB マネージャ
  [ ] 起動時に databases.json を読み込み、全 DB を libsql::Builder::new_local() でオープン
  [ ] DB 名 → Arc<dyn SqldAdapter> のマップをメモリ上で管理（RwLock<HashMap>）
  [ ] 新規 DB 作成時にマップへ追加・databases.json を更新
  [ ] DB 削除時にマップから除去・ファイル削除・databases.json を更新
  参照: §3.4, §8.1 Step 5〜6
```

---


#### 実装詳細

#### 14.7 DB 名バリデーション

```rust
// db/mod.rs
use std::sync::LazyLock;
use regex::Regex;

static DB_NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9_-]{1,127}$").unwrap()
});

const RESERVED_NAMES: &[&str]  = &["meta", "admin"];
const BRANCH_SEP:     &str     = "___";

pub fn validate_db_name(name: &str) -> Result<(), AppError> {
    if !DB_NAME_RE.is_match(name) {
        return Err(AppError::InvalidDbName);
    }
    if RESERVED_NAMES.contains(&name) || name.contains(BRANCH_SEP) {
        return Err(AppError::DbReservedName);
    }
    Ok(())
}

/// ブランチ DB の内部名を生成する
pub fn branch_db_name(source: &str, branch: &str) -> String {
    format!("{}{}{}", source, BRANCH_SEP, branch)
}
```


#### 14.17 db/meta.rs 実装

```rust
// db/meta.rs

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabasesMeta { pub databases: Vec<DbInfo> }

impl DatabasesMeta {
    /// meta/databases.json をロードする（なければ空を返す、保存はしない）
    pub fn load(data_dir: &Path) -> anyhow::Result<Self> {
        let path = data_dir.join("meta").join("databases.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(&path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// meta/databases.json にアトミック保存（json.tmp → rename）
    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        let path = data_dir.join("meta").join("databases.json");
        let tmp  = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self)?;
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}
```

---
