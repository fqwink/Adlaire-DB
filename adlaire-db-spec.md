# Adlaire DB 仕様書（プロトタイプ版）

**バージョン：** 1.3  
**ステータス：** 確定  
**最終更新：** 2026-09-09  

---

## 1. 概要

### 1.1 プロジェクト概要
Rust で実装される統合DBエンジン。データ整合性・保全・可用性を同率で重視し、KV + イベント型アーキテクチャで堅牢性を実現。

**ポジション：** 「SQLite の隣人として置ける、次の選択肢」。シングルバイナリで起動でき、すべての変更が改ざん検知可能な証拠として残る。immudb のように削除を禁止せず、FoundationDB のような分散インフラも不要。小チームの本番サービスから始めて、監査・コンプライアンス要件が来たとき追加実装ゼロで対応できる DB。

### 1.2 設計目標
- **データ整合性** ：改ざん検知、トランザクション完全性
- **データ保全** ：完全な履歴追跡、復旧可能性
- **可用性** ：ブロッキング最小化、非同期対応
- **移行容易性** ：SQLite からの移行パスを標準提供（JSON import / データ変換ツール）
- **外部検証可能性** ：専用クライアント不要でファイルから直接ハッシュチェーンを検証できる
- **デプロイ簡易性** ：シングルバイナリ起動を第一級市民とする（Docker は選択肢の一つ）

### 1.3 プロトタイプ スコープ
- **ネットワークサーバーモード のみ** ：TCP（ポート 9876）+ REST API（ポート 8080）
- **ファイルベース設計** ：複数ファイルで構成（単一ファイルではない）
- **シングルバイナリ起動** ：`./adlaire-db --data ./mydb --port 9876` で即起動
- JSONデータソース対応
- CRUD（作成・読取・更新・削除）全て対応（ただし Delete は論理削除のみ → 3.1.3 参照）
- JOIN 機能サポート
- ACID トランザクション機構
- SQLite データ移行ツール（JSON 経由インポート）
- **将来の分散対応を視野に入れた設計**（初期はシングルマシン実装）

### 1.4 競合との差別化

| 項目 | immudb | FoundationDB | **Adlaire DB** |
|------|--------|-------------|----------------|
| 物理削除 | 不可（設計上禁止） | 可 | **論理削除のみ（削除事実は証明可能）** |
| 主 DBMS として使用 | 困難（補助 DB 推奨） | 可（分散環境） | **可（単体で完結）** |
| デプロイ | サーバー + クライアント | 複数ロール（複雑） | **シングルバイナリ** |
| 外部検証 | 専用クライアント必要 | なし | **ファイル単体で検証可能** |
| SQLite 移行 | なし | なし | **移行ツール標準搭載** |
| 分散対応 | あり | ネイティブ | 将来 Phase 2–4 |
| 対象規模 | 中〜大 | 大 | **小〜中（スケールアップ可能）** |

---

## 2. アーキテクチャ

### 2.1 全体構成

```
┌─────────────────────────────────────┐
│     Query Interface Layer           │
│  ├─ KV Store API                    │
│  │  (Get, Set, Delete, Scan)        │
│  └─ Event Query API                 │
│     (GetHistory, FollowChain)       │
├─────────────────────────────────────┤
│     JOIN Engine Layer               │
│  (複数キーの結合ロジック)            │
├─────────────────────────────────────┤
│     Transaction / ACID Layer        │
│  (ロック、ロールバック、一貫性)      │
├─────────────────────────────────────┤
│     Core Storage Engine             │
│  (KV ストア + イベント型)            │
├─────────────────────────────────────┤
│     Persistence Layer               │
│  (ファイルI/O、ストレージ形式)       │
└─────────────────────────────────────┘
```

### 2.2 データモデル

#### 2.2.1 レコード構造
```
Record {
    key: String,                    // 一意キー
    value: JSON,                    // データ（JSON形式）
    version: u64,                   // バージョンID
    transaction_id: u64,            // トランザクションID
    timestamp: i64,                 // タイムスタンプ
    hash: String,                   // SHA-256ハッシュ
    prev_hash: String,              // 前レコードのハッシュ
}
```

#### 2.2.2 イベント構造
```
Event {
    id: String,                     // イベントID（UUID）
    event_type: EventType,          // イベント型
    key: String,                    // 対象キー
    payload: JSON,                  // ペイロード（新値）
    timestamp: i64,                 // イベント発生時刻
    transaction_id: u64,            // トランザクションID
    prev_hash: String,              // チェーン検証用
    hash: String,                   // このイベントのハッシュ
}

enum EventType {
    Created,                        // レコード作成
    Updated,                        // レコード更新
    Deleted,                        // レコード削除
    Recovered,                      // ロールバック復旧
}
```

### 2.2 ファイルベース構成（ネットワークサーバーモード）

#### 2.2.1 ストレージ構成（Phase 1：シングルマシン）

Adlaire DB サーバーは、**最小3ファイル構成** でデータを管理。これにより、データ整合性を保ちながら、シンプルで拡張性のある設計を実現。

```
adlaire_db/（ディレクトリ）
├── .lock                # ファイルロック（排他制御用）
├── shard_0/
│   ├── metadata.dat     # メタデータ（Version、チェックサム、バージョン情報、クラスタ情報）
│   ├── data.kv          # KV Store + イベントログ（複合）
│   └── txlog.dat        # トランザクションログ + ジャーナルログ（WAL）
├── shard_1/
│   ├── metadata.dat
│   ├── data.kv
│   └── txlog.dat
├── shard_2/
│   ├── metadata.dat
│   ├── data.kv
│   └── txlog.dat
├── shard_3/
│   ├── metadata.dat
│   ├── data.kv
│   └── txlog.dat
└── ...（複数シャード）

【ロールバック用バージョン保持】

shard_0_v1/              # 1世代前のバージョン
├── metadata.dat
├── data.kv
└── txlog.dat

shard_0_v2/              # 2世代前のバージョン
├── metadata.dat
├── data.kv
└── txlog.dat
```

#### 2.2.2 整合性向上施策

##### 2.2.2.1 チェックサム/ハッシュ値による改ざん検知

**実装方法：**
```
各ファイルごとに SHA256 ハッシュ値を計算・保存

metadata.json.sha256:
  # メタデータのハッシュ値
  sha256=a3c4f2e8d9b1c6a7e2f8...

data.kv.sha256:
  # KV Store のハッシュ値
  sha256=b4d5e3f9a2c7d8b6e3f9...

events.log.sha256:
  # イベントログのハッシュ値
  sha256=c5e6f4a0b3d8e9c7f4a0...
```

**チェック処理：**
```rust
fn verify_file_integrity(file_path: &str, expected_hash: &str) -> bool {
    let file_content = read_file(file_path);
    let computed_hash = sha256(&file_content);
    computed_hash == expected_hash
}
```

**整合性チェック：サーバー起動時、トランザクション完了時**

##### 2.2.2.2 メタデータの冗長化

**実装方法：**
```json
// metadata.json（プライマリ）
{
  "version": "1.0",
  "shard_count": 4,
  "created_at": "2026-09-09T12:00:00Z",
  "cluster_nodes": [],
  "last_checkpoint": {
    "timestamp": "2026-09-09T12:30:00Z",
    "hash": "a3c4f2e8d9b1c6a7e2f8..."
  }
}

// metadata.json.bak（バックアップ）
// 定期的に metadata.json をコピー
// 片方が破損時、もう片方から復旧
```

**冗長化戦略：**
- メタデータ更新時：プライマリ → バックアップ → チェックサム の順序で書き込み
- 片方が破損時：他方から復旧
- ハッシュ検証で健全性確認

##### 2.2.2.3 トランザクション日誌の強化

**txlog.dat フォーマット：**
```
[TX ID: 8B][TX Type: 1B][Timestamp: 8B][Status: 1B][Data Length: 4B][Data][Checksum: 32B]
     ↓            ↓            ↓           ↓           ↓        ↓        ↓
  一意識別子   操作種別     タイムスタンプ  実行状態  ペイロード長  内容   SHA256
```

**トランザクション状態：**
```
0x00 = PENDING    # トランザクション開始（未コミット）
0x01 = COMMITTED  # コミット完了
0x02 = ABORTED    # ロールバック
0x03 = FAILED     # エラーで失敗
```

**リカバリプロセス：**
```rust
fn recover_from_txlog() {
    let log_entries = read_txlog("shard_0/txlog.dat");
    
    for entry in log_entries {
        match entry.status {
            PENDING => {
                // 未完了トランザクション → ロールバック
                rollback_transaction(entry.tx_id);
            }
            COMMITTED => {
                // コミット済み → 何もしない
            }
            ABORTED | FAILED => {
                // 既にロールバック済み
            }
        }
    }
}
```

##### 2.2.2.4 ファイルロック機構

**実装方法（.lock ファイル）：**
```
.lock ファイルに以下の情報を記録：

{
  "lock_holder": "server_instance_1",
  "pid": 12345,
  "timestamp": "2026-09-09T12:00:00Z",
  "expires_at": "2026-09-09T12:05:00Z"
}
```

**ロック取得：**
```rust
fn acquire_lock(lock_path: &str, timeout: Duration) -> Result<LockGuard> {
    let deadline = Instant::now() + timeout;
    
    loop {
        if try_create_lock_file(lock_path) {
            return Ok(LockGuard::new(lock_path));
        }
        
        // ロック持有者が死亡（タイムアウト）したかチェック
        if is_lock_expired(lock_path) {
            remove_stale_lock(lock_path);
            continue;
        }
        
        if Instant::now() > deadline {
            return Err(LockTimeout);
        }
        
        sleep(Duration::from_millis(100));
    }
}
```

**複数プロセスからのアクセス制御：**
- プロセス1：.lock 取得 → ファイル操作 → .lock 解放
- プロセス2：.lock 待機 → タイムアウト時は古いロックを削除
- デッドロック検知：タイムスタンプが古いロックは強制削除

##### 2.2.2.5 ジャーナルログ（Write-Ahead Logging, WAL）

**WAL の動作：**
```
Client が更新リクエスト
  ↓
1. journal.log にトランザクション記録（書き込み待機）
  ↓
2. メモリに操作内容をバッファリング
  ↓
3. journal.log にコミットマーク追加
  ↓
4. 実際のファイルを更新（data.kv、events.log等）
  ↓
5. チェックサムを計算・保存
  ↓
6. Client にレスポンス
```

**障害時のリカバリ：**
```rust
fn recover_from_wal(journal_path: &str) {
    let entries = read_journal(journal_path);
    
    for entry in entries {
        match entry.state {
            NOT_COMMITTED => {
                // 障害発生前に未コミット → スキップ（ロールバック）
            }
            COMMITTED => {
                // コミット済み → ファイル更新を再実行
                apply_changes(entry);
            }
            APPLIED => {
                // 既に適用済み → スキップ
            }
        }
    }
    
    // journal.log をクリア
    truncate_journal(journal_path);
}
```

**journal.log フォーマット：**
```
[TX ID: 8B][State: 1B][Timestamp: 8B][Data Length: 4B][Data][Checksum: 32B]

State:
  0x00 = NOT_COMMITTED  # 未コミット
  0x01 = COMMITTED      # コミット（この後ファイル更新）
  0x02 = APPLIED        # ファイル更新完了
```

##### 2.2.2.6 バージョニングとロールバック

**ファイル構成（バージョン管理）：**
```
shard_0/data.kv                    # 現在のバージョン
shard_0_v1/data.kv                 # 1世代前
shard_0_v2/data.kv                 # 2世代前
shard_0_v3/data.kv                 # 3世代前

メタデータに世代情報を記録：

{
  "shard_0": {
    "current_version": 4,
    "versions": [
      {"version": 4, "timestamp": "2026-09-09T12:30:00Z", "hash": "abc..."},
      {"version": 3, "timestamp": "2026-09-09T12:25:00Z", "hash": "def..."},
      {"version": 2, "timestamp": "2026-09-09T12:20:00Z", "hash": "ghi..."}
    ],
    "retention_policy": "keep_last_3_versions"
  }
}
```

**ロールバック手順：**
```rust
fn rollback_to_version(shard_id: u32, version: u32) -> Result<()> {
    // 1. トランザクションロック取得
    let _lock = acquire_lock(&format!("shard_{}.lock", shard_id))?;
    
    // 2. ロールバック対象バージョンのハッシュ検証
    let target = get_version_metadata(shard_id, version)?;
    verify_file_integrity(&format!("shard_{}_v{}", shard_id, version), &target.hash)?;
    
    // 3. journal.log に ROLLBACK 記録
    log_rollback_intent(shard_id, version)?;
    
    // 4. ファイル復元
    copy_file(&format!("shard_{}_v{}/data.kv", shard_id, version),
              &format!("shard_{}/data.kv", shard_id))?;
    
    // 5. チェックサム再計算
    recalculate_checksums(shard_id)?;
    
    // 6. メタデータ更新
    update_current_version(shard_id, version)?;
    
    // 7. journal.log に ROLLBACK 完了記録
    log_rollback_complete(shard_id, version)?;
    
    Ok(())
}
```

**バージョン保持ポリシー：**
```json
{
  "retention_policy": {
    "keep_versions": 3,
    "keep_duration_hours": 24,
    "compression": "gzip_old_versions"
  }
}
```

#### 2.2.3 整合性チェックの実行タイミング

**metadata.json**
```json
{
  "version": "1.0",
  "shard_count": 4,
  "created_at": "2026-09-09T12:00:00Z",
  "cluster_nodes": []
}
```

**shard_map.json**
```json
{
  "shards": [
    {"id": 0, "key_range": ["0000", "3fff"], "node": "localhost:9876"},
    {"id": 1, "key_range": ["4000", "7fff"], "node": "localhost:9876"},
    {"id": 2, "key_range": ["8000", "bfff"], "node": "localhost:9876"},
    {"id": 3, "key_range": ["c000", "ffff"], "node": "localhost:9876"}
  ]
}
```

**data.kv（バイナリ形式）**
```
[Key Length: 4B][Key][Value Length: 4B][Value][Key Length: 4B][Key]...
```

**events.log（アペンド・オンリー）**
```
[Timestamp: 8B][Event Type: 1B][Data Length: 4B][Event Data]...
```

**txlog.dat（トランザクションログ）**
```
[TX ID: 8B][TX Type: 1B][Timestamp: 8B][Status: 1B][Data Length: 4B][Data]...
```

#### 2.2.3 将来の分散対応設計

ファイル構成を分散対応として設計し、将来以下を実装可能：

**Phase 2：レプリケーション（Master-Replica）**
```
Master Node: shard_0, shard_1, shard_2, shard_3
  ↓ (複製)
Replica Node A: shard_0, shard_1, shard_2, shard_3
  ↓ (複製)
Replica Node B: shard_0, shard_1, shard_2, shard_3

metadata.json で "cluster_nodes" に Replica を追加
```

**Phase 3：シャーディング（複数ノード分散）**
```
Node A: Shard 0, 1
Node B: Shard 2, 3
Node C: Shard 4, 5

shard_map.json でシャード配置を管理
各クライアント要求に対し、Coordinator が適切なノードにルーティング
```

**Phase 4：分散トランザクション（2-Phase Commit）**
```
Client が複数シャードにまたがるトランザクション実行
→ Coordinator が 2-Phase Commit で調整
→ すべてのシャードで ACID を保証
→ 一部失敗時は全ノードロールバック
```

---

## 3. KV Store API 仕様

### 3.1 基本操作

#### 3.1.1 Get（読取）
```rust
pub fn get(&self, key: &str) -> Result<Option<JSON>>
```
- **入力** ：キー（String）
- **出力** ：値（JSON）
- **イベント記録** ：記録されない（読取操作）
- **パフォーマンス** ：O(1)

#### 3.1.2 Set（作成・更新）
```rust
pub fn set(&mut self, key: &str, value: JSON) -> Result<()>
```
- **入力** ：キー（String）、値（JSON）
- **出力** ：成功/失敗
- **イベント記録** ：
  - キーが新規 → `Created` イベント
  - キーが既存 → `Updated` イベント
- **トランザクション** ：自動的にトランザクションでラップ
- **パフォーマンス** ：O(1) + イベント記録（O(log n)）

#### 3.1.3 Delete（削除）
```rust
pub fn delete(&mut self, key: &str) -> Result<()>
```
- **入力** ：キー（String）
- **出力** ：成功/失敗
- **イベント記録** ：`Deleted` イベント
- **削除モデル（重要）** ：**物理削除 API は提供しない**。Delete は常に論理削除。
  - KV の現在値は「削除済み」状態に遷移（`scan` の結果から除外）
  - `Deleted` イベントがハッシュチェーンに永続記録される → 「誰がいつ削除したか」を外部検証可能
  - immudb との差異：Adlaire DB は「削除の実行」と「削除の証明」を両立する
  - GDPR「忘れられる権利」対応：削除は実行できる。削除した事実の監査ログは消去できない（設計上の制約として明示）
- **パフォーマンス** ：O(1) + イベント記録

#### 3.1.4 Scan（全キー列挙）
```rust
pub fn scan(&self, prefix: Option<&str>) -> Result<Vec<String>>
```
- **入力** ：プリフィックス（オプション）
- **出力** ：マッチするキーのベクタ
- **イベント記録** ：記録されない
- **パフォーマンス** ：O(n)

---

## 4. イベント型仕様

### 4.1 イベント記録

#### 4.1.1 イベントチェーン
```
Event1 → hash: ABC123
  ↓
Event2 → prev_hash: ABC123, hash: DEF456
  ↓
Event3 → prev_hash: DEF456, hash: GHI789
```

- **ハッシュ検証** ：各イベントの `hash` は、イベント内容 + `prev_hash` から SHA-256 で計算
- **チェーン検証** ：各イベントの `prev_hash` が前イベントの `hash` と一致することで整合性確認

#### 4.1.2 イベント永続化
```
EventLog ファイル形式：
{
    "events": [
        {
            "id": "...",
            "event_type": "Created",
            "key": "user:123",
            "payload": {...},
            "timestamp": 1694250000000,
            "transaction_id": 1,
            "prev_hash": "...",
            "hash": "..."
        },
        ...
    ]
}
```

### 4.2 イベントクエリAPI

#### 4.2.1 GetHistory（履歴取得）
```rust
pub fn get_history(&self, key: &str) -> Result<Vec<Event>>
```
- **入力** ：キー（String）
- **出力** ：そのキーに関連する全イベント（時系列順）
- **用途** ：変更履歴の確認

#### 4.2.2 FollowChain（チェーン検証）
```rust
pub fn follow_chain(&self) -> Result<bool>
```
- **入力** ：なし
- **出力** ：チェーン全体が正常か（true/false）
- **用途** ：改ざん検知
- **仕組み** ：全イベントのハッシュチェーンを検証

#### 4.2.3 GetEventsSince（特定時刻以降）
```rust
pub fn get_events_since(&self, timestamp: i64) -> Result<Vec<Event>>
```
- **入力** ：タイムスタンプ
- **出力** ：その時刻以降の全イベント
- **用途** ：レプリケーション、監査

### 4.3 外部検証（External Verification）

**設計思想：** 専用クライアントを必要とせず、`data.kv` と `event_log.jsonl` を受け取った第三者が独自にハッシュチェーンを検証できる。これは immudb との重要な差異であり、Adlaire DB の外部検証可能性の核心。

#### 4.3.1 検証ファイル仕様

サーバーが以下のファイルを監査者に提供するだけで検証が完結する：

```
adlaire_db/
├── shard_0/
│   ├── event_log.jsonl      # イベントログ（JSONL形式、人間可読）
│   └── metadata.dat         # チェーン先頭ハッシュ含む
└── verification_manifest.json  # 各ファイルの SHA256 + 検証手順書
```

#### 4.3.2 検証アルゴリズム（疑似コード）

```python
# 任意のプログラミング言語で実装可能（専用クライアント不要）
def verify_chain(event_log_path):
    events = load_jsonl(event_log_path)
    prev_hash = "0" * 64  # genesis hash

    for event in events:
        # イベント内容 + prev_hash から SHA256 を再計算
        computed = sha256(event["id"] + event["event_type"] +
                          event["key"] + str(event["timestamp"]) +
                          str(event["transaction_id"]) + prev_hash)
        
        if computed != event["hash"]:
            return False, f"改ざん検出: event_id={event['id']}"
        
        prev_hash = event["hash"]
    
    return True, "チェーン検証成功"
```

#### 4.3.3 検証 API エンドポイント

```bash
# 監査用エクスポート（REST API）
GET /api/v1/audit/export?from=2026-09-01&to=2026-09-09

# レスポンス：event_log.jsonl + verification_manifest.json を tar.gz で返す
# 監査者はこのファイルを受け取り、上記アルゴリズムで独自検証する
```

#### 4.3.4 削除の証明

```json
// 「user:1 が 2026-09-09 12:30 に削除された」という事実を第三者が検証できる
{
  "id": "evt-42",
  "event_type": "Deleted",
  "key": "user:1",
  "payload": null,
  "timestamp": 1757503800000,
  "transaction_id": 1001,
  "prev_hash": "abc123...",
  "hash": "def456..."
}
// このエントリはハッシュチェーンに永続記録され、削除も改ざんも不可能
```

---

## 5. JOIN 仕様

### 5.1 JOIN の概念

#### 5.1.1 基本的な JOIN
複数のキーに関連するデータを結合して取得。

```
例：
ユーザーデータ：
  key: "user:1", value: { "name": "John", "dept_id": "D1" }

部門データ：
  key: "dept:D1", value: { "name": "Engineering" }

JOIN クエリ：
  join(
    primary_key: "user:1",
    join_keys: ["dept_id"],          // user の dept_id を参照
    join_targets: ["dept:${dept_id}"] // dept:{dept_id} キーを結合
  )

結果：
  {
    "user": { "name": "John", "dept_id": "D1" },
    "dept": { "name": "Engineering" }
  }
```

### 5.2 JOIN 実装方式

#### 5.2.1 Hash Join （推奨）
```rust
pub fn join(
    &self,
    primary_key: &str,
    join_specs: Vec<JoinSpec>,
) -> Result<JSON>

struct JoinSpec {
    join_key: String,                   // プライマリのキー名
    join_target_pattern: String,        // 結合対象キーパターン
    alias: String,                      // 結果でのエイリアス
}
```

- **処理フロー** ：
  1. プライマリキーの値を取得
  2. `join_key` から結合対象キーを抽出
  3. 結合対象キーから値を取得
  4. 結果を結合

- **パフォーマンス** ：O(k) （k = 結合数）

#### 5.2.2 スカラー JOIN
```rust
pub fn scalar_join(
    &self,
    key: &str,
    joins: Vec<(&str, &str)>,  // (キーパターン, エイリアス)
) -> Result<JSON>
```

- **用途** ：シンプルな1対1結合
- **パフォーマンス** ：O(1) per join

---

## 6. トランザクション / ACID 仕様

### 6.1 ACID 要件実装

#### 6.1.1 Atomicity（原子性）
- **実装** ：操作（Get/Set/Delete）はすべてトランザクション内でラップ
- **トランザクション ID** ：各操作に一意のTX IDを割り当て
- **ロールバック** ：イベント型により、全履歴を保持。ロールバック時は復旧イベントを記録

```rust
pub fn begin_transaction(&mut self) -> TransactionHandle
pub fn commit(&mut self, handle: TransactionHandle) -> Result<()>
pub fn rollback(&mut self, handle: TransactionHandle) -> Result<()>
```

#### 6.1.2 Consistency（一貫性）
- **実装** ：イベントチェーン検証により、全レコードの整合性を保証
- **バリデーション** ：Set操作時、JSONスキーマバリデーション（オプション）

#### 6.1.3 Isolation（分離）
- **実装** ：MVCC（マルチバージョン同時実行制御）
  - 各トランザクションは独自のバージョンビューを保持
  - 読取トランザクションは書込トランザクションをブロックしない

```rust
struct TransactionView {
    version: u64,
    visible_keys: HashMap<String, JSON>,
}
```

#### 6.1.4 Durability（永続性）
- **実装** ：
  1. メモリ内変更
  2. イベントログファイルに即座に書き込み（Write-Ahead Logging相当）
  3. コミット完了

### 6.2 ロック機構

#### 6.2.1 行レベルロック
```rust
pub fn lock_row(&mut self, key: &str, lock_type: LockType) -> Result<()>

enum LockType {
    Shared,      // 読取ロック（複数トランザクションが同時保持可能）
    Exclusive,   // 書込ロック（1つのトランザクションのみ保持）
}
```

- **デッドロック対策** ：タイムアウト機構（デフォルト5秒）

#### 6.2.2 ロック管理
```rust
struct LockManager {
    locks: HashMap<String, LockInfo>,
}

struct LockInfo {
    key: String,
    transaction_id: u64,
    lock_type: LockType,
    acquired_at: i64,
}
```

---

## 7. ストレージ形式

### 7.1 ファイル構成

```
database/
├── kv_store.bin          // KV データ本体
├── event_log.jsonl       // イベントログ（JSONL形式）
└── metadata.json         // メタデータ
```

### 7.2 KV Store ファイル形式

```
バイナリフォーマット（カスタム）：

[Header]
  magic: "KVDB" (4 bytes)
  version: 1 (1 byte)
  reserved: (3 bytes)

[Records]
  record_count: u32
  
  [Record 1]
    key_len: u16
    key: String
    value_len: u32
    value: JSON (バイナリ化)
    version: u64
    hash: String
  
  [Record 2]
  ...
```

### 7.3 イベントログ形式（JSONL）

```
{"id":"evt-1", "event_type":"Created", "key":"user:1", ...}
{"id":"evt-2", "event_type":"Updated", "key":"user:1", ...}
{"id":"evt-3", "event_type":"Deleted", "key":"user:1", ...}
```

**利点** ：
- テキストベースで人間が読みやすい
- ストリーム処理容易
- JSONデータソースとの相互変換が簡単

### 7.4 メタデータ（metadata.json）

```json
{
    "created_at": 1694250000000,
    "last_updated": 1694250010000,
    "kv_record_count": 150,
    "event_log_count": 500,
    "total_size_bytes": 125000,
    "chain_valid": true,
    "last_event_hash": "ABC123..."
}
```

---

## 8. インメモリ管理

### 8.1 メモリ構造

```rust
pub struct Database {
    kv_store: HashMap<String, Record>,
    event_log: Vec<Event>,
    lock_manager: LockManager,
    transaction_manager: TransactionManager,
    version_counter: AtomicU64,
}
```

### 8.2 メモリ最適化

- **ハッシュテーブル** ：キーベースの O(1) アクセス
- **イベントログ** ：Vec で逐次追記（O(1) amortized）
- **バージョン管理** ：AtomicU64 で thread-safe

### 8.3 イベントログ圧縮（Compaction）

immudb の教訓：ストレージが無限増大し、インデックスの削除もできないと運用上の問題になる。Adlaire DB は Phase 1 から圧縮ポリシーを設計に組み込む。

**圧縮ポリシー（設定ファイルで変更可能）：**
```
keep_versions: 3              # 各シャードの最新 N 世代を保持
keep_duration_hours: 168      # 直近 7 日間のイベントは必ず保持
compress_after_hours: 24      # 24 時間より古いイベントを gzip 圧縮
auto_compact_threshold_mb: 500  # シャードサイズがこの値を超えたら自動圧縮
```

**圧縮対象：**
- 圧縮：古いイベントログエントリ（`events.log` の古い部分を gzip 圧縮）
- 保持：ハッシュチェーンの先頭ハッシュ（チェーン検証の起点として永続保持）
- 非対象：現在有効な KV 値（最新バージョン）

**圧縮後の外部検証：**
圧縮してもチェーン検証は可能。圧縮ファイルに "最後のハッシュ" を付記し、後続チェーンの継続性を保証する。

**実装優先度：** Phase 1（Week 3–4 の永続化実装と同時に設計・実装）

---

## 9. エラーハンドリング

### 9.1 エラー型

```rust
pub enum DbError {
    KeyNotFound(String),
    InvalidJSON(String),
    TransactionConflict(String),
    LockTimeout,
    IOError(String),
    ChainValidationFailed,
    TransactionRollback(String),
}

pub type Result<T> = std::result::Result<T, DbError>;
```

### 9.2 エラーレベル

| レベル | 内容 | 対応 |
|---|---|---|
| **Fatal** | ハードウェア障害、ディスク満杯 | エラーログ記録、プロセス停止 |
| **Error** | トランザクション競合、ロック失敗 | エラー返却、リトライ推奨 |
| **Warning** | スキーマ検証失敗 | ログ記録、処理継続 |
| **Info** | 操作ログ | デバッグ用ログ記録 |

---

## 10. テストケース

### 10.1 ユニットテスト

#### 10.1.1 KV Store テスト
```rust
#[test]
fn test_kv_set_get() {
    // key:value をセット → 取得 → 一致確認
}

#[test]
fn test_kv_delete() {
    // key をセット → 削除 → 削除確認
}

#[test]
fn test_kv_scan() {
    // 複数キー → プリフィックス検索
}
```

#### 10.1.2 イベント型テスト
```rust
#[test]
fn test_event_chain_integrity() {
    // イベント記録 → チェーン検証
}

#[test]
fn test_event_hash_tamper_detection() {
    // イベント改ざん → 検出確認
}

#[test]
fn test_get_history() {
    // キーの変更履歴を取得 → 順序確認
}
```

#### 10.1.3 JOIN テスト
```rust
#[test]
fn test_hash_join_basic() {
    // user:1 と dept:D1 を結合
    // 結果が両方のデータを含むか確認
}

#[test]
fn test_scalar_join_multiple() {
    // 複数 JOIN → 全結果確認
}
```

#### 10.1.4 トランザクションテスト
```rust
#[test]
fn test_transaction_commit() {
    // tx.begin → set → commit → 永続化確認
}

#[test]
fn test_transaction_rollback() {
    // tx.begin → set → rollback → 元の状態確認
}

#[test]
fn test_mvcc_isolation() {
    // tx1 読取、tx2 書込 → 分離確認
}

#[test]
fn test_lock_timeout() {
    // 2つの tx が同じキーをロック → タイムアウト確認
}
```

### 10.2 統合テスト

#### 10.2.1 CRUD フローテスト
```rust
#[test]
fn test_crud_workflow_json() {
    // JSONデータソース → CREATE → READ → UPDATE → DELETE
    // 全操作がイベント記録されるか確認
}
```

#### 10.2.2 ファイル永続化テスト
```rust
#[test]
fn test_persistence_across_restart() {
    // DB.save() → プロセス終了 → DB.load()
    // 全データが復旧されるか確認
}
```

#### 10.2.3 イベント復旧テスト
```rust
#[test]
fn test_recovery_from_event_log() {
    // イベントログから状態を完全復旧
    // 復旧後の状態が元の状態と一致するか確認
}
```

---

## 11. 実装フェーズ・スケジュール

### 11.1 Phase 1：基本実装（3-4週）

**目標** ：KV + イベント型（シングルマシン版）の動作

| 項目 | 期間 | 内容 |
|------|------|------|
| **Week 1-2** | 2週 | KV Store実装、Get/Set/Delete（論理削除）、メモリ管理 |
| **Week 2-3** | 1.5週 | イベントログ実装、ハッシュチェーン検証、外部検証エクスポート API |
| **Week 3-4** | 1.5週 | ファイル永続化、ロード機能、イベントログ圧縮ポリシー実装 |
| **Week 4** | 0.5週 | SQLite 移行ツール（`sqlite-to-adlaire` コマンド、JSON 経由インポート） |

**成果物** ：
- KV Store の基本操作が動く（Delete は論理削除のみ）
- イベント記録・検証が動く
- ファイルの save/load が動く
- 外部検証エクスポート（`/api/v1/audit/export`）が動く
- イベントログ圧縮が動く（手動トリガー）
- SQLite → Adlaire DB 移行ツールが動く

**テスト** ：ユニットテスト + 基本的な統合テスト

---

### 11.2 Phase 2：JOIN 実装（3-4週）

**目標** ：JOIN 機能の実装・テスト

| 項目 | 期間 | 内容 |
|------|------|------|
| **Week 1-2** | 2週 | Hash Join実装、スカラー JOIN |
| **Week 2-3** | 1週 | エラーハンドリング、エッジケース対応 |
| **Week 3-4** | 1週 | JOIN パフォーマンステスト、最適化 |

**成果物** ：
- Hash Join が動く
- 複数キーの結合取得が可能
- JOIN テストが全てパス

---

### 11.3 Phase 3：トランザクション・ACID（2-3週）

**目標** ：ACID 機構の完全実装

| 項目 | 期間 | 内容 |
|------|------|------|
| **Week 1-2** | 1.5週 | ロック機構、トランザクションマネージャー実装 |
| **Week 2-3** | 1.5週 | MVCC、ロールバック、コンフリクト検出 |

**成果物** ：
- Begin/Commit/Rollback が動く
- ロック・デッドロック対策が動く
- トランザクションテストが全てパス

---

### 11.4 Phase 4：テスト・最適化（1-2週）

**目標** ：全機能テスト + パフォーマンス最適化

| 項目 | 期間 | 内容 |
|------|------|------|
| **Week 1** | 1週 | CRUD統合テスト、ストレス テスト |
| **Week 2** | 1週 | パフォーマンス測定、ボトルネック最適化 |

**成果物** ：
- 全テストケース実装・パス
- ベンチマーク報告書
- ドキュメント完成

---

### 11.5 全体スケジュール

```
Phase 1 (KV + イベント型)：Weeks 1-4
  ├─ Week 1-2：KV Store 実装
  ├─ Week 2-3：イベント実装
  └─ Week 3-4：永続化実装

Phase 2 (JOIN)：Weeks 5-8
  ├─ Week 5-6：Hash Join 実装
  ├─ Week 6-7：エラー処理
  └─ Week 7-8：最適化

Phase 3 (ACID)：Weeks 9-11
  ├─ Week 9-10：ロック・トランザクション
  └─ Week 10-11：MVCC・ロールバック

Phase 4 (テスト・最適化)：Weeks 12-13
  ├─ Week 12：統合テスト
  └─ Week 13：最適化

総計：約13週（約3ヶ月）
```

---

## 12. JSONデータソース対応

### 12.1 JSON入力フォーマット

```json
{
  "operations": [
    {
      "op": "create",
      "key": "user:1",
      "value": {
        "name": "John",
        "email": "john@example.com",
        "dept_id": "D1"
      }
    },
    {
      "op": "update",
      "key": "user:1",
      "value": {
        "name": "John",
        "email": "john.new@example.com",
        "dept_id": "D1"
      }
    },
    {
      "op": "delete",
      "key": "user:1"
    }
  ]
}
```

### 12.2 JSON出力フォーマット

#### 12.2.1 イベントログ出力
```json
{
  "events": [
    {
      "id": "evt-1",
      "event_type": "Created",
      "key": "user:1",
      "payload": {...},
      "timestamp": 1694250000000,
      "hash": "ABC123..."
    }
  ]
}
```

#### 12.2.2 JOIN 結果出力
```json
{
  "user": {
    "name": "John",
    "email": "john@example.com",
    "dept_id": "D1"
  },
  "dept": {
    "name": "Engineering"
  }
}
```

---

## 13. API 概要

### 13.1 Rust API（内部）

```rust
pub struct Database {
    // コア操作
    pub fn get(&self, key: &str) -> Result<Option<JSON>>;
    pub fn set(&mut self, key: &str, value: JSON) -> Result<()>;
    pub fn delete(&mut self, key: &str) -> Result<()>;
    pub fn scan(&self, prefix: Option<&str>) -> Result<Vec<String>>;
    
    // イベント操作
    pub fn get_history(&self, key: &str) -> Result<Vec<Event>>;
    pub fn follow_chain(&self) -> Result<bool>;
    pub fn get_events_since(&self, timestamp: i64) -> Result<Vec<Event>>;
    
    // JOIN
    pub fn join(&self, primary_key: &str, specs: Vec<JoinSpec>) -> Result<JSON>;
    pub fn scalar_join(&self, key: &str, joins: Vec<(&str, &str)>) -> Result<JSON>;
    
    // トランザクション
    pub fn begin_transaction(&mut self) -> TransactionHandle;
    pub fn commit(&mut self, handle: TransactionHandle) -> Result<()>;
    pub fn rollback(&mut self, handle: TransactionHandle) -> Result<()>;
    
    // ロック
    pub fn lock_row(&mut self, key: &str, lock_type: LockType) -> Result<()>;
    pub fn unlock_row(&mut self, key: &str) -> Result<()>;
    
    // 永続化
    pub fn save(&self, path: &str) -> Result<()>;
    pub fn load(path: &str) -> Result<Self>;
}
```

---

## 14. 今後の拡張（プロトタイプ後）

- **分散対応** ：イベントレプリケーション、マスター・スレーブ構成
- **GC戦略** ：履歴圧縮、ウィンドウ管理
- **インデックス最適化** ：B+ Tree 導入
- **SQLクエリ層** ：SQL パーサ・エグゼキューター（オプション）
- **外部API** ：REST API、gRPC インターフェース

---

## 15. 開発環境（Docker ベース）

### 15.1 開発環境構成

**Docker ベースの開発環境** により、Windows/macOS/Linux 問わず統一された環境で開発可能。

```
開発者PC（任意のOS）
  ↓
Docker Desktop（またはインストール版）
  ↓
Rust開発コンテナ（Ubuntu 24.04 LTS + Rust 1.70+）
  ↓
cargo build/test/cross-compile
```

### 15.2 プロジェクト構成

```
adlaire-db/
├── Dockerfile                 # 開発用コンテナイメージ定義
├── docker-compose.yml         # コンテナオーケストレーション
├── .dockerignore              # Docker ビルド時の除外ファイル
├── Cargo.toml                 # Rust パッケージ定義
├── Cargo.lock
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── kv_store.rs
│   ├── event_log.rs
│   ├── join_engine.rs
│   ├── transaction.rs
│   └── ...
├── tests/
│   ├── unit_tests.rs
│   ├── integration_tests.rs
│   └── ...
├── scripts/
│   ├── build.sh               # ビルドスクリプト
│   ├── test.sh                # テストスクリプト
│   └── cross-compile.sh       # クロスコンパイル
├── docs/
│   └── DEVELOPMENT.md         # 開発ガイド
└── README.md
```

### 15.3 Dockerfile

```dockerfile
# Dockerfile

FROM rust:1.70-bullseye

WORKDIR /workspace

# 必要なツールをインストール
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# クロスコンパイル用ツールをインストール
RUN rustup target add x86_64-unknown-linux-gnu
RUN rustup target add aarch64-unknown-linux-gnu

# Cargo キャッシュの最適化
ENV CARGO_HOME=/workspace/.cargo

ENTRYPOINT ["/bin/bash"]
```

### 15.4 docker-compose.yml

```yaml
# docker-compose.yml

version: '3.8'

services:
  dev:
    build:
      context: .
      dockerfile: Dockerfile
    image: adlaire-db:dev
    container_name: adlaire-db-dev
    volumes:
      # ワーキングディレクトリをマウント
      - .:/workspace
      # Cargo キャッシュの永続化
      - cargo_cache:/workspace/.cargo
    working_dir: /workspace
    environment:
      RUST_BACKTRACE: 1
    stdin_open: true
    tty: true
    # ビルド後、bash を起動し、対話的に操作可能

volumes:
  cargo_cache:
    driver: local
```

### 15.5 .dockerignore

```
.git
.gitignore
target/
.DS_Store
*.swp
*.swo
*.swn
*.log
.idea/
.vscode/
*.backup
.env
.env.local
```

### 15.6 開発ワークフロー

#### 15.6.1 初期セットアップ

```bash
# 1. リポジトリクローン
git clone https://github.com/adlaire-group/adlaire-db.git
cd adlaire-db

# 2. Docker イメージビルド
docker-compose build

# 3. 開発コンテナ起動
docker-compose run --rm dev
```

#### 15.6.2 ビルド

```bash
# コンテナ内で実行
cargo build --release

# バイナリ確認
ls -la target/release/adlaire-db
```

#### 15.6.3 ユニットテスト

```bash
# コンテナ内で実行
cargo test

# 特定のテストのみ実行
cargo test test_kv_set_get
```

#### 15.6.4 統合テスト

```bash
# コンテナ内で実行
cargo test --test '*'
```

#### 15.6.5 x86_64 Linux 用クロスコンパイル

```bash
# コンテナ内で実行
cargo build --release --target x86_64-unknown-linux-gnu

# バイナリ確認
file target/x86_64-unknown-linux-gnu/release/adlaire-db
```

#### 15.6.6 ARM64 Linux 用クロスコンパイル

```bash
# コンテナ内で実行
cargo build --release --target aarch64-unknown-linux-gnu

# バイナリ確認
file target/aarch64-unknown-linux-gnu/release/adlaire-db
```

#### 15.6.7 全プラットフォーム用ビルド

```bash
# scripts/cross-compile.sh
#!/bin/bash

# x86_64
cargo build --release --target x86_64-unknown-linux-gnu

# ARM64
cargo build --release --target aarch64-unknown-linux-gnu

# バイナリリスト
echo "=== Build Artifacts ==="
ls -la target/x86_64-unknown-linux-gnu/release/adlaire-db
ls -la target/aarch64-unknown-linux-gnu/release/adlaire-db
```

実行：
```bash
docker-compose run --rm dev bash scripts/cross-compile.sh
```

### 15.7 デバッグ

#### 15.7.1 コンテナ内で RUST_LOG 設定

```bash
export RUST_LOG=debug
cargo run --release

# または
RUST_LOG=debug cargo run --release
```

#### 15.7.2 lldb（デバッガ）を使用

```bash
# Dockerfile に lldb をインストール
RUN apt-get install -y lldb

# コンテナ内で実行
lldb ./target/release/adlaire-db
```

### 15.8 CI/CD 統合（GitHub Actions 例）

`.github/workflows/build.yml`
```yaml
name: Build & Test

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Build Docker image
        run: docker-compose build
      
      - name: Run tests
        run: docker-compose run --rm dev cargo test
      
      - name: Build x86_64
        run: docker-compose run --rm dev cargo build --release --target x86_64-unknown-linux-gnu
      
      - name: Build ARM64
        run: docker-compose run --rm dev cargo build --release --target aarch64-unknown-linux-gnu
      
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: binaries
          path: target/*/release/adlaire-db
```

### 15.9 トラブルシューティング

#### Docker イメージビルド失敗
```bash
# キャッシュクリア
docker-compose build --no-cache

# ディスク確認
docker system df
docker system prune
```

#### Cargo ダウンロード遅い
```bash
# Cargo レジストリを変更（.cargo/config.toml）
[source.crates-io]
replace-with = 'mirrors'

[source.mirrors]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"
```

---


## 16. 本番環境サーバ構成・デプロイ方法

### 16.1 本番サーバ仕様（3パターン）

#### 16.1.1 ハードウェア仕様

| 仕様 | 最小要件 | 推奨 | 高パフォーマンス |
|------|---------|------|---------|
| **CPU** | シングルコア以上 | 1コア | 4コア |
| **メモリ** | 512MB以上 | 2GB | 8GB |
| **ストレージ** | 10GB以上 | 50GB | 200GB |
| **ネットワーク** | 標準 | Gigabit Ethernet | Gigabit Ethernet（冗長化推奨） |
| **電源** | 標準 | 標準 | 冗長PSU + UPS |

**用途別ガイド：**
- **最小要件** ：開発環境、テスト環境、ラズパイ等の低スペック環境
- **推奨** ：標準本番環境（小～中規模運用）
- **高パフォーマンス** ：大規模本番環境、高トラフィック対応

#### 16.1.2 OS・ランタイム

| 項目 | 仕様 |
|------|------|
| **OS（本番環境）** | Ubuntu 24.04 LTS のみ |
| **CPU アーキテクチャ** | x86_64 / ARM64（aarch64） |
| **Systemd** | サービス管理用 |
| **Firewall** | iptables / ufw |

**注記** ：
- 本番環境：Ubuntu 24.04 LTS のみ
- テスト環境：Ubuntu 24.04 LTS（本番と同一）
- 開発環境：Windows/macOS/Linux（開発者のPC で Docker ベース開発）
- バイナリ：事前にテスト環境（Ubuntu 24.04 LTS）で生成・テスト後、本番へデプロイ

#### 16.1.3 ディレクトリ構成

```
/var/lib/adlaire-db/
├── bin/
│   └── adlaire-db          # バイナリ実行ファイル
├── data/
│   ├── kv_store.bin         # KV データ
│   ├── event_log.jsonl      # イベントログ
│   └── metadata.json        # メタデータ
├── logs/
│   ├── access.log           # アクセスログ
│   ├── error.log            # エラーログ
│   └── audit.log            # 監査ログ
├── backup/
│   ├── 2026-09-09.tar.gz
│   └── 2026-09-10.tar.gz
└── config/
    └── adlaire-db.toml      # 設定ファイル
```

---

### 16.2 デプロイ方法（推奨）

#### 16.2.1 デプロイ方針：事前バイナリ化

**重要：本番環境ではソースコードを配布しない。ローカルで事前にバイナリ化し、テスト後に確定バイナリをデプロイ**

---

#### 16.2.2 デプロイ手順

**Step 1：ローカル開発環境でバイナリ化（開発者PC）**

**1-A：単一プラットフォーム用ビルド**
```bash
# ローカルで最終ビルド（Release モード）
cargo build --release

# バイナリ確認
./target/release/adlaire-db --version

# バイナリ情報確認
file ./target/release/adlaire-db
ldd ./target/release/adlaire-db  # 依存ライブラリ確認
```

**1-B：複数 Linux プラットフォーム用クロスコンパイル**
```bash
# x86_64 Linux 用
cargo build --release --target x86_64-unknown-linux-gnu
file target/x86_64-unknown-linux-gnu/release/adlaire-db

# ARM64 Linux 用（Apple Silicon や AWS Graviton 等）
cargo build --release --target aarch64-unknown-linux-gnu
file target/aarch64-unknown-linux-gnu/release/adlaire-db

# 本番環境のプラットフォームに合わせてバイナリを生成
```

**Step 2：テスト環境でバイナリ検証**
```bash
# テスト環境へ転送
scp target/release/adlaire-db \
  test-admin@test-db-01:/tmp/

# テスト環境で実行テスト
ssh test-admin@test-db-01 << 'EOF'
  /tmp/adlaire-db --version
  /tmp/adlaire-db --help
  # ユニットテスト・統合テスト実行
  # パフォーマンステスト実行
EOF

# テスト完了後、バイナリを確定
echo "バイナリテスト合格 - v1.0.0"
```

**Step 3：確定バイナリのアーティファクト化**
```bash
# テスト合格したバイナリをリリースパッケージ化
mkdir -p release-v1.0.0/bin
cp target/release/adlaire-db release-v1.0.0/bin/
cp docs/ release-v1.0.0/
cp config/adlaire-db.toml.example release-v1.0.0/config/

# パッケージ作成（ソースコードは含めない）
tar -czf adlaire-db-v1.0.0-x86_64-linux-TESTED.tar.gz \
  release-v1.0.0/

# チェックサム生成
sha256sum adlaire-db-v1.0.0-x86_64-linux-TESTED.tar.gz > checksums.txt

# バイナリ情報を記録
cat > adlaire-db-v1.0.0-MANIFEST.txt << 'MANIFEST'
Product: Adlaire DB Engine
Version: v1.0.0
Binary: x86_64 Linux ELF
Build Date: 2026-09-09
Status: TESTED & APPROVED
Checksum: (sha256sum)
MANIFEST
```

**Step 4：本番サーバへ転送（バイナリのみ）**
```bash
# 確定バイナリのみを本番環境に転送
# ソースコードは転送しない
scp adlaire-db-v1.0.0-x86_64-linux-TESTED.tar.gz \
  admin@prod-db-01:/tmp/

scp adlaire-db-v1.0.0-MANIFEST.txt \
  admin@prod-db-01:/tmp/

# SSH で本番環境に展開
ssh admin@prod-db-01 << 'EOF'
  # バイナリ転送確認
  sha256sum -c /tmp/checksums.txt
  
  # 展開
  tar -xzf /tmp/adlaire-db-v1.0.0-x86_64-linux-TESTED.tar.gz \
    -C /var/lib/adlaire-db/
  
  # 権限設定
  chmod 0755 /var/lib/adlaire-db/bin/adlaire-db
  chown adlaire-db:adlaire-db /var/lib/adlaire-db/bin/adlaire-db
  
  # バイナリ検証
  /var/lib/adlaire-db/bin/adlaire-db --version
  
  # マニフェスト保存
  cp /tmp/adlaire-db-v1.0.0-MANIFEST.txt /var/lib/adlaire-db/
EOF
```

**Step 5：Systemd サービス登録**

`/etc/systemd/system/adlaire-db.service`
```ini
[Unit]
Description=Adlaire DB Engine
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=adlaire-db
Group=adlaire-db
WorkingDirectory=/var/lib/adlaire-db
ExecStart=/var/lib/adlaire-db/bin/adlaire-db --config /var/lib/adlaire-db/config/adlaire-db.toml
ExecReload=/bin/kill -SIGHUP $MAINPID
KillMode=process
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

**Step 6：サービス開始**
```bash
sudo systemctl daemon-reload
sudo systemctl enable adlaire-db
sudo systemctl start adlaire-db
sudo systemctl status adlaire-db
```

#### 16.2.2 ロールバック手順

```bash
# 前バージョンをバックアップから復旧
sudo systemctl stop adlaire-db
sudo rm -rf /var/lib/adlaire-db/bin/adlaire-db
sudo tar -xzf /tmp/adlaire-db-v0.9.0-x86_64-linux.tar.gz \
  -C /var/lib/adlaire-db/bin/
sudo systemctl start adlaire-db
```

---

### 16.3 本番環境の初期設定

#### 16.3.1 設定ファイル（adlaire-db.toml）

```toml
[server]
listen_port = 9876
bind_address = "0.0.0.0"

[storage]
data_dir = "/var/lib/adlaire-db/data"
backup_dir = "/var/lib/adlaire-db/backup"

[logging]
log_level = "INFO"          # DEBUG / INFO / WARN / ERROR
log_file = "/var/lib/adlaire-db/logs/app.log"
audit_log = "/var/lib/adlaire-db/logs/audit.log"
max_log_size_mb = 100
max_log_files = 10

[transaction]
timeout_seconds = 300
lock_timeout_seconds = 5

[performance]
buffer_size_mb = 512
flush_interval_seconds = 5

[backup]
auto_backup_enabled = true
backup_interval_hours = 24
retention_count = 7
```

#### 16.3.2 ユーザー・権限設定

```bash
# adlaire-db ユーザー作成
sudo useradd -r -s /bin/false adlaire-db

# ディレクトリ権限設定
sudo chown -R adlaire-db:adlaire-db /var/lib/adlaire-db
sudo chmod -R 0750 /var/lib/adlaire-db
sudo chmod 0755 /var/lib/adlaire-db/bin/adlaire-db
```

---

### 16.4 本番環境の監視・運用

#### 16.4.1 ヘルスチェック

```bash
#!/bin/bash
# healthcheck.sh

# プロセス確認
if ! pgrep -f adlaire-db > /dev/null; then
  echo "CRITICAL: adlaire-db プロセスが起動していません"
  exit 2
fi

# ファイルシステム確認
df -h /var/lib/adlaire-db/ | tail -1 | awk '{
  if ($5 > 80) {
    print "WARNING: ディスク使用率が " $5 " です"
    exit 1
  }
}'

# イベントログファイルチェック
if [ ! -f /var/lib/adlaire-db/data/event_log.jsonl ]; then
  echo "CRITICAL: event_log.jsonl が見つかりません"
  exit 2
fi

echo "OK: adlaire-db ヘルスチェック合格"
exit 0
```

実行方法：
```bash
# Cron で1時間ごとに実行
0 * * * * /var/lib/adlaire-db/scripts/healthcheck.sh
```

#### 16.4.2 ロギング・監視

**ログ監視ツール** ：Prometheus + Grafana

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'adlaire-db'
    static_configs:
      - targets: ['localhost:9876']
    metrics_path: '/metrics'
```

**メトリクス監視項目** ：
- プロセスメモリ使用量
- ディスク使用率
- イベントログサイズ
- トランザクション数（時系列）
- ロック競合数
- エラー発生数

#### 16.4.3 バックアップ戦略

**バックアップ対象** ：
- `/var/lib/adlaire-db/data/` （全て）
- `/var/lib/adlaire-db/config/` （設定）

**バックアップスケジュール** ：
```bash
# 日次フル バックアップ（03:00 UTC）
0 3 * * * /var/lib/adlaire-db/scripts/backup.sh full

# 6時間ごと増分バックアップ
0 */6 * * * /var/lib/adlaire-db/scripts/backup.sh incremental
```

**保持ポリシー** ：
- 日次：直近7日間
- 週次：直近4週間
- 月次：直近12ヶ月

---

### 16.5 本番環境での段階的導入

#### 16.5.1 Phase 1：読取のみ（Week 1-2）
- DBは起動しているが、**読取オンリーモード**
- 既存システムとの並行運用
- パフォーマンス・安定性を観察

#### 16.5.2 Phase 2：段階的書込（Week 3-4）
- 新規データは DB に書き込み開始
- 既存データはレガシーシステムで保持
- トランザクション動作確認

#### 16.5.3 Phase 3：完全運用（Week 5+）
- 全データを DB で管理
- レガシーシステムとの連携終了
- 本番運用開始

---

## 17. まとめ

このプロトタイプ版DBエンジンは、**データ整合性・保全・可用性を同率で重視** し、**KV + イベント型アーキテクチャ** で実現する。

**開発フロー** ：ローカル開発 → テスト環境検証 → 本番環境段階的導入

**本番環境** ：Ubuntu 24.04 LTS、8コアCPU、16GB以上メモリ

**次ステップ** ：
1. 開発環境セットアップ（Cargo.toml、プロジェクト構成）
2. Phase 1 実装開始（KV + イベント型）
3. テスト環境構築
4. 本番環境構築

---

## 18. ネットワークインターフェース

Adlaire DB は以下の 2 つのプロトコルをサポートします：

### 18.1 TCP（ポート 9876）：カスタムバイナリプロトコル

**用途** ：高パフォーマンス、低レイテンシ通信

#### 18.1.1 プロトコル仕様

```
【パケット構造】

┌──────────────────┬─────────────┬──────────────────┐
│ 4 Bytes (Length) │ 1 Byte (Seq)│   Payload        │
└──────────────────┴─────────────┴──────────────────┘

- Length: パケット長（ペイロードのみ）
- Seq: シーケンス番号（リクエスト/レスポンス対応）
- Payload: JSON または MessagePack形式
```

#### 18.1.2 対応クライアント言語

- Rust
- Python
- Node.js / JavaScript
- Go
- Java
- その他（カスタムクライアント実装）

#### 18.1.3 クライアント実装例（Python）

```python
import socket
import json
import struct

class AdlaireDBClient:
    def __init__(self, host='localhost', port=9876):
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.socket.connect((host, port))
        self.seq = 0
    
    def _send_request(self, command, **kwargs):
        """リクエスト送信"""
        self.seq += 1
        payload = json.dumps({
            'command': command,
            'seq': self.seq,
            **kwargs
        }).encode('utf-8')
        
        # パケット化
        length = len(payload)
        packet = struct.pack('!I', length) + struct.pack('!B', self.seq) + payload
        self.socket.sendall(packet)
    
    def _recv_response(self):
        """レスポンス受信"""
        # 長さ取得
        length_data = self.socket.recv(4)
        length = struct.unpack('!I', length_data)[0]
        
        # シーケンス取得
        seq_data = self.socket.recv(1)
        
        # ペイロード取得
        payload = self.socket.recv(length)
        return json.loads(payload.decode('utf-8'))
    
    def get(self, key):
        """KV Get"""
        self._send_request('GET', key=key)
        return self._recv_response()
    
    def set(self, key, value):
        """KV Set"""
        self._send_request('SET', key=key, value=value)
        return self._recv_response()
    
    def close(self):
        self.socket.close()

# 使用例
client = AdlaireDBClient()
result = client.get('user:1')
print(result)
client.close()
```

#### 18.1.4 コマンド仕様

**KV Get**
```json
{
  "command": "GET",
  "key": "user:1"
}
```

**KV Set**
```json
{
  "command": "SET",
  "key": "user:1",
  "value": {"name": "John", "email": "john@example.com"}
}
```

**KV Delete**
```json
{
  "command": "DELETE",
  "key": "user:1"
}
```

**Event Log Append**
```json
{
  "command": "APPEND_EVENT",
  "key": "order:1",
  "event": {"action": "created", "timestamp": "2026-09-09T12:00:00Z"}
}
```

**Event Log Query**
```json
{
  "command": "GET_HISTORY",
  "key": "order:1",
  "from": 0,
  "limit": 100
}
```

**JOIN**
```json
{
  "command": "JOIN",
  "left_key": "user:1",
  "right_table": "dept",
  "on": "dept_id"
}
```

---

### 18.2 REST API（ポート 8080）

**用途** ：標準インターフェース、ウェブアプリケーション、ロードバランサー対応

#### 18.2.1 API エンドポイント

**基本URL** ：`http://localhost:8080/api/v1`

#### 18.2.2 KV API

**Get**
```bash
GET /api/v1/kv/:key

# リクエスト例
curl http://localhost:8080/api/v1/kv/user:1

# レスポンス
{
  "success": true,
  "key": "user:1",
  "value": {"name": "John", "email": "john@example.com"}
}
```

**Set**
```bash
PUT /api/v1/kv/:key

# リクエスト例
curl -X PUT http://localhost:8080/api/v1/kv/user:1 \
  -H "Content-Type: application/json" \
  -d '{"name": "John", "email": "john@example.com"}'

# レスポンス
{
  "success": true,
  "key": "user:1",
  "message": "Key set successfully"
}
```

**Delete**
```bash
DELETE /api/v1/kv/:key

# リクエスト例
curl -X DELETE http://localhost:8080/api/v1/kv/user:1

# レスポンス
{
  "success": true,
  "key": "user:1",
  "message": "Key deleted successfully"
}
```

#### 18.2.3 Event Log API

**Append Event**
```bash
POST /api/v1/events/:key

# リクエスト例
curl -X POST http://localhost:8080/api/v1/events/order:1 \
  -H "Content-Type: application/json" \
  -d '{"action": "created", "timestamp": "2026-09-09T12:00:00Z"}'

# レスポンス
{
  "success": true,
  "key": "order:1",
  "event_id": 1,
  "message": "Event appended successfully"
}
```

**Get History**
```bash
GET /api/v1/events/:key?from=0&limit=100

# リクエスト例
curl "http://localhost:8080/api/v1/events/order:1?from=0&limit=100"

# レスポンス
{
  "success": true,
  "key": "order:1",
  "total": 5,
  "events": [
    {"id": 1, "action": "created", "timestamp": "2026-09-09T12:00:00Z"},
    {"id": 2, "action": "updated", "timestamp": "2026-09-09T12:05:00Z"}
  ]
}
```

#### 18.2.4 JOIN API

```bash
GET /api/v1/join/:left_key/:right_table?on=field

# リクエスト例
curl "http://localhost:8080/api/v1/join/user:1/dept?on=dept_id"

# レスポンス
{
  "success": true,
  "left_key": "user:1",
  "right_table": "dept",
  "result": {
    "user": {"name": "John", "dept_id": "10"},
    "dept": {"id": "10", "name": "Engineering"}
  }
}
```

#### 18.2.5 クライアント実装例（PHP）

```php
<?php

class AdlaireDBClient {
    private $baseUrl;
    
    public function __construct($host = 'localhost', $port = 8080) {
        $this->baseUrl = "http://{$host}:{$port}/api/v1";
    }
    
    public function get($key) {
        $url = $this->baseUrl . "/kv/" . urlencode($key);
        $response = file_get_contents($url);
        return json_decode($response, true);
    }
    
    public function set($key, $value) {
        $url = $this->baseUrl . "/kv/" . urlencode($key);
        $opts = [
            'http' => [
                'method' => 'PUT',
                'header' => 'Content-Type: application/json',
                'content' => json_encode($value)
            ]
        ];
        $context = stream_context_create($opts);
        $response = file_get_contents($url, false, $context);
        return json_decode($response, true);
    }
    
    public function delete($key) {
        $url = $this->baseUrl . "/kv/" . urlencode($key);
        $opts = [
            'http' => ['method' => 'DELETE']
        ];
        $context = stream_context_create($opts);
        $response = file_get_contents($url, false, $context);
        return json_decode($response, true);
    }
    
    public function appendEvent($key, $event) {
        $url = $this->baseUrl . "/events/" . urlencode($key);
        $opts = [
            'http' => [
                'method' => 'POST',
                'header' => 'Content-Type: application/json',
                'content' => json_encode($event)
            ]
        ];
        $context = stream_context_create($opts);
        $response = file_get_contents($url, false, $context);
        return json_decode($response, true);
    }
    
    public function getHistory($key, $from = 0, $limit = 100) {
        $url = $this->baseUrl . "/events/" . urlencode($key) . "?from=$from&limit=$limit";
        $response = file_get_contents($url);
        return json_decode($response, true);
    }
    
    public function join($leftKey, $rightTable, $on) {
        $url = $this->baseUrl . "/join/" . urlencode($leftKey) . "/" . urlencode($rightTable) . "?on=" . urlencode($on);
        $response = file_get_contents($url);
        return json_decode($response, true);
    }
}

// 使用例
$db = new AdlaireDBClient();

// Get
$result = $db->get('user:1');
var_dump($result);

// Set
$db->set('user:1', ['name' => 'John', 'email' => 'john@example.com']);

// Get History
$history = $db->getHistory('order:1');
var_dump($history);

?>
```

#### 18.2.6 クライアント実装例（JavaScript）

```javascript
class AdlaireDBClient {
    constructor(host = 'localhost', port = 8080) {
        this.baseUrl = `http://${host}:${port}/api/v1`;
    }
    
    async get(key) {
        const response = await fetch(`${this.baseUrl}/kv/${encodeURIComponent(key)}`);
        return response.json();
    }
    
    async set(key, value) {
        const response = await fetch(`${this.baseUrl}/kv/${encodeURIComponent(key)}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(value)
        });
        return response.json();
    }
    
    async delete(key) {
        const response = await fetch(`${this.baseUrl}/kv/${encodeURIComponent(key)}`, {
            method: 'DELETE'
        });
        return response.json();
    }
    
    async appendEvent(key, event) {
        const response = await fetch(`${this.baseUrl}/events/${encodeURIComponent(key)}`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(event)
        });
        return response.json();
    }
    
    async getHistory(key, from = 0, limit = 100) {
        const response = await fetch(
            `${this.baseUrl}/events/${encodeURIComponent(key)}?from=${from}&limit=${limit}`
        );
        return response.json();
    }
    
    async join(leftKey, rightTable, on) {
        const response = await fetch(
            `${this.baseUrl}/join/${encodeURIComponent(leftKey)}/${encodeURIComponent(rightTable)}?on=${encodeURIComponent(on)}`
        );
        return response.json();
    }
}

// 使用例
const db = new AdlaireDBClient();

// Get
const result = await db.get('user:1');
console.log(result);

// Set
await db.set('user:1', {name: 'John', email: 'john@example.com'});

// Get History
const history = await db.getHistory('order:1');
console.log(history);
```

---

### 18.3 パフォーマンス比較

| 項目 | TCP（ポート 9876） | REST API（ポート 8080） |
|------|---|---|
| **レイテンシ** | 低（1-5ms） | 中（5-15ms） |
| **スループット** | 高 | 中 |
| **オーバーヘッド** | 少ない | HTTP ヘッダ分多い |
| **用途** | リアルタイム処理 | ウェブアプリ |
| **ロードバランサー** | 対応可 | 標準対応 |
| **ブラウザ接続** | 不可 | 可（JavaScript Fetch API） |

---

### 18.4 サーバ実装（Rust）概要

```rust
// 擬似コード

use tokio::net::TcpListener;
use axum::Router;

#[tokio::main]
async fn main() {
    // TCP サーバ（ポート 9876）
    let tcp_listener = TcpListener::bind("127.0.0.1:9876").await.unwrap();
    tokio::spawn(async move {
        handle_tcp_connections(tcp_listener).await;
    });
    
    // REST API サーバ（ポート 8080）
    let app = Router::new()
        .route("/api/v1/kv/:key", axum::routing::get(get_kv)
            .put(set_kv)
            .delete(delete_kv))
        .route("/api/v1/events/:key", axum::routing::post(append_event)
            .get(get_history))
        .route("/api/v1/join/:left_key/:right_table", axum::routing::get(join));
    
    axum::Server::bind(&"127.0.0.1:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

---

```
1. サーバー起動時
   ├─ metadata.json, shard_map.json のハッシュ検証
   ├─ 全シャードの data.kv チェックサム検証
   ├─ journal.log の未コミットトランザクションリカバリ
   └─ ロック残骸クリア

2. トランザクション実行前
   ├─ ファイルロック取得
   └─ ロック持有者の生存確認

3. トランザクション完了後（コミット時）
   ├─ journal.log に COMMITTED 記録
   ├─ data.kv 更新
   ├─ チェックサム再計算
   ├─ メタデータ更新（新バージョン情報）
   └─ ロック解放

4. 定期チェック（例：1時間ごと）
   ├─ 全ファイルのハッシュ検証
   ├─ デッドロック検知・解放
   └─ 古いバージョンの圧縮・削除

5. エラー発生時
   ├─ journal.log から状態確認
   ├─ 必要に応じてロールバック実行
   └─ 監査ログに記録
```

#### 2.2.4 ファイル仕様詳細

**【ファイル1】metadata.dat（バイナリ形式、約1-10KB）**

メタデータ、チェックサム、バージョン情報、クラスタ情報を統合管理。

```
【ヘッダ部（固定）】
[Magic: 4B]             # "AADB" (0x41414442)
[Version: 2B]           # フォーマットバージョン
[Header Size: 2B]       # ヘッダサイズ

【メタデータセクション】
[Shard ID: 4B]
[Shard Version: 4B]
[Timestamp: 8B]
[Data Size: 8B]         # data.kv のサイズ
[Data Hash: 32B]        # data.kv の SHA256
[TxLog Size: 8B]        # txlog.dat のサイズ
[TxLog Hash: 32B]       # txlog.dat の SHA256
[Metadata Hash: 32B]    # このセクション自体のハッシュ

【クラスタ情報セクション】
[Cluster Mode: 1B]      # 0=Single, 1=Replicated, 2=Sharded
[Node Count: 2B]
[Node URL Array]        # "node1:9876|node2:9876|..."

【バージョン管理セクション】
[Current Version: 4B]
[Version Count: 2B]
[Version Entry 1: Timestamp + Hash]
[Version Entry 2: Timestamp + Hash]
...
```

**JSON表現（参考）：**
```json
{
  "magic": "AADB",
  "version": 1,
  "shard_id": 0,
  "shard_version": 4,
  "timestamp": "2026-09-09T12:30:00Z",
  "data_kv": {
    "size": 1048576,
    "hash": "a3c4f2e8d9b1c6a7e2f8d9b1c6a7e2f8"
  },
  "txlog": {
    "size": 8192,
    "hash": "b4d5e3f9a2c7d8b6e3f9a2c7d8b6e3f9"
  },
  "cluster": {
    "mode": "single",
    "nodes": ["localhost:9876"]
  },
  "versions": [
    {"version": 4, "timestamp": "2026-09-09T12:30:00Z", "hash": "c5e6f4..."},
    {"version": 3, "timestamp": "2026-09-09T12:25:00Z", "hash": "d6f7e5..."}
  ]
}
```

---

**【ファイル2】data.kv（バイナリ形式、可変、通常 1MB～ GB単位）**

KV Store とイベントログを複合管理（アペンド・オンリー）。

```
【KV Store 部】
[Key1 Length: 4B][Key1][Value1 Length: 4B][Value1: JSON]
[Key2 Length: 4B][Key2][Value2 Length: 4B][Value2: JSON]
...

【イベントログ部（アペンド・オンリー）】
各キーに対する変更イベント：
[Timestamp: 8B][Operation: 1B][Key Length: 4B][Key][Event Data Length: 4B][Event Data: JSON]
...

【インデックス（メモリキャッシュまたは別途管理）】
Key → オフセット マッピング
例：
  "user:1" → Offset: 1024
  "user:2" → Offset: 2048
```

**例データ：**
```
Offset 0:
  [4][user:1][52][{"name": "John", "email": "john@example.com"}]
  [4][user:2][48][{"name": "Jane", "email": "jane@example.com"}]

イベント部：
  [1694250600000][SET]["user:1"][52][{"name": "John", "email": "john@example.com"}]
  [1694250605000][UPDATE]["user:1"][52][{"name": "John Doe", "email": "john@example.com"}]
```

---

**【ファイル3】txlog.dat（バイナリ形式、通常 10KB～ MB単位）**

トランザクションログ + ジャーナルログ（Write-Ahead Logging）を統合。

```
【トランザクションログ部】
[TX ID: 8B][TX Type: 1B][Status: 1B][Timestamp: 8B][Data Length: 4B][Data][Checksum: 32B]

TX Type:
  0x00 = SET      # KV 設定
  0x01 = DELETE   # KV 削除
  0x02 = APPEND   # イベント追加
  0x03 = BATCH    # バッチ操作

Status:
  0x00 = PENDING    # トランザクション開始（未コミット）
  0x01 = COMMITTED  # コミット完了
  0x02 = APPLIED    # ファイル適用完了
  0x03 = ABORTED    # ロールバック

【ジャーナルログ部（WAL - Write-Ahead Logging）】
トランザクションのコミット前にこのセクションに記録
[JournalState: 1B][TX ID: 8B][Data][Checksum: 32B]
...
```

**例：**
```
TX 1001:
  [001001][SET][PENDING][1694250600][52][{"user:1": {...}}][hash...]
  [001001][SET][COMMITTED][1694250600][52][{"user:1": {...}}][hash...]
  [001001][SET][APPLIED][1694250601][52][{"user:1": {...}}][hash...]

TX 1002:
  [001002][APPEND][PENDING][1694250605][...][hash...]
  [001002][APPEND][COMMITTED][1694250605][...][hash...]
```

#### 2.2.5 将来の分散対応設計

ファイル構成を分散対応として設計し、将来以下を実装可能：

**Phase 2：レプリケーション（Master-Replica）**
```
Master Node: shard_0, shard_1, shard_2, shard_3
  ↓ (複製 + チェックサム同期)
Replica Node A: shard_0, shard_1, shard_2, shard_3
  ↓ (複製 + チェックサム同期)
Replica Node B: shard_0, shard_1, shard_2, shard_3

metadata.json で "cluster_nodes" に Replica を追加
チェックサムが一致していることで整合性保証
```

**Phase 3：シャーディング（複数ノード分散）**
```
Node A: Shard 0, 1
Node B: Shard 2, 3
Node C: Shard 4, 5

shard_map.json でシャード配置を管理
各クライアント要求に対し、Coordinator が適切なノードにルーティング
ハッシュ検証で各ノードのデータ整合性確認
```

**Phase 4：分散トランザクション（2-Phase Commit）**
```
Client が複数シャードにまたがるトランザクション実行
→ Coordinator が 2-Phase Commit で調整
→ すべてのシャードで ACID を保証
→ 一部失敗時は全ノードロールバック
→ journal.log でリカバリポイント管理
```

---

---

## 19. パフォーマンス基準

### 19.1 パフォーマンス目標

| 項目 | TCP（ポート 9876） | REST API（ポート 8080） | 測定環境 |
|------|---|---|---|
| **レイテンシ** | < 5ms（P99） | < 15ms（P99） | 推奨仕様マシン |
| **スループット** | 1000+ ops/sec | 500+ ops/sec | 推奨仕様マシン |
| **最大接続数** | 1000+ 同時接続 | 500+ 同時接続 | 推奨仕様マシン |
| **メモリ使用量** | < 500MB（待機時） | < 800MB（100ops/sec） | 推奨仕様マシン |

### 19.2 大規模データテスト基準

**データサイズ別性能テスト：**
```
シナリオ1：小規模（< 10MB）
  - 各キー：< 1KB
  - 総キー数：< 10,000
  - 目標レイテンシ：< 5ms

シナリオ2：中規模（10MB - 1GB）
  - 各キー：< 100KB
  - 総キー数：< 100,000
  - 目標レイテンシ：< 10ms
  - インデックスメモリ：< 50MB

シナリオ3：大規模（1GB - 10GB）
  - 各キー：< 1MB
  - 総キー数：< 1,000,000
  - 目標レイテンシ：< 50ms
  - インデックスメモリ：< 200MB（永続化推奨）

シナリオ4：超大規模（> 10GB）
  - インデックス永続化必須
  - SSD ストレージ必須
  - メモリマップドI/O 推奨
```

### 19.3 インデックス戦略

**Phase 1（メモリベース）：**
```rust
HashMap<String, u64> // Key -> Offset マッピング
- メモリ効率：1キーあたり約100-200バイト
- 対応データサイズ：< 1GB推奨
```

**Phase 1.5（永続化オプション）：**
```
ディスク上の B+Tree インデックス
- 構造：shard_N/index.btree
- メモリ使用量削減：> 50%
- 対応データサイズ：< 10GB
```

**Phase 2（分散インデックス）：**
```
各ノードで独立したインデックス管理
Coordinator が複数ノードのインデックス集約
```

---

## 20. 分散実装の詳細仕様（Phase 2-4）

FoundationDB の「アンバンドル・アーキテクチャ」と OCC+MVCC トランザクションモデルを参考に設計する。FDB の既知の制約（トランザクション 5 秒ハード制限、ACL なし）を Adlaire-DB では改善する。

### 20.1 アンバンドル・アーキテクチャ（Unbundled Architecture）

FDB はすべてのコンポーネントを独立したロールに分離し、各ロールが単一責務を持つ。Adlaire-DB の分散フェーズもこの原則を採用する。

**ロールマップ：**
```
┌─────────────────────────────────────────────────────────┐
│                       クライアント                       │
└────────────────────────┬────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────┐
│                  Coordinator 層                          │
│  ├─ Cluster Controller（クラスタ状態管理・世代管理）      │
│  ├─ Coordinators（クォーラム選挙・設定保存）              │
│  └─ Master / Sequencer（バージョン払い出し）             │
└───────┬────────────────────────────────────────┬────────┘
        │                                        │
┌───────▼──────────┐                  ┌──────────▼───────┐
│   プロキシ層      │                  │   ストレージ層    │
│  ├─ GRV Proxy   │                  │  ├─ TLog         │
│  │  (ReadVer.)  │                  │  │  (WAL-first)  │
│  └─ Commit Proxy│                  │  ├─ StorageServer│
│     (コミット)   │                  │  │  (KV + Event) │
└───────┬──────────┘                  │  └─ Data         │
        │                             │     Distributor  │
┌───────▼──────────┐                  └──────────────────┘
│   競合検出層      │
│  └─ Resolver    │
│     (OCC 検証)  │
└──────────────────┘
```

**各ロールの責務：**

| ロール | 責務 | Adlaire-DB 実装方針 |
|--------|------|---------------------|
| Cluster Controller | クラスタ全体の状態監視・世代管理 | Phase 2 から導入 |
| Coordinators | クォーラム（奇数台）による設定保存 | Phase 2: 3 台、Phase 3: 5 台 |
| Master/Sequencer | 単調増加バージョン番号（ReadVersion）払い出し | Phase 3 から独立プロセス |
| GRV Proxy | クライアントからの GetReadVersion 集約 | Phase 3 |
| Commit Proxy | コミット要求受付・TLog への書き込み指示 | Phase 3 |
| Resolver | OCC 競合検出（最近コミットされた書き込み履歴保持） | Phase 4 |
| TLog（Transaction Log） | WAL-first 永続化（ストレージへの非同期適用） | Phase 2 から |
| Storage Server | KV データ + イベントログ保持・読み取り提供 | Phase 2 から |
| Data Distributor | シャード再配置・レプリカ均衡化 | Phase 3 |
| Ratekeeper | バックプレッシャー制御 | Phase 3 |

### 20.2 OCC + MVCC トランザクションモデル

FDB の OCC（楽観的並行制御）+ MVCC（多版並行制御）を Adlaire-DB の分散フェーズに採用する。

**設計原則：**
- **読み取り時にロックを取得しない**（OCC）。読み取りはすべてスナップショットバージョンで行う（MVCC）
- **書き込みはクライアントバッファに蓄積**し、コミット時のみサーバーへ送信
- **競合検出はコミット時**に Resolver が実施。競合があれば即座に ABORT（再試行はクライアント責務）

**トランザクションライフサイクル：**
```
1. BEGIN
   - GRV Proxy から ReadVersion（RV）を取得
   - クライアントは「RV 時点のスナップショット」で読み取り

2. 読み取り（MVCC）
   - Storage Server に RV を指定してリクエスト
   - RV より新しい書き込みは見えない
   - 読んだキーのセット（read_set）をローカル追跡

3. 書き込み
   - 変更をすべてクライアントのローカルバッファに蓄積
   - サーバーへの反映はコミット時まで保留

4. COMMIT
   a. Commit Proxy へ {read_set, write_set, payload} 送信
   b. Proxy が CommitVersion（CV）を割り当て
   c. Resolver で競合チェック
      - read_set のキーが RV ～ CV 間に書き込まれていないか確認
      - 書き込み履歴保持期間：設定可能（→ 20.8 参照）
   d. 競合なし → TLog に WAL 書き込み（永続化）
      競合あり → ABORT（クライアントに通知、再試行）
   e. TLog 永続化後にクライアントへ成功応答
   f. Storage Server へ非同期適用

5. ABORT / RETRY
   - 競合 ABORT を受けたクライアントは RV を再取得して最初からやり直し
```

**Adlaire-DB 独自の改善点（FDB との差分）：**
- FDB はトランザクション制限が**ハードコード**（5 秒、10MB、10KB key、100KB value）
- Adlaire-DB は**設定ファイルで調整可能**（→ 20.8 参照）
- FDB は ACL/認証なし → Adlaire-DB は Phase 2 から認証を組み込む（→ 20.9 参照）

### 20.3 WAL-first 耐久性設計

**原則：** ストレージへの反映よりも Transaction Log への WAL 書き込みを優先する。

```
クライアント → Commit Proxy
                    ↓
              CommitVersion 割り当て
                    ↓
           ┌─── TLog A（WAL 書き込み）───┐
           ├─── TLog B（WAL 書き込み）───┤  ← 3 台中 2 台以上成功で COMMIT
           └─── TLog C（WAL 書き込み）───┘
                    ↓（非同期）
              Storage Server A
              Storage Server B
              Storage Server C
```

**耐久性保証：**
- TLog への書き込みが完了した時点でトランザクション永続化済みとみなす
- Storage Server は TLog から非同期でデータを取り込む
- Storage Server クラッシュ時は TLog から再適用してリカバリ

### 20.4 Generation-based リカバリ

FDB の Generation-based Recovery を採用。システム障害時の回復を高速化する。

**Generation の定義：**
- すべてのコミットには **CommitVersion（CV）** と **世代番号（Generation ID）** が付与される
- `(Generation ID, CommitVersion)` の組みが書き込みの一意識別子となる

**リカバリフロー：**
```
障害発生（Master クラッシュ、ネットワーク分断など）

1. Cluster Controller が障害を検知
2. 新しい Generation ID を発行
3. 旧 Generation の Commit Proxy / Resolver が無効化
   - 旧世代のコミット試行は即座に ABORT（クライアント再試行）
4. 新 Master が TLog の最新状態から前 Generation の未完了 TX を復元
5. Storage Server が新 Generation の TLog からデータ同期
6. 新 Commit Proxy / Resolver 起動 → サービス再開
```

**ステートレス設計：**
- Commit Proxy / Resolver / GRV Proxy はステートレス
- クラッシュ後に即座に再起動可能（状態復元不要）
- 状態は TLog と Storage Server が保持

### 20.5 Phase 2：レプリケーション（WAL-first 非同期レプリケーション）

**アーキテクチャ：**
```
Primary Node
  │  Transaction Log（WAL）
  │  ├─ 書き込み完了 → クライアントへ成功応答
  │  └─ 非同期配信
  ├─→ Replica Node A（Storage Server）
  └─→ Replica Node B（Storage Server）
```

**レプリケーション戦略：**
- **方式**：WAL ストリーム非同期レプリケーション（RPO: < 1 秒）
- **レプリカ数**：デフォルト 2（設定可能）
- **読み取り**：Replica からも提供（Stale Read 許容 or 最新保証は接続オプションで選択）
- **フェイルオーバー**：Cluster Controller による自動昇格（手動オーバーライド可）

**イベントログ・ハッシュチェーン検証（Adlaire-DB 独自）：**
- Replica は WAL 適用後にイベントログのハッシュチェーンを独立検証する
- Primary との `event_log_root_hash` を定期比較（デフォルト: 60 秒ごと）
- 不一致を検出した場合は Cluster Controller に通知し、Replica を隔離

```rust
// Replica のハッシュチェーン検証フロー（擬似コード）
async fn verify_replica_hash_chain(
    replica: &ReplicaNode,
    primary_root: &Hash,
) -> VerifyResult {
    let replica_root = replica.compute_event_log_root_hash().await?;
    if replica_root != *primary_root {
        alert_cluster_controller(VerifyAlert::HashChainMismatch {
            replica_id: replica.id,
            expected: primary_root.clone(),
            actual: replica_root,
        });
        return Err(VerifyError::ChainMismatch);
    }
    Ok(VerifyResult::Ok)
}
```

### 20.6 Phase 3：シャーディング（Range-based Sharding）

FDB の Range-based Sharding（Ordered Key-Value）を採用。Hash-based よりも範囲スキャンに有利。

**シャード配置：**
```
key_range に基づくレンジ分割：

  Shard 0 (Node A): key < "m"
  Shard 1 (Node B): "m" <= key < "t"
  Shard 2 (Node C): key >= "t"

shard_map.json:
  {
    "shards": [
      {"id": 0, "range": ["", "m"],    "primary": "node-a", "replicas": ["node-b"]},
      {"id": 1, "range": ["m", "t"],   "primary": "node-b", "replicas": ["node-c"]},
      {"id": 2, "range": ["t", null],  "primary": "node-c", "replicas": ["node-a"]}
    ]
  }
```

**ルーティング：**
```
クライアント → GRV Proxy（ReadVersion 取得）
                    ↓
              Commit Proxy（shard_map から対象シャード解決）
                    ↓
              該当 Storage Server へリクエスト
```

**シャード再配置（Data Distributor）：**
- 負荷均衡（各シャードのサイズ・アクセス頻度をモニタリング）
- 閾値（デフォルト: シャードサイズ > 500MB）を超えたら分割
- Data Distributor が自動的に新シャードへキーを移動

### 20.7 Phase 4：分散トランザクション（OCC ベース）

**OCC による分散トランザクション（FDB 相当）：**

2PC（Two-Phase Commit）は「ロック保持中の参加者クラッシュ」で停止するリスクがある。OCC は読み取り時にロックを取らないため、2PC のブロッキング問題を回避できる。

```
分散 OCC フロー：

1. クライアントが ReadVersion 取得（GRV Proxy）
2. 複数シャードにまたがる読み取り（ロックなし、MVCC スナップショット）
3. 書き込みをすべてローカルバッファに蓄積
4. COMMIT リクエスト → Commit Proxy
5. Resolver が全シャードの read_set に対して競合チェック
   - Resolver は最近コミットされた書き込み履歴を保持（設定可能期間）
   - 競合なし → 全シャードの TLog に一括 WAL 書き込み
   - 競合あり → ABORT（クライアント再試行）
6. TLog 永続化完了 → クライアントへ成功応答
7. Storage Server 群へ非同期適用
```

**ネットワーク分断時の動作：**
```
分断検知：
  - Cluster Controller が Coordinator クォーラムで分断を判定
  - クォーラム外のパーティションはリクエストを受け付けない（CAP の C 優先）

分断回復後：
  - 新 Generation でリカバリ（20.4 参照）
  - 隔離されたノードは TLog から最新状態に同期後に復帰
```

### 20.8 設定可能なトランザクション制約

FDB ではトランザクション制限がハードコードされている（5 秒、10MB）。Adlaire-DB では設定ファイルで調整可能とする。

**設定項目（`adlaire-db.toml` 内 `[distributed]` セクション）：**
```toml
[distributed.transaction]
timeout_seconds = 10          # FDB は 5 秒ハード制限（Adlaire-DB は設定可能）
max_payload_bytes = 20971520  # デフォルト 20MB（FDB は 10MB）
max_key_bytes = 10240         # デフォルト 10KB（FDB 同等）
max_value_bytes = 204800      # デフォルト 200KB（FDB は 100KB）
max_read_keys = 100000        # 1 TX あたりの最大 read キー数

[distributed.resolver]
conflict_window_seconds = 30  # Resolver が保持する書き込み履歴（FDB は 5 秒ハード）

[distributed.replication]
replica_count = 2             # デフォルトレプリカ数
hash_check_interval_seconds = 60  # イベントログ・ハッシュ検証間隔
```

**制約超過時の動作：**
- `timeout_seconds` 超過 → トランザクション ABORT（クライアントにタイムアウトエラー）
- `max_payload_bytes` 超過 → コミット前にクライアントエラー
- `max_read_keys` 超過 → 警告ログ + メトリクス記録（デフォルトは拒否しない；設定で拒否に変更可）

### 20.9 認証・アクセス制御（分散フェーズ）

**FDB の既知の弱点：ゼロ ACL。**  
FDB ネットワークに到達できたクライアントはすべての操作を行える。  
Adlaire-DB は Phase 2 から認証を組み込む。

**Phase 2 〜 4 の認証方式：**
```
Phase 2（レプリケーション）:
  - API キー認証（Phase 1 の延長）
  - TLS 必須（ノード間通信を含む）

Phase 3（シャーディング）:
  - JWT ベース認証
  - ロールベースアクセス制御（RBAC）
  - ロール: admin / writer / reader / auditor

Phase 4（完全分散）:
  - mTLS（相互 TLS）でノード間通信を認証
  - RBAC に加えてキー名前空間ベースのアクセス制御
  - 監査ログに操作元ユーザー/サービスを記録
```

**ノード間認証（Phase 3+）：**
```toml
[distributed.security]
mtls_enabled = true
ca_cert_path = "/etc/adlaire-db/ca.crt"
node_cert_path = "/etc/adlaire-db/node.crt"
node_key_path = "/etc/adlaire-db/node.key"
```

### 20.10 Layers：データモデルの抽象化

FDB の「Layers」概念：生の KV の上に高レベルデータモデルを独立レイヤーとして実装する。

Adlaire-DB では Phase 1 の KV + Event Log を基盤レイヤーとし、将来の拡張をレイヤーとして追加できる設計を明示する：

```
┌─────────────────────────────────────┐
│  将来のレイヤー（Phase 3+）          │
│  ├─ SQL Layer（SELECT/JOIN の解析）  │
│  ├─ Document Layer（JSON クエリ）    │
│  └─ Record Layer（スキーマ定義）     │
├─────────────────────────────────────┤
│  Adlaire-DB 基盤（Phase 1）          │
│  ├─ KV Store API                    │
│  └─ Event Log API（ハッシュチェーン）│
├─────────────────────────────────────┤
│  Storage（TLog + Storage Server）   │
└─────────────────────────────────────┘
```

各レイヤーは基盤の KV + Event Log API だけを使い、ストレージ実装を知らない。これにより分散フェーズへの移行が上位レイヤーに影響を与えない。

---

## 21. 監視・ロギング仕様

### 21.1 Prometheus メトリクス

**基本メトリクス：**
```
adlaire_db_requests_total
  # リクエスト総数（operation タグ：GET, SET, DELETE, APPEND, JOIN）

adlaire_db_request_duration_seconds
  # リクエストレイテンシ分布（TCP vs REST API）

adlaire_db_errors_total
  # エラー数（error_type タグ：TIMEOUT, CORRUPTED, LOCK_TIMEOUT）

adlaire_db_storage_bytes
  # ストレージ使用量（shard ごと、ファイルタイプごと）

adlaire_db_cache_hits_total
  # インデックスキャッシュヒット率

adlaire_db_transaction_duration_seconds
  # トランザクション実行時間

adlaire_db_replication_lag_seconds
  # レプリケーション遅延（Replica 対象）

adlaire_db_connections_active
  # アクティブ接続数
```

**健全性チェック：**
```
adlaire_db_file_integrity_check
  # ファイル整合性検査結果（PASS/FAIL）

adlaire_db_lock_contention
  # ロック競合度（高い場合は性能低下の指標）

adlaire_db_wal_lag_entries
  # ジャーナルログ未処理エントリ数
```

### 21.2 ログフォーマット（JSON structured logging）

**ログ出力標準形式：**
```json
{
  "timestamp": "2026-09-09T12:30:45.123Z",
  "level": "INFO|WARN|ERROR",
  "service": "adlaire-db",
  "shard_id": 0,
  "request_id": "req-abc123",
  "operation": "SET|GET|DELETE|APPEND|JOIN",
  "key": "user:1",
  "duration_ms": 2.5,
  "status": "SUCCESS|FAILURE|TIMEOUT",
  "error_code": "LOCK_TIMEOUT|FILE_CORRUPTED|...",
  "error_message": "説明文",
  "client_addr": "192.168.1.100:54321",
  "protocol": "TCP|HTTP"
}
```

**ログレベル定義：**
```
DEBUG   : 詳細トレース（開発用）
INFO    : 通常操作（リクエスト受領、コミット完了）
WARN    : 要注意（ロック待機、リトライ）
ERROR   : エラー（ファイル破損、タイムアウト）
FATAL   : サービス停止（起動失敗、致命的障害）
```

### 21.3 Audit Log（監査ログ）

**記録対象：**
```
- 全トランザクション（SET, DELETE, APPEND）
- ファイルロック操作
- バージョン切り替え（ロールバック）
- ノード追加/削除（分散時）
- 権限操作（将来）

記録形式：

{
  "audit_id": "audit-2026090912304512345",
  "timestamp": "2026-09-09T12:30:45.123Z",
  "event_type": "DATA_MODIFICATION|ROLLBACK|NODE_CHANGE",
  "shard_id": 0,
  "tx_id": 1001,
  "operation": "SET|DELETE|APPEND",
  "key": "user:1",
  "old_version": 3,
  "new_version": 4,
  "user": "system|client_addr",
  "status": "SUCCESS|FAILED"
}
```

---

## 22. テスト戦略

### 22.1 ユニットテスト

**対象範囲：**
```
- KV Store 操作（Set, Get, Delete, Scan）
- イベントログ操作（Append, Query）
- チェックサム検証
- ロック機構
- トランザクション（Commit, Rollback）
- ファイル I/O
```

**目標：** コードカバレッジ > 80%

### 22.2 統合テスト

**シナリオ例：**
```
シナリオ1：基本シーケンス
  1. KV Set
  2. KV Get → 検証
  3. Event Append
  4. Event Query → 検証
  5. KV Delete

シナリオ2：トランザクション
  1. TX開始（複数キー操作）
  2. 途中エラーシミュレーション
  3. Rollback → 元の状態確認

シナリオ3：並行アクセス
  1. スレッド1, 2, 3 が同時にアクセス
  2. ロック競合を確認
  3. 結果の一貫性検証
```

### 22.3 Chaos Engineering（障害注入テスト）

**テストシナリオ：**
```
1. ファイル破損
   - data.kv を一部上書き
   - 起動時の整合性チェック検証

2. ネットワーク遅延
   - TCP リクエストに人為的遅延追加
   - レイテンシ基準を満たす確認

3. ディスク容量不足
   - ストレージ満杯をシミュレート
   - エラーハンドリング検証

4. プロセスクラッシュ
   - 途中強制終了
   - WAL リカバリ検証

5. ロック デッドロック
   - 複数トランザクションでデッドロック意図的発生
   - タイムアウト・解放の検証
```

### 22.4 ストレステスト

**負荷基準：**
```
負荷1：高スループット
  - 1000+ ops/sec を 10分間継続
  - メモリリーク検証
  - CPU 使用率監視

負荷2：大規模データ
  - 1GB データ投入
  - 検索レイテンシ測定
  - インデックスメモリ使用量確認

負荷3：長時間運用
  - 24時間連続稼働
  - ログローテーション動作確認
  - ディスク使用量増加率監視
```

### 22.5 フェイルオーバーテスト（Phase 2+）

**テスト項目：**
```
1. Master 故障
   - Replica → Master 昇格
   - クライアント自動リコネクト

2. ネットワーク分断
   - 一部ノード隔離
   - Quorum 判定動作確認

3. データ不整合検知
   - チェックサムエラー検出
   - 自動修復/手動介入フロー
```

### 22.6 決定論的シミュレーションテスト（DST）

FoundationDB は Flow 言語と決定論的シミュレーターにより約 1 兆 CPU 時間分のテストを実施し、Jepsen が「既知の全障害パターンに耐性がある」と評価した。Adlaire-DB の分散フェーズ（Phase 2+）では Rust の `turmoil` クレートを使用して同等のテスト戦略を実装する。

**基本原則：**
- すべての非決定論的要素（ネットワーク、ディスク I/O、時刻、乱数）をシミュレータが制御する
- シード値を固定すると同じ実行シーケンスが再現される（バグの再現が容易）
- シード値を変えると異なる障害シナリオを網羅できる

**Rust 実装方針（`turmoil` クレート）：**
```rust
// turmoil を使った DST の例（擬似コード）
#[tokio::test]
async fn test_distributed_commit_with_network_fault() {
    let mut sim = turmoil::Builder::new()
        .simulation_duration(Duration::from_secs(60))
        .build();

    sim.host("primary", || async {
        AdlaireNode::new_primary().await.run().await
    });

    sim.host("replica-a", || async {
        AdlaireNode::new_replica("primary").await.run().await
    });

    sim.client("client", async {
        // ネットワーク分断を注入
        turmoil::partition("primary", "replica-a");

        // この状態でのコミット動作を検証
        let result = client.set("key", "value").await;
        assert!(result.is_ok()); // Primary への書き込みは成功

        // 分断解消
        turmoil::repair("primary", "replica-a");

        // Replica が追いついたことを確認
        tokio::time::sleep(Duration::from_secs(5)).await;
        let replica_val = replica_client.get("key").await;
        assert_eq!(replica_val, Some("value"));
    });

    sim.run().unwrap();
}
```

**テストシナリオ網羅（シード値で制御）：**
```
ネットワーク層:
  - パケットロス（0〜100%）
  - 遅延（0ms〜10 秒）
  - ネットワーク分断（部分・完全）
  - パーティション分割（少数派 vs 多数派）

ノード層:
  - クラッシュ・再起動（任意のタイミング）
  - ディスク書き込み失敗
  - ディスク読み取り遅延
  - OOM（メモリ不足）シミュレーション

時刻:
  - クロックスキュー（ノード間で最大 ±5 秒）
  - 時刻の急激な前後移動
```

**正当性プロパティ（検証すべき不変条件）：**
```
1. Linearizability（線形一貫性）
   - コミット成功したすべての書き込みは以降の読み取りで見える

2. ハッシュチェーン整合性
   - 任意のノードの event_log 先頭ハッシュが一致する
   - 削除済みキーの Deleted イベントが必ず存在する

3. ACID トランザクション
   - コミット成功後にクラッシュしても再起動後にデータが存在する
   - ABORT されたトランザクションの影響がゼロ

4. フェイルオーバー
   - Primary クラッシュ後 30 秒以内に Replica が昇格する
   - 昇格後の Replica にコミット済みデータが全て存在する
```

**CI 統合：**
```yaml
# .github/workflows/dst.yml
- name: DST（決定論的シミュレーション）
  run: cargo test --test dst -- --test-threads=8
  env:
    ADLAIRE_DST_SEEDS: "0,1,2,...,999"   # 1000 シード並列実行
    ADLAIRE_DST_DURATION: "30s"          # シードあたりの実行時間
```

---

## 23. バックアップ・リカバリ仕様

### 23.1 バックアップ戦略

**バックアップポリシー：**
```
日次フルバックアップ：毎日 深夜 2時
  ├─ 対象：全シャード（metadata.dat, data.kv, txlog.dat）
  ├─ 保持：7日分
  └─ 検証：チェックサム確認

増分バックアップ：6時間ごと
  ├─ 対象：前回バックアップ以降の変更分
  ├─ 保持：最新3世代
  └─ 検証：差分ハッシュ確認
```

**バックアップ先：**
```
推奨構成：
  ├─ ローカルストレージ：1世代
  ├─ NAS/外部ストレージ：3世代（異なる物理場所）
  └─ クラウドストレージ（S3等）：最新1世代 + 長期保管
```

### 23.2 リカバリ手順

**シナリオ1：ファイル一部破損**
```
1. サーバー起動
2. ファイル整合性チェック失敗
3. 自動で前バージョン（shard_0_v1）から復旧
4. 操作ログ再実行（txlog.dat）
5. 正常起動
```

**シナリオ2：ディスク全損**
```
1. 新ディスク準備
2. 最新バックアップ復元
3. WAL リログ（復旧ポイント以降の変更）
4. 整合性検証
5. 本番再起動
```

**シナリオ3：Point-in-Time Recovery（PITR）**
```
目的：特定時点（例：2026-09-09 12:00:00）までのデータを復旧

手順：
1. その時点のバックアップを特定
2. 該当バックアップから復元
3. 該当時点までの WAL エントリを適用
   （以降のエントリはスキップ）
4. 整合性検証
5. サービス再開
```

### 23.3 RTO/RPO 目標

| 項目 | 目標値 | 説明 |
|------|--------|------|
| **RTO** | < 30分 | サービス復旧時間 |
| **RPO** | < 6時間 | 増分バックアップ間隔 |
| **バックアップ検証** | 毎日 | リカバリテスト月1回実施 |

### 23.4 リカバリテスト計画

**月次テスト（本番環境と同じセットアップで実行）：**
```
1. 最新バックアップ取得
2. テスト環境に復元
3. 整合性検証
4. 運用前後でのデータ差分確認
5. 復旧にかかった時間記録
6. テスト結果を監視システムに記録
```

---

---

## 24. セキュリティ仕様

### 24.1 認証機構（API Key）

**認証方式：API Key のみ**

```
【API Key 認証】
  実装難度：低
  用途：全て（開発環境、本番環境）
  
  クライアント側：
    Authorization: Bearer sk_live_abc123def456...
  
  サーバー側：
    ├─ リクエストから Authorization ヘッダを抽出
    ├─ API Key をハッシュ化
    ├─ metadata.dat のハッシュ値と比較
    ├─ 一致時：リクエスト処理
    └─ 不一致時：401 Unauthorized 返答
```

**API Key 仕様：**

```
【形式】
  sk_live_<32文字のランダム文字列>
  
  生成方法：
    openssl rand -hex 16 > api_key.txt
    
【有効期限】
  ├─ デフォルト：無期限
  ├─ オプション設定：1年、3年等を指定可
  └─ 期限切れ時：自動 401 返答

【保存方式】
  metadata.dat の credentials セクション：
  
  [API Key Hash: 32B]
  [Role: 1B]
  [Created: 8B]
  [Expires: 8B]（無期限時は 0）
  [Status: 1B]（Active/Inactive/Expired）

【初回発行】
  1. サーバーが API Key を生成
  2. 平文を一度だけクライアントに表示
  3. その後は保存しない（ハッシュのみ保存）
  4. クライアントは安全に保管
  5. 紛失時は新規生成が必要
```

**ロールの role フィールド定義：**
```
0x00 = Admin          # 全操作可能
0x01 = DataWriter     # SET, DELETE, APPEND, GET 可能
0x02 = DataReader     # GET, 履歴参照のみ
```

---

### 24.2 通信暗号化（TLS 1.3）

**推奨：TLS 1.3 必須**

```
【TCP（ポート 9876）】
  実装：tokio-tls でラッピング
  プロトコル：TLS 1.3 のみ（TLS 1.2以下は非対応）
  
  設定ファイル：
  {
    "tls": {
      "enabled": true,
      "min_version": "TLS 1.3",
      "cert_path": "/etc/adlaire-db/cert.pem",
      "key_path": "/etc/adlaire-db/key.pem"
    }
  }

【REST API（ポート 8080 → ポート 443）】
  実装：Axum + tokio-tls で HTTPS 対応
  
  リダイレクト設定：
    HTTP ポート 8080 でリッスン
    ↓
    全リクエストを HTTPS ポート 443 へ 301 リダイレクト
  
  HSTS（HTTP Strict-Transport-Security）ヘッダ：
    Strict-Transport-Security: max-age=31536000; includeSubDomains
    （1年間、全サブドメイン適用）
```

**証明書管理：**

```
【開発環境】
  自己署名証明書（有効期限 365日）：
  
  openssl req -x509 -newkey rsa:4096 \
    -keyout key.pem -out cert.pem \
    -days 365 -nodes

【本番環境】
  Let's Encrypt（無料）または商用 CA：
  
  certbot を systemd timer で定期実行
  ├─ 実行間隔：毎週水曜日 深夜 2時
  ├─ 自動更新：有効期限 30日前に更新
  └─ 更新後：Adlaire DB サーバー再起動
```

---

### 24.3 アクセス制御（RBAC）

**ロール定義と権限：**

```
【Admin（role = 0x00）】
  権限：
    ├─ KV Get/Set/Delete ✓
    ├─ Event Append/Query ✓
    ├─ Join ✓
    ├─ API Key 管理（生成・無効化・ローテーション）✓
    ├─ バックアップ・リストア ✓
    ├─ ロールバック ✓
    ├─ 監視設定変更 ✓
    └─ ノード管理（分散時）✓

【DataWriter（role = 0x01）】
  権限：
    ├─ KV Get ✓
    ├─ KV Set ✓
    ├─ KV Delete ✓
    ├─ Event Append ✓
    ├─ Event Query ✓
    ├─ Join ✓
    ├─ ロールバック ✗（Admin のみ）
    ├─ バックアップ ✗（Admin のみ）
    └─ API Key 管理 ✗（Admin のみ）

【DataReader（role = 0x02）】
  権限：
    ├─ KV Get ✓（読み取り専用）
    ├─ Event Query ✓（読み取り専用）
    ├─ Join ✓
    ├─ 書き込み操作 ✗（全て拒否）
    └─ 設定変更 ✗（全て拒否）
```

**権限チェック実装：**

```rust
fn check_permission(api_key: &str, operation: Operation) -> Result<()> {
    let role = get_role_for_api_key(api_key)?;
    
    match (role, operation) {
        (Admin, _) => Ok(()),                    // 全て許可
        (DataWriter, Op::Get | Op::Set | Op::Delete | Op::Append) => Ok(()),
        (DataReader, Op::Get | Op::Query) => Ok(()),
        _ => Err(PermissionDenied),
    }
}
```

---

### 24.4 ファイルシステム権限

**Linux ユーザー・グループ分離：**

```
【ディレクトリ構成】
  /var/lib/adlaire-db/
    ├─ Owner: adlaire-db:adlaire-db
    ├─ Permission: 700 (rwx------)
    ├─ shard_0/
    ├─ shard_1/
    └─ ...

【ファイルレベル権限】
  metadata.dat, data.kv, txlog.dat
    ├─ Owner: adlaire-db:adlaire-db
    ├─ Permission: 600 (rw-------)
    └─ 他ユーザー・グループアクセス禁止

【バックアップファイル権限】
  /backup/adlaire-db/
    ├─ Permission: 600 (rw-------)
    └─ Owner: backup:backup（別ユーザー）
```

**systemd サービス設定：**

```ini
[Service]
User=adlaire-db
Group=adlaire-db
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
NoNewPrivileges=yes
ReadWritePaths=/var/lib/adlaire-db
```

---

### 24.5 Audit Log（監査ログ）

**記録対象：**

```
【API レベル】
  ✓ API Key 認証成功・失敗
  ✓ 権限不足エラー
  ✓ Rate Limit 超過

【データ操作レベル】
  ✓ SET / DELETE / APPEND（全て）
  ✓ ロールバック実行
  ✓ トランザクション コミット/ロールバック

【管理レベル】
  ✓ API Key 生成・無効化
  ✓ ロール変更
  ✓ バックアップ・リストア実行
  ✓ ノード追加/削除（分散時）

【エラーレベル】
  ✓ ファイル整合性チェック失敗
  ✓ ロック デッドロック検知
  ✓ ディスク容量不足
```

**ログレコード形式：**

```json
{
  "audit_id": "audit-20260909-123045-abc123",
  "timestamp": "2026-09-09T12:30:45.123Z",
  "level": "INFO|WARN|ERROR",
  "event_type": "API_AUTH|DATA_WRITE|ADMIN_ACTION",
  "api_key_hash": "sha256(sk_live_xxx)[:8]",
  "role": "Admin|DataWriter|DataReader",
  "operation": "SET|DELETE|APPEND|GET|ROLLBACK|...",
  "resource": {
    "shard_id": 0,
    "key": "user:1"
  },
  "result": "SUCCESS|FAILURE|PERMISSION_DENIED",
  "error_code": null,
  "error_message": null,
  "client_addr": "192.168.1.100:54321",
  "protocol": "TCP|HTTP"
}
```

**ログ保持ポリシー：**

```
【ローカルストレージ】
  期間：最新 1ヶ月
  ローテーション：日次（自動圧縮 gzip）
  保存先：/var/log/adlaire-db/audit.log.gz

【長期保存】
  期間：最低 1年間
  保存先：クラウドストレージ（S3等）またはアーカイブサーバー
  暗号化：AES-256 で暗号化して保存

【改ざん防止】
  ├─ ログにハッシュチェーン付与
  ├─ 日次ハッシュ検証
  └─ 改ざん検知時は alert
```

---

### 24.6 レート制限（DDoS 対策）

**実装方式：API Key 単位のレート制限**

```
【デフォルト設定】
  API Key ごと：100 req/sec
  バースト許容：500 req（5秒間のスパイク対応）
  
【超過時の動作】
  ├─ 100-500 req/sec：処理継続
  ├─ > 500 req/sec：429 Too Many Requests 返答
  └─ クライアント側は Retry-After ヘッダを確認

【設定例】
  {
    "rate_limiting": {
      "default_rps": 100,
      "burst_allowance": 500,
      "window_seconds": 5,
      "api_key_overrides": {
        "sk_live_premium_xxx": {
          "rps": 1000,
          "burst": 5000
        }
      }
    }
  }
```

---

### 24.7 本番環境セキュリティチェックリスト

```
【通信セキュリティ】
  □ TLS 1.3 有効化（ポート 443）
  □ 証明書有効期限確認
  □ 自動更新設定（certbot）
  □ HSTS ヘッダ設定確認

【認証・認可】
  □ API Key 設定完了
  □ ロール設定確認（Admin/DataWriter/DataReader）
  □ 初回 API Key 配布済み
  □ 無効化される古い API Key 削除

【ファイルシステム】
  □ ファイルシステム権限確認（700/600）
  □ ユーザー・グループ設定確認（adlaire-db:adlaire-db）
  □ SELinux / AppArmor 設定（オプション）
  □ バックアップファイル権限確認（600）

【ファイアウォール】
  □ TCP 9876 → Admin サーバーのみ許可
  □ TCP 443 → 外部アクセス許可
  □ TCP 8080 → HTTPS リダイレクト用

【監視・ロギング】
  □ Audit Log 有効化
  □ ログ送信先設定（ローカル + クラウド）
  □ ログローテーション設定
  □ 異常ログ検知アラート設定

【バックアップ】
  □ バックアップスケジュール確認
  □ バックアップ暗号化設定
  □ リストアテスト実施（月1回）
  □ バックアップ保存先確認

【運用】
  □ セキュリティ監査計画（月1回）
  □ インシデント対応計画書作成
  □ セキュリティパッチ適用ポリシー
```

---

### 24.8 API Key ローテーション手順

**定期ローテーション（推奨：年1回）：**

```
1. 新 API Key 生成
   adlaire-db-admin gen-api-key --role DataWriter

2. 新 API Key をクライアントに配布
   ├─ セキュアな方法（PGP暗号化等）
   └─ 有効期限を伝達

3. 旧 API Key の猶予期間（7日間）
   ├─ 新旧両方の API Key を受け付け
   └─ クライアントに移行期間を与える

4. 旧 API Key を無効化
   adlaire-db-admin revoke-api-key sk_live_old_xxx

5. ログ確認
   ├─ 旧 API Key からのアクセスが完全に停止
   └─ クライアント移行完了を確認
```

---

---

## 25. データ保存時暗号化（Phase 1.5）

### 25.1 暗号化方式

**AES-256-GCM（Galois/Counter Mode）**

```
アルゴリズム：AES-256-GCM
鍵長：256 ビット
IV（初期化ベクトル）：96 ビット（推奨）
タグ長：128 ビット（認証タグ）
```

### 25.2 鍵管理

```
【Master Key】
  ├─ 生成：環境変数 ADLAIRE_MASTER_KEY から読み込み
  ├─ 形式：Base64 エンコード（256 ビット）
  ├─ 保管：HashiCorp Vault / AWS Secrets Manager 推奨
  └─ 更新：年1回のローテーション

【Data Key】
  ├─ 生成：ファイルごとにランダム生成
  ├─ 保存：metadata.dat に格納
  ├─ Master Key で保護
  └─ IV：metadata.dat ヘッダに平文保存

【Key Rotation】
  新 Master Key に移行する際：
    1. 全 data.kv を新 Key で復号化
    2. 新 Master Key で再暗号化
    3. メタデータ更新
    4. 旧 Key は 90日間保持後削除
```

### 25.3 実装仕様

**暗号化対象：**
```
✅ data.kv（KV Store + イベントログ）
✅ txlog.dat（トランザクションログ）
❌ metadata.dat（平文、鍵情報含む）
```

**コード例（Rust）：**

```rust
use aes_gcm::{Aes256Gcm, Nonce, Key};
use aes_gcm::aead::{Aad, Payload};

fn encrypt_data(plaintext: &[u8], key: &Key<Aes256Gcm>, iv: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(iv);
    
    cipher.encrypt(nonce, Payload::from(plaintext))
        .map_err(|e| EncryptionError(e))
}

fn decrypt_data(ciphertext: &[u8], key: &Key<Aes256Gcm>, iv: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(iv);
    
    cipher.decrypt(nonce, Payload::from(ciphertext))
        .map_err(|e| DecryptionError(e))
}
```

### 25.4 Phase 1.5 スケジュール

```
【Phase 1.5：データ保存時暗号化実装】
期間：2週間（Phase 1 後、8-10週目）

Week 8-9：
  1. AES-256-GCM 実装
  2. 鍵管理機構実装
  3. ファイル暗号化・復号化機能
  4. Key Rotation 機能
  5. ユニットテスト

Week 10：
  1. 統合テスト
  2. パフォーマンス測定（暗号化 overhead 評価）
  3. 本番環境デプロイ準備
```

---

## 26. API リファレンス（実装例）

### 26.1 REST API エンドポイント

**基本情報：**
```
ホスト：localhost:443（本番環境 HTTPS）
ポート：8080 → 443 へリダイレクト
認証：Authorization: Bearer sk_live_...
```

### 26.2 KV Store API

#### 26.2.1 GET キー取得

**エンドポイント：**
```
GET /api/v1/kv/:key
```

**Python 実装例：**
```python
import requests

url = "https://localhost:443/api/v1/kv/user:1"
headers = {
    "Authorization": "Bearer sk_live_abc123def456...",
    "Content-Type": "application/json"
}

response = requests.get(url, headers=headers, verify=False)  # verify=True 本番環境
if response.status_code == 200:
    data = response.json()
    print(f"Value: {data}")
elif response.status_code == 401:
    print("Unauthorized - Invalid API Key")
elif response.status_code == 404:
    print("Key not found")
```

**Node.js 実装例：**
```javascript
const axios = require('axios');

const config = {
    method: 'GET',
    url: 'https://localhost:443/api/v1/kv/user:1',
    headers: {
        'Authorization': 'Bearer sk_live_abc123def456...',
        'Content-Type': 'application/json'
    },
    httpsAgent: new https.Agent({ rejectUnauthorized: false })  // 本番環境は true
};

axios(config)
    .then(response => {
        console.log('Value:', response.data);
    })
    .catch(error => {
        console.error('Error:', error.response.status);
    });
```

**Go 実装例：**
```go
package main

import (
    "crypto/tls"
    "fmt"
    "io/ioutil"
    "net/http"
)

func main() {
    client := &http.Client{
        Transport: &http.Transport{
            TLSClientConfig: &tls.Config{InsecureSkipVerify: false}, // 本番環境
        },
    }

    req, _ := http.NewRequest("GET", "https://localhost:443/api/v1/kv/user:1", nil)
    req.Header.Add("Authorization", "Bearer sk_live_abc123def456...")

    resp, err := client.Do(req)
    if err != nil {
        fmt.Println("Error:", err)
        return
    }
    defer resp.Body.Close()

    body, _ := ioutil.ReadAll(resp.Body)
    fmt.Println("Value:", string(body))
}
```

#### 26.2.2 SET キー設定

**エンドポイント：**
```
PUT /api/v1/kv/:key
Content-Type: application/json

Body:
{
  "value": {...}
}
```

**Rust 実装例：**
```rust
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() {
    let client = Client::new();
    
    let body = json!({
        "value": {
            "name": "John Doe",
            "email": "john@example.com"
        }
    });

    let response = client
        .put("https://localhost:443/api/v1/kv/user:1")
        .header("Authorization", "Bearer sk_live_abc123def456...")
        .json(&body)
        .send()
        .await
        .unwrap();

    if response.status().is_success() {
        println!("Key set successfully");
    } else {
        println!("Error: {}", response.status());
    }
}
```

**PHP 実装例：**
```php
<?php
$ch = curl_init();

$url = "https://localhost:443/api/v1/kv/user:1";
$data = json_encode([
    'value' => [
        'name' => 'John Doe',
        'email' => 'john@example.com'
    ]
]);

curl_setopt($ch, CURLOPT_URL, $url);
curl_setopt($ch, CURLOPT_CUSTOMREQUEST, "PUT");
curl_setopt($ch, CURLOPT_POSTFIELDS, $data);
curl_setopt($ch, CURLOPT_HTTPHEADER, [
    'Content-Type: application/json',
    'Authorization: Bearer sk_live_abc123def456...'
]);
curl_setopt($ch, CURLOPT_SSL_VERIFYPEER, true); // 本番環境

$response = curl_exec($ch);
$statusCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);

if ($statusCode == 200) {
    echo "Key set successfully\n";
} else {
    echo "Error: $statusCode\n";
}

curl_close($ch);
?>
```

#### 26.2.3 DELETE キー削除

**エンドポイント：**
```
DELETE /api/v1/kv/:key
```

**Python 実装例：**
```python
import requests

url = "https://localhost:443/api/v1/kv/user:1"
headers = {
    "Authorization": "Bearer sk_live_abc123def456..."
}

response = requests.delete(url, headers=headers, verify=True)

if response.status_code == 200:
    print("Key deleted successfully")
else:
    print(f"Error: {response.status_code}")
```

### 26.3 Event API

#### 26.3.1 APPEND イベント追加

**エンドポイント：**
```
POST /api/v1/events/:key
Content-Type: application/json

Body:
{
  "event_type": "UPDATE",
  "data": {...}
}
```

**Node.js 実装例：**
```javascript
const axios = require('axios');

const url = 'https://localhost:443/api/v1/events/user:1';
const data = {
    event_type: 'UPDATE',
    data: {
        field: 'email',
        old_value: 'old@example.com',
        new_value: 'new@example.com'
    }
};

axios.post(url, data, {
    headers: {
        'Authorization': 'Bearer sk_live_abc123def456...',
        'Content-Type': 'application/json'
    },
    httpsAgent: new (require('https')).Agent({ rejectUnauthorized: false })
})
.then(response => {
    console.log('Event appended:', response.data);
})
.catch(error => {
    console.error('Error:', error.response.status);
});
```

#### 26.3.2 GET イベント履歴

**エンドポイント：**
```
GET /api/v1/events/:key?limit=10&offset=0
```

**Go 実装例：**
```go
package main

import (
    "fmt"
    "io/ioutil"
    "net/http"
    "net/url"
)

func main() {
    query := url.Values{
        "limit":  []string{"10"},
        "offset": []string{"0"},
    }

    req, _ := http.NewRequest(
        "GET",
        fmt.Sprintf("https://localhost:443/api/v1/events/user:1?%s", query.Encode()),
        nil,
    )
    req.Header.Add("Authorization", "Bearer sk_live_abc123def456...")

    client := &http.Client{}
    resp, _ := client.Do(req)
    defer resp.Body.Close()

    body, _ := ioutil.ReadAll(resp.Body)
    fmt.Println("Events:", string(body))
}
```

### 26.4 Join API

**エンドポイント：**
```
GET /api/v1/join/:left_key/:right_table?on=field
```

**Rust 実装例：**
```rust
use reqwest::Client;

#[tokio::main]
async fn main() {
    let client = Client::new();

    let response = client
        .get("https://localhost:443/api/v1/join/order:1/order_items?on=order_id")
        .header("Authorization", "Bearer sk_live_abc123def456...")
        .send()
        .await
        .unwrap();

    if response.status().is_success() {
        let body = response.json::<serde_json::Value>().await.unwrap();
        println!("Join result: {}", body);
    }
}
```

---

## 27. トラブルシューティングガイド

### 27.1 よくある問題と対応

#### **問題1：ロック タイムアウト（Lock Timeout）**

**症状：**
```
エラー: "Lock acquisition timeout"
ログ: "Waiting for lock .lock (30s timeout)"
```

**原因：**
```
1. 複数クライアントが同時にアクセス
2. トランザクション実行時間が長い
3. デッドロック状態
```

**診断コマンド：**
```bash
# ロックファイルの確認
ls -la /var/lib/adlaire-db/.lock

# ロックファイルの年齢確認（古すぎる場合は故障）
stat /var/lib/adlaire-db/.lock | grep Modify

# プロセスの確認
ps aux | grep adlaire-db
```

**対応方法：**
```
軽度（ロック待機時間 < 5秒）：
  → 正常。ロック競合の自動解決を待つ

中度（ロック待機時間 5-30秒）：
  → 警告。Audit Log を確認
  → 長時間トランザクションがないか確認
  → 並行接続数を削減検討

重度（ロック タイムアウト）：
  → 古いロックファイルを削除
  ```bash
  rm /var/lib/adlaire-db/.lock
  systemctl restart adlaire-db
  ```
  → 必ず Audit Log で原因調査後に実施
```

#### **問題2：メモリ不足（Out of Memory）**

**症状：**
```
エラー: "Cannot allocate memory"
ログ: "Memory usage 95%+"
サーバー: 突然プロセスが強制終了
```

**原因：**
```
1. インデックスメモリが大きい（1GB+ データ）
2. メモリリーク（接続ごとにメモリ増加）
3. キャッシュが無限増殖
```

**診断コマンド：**
```bash
# メモリ使用量確認
free -h

# Adlaire DB プロセスのメモリ確認
ps aux | grep adlaire-db
# または
top -p $(pgrep adlaire-db)

# メモリプロファイリング（Prometheus）
curl http://localhost:9090/api/v1/query?query=memory_usage_percent
```

**対応方法：**
```
【一時的な対応】
1. 待機中のクライアント接続を切断
   systemctl reload adlaire-db

2. インデックスキャッシュをクリア（Phase 1）
   # REST API経由
   curl -X POST https://localhost:443/api/v1/admin/cache/clear \
     -H "Authorization: Bearer sk_live_admin_xxx"

【恒久的な対応】
1. サーバーメモリをアップグレード
   現在：4GB → 推奨：8-16GB

2. インデックス永続化を導入（Phase 1.5）
   → ディスクベースのインデックスを使用
   → メモリ使用量 50%削減

3. データサイズを分割
   → シャード数を増やす
   → 1シャードあたりの容量削減
```

#### **問題3：ディスク容量不足**

**症状：**
```
エラー: "No space left on device"
ログ: "Write to file failed"
```

**診断コマンド：**
```bash
# ディスク使用率確認
df -h /var/lib/adlaire-db

# ファイルサイズ確認
du -sh /var/lib/adlaire-db/*

# ログローテーション確認
ls -lh /var/log/adlaire-db/
```

**対応方法：**
```
【緊急対応】
1. 古いログを削除
   find /var/log/adlaire-db -name "*.gz" -mtime +30 -delete

2. バックアップファイルを削除（重要：前提：別途保管確認）
   rm -rf /var/lib/adlaire-db/shard_0_v3/

【根本対応】
1. ストレージをアップグレード
   現在：200GB → 推奨：500GB-1TB

2. バックアップ先をクラウドへ移行
   ローカル：最新1世代のみ
   クラウド（S3）：長期保管
```

#### **問題4：ファイル破損（File Corruption）**

**症状：**
```
エラー: "Checksum mismatch"
ログ: "File integrity check failed"
```

**診断コマンド：**
```bash
# チェックサム検証
sha256sum /var/lib/adlaire-db/shard_0/data.kv
# metadata.dat のハッシュと比較
cat /var/lib/adlaire-db/shard_0/metadata.dat | grep -A1 "data_kv_hash"

# ファイルシステムチェック
fsck /dev/sda1  # デバイス名は環境に応じて変更
```

**対応方法：**
```
【Phase 1（ロールバック）】
1. 前バージョンから復旧
   cp -r /var/lib/adlaire-db/shard_0_v1/* \
         /var/lib/adlaire-db/shard_0/
   systemctl restart adlaire-db

2. WAL から操作を再実行（自動）
   → サーバー起動時に自動リカバリ

【Phase 1.5（PITR）】
1. バックアップから復旧
   tar xzf /backup/adlaire-db/shard_0-2026-09-09.tar.gz -C /var/lib/adlaire-db/
   systemctl restart adlaire-db

2. 特定時点まで WAL を再適用
```

#### **問題5：ネットワーク分断（Network Partition）**

**症状（Phase 2 以降）：**
```
エラー: "Connection refused to replica node"
ログ: "Network partition detected"
```

**診断コマンド：**
```bash
# ネットワーク接続確認
ping <replica_node_ip>
telnet <replica_node_ip> 9876

# DNS 解決確認
nslookup replica-node.example.com

# Firewall ルール確認
sudo iptables -L -n | grep 9876
```

**対応方法：**
```
【検知時】
1. Audit Log で分断時刻を特定
2. 分散構成で Quorum 判定
   - Master が応答可能 → Master 継続
   - Replica のみ応答 → 新 Master 選出待機

【復旧時】
1. ネットワーク復旧を確認
2. ノード間の整合性確認
3. チェックサムで検証
4. 必要に応じて再レプリケーション
```

### 27.2 ログからの原因特定

**ログレベル別の読み方：**

```
【ERROR ログが出た場合】
1. Timestamp 記録
   "2026-09-09T12:30:45.123Z"

2. Operation と Key 確認
   "operation": "SET", "key": "user:1"

3. error_code 確認
   "error_code": "LOCK_TIMEOUT"

4. 前後 5分のログを調査
   該当エラーの前兆がないか確認

【トランザクション失敗の場合】
1. TX ID で該当トランザクション検索
   grep "tx_id: 1001" /var/log/adlaire-db/*.json

2. PENDING → COMMITTED → APPLIED フロー確認
   PENDING のまま停止していないか

3. ロールバック理由確認
   "status": "ABORTED", "error_message": "..."
```

**ログ検索コマンド例：**

```bash
# 特定時刻のエラー検索
grep "2026-09-09T12:30" /var/log/adlaire-db/audit.log | grep ERROR

# 特定キーの操作履歴
grep '"key": "user:1"' /var/log/adlaire-db/*.json | jq '.'

# エラー頻度の集計
grep ERROR /var/log/adlaire-db/*.json | \
  jq -r '.error_code' | sort | uniq -c | sort -rn

# 遅いリクエストの検出
jq 'select(.duration_ms > 50)' /var/log/adlaire-db/*.json
```

---

## 28. 実装リスク・対策

### 28.1 リスク 1：インデックス永続化（Phase 1）

**懸念：**
```
├─ 1GB 以上のデータでメモリ不足の可能性
├─ シナリオ3 テスト時にメモリが問題になる可能性
└─ インデックス永続化は実装後の追加になる
```

**対策：**
```
├─ Phase 1 実装時にインデックス永続化の設計済みにする
│  └─ B+Tree インデックス設計を事前に完成
│
├─ Phase 1 最後にオプション実装
│  └─ Week 12-13 で B+Tree インデックスをオプション機能化
│
└─ 早期段階でベンチマークして判断
   ├─ Week 4：100MB データでテスト
   ├─ Week 8：500MB データでテスト
   └─ Week 12：1GB データでテスト
      → メモリ使用量 > 1GB なら B+Tree 導入
```

**実装優先度：**
```
【必須】KV Store + イベントログ（メモリ）
【オプション】B+Tree インデックス（ディスク永続化）
```

---

### 28.2 リスク 2：2-Phase Commit 複雑性（Phase 2-4）

**懸念：**
```
├─ Phase 2-4 の 2-Phase Commit は実装難度が高い
├─ ネットワーク分断時の動作確認が困難
├─ Quorum 管理でバグが増える可能性
└─ Phase 4 に到達するまで 6ヶ月以上要する予想
```

**対策：**
```
├─ Phase 2-3 で充分なテスト期間確保
│  ├─ Phase 2（レプリケーション）：3-4週間
│  │  └─ Master-Replica 間の整合性テスト
│  │
│  └─ Phase 3（シャーディング）：3-4週間
│     └─ 複数ノード間のルーティング検証
│
├─ Chaos Engineering で分散環境を徹底テスト
│  ├─ ネットワーク遅延注入
│  ├─ ノードクラッシュシミュレーション
│  ├─ パケットロス注入
│  └─ 分断検知・復旧テスト
│
└─ 分散対応は慎重に段階的実施
   ├─ Phase 2：単純レプリケーション
   ├─ Phase 3：読み取り分散（Read-only Replica）
   └─ Phase 4：書き込み分散（2-Phase Commit）
      → 各フェーズで 1ヶ月以上のテスト期間
```

**テスト計画：**
```
【Phase 2 テスト（レプリケーション）】
- 同期レプリケーション：RPO < 1秒
- ノード故障時の Failover
- データ整合性検証

【Phase 3 テスト（シャーディング）】
- キー分散確認
- ルーティング動作
- ホットスポット検査

【Phase 4 テスト（分散トランザクション）】
- 2-Phase Commit のタイムアウト処理
- ネットワーク分断検知
- Quorum ベースの判定
- リカバリテスト
```

---

### 28.3 リスク 3：本番監視・ロギング構築（運用準備）

**懸念：**
```
├─ Prometheus メトリクス 13個の定義
├─ Grafana ダッシュボード 4種類の構築
├─ Alertmanager ルール設定
└─ 運用スタッフの教育が必要
```

**対策：**
```
├─ Terraform / Ansible で IaC 化推奨
│  ├─ Prometheus 設定を Terraform で管理
│  ├─ Grafana ダッシュボードを JSON で定義
│  ├─ Alertmanager ルールを YAML で版管理
│  └─ Git で全構成を追跡可能
│
├─ テンプレート化したダッシュボード・ルール提供
│  ├─ 汎用テンプレート（Grafana）を提供
│  ├─ Alert Rule テンプレート（YAML）を提供
│  └─ Terraform モジュール化して再利用可能化
│
└─ Phase 1 完成時に運用マニュアルを完成させる
   ├─ セクション 21 監視・ロギングマニュアル
   ├─ セクション 27 トラブルシューティングガイド
   └─ 運用チェックリスト
      └─ 日次点検項目
      └─ 月次点検項目
      └─ 年次セキュリティ監査
```

**実装スケジュール：**
```
Phase 1 Week 10-13：
  ├─ Week 10：Prometheus + Grafana 基本構築
  ├─ Week 11：メトリクス 13個定義・ダッシュボード作成
  ├─ Week 12：Alertmanager ルール定義
  └─ Week 13：運用マニュアル完成
```

---

## 29. 将来対応の検討課題

### OS 対応拡張

現在は **Ubuntu 24.04 LTS のみ** とするが、将来以下の対応を検討できる：

```
検討対象：
  ├─ RHEL 9.x
  ├─ CentOS 9.x
  └─ Debian 12
```

実装タイミングはユーザー要望とビジネスニーズに応じて柔軟に対応する。

---
