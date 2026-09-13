### Phase 8：Turso Cloud 互換管理モデル

**目標**：Phase 7 の管理 API を Turso Cloud 互換の管理モデルへ拡張し、location、organization/group、quota/usage を DB・token・admin 権限の正式な管理境界として扱う。

Phase 8 は Turso Cloud 互換を優先するための前倒しフェーズである。WebSocket、ATTACH、replication、backup、branch より先に、管理モデルと metadata の互換性を固める。

**Phase 8 の対象：**

- データベースロケーション（location / region）
- 組織・グループ管理（organization / group）
- ストレージクォータと使用量（quota / usage）
- 既存 `/admin/v1/databases`、`/admin/v1/tokens` の互換拡張
- 既存 `databases.json`、`tokens.json` からの metadata migration

**Phase 8 API 契約：**

Phase 8 で公開する API は §9.5 の Phase 8 行を正とする。`/admin/v1/*` は Admin token 必須、`/v1/*` は Platform token 必須とし、unknown field は `INVALID_REQUEST` とする。一覧 API は §9.1.27 に従い、`limit`（既定 100、最大 500）と `cursor` を受け付ける。Phase 8 では `cursor` は base64url without padding の opaque string とし、endpoint、resource kind、scope、filter、sort に binding する。filter query は `organization`、`group`、`database` のみ許可し、不明 query は `INVALID_REQUEST` とする。

`PUT /admin/v1/quotas/{scope}` の `{scope}` は URL encode 済みの `organization:{id}`、`group:{id}`、`database:{name}` のいずれかとする。scope type が不明な場合は `INVALID_REQUEST`、scope が存在しない場合は対応する `ORG_NOT_FOUND`、`GROUP_NOT_FOUND`、`DB_NOT_FOUND` を返す。

Phase 8 の `/v1/*` Turso 互換 API は、公式 Turso Platform API の主要 path、status、response wrapper、field casing を優先する。`/admin/v1/*` との差分は互換差分として扱い、片方の実装都合で他方の契約を変えてはならない。

**Phase 8 `/v1/*` route 解決順固定表：**

`/v1/*` router は下表の順で完全一致または path parameter 一致を評価する。同じ path で method だけが異なる場合は `405 METHOD_NOT_ALLOWED`、下表と unsupported 固定表のどちらにも該当しない path は `404 ENDPOINT_NOT_FOUND` とする。trailing slash は別 path と扱い、自動 redirect・自動補正を行わない。

| 順位 | Route pattern | 対象 method | 一致時の扱い |
|------|---------------|-------------|--------------|
| 1 | `/v1/auth/validate` | `GET` | Platform token 検証 |
| 2 | `/v1/auth/api-tokens/{tokenName}` | `POST`, `DELETE` | Platform API token 作成・失効 |
| 3 | `/v1/locations` | `GET` | location 一覧 |
| 4 | `/v1/organizations` | `GET` | organization 一覧 |
| 5 | `/v1/organizations/{organizationSlug}` | `PATCH` | organization 更新 |
| 6 | `/v1/organizations/{organizationSlug}/usage` | `GET` | organization usage |
| 7 | `/v1/organizations/{organizationSlug}/audit-logs` | `GET` | unsupported audit logs stub |
| 8 | `/v1/organizations/{organizationSlug}/groups/{groupName}/configuration` | `PATCH` | group configuration 更新。group 詳細より先に評価する |
| 9 | `/v1/organizations/{organizationSlug}/groups/{groupName}/auth/rotate` | `POST` | group 配下 DB token 一括失効。group 詳細より先に評価する |
| 10 | `/v1/organizations/{organizationSlug}/groups/{groupName}/transfer` | `POST` | unsupported group transfer stub。group 詳細より先に評価する |
| 11 | `/v1/organizations/{organizationSlug}/groups/{groupName}` | `GET` | group 詳細。`groups` 一覧より先に評価する |
| 12 | `/v1/organizations/{organizationSlug}/groups` | `GET`, `POST` | group 一覧・作成 |
| 13 | `/v1/organizations/{organizationSlug}/databases/{databaseName}/auth/tokens` | `POST` | DB token 作成。DB 詳細より先に評価する |
| 14 | `/v1/organizations/{organizationSlug}/databases/{databaseName}/auth/rotate` | `POST` | DB token 一括失効。DB 詳細より先に評価する |
| 15 | `/v1/organizations/{organizationSlug}/databases/{databaseName}/configuration` | `PATCH` | DB configuration 更新。DB 詳細より先に評価する |
| 16 | `/v1/organizations/{organizationSlug}/databases/{databaseName}/stats` | `GET` | unsupported stats stub |
| 17 | `/v1/organizations/{organizationSlug}/databases/{databaseName}` | `GET`, `DELETE` | DB 詳細・削除 |
| 18 | `/v1/organizations/{organizationSlug}/databases` | `GET`, `POST` | DB 一覧・作成 |
| 19 | `/v1/organizations/{organizationSlug}/members...`、`/invites...`、`/plans...`、`/billing...`、`/overages...`、`/v1/upload` | 固定表参照 | unsupported stub |

Path parameter は percent decode 後に validation する。decode 不能、decode 後の空文字、`/` を含む値、`.`、`..` は `400 INVALID_REQUEST` とする。`organizationSlug`、`groupName`、`databaseName` は URL 内では case-sensitive とし、大小文字補正を行わない。

**Phase 8 `/v1/*` request validation 固定表：**

| 対象 | 規則 | 失敗時 |
|------|------|--------|
| `GET` | request body 禁止。`Content-Length: 0` または body なしのみ許可 | `400 INVALID_REQUEST` |
| `POST` / `PATCH` | body を持つ場合は `Content-Type: application/json` 必須。`; charset=utf-8` は許可。body は JSON object 必須。endpoint 表で body なし可と明記された POST は body なしを許可 | `400 INVALID_REQUEST` |
| unknown query | endpoint 固有表にない query key は拒否。互換のため黙って無視しない | `400 INVALID_REQUEST` |
| duplicate query key | 同一 query key の複数指定は禁止 | `400 INVALID_REQUEST` |
| empty query value | `cursor` 以外の空文字は禁止。`cursor=` も無効 cursor として拒否 | `400 INVALID_REQUEST` |
| unknown body field | endpoint 固有表にない field は拒否 | `400 INVALID_REQUEST` |
| JSON scalar/array body | body が object でない場合は禁止 | `400 INVALID_REQUEST` |
| trailing slash | `/v1/locations/` など末尾 slash は未定義 path | `404 ENDPOINT_NOT_FOUND` |
| unsupported method | 既知 path に未定義 method を送った場合 | `405 METHOD_NOT_ALLOWED` |

**Phase 8 API schema 固定表：**

| API | Request | Success response | Validation |
|-----|---------|------------------|------------|
| `GET /admin/v1/organizations` | query `limit?`, `cursor?` | `{"organizations":[OrganizationInfo],"next_cursor": string|null}` | 不明 query は `INVALID_REQUEST` |
| `POST /admin/v1/organizations` | `{name, slug?}` | 201 `OrganizationInfo` | `name`/`slug` は `^[a-zA-Z0-9_-]{1,63}$`。`slug` 省略時は `name` と同じ。重複は `ORG_ALREADY_EXISTS` |
| `GET /admin/v1/organizations/{org}` | body なし | `OrganizationInfo` | `{org}` は `id` または `slug` |
| `DELETE /admin/v1/organizations/{org}` | body なし | 204 | `default` は削除禁止で `403 ORG_SCOPE_DENIED`。配下 group/DB/token がある場合も `403 ORG_SCOPE_DENIED` |
| `GET /admin/v1/groups` | query `organization?`, `limit?`, `cursor?` | `{"groups":[GroupInfo],"next_cursor": string|null}` | `organization` が存在しなければ `ORG_NOT_FOUND` |
| `POST /admin/v1/groups` | `{organization, name, slug?, location?}` | 201 `GroupInfo` | `location` 省略時は `default`。同一 organization 内の name/slug 重複は `GROUP_ALREADY_EXISTS` |
| `GET /admin/v1/groups/{group}` | body なし | `GroupInfo` | group 名が複数 organization で重複する場合は `organization` query 必須。未指定なら `INVALID_REQUEST` |
| `DELETE /admin/v1/groups/{group}` | body なし | 204 | `default` は削除禁止。配下 DB/token/quota がある場合は `403 ORG_SCOPE_DENIED` |
| `GET /admin/v1/locations` | query `limit?`, `cursor?` | `{"locations":[LocationInfo],"next_cursor": string|null}` | 不明 query は `INVALID_REQUEST` |
| `POST /admin/v1/locations` | `{name, provider?, region?, primary?}` | 201 `LocationInfo` | `provider` 省略時 `"self-hosted"`、`region` 省略時 `"local"`、`primary` 省略時 `false` |
| `GET /admin/v1/locations/{location}` | body なし | `LocationInfo` | `{location}` は `id` または `name` |
| `DELETE /admin/v1/locations/{location}` | body なし | 204 | DB/group が参照中なら `403 ORG_SCOPE_DENIED`。`default` は削除禁止 |
| `GET /admin/v1/quotas` | query `organization?`, `group?`, `database?`, `limit?`, `cursor?` | `{"quotas":[QuotaInfo],"next_cursor": string|null}` | scope query は同時に 1 種類のみ。複数指定は `INVALID_REQUEST` |
| `PUT /admin/v1/quotas/{scope}` | `{storage_bytes, rows?, write_ops_per_minute?}` | 200 `QuotaInfo` | `storage_bytes` は 0 以上の integer。`null` は無制限。負数は `INVALID_REQUEST` |
| `GET /admin/v1/usage` | query `organization?`, `group?`, `database?`, `limit?`, `cursor?` | `{"usage":[UsageInfo],"next_cursor": string|null}` | usage 再計測不能時は `USAGE_UNAVAILABLE` |

**Phase 8 DTO schema：**

```json
{
  "OrganizationInfo": {
    "id": "org_default",
    "name": "default",
    "slug": "default",
    "created_at": "2026-09-13T00:00:00Z"
  },
  "GroupInfo": {
    "id": "grp_default",
    "organization": "default",
    "name": "default",
    "slug": "default",
    "location": "default",
    "created_at": "2026-09-13T00:00:00Z"
  },
  "LocationInfo": {
    "id": "loc_default",
    "name": "default",
    "provider": "self-hosted",
    "region": "local",
    "primary": true,
    "created_at": "2026-09-13T00:00:00Z"
  },
  "QuotaInfo": {
    "scope_type": "organization",
    "scope": "default",
    "storage_bytes": 10737418240,
    "rows": null,
    "write_ops_per_minute": null,
    "updated_at": "2026-09-13T00:00:00Z"
  },
  "UsageInfo": {
    "scope_type": "database",
    "scope": "default",
    "storage_bytes": 0,
    "rows": null,
    "updated_at": "2026-09-13T00:00:00Z"
  }
}
```

上記 DTO の field はすべて必須である。`rows` と `write_ops_per_minute` だけ `null` 可とする。`id` は実装内で `org_`、`grp_`、`loc_` prefix を付けた一意文字列にする。既存 Turso Cloud の slug 互換を優先するため、API path では `id` と `slug/name` の両方を解決できるようにする。

**Phase 8 Turso 互換 DTO schema：**

```json
{
  "TursoDatabaseInfo": {
    "DbId": "550e8400-e29b-41d4-a716-446655440000",
    "Hostname": "my-db-default.adlaire.local",
    "Name": "my-db",
    "block_reads": false,
    "block_writes": false,
    "regions": ["default"],
    "primaryRegion": "default",
    "group": "default",
    "delete_protection": false,
    "parent": null
  },
  "TursoGroupInfo": {
    "name": "default",
    "version": "adlaire-0.64",
    "uuid": "grp_default",
    "locations": ["default"],
    "primary": "default",
    "delete_protection": false
  },
  "TursoOrganizationInfo": {
    "name": "default",
    "slug": "default",
    "type": "personal",
    "overages": false,
    "require_mfa": false,
    "blocked_reads": false,
    "blocked_writes": false,
    "plan_id": "self-hosted",
    "plan_timeline": "none",
    "platform": "adlaire"
  },
  "TursoOrganizationUsage": {
    "uuid": "org_default",
    "usage": {
      "rows_read": 0,
      "rows_written": 0,
      "databases": 1,
      "locations": 1,
      "storage_bytes": 4096,
      "groups": 1,
      "bytes_synced": 0
    },
    "databases": [
      {
        "uuid": "550e8400-e29b-41d4-a716-446655440000",
        "instances": [
          {
            "uuid": "550e8400-e29b-41d4-a716-446655440000",
            "usage": {
              "rows_read": 0,
              "rows_written": 0,
              "storage_bytes": 4096,
              "bytes_synced": 0
            }
          }
        ],
        "total": {
          "rows_read": 0,
          "rows_written": 0,
          "storage_bytes": 4096,
          "bytes_synced": 0
        }
      }
    ]
  }
}
```

**Phase 8 Turso Platform API token DTO schema：**

```json
{
  "TursoApiTokenInfo": {
    "name": "ci-token",
    "id": "tok_550e8400e29b41d4a716446655440000",
    "token": "adlpt_..."
  },
  "TursoApiTokenValidateInfo": {
    "exp": -1
  }
}
```

`token` は `POST /v1/auth/api-tokens/{tokenName}` の成功応答で 1 回だけ返す。`GET /v1/auth/validate` と `DELETE /v1/auth/api-tokens/{tokenName}` は token secret を返さない。Phase 8 の Platform API token は既存 `tokens.json` に `source:"turso-platform-api-token"`、`name:"{tokenName}"`、`organization_scope:null|string`、`platform_token:true` を付与して保存する。

| Turso API | Status | Response wrapper | Field casing rule |
|-----------|--------|------------------|-------------------|
| `GET /v1/auth/validate` | 200 | `{"exp": integer}` | 無期限 token は `-1` |
| `POST /v1/auth/api-tokens/{tokenName}` | 200 | `{"name": string, "id": string, "token": string}` | `token` は作成時のみ返す |
| `DELETE /v1/auth/api-tokens/{tokenName}` | 200 | `{"token": string}` | 値は token 名。secret ではない |
| `GET /v1/locations` | 200 | `{"locations":{...}}` | location code は object key、display name は string value |
| `GET /v1/organizations` | 200 | `[TursoOrganizationInfo]` | wrapper object は返さない |
| `PATCH /v1/organizations/{org}` | 200 | `{"organization":{...}}` | `overages`、`require_mfa` 以外の body field は `INVALID_REQUEST` |
| `GET /v1/organizations/{org}/usage` | 200 | `{"organization":{...}}` | usage field は snake_case |
| `GET /v1/organizations/{org}/groups` | 200 | `{"groups":[...]}` | group fields は Turso casing |
| `GET /v1/organizations/{org}/groups/{group}` | 200 | `{"group":{...}}` | retrieve は list 要素と同じ schema |
| `POST /v1/organizations/{org}/groups` | 200 | `{"group":{...}}` | 201 を返さない |
| `PATCH /v1/organizations/{org}/groups/{group}/configuration` | 200 | `{"delete_protection": boolean}` | wrapper 名を付けず configuration object を返す |
| `POST /v1/organizations/{org}/groups/{group}/auth/rotate` | 200 | body なし | group 配下 DB token を全失効。Platform API token は失効しない |
| `POST /v1/organizations/{org}/groups/{group}/transfer` | 501 | `{"error":"not implemented","code":"NOT_IMPLEMENTED"}` | transfer は Phase 8 では実装しない |
| `GET /v1/organizations/{org}/databases` | 200 | `{"databases":[...]}` | `DbId`、`Hostname`、`Name` は大文字始まりを維持 |
| `GET /v1/organizations/{org}/databases/{db}` | 200 | `{"database":{...}}` | retrieve は list 要素と同じ schema |
| `POST /v1/organizations/{org}/databases` | 200 | `{"database":{...}}` | 201 を返さない |
| `DELETE /v1/organizations/{org}/databases/{db}` | 200 | `{"database": string}` | 削除した DB 名を返す。204 ではない |
| `PATCH /v1/organizations/{org}/databases/{db}/configuration` | 200 | `{"size_limit": string|null, "allow_attach": boolean, "block_reads": boolean, "block_writes": boolean, "delete_protection": boolean}` | wrapper 名を付けず configuration object を返す |
| `POST /v1/organizations/{org}/databases/{db}/auth/tokens` | 200 | `{"jwt":"..."}` | `token` ではなく `jwt` |
| `POST /v1/organizations/{org}/databases/{db}/auth/rotate` | 200 | body なし | 既存 DB token を全失効。新 token は返さない |

`/v1/*` では request body の `name` validation を Turso 互換 DB 名規則 `^[a-z0-9-]{1,64}$` に固定する。`size_limit` は bytes 数値文字列または `kb`/`mb`/`gb` suffix を受け付け、quota の `storage_bytes` に変換する。`seed`、`remote_encryption`、database upload、CSV import、dump import は Phase 8 対象外であり、指定された場合は `400 INVALID_REQUEST` を返す。特に `seed.type:"database"`、`seed.name`、`seed.database`、`parent` body field による branch/clone 作成は Phase 15 まで受け付けず、Phase 8 では `400 INVALID_REQUEST` に固定する。

**Turso database token 互換：**

`POST /v1/organizations/{organizationSlug}/databases/{databaseName}/auth/tokens` は Turso 互換の query を受け付ける。

| Query/body | Allowed | Adlaire mapping |
|------------|---------|-----------------|
| `expiration` | `never` または `<number><s|m|h|d|w>` の連結。例: `2w1d30m` | JWT `exp`。`never` は `exp` なし |
| `authorization` | `full-access` / `read-only` | `full-access` → `a:"rw"`、`read-only` → `a:"ro"` |
| body `permissions` | object 可。ただし Phase 8 では table/action permission は実装しない | 空 object または省略のみ許可。非空は `INVALID_REQUEST` |

response は必ず `{"jwt":"<token>"}` とし、`id`、`access`、`expires_at` は返さない。発行した token metadata は既存 `tokens.json` に保存し、`source:"turso-platform-api"`、`database:"{databaseName}"`、`organization_scope:"{organizationSlug}"` を付与する。

**Turso Platform API token 互換：**

| API | Request | Success | Validation / persistence |
|-----|---------|---------|--------------------------|
| `GET /v1/auth/validate` | body なし | 200 `{"exp": -1}` または `{"exp": unix_seconds}` | 現在の Platform token が `tokens.json` 管理 token ならその expiry、Phase 8 の Admin token 代用なら `-1` |
| `POST /v1/auth/api-tokens/{tokenName}` | body `{organization?}` または body なし | 200 `{"name":"{tokenName}","id":"tok_...","token":"adlpt_..."}` | `tokenName` は `^[a-zA-Z0-9_-]{1,64}$`。`organization` 指定時は slug/id 解決必須。重複 tokenName は `409 DB_ALREADY_EXISTS` ではなく `400 INVALID_REQUEST` |
| `DELETE /v1/auth/api-tokens/{tokenName}` | body なし | 200 `{"token":"{tokenName}"}` | tokenName が存在しない場合は `404 TOKEN_NOT_FOUND`。失効済みなら 200 を返す |

Platform API token の secret は `adlpt_` prefix の不透明文字列とし、JWT ではない。検証は Bearer 完全一致で行う。作成した token は `tokens.json` に保存し、`revoked:false`、`platform_token:true`、`organization_scope` を持つ。`POST /v1/auth/api-tokens/{tokenName}` の応答以外では secret を返さず、log にも出さない。Phase 8 では Platform API token の権限は Admin token と同等だが、`organization_scope` がある場合は対象 organization 外の `/v1/*` 操作を `403 ORG_SCOPE_DENIED` とする。

**Turso database delete / auth rotate 互換：**

| API | Request | Success | Validation / persistence |
|-----|---------|---------|--------------------------|
| `DELETE /v1/organizations/{organizationSlug}/databases/{databaseName}` | body なし | 200 `{"database":"{databaseName}"}` | DB がない場合は `404 DB_NOT_FOUND`。DB directory 削除と `databases.json` 更新を Phase 7 delete と同じ atomic 手順で行う |
| `POST /v1/organizations/{organizationSlug}/databases/{databaseName}/auth/rotate` | body なし | 200 body なし | 対象 DB に紐づく `source:"turso-platform-api"` token をすべて `revoked:true` にする。Platform API token 自体は失効しない |

`auth/rotate` は新しい DB token を発行しない。失効対象が 0 件でも DB が存在すれば 200 とする。DB 削除時は当該 DB の DB token をすべて失効し、Platform API token は削除しない。

**Turso configuration / group token 互換：**

| API | Request | Success | Validation / persistence |
|-----|---------|---------|--------------------------|
| `PATCH /v1/organizations/{organizationSlug}/databases/{databaseName}/configuration` | `{size_limit?, delete_protection?, block_reads?, block_writes?, allow_attach?}` | 200 `{"size_limit": string|null, "allow_attach": boolean, "block_reads": boolean, "block_writes": boolean, "delete_protection": boolean}` | DB がない場合は `404 DB_NOT_FOUND`。unknown field は `INVALID_REQUEST`。少なくとも 1 field 必須。`size_limit` は quota、他 field は `databases.json` に atomic 永続化 |
| `PATCH /v1/organizations/{organizationSlug}/groups/{groupName}/configuration` | `{delete_protection}` | 200 `{"delete_protection": boolean}` | group がない場合は `404 GROUP_NOT_FOUND`。unknown field は `INVALID_REQUEST`。`delete_protection` は boolean 必須で `groups.json` に atomic 永続化 |
| `POST /v1/organizations/{organizationSlug}/groups/{groupName}/auth/rotate` | body なし | 200 body なし | group がない場合は `404 GROUP_NOT_FOUND`。group 配下 DB に紐づく `source:"turso-platform-api"` token と `group_scope` token をすべて `revoked:true` にする。Platform API token 自体は失効しない |

configuration の field 意味は以下に固定する。

| Field | 型 | 既定値 | 実装効果 |
|-------|----|--------|----------|
| `size_limit` | string / null | null | database scope quota の `storage_bytes`。`null` は無制限。現在使用量より小さい値も受け付け、既存データは読めるが以後の write/import/restore/replication apply/branch create は quota 判定で拒否する |
| `delete_protection` | boolean | false | `true` の DB は `/v1/*` と `/admin/v1/*` の削除を `403 ORG_SCOPE_DENIED` で拒否する。`true` の group は group 削除と配下 DB 削除を同じく拒否する |
| `block_reads` | boolean | false | `true` の DB への hrana read SQL、WebSocket read、backup download、export を `403 PERMISSION_DENIED` で拒否する。管理 API の一覧・詳細は拒否しない |
| `block_writes` | boolean | false | `true` の DB への write SQL、restore、replication apply、branch create、import を `403 PERMISSION_DENIED` で拒否する。quota 超過時は `QUOTA_EXCEEDED` を優先する |
| `allow_attach` | boolean | true | Phase 10 ATTACH の DB 単位許可。`false` の DB を source/target に含む ATTACH は `403 PERMISSION_DENIED` |

`block_reads` と `block_writes` は Turso DTO の同名 field にそのまま反映する。ただし `block_writes` は quota 超過の派生状態ではなく、configuration の明示値を返す。quota 超過を示す場合は error code `QUOTA_EXCEEDED` と usage/quota API で表現する。

**Turso 互換差分固定表：**

| Turso Cloud 機能 | Phase 8 Adlaire の扱い | 理由 |
|------------------|-------------------------|------|
| database upload / seed | `INVALID_REQUEST`。`seed.type:"database"` を含む branch/clone seed も Phase 8 では拒否 | Phase 14 restore/PITR と Phase 15 branch に分離するため Phase 8 対象外 |
| encrypted database | `INVALID_REQUEST` | encryption key/cipher 管理は本仕様の security boundary 外 |
| branch parent filter | query は受け付けるが Phase 8 では空配列。Phase 15 以降に branch metadata と接続する | branch は Phase 15 |
| group delete protection | `PATCH /groups/{group}/configuration` の `delete_protection` を保存して DTO に反映する | Turso Cloud 互換 API と削除保護を自己ホストでも対象に含めるため |
| multiple replica regions per group | Phase 8 は 1 primary location のみ | replication/HA は Phase 11〜18 |

**Phase 8 Turso unsupported API 固定表：**

| API family | Method | Phase 8 response | 理由 |
|------------|--------|------------------|------|
| `/v1/organizations/{org}/members...` | `GET`, `POST`, `PATCH`, `DELETE` | 501 `{"error":"not implemented","code":"NOT_IMPLEMENTED"}` | 自己ホスト単一運営者では user directory を持たない |
| `/v1/organizations/{org}/invites...` | `GET`, `POST`, `PATCH`, `DELETE` | 501 `NOT_IMPLEMENTED` | 招待 workflow と user directory を持たない |
| `/v1/organizations/{org}/plans` | `GET` | 501 `NOT_IMPLEMENTED`。ただし organization DTO の plan fields は固定値を返す | 課金連動は対象外。quota は Adlaire metadata で管理 |
| `/v1/organizations/{org}/plans` | `POST`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | Phase 8 では更新 API として公開しない |
| `/v1/organizations/{org}/billing...`、`/overages...` | `GET`, `POST`, `PATCH`, `DELETE` | 501 `NOT_IMPLEMENTED` | 課金連動は対象外 |
| `/v1/organizations/{org}/audit-logs` | `GET` | 501 `NOT_IMPLEMENTED` | audit log 永続化は Phase 8 の metadata 境界外。将来 Phase で専用 schema を定義する |
| `/v1/organizations/{org}/audit-logs` | `POST`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | 読み取り系 stub のみ定義 |
| `/v1/organizations/{org}/databases/{db}/stats` | `GET` | 501 `NOT_IMPLEMENTED` | SQL text を集計・保存しない秘匿方針を優先 |
| `/v1/organizations/{org}/databases/{db}/stats` | `POST`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | 読み取り系 stub のみ定義 |
| `/v1/organizations/{org}/groups/{group}/transfer` | `POST` | 501 `NOT_IMPLEMENTED` | owner/user directory と cross-organization transfer workflow を持たない |
| `/v1/organizations/{org}/groups/{group}/transfer` | `GET`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | transfer 作成以外の method は未定義 |
| `/v1/upload` | `POST` | 501 `NOT_IMPLEMENTED` | Phase 14 restore API として別管理し、database token upload は Phase 8 対象外 |
| `/v1/upload` | `GET`, `PATCH`, `DELETE` | 405 `METHOD_NOT_ALLOWED` | upload 作成以外の method は未定義 |
| encrypted database create/upload | body feature flag | 400 `INVALID_REQUEST` | request body feature flag として指定されるため、未対応入力として拒否 |
| seed / CSV / dump import | body feature flag | 400 `INVALID_REQUEST` | request body feature flag として指定されるため、未対応入力として拒否 |
| database branch seed | body `seed.type:"database"`、`seed.name`、`seed.database`、`parent` | 400 `INVALID_REQUEST` | branch/clone は Phase 15 で専用 metadata と atomic 作成手順を定義する |

501 stub は success response ではない。ログは WARN `not implemented endpoint` とし、request body、token、SQL、upload binary はログに出さない。

**既存 API の Phase 8 拡張：**

- `POST /admin/v1/databases` は `{name, organization?, group?, location?, quota?}` を受け付ける。省略時はすべて `"default"` を使う
- `DbInfo` は Phase 8 以降 `{name, created_at, path, organization, group, location, quota?, usage?}` を返す
- `POST /admin/v1/tokens` は `{access, expiry?, dbs?, organization_scope?, group_scope?}` を受け付ける
- token metadata は Phase 8 以降 `organization_scope` と `group_scope` を返す。ただし JWT 文字列は従来通り作成時のみ返す
- Phase 7 クライアントが送る `{name}`、`{access, expiry?, dbs?}` は後方互換として成功しなければならない

**Phase 8 metadata schema：**

```json
{
  "databases": [
    { "name": "default", "organization": "default", "group": "default", "location": "default", "delete_protection": false, "block_reads": false, "block_writes": false, "allow_attach": true }
  ],
  "organizations": [
    { "id": "default", "name": "default", "slug": "default", "created_at": "2026-09-12T00:00:00Z" }
  ],
  "groups": [
    { "id": "default", "organization": "default", "name": "default", "slug": "default", "location": "default", "delete_protection": false, "created_at": "2026-09-12T00:00:00Z" }
  ],
  "locations": [
    { "id": "default", "name": "default", "provider": "self-hosted", "region": "local", "primary": true }
  ],
  "quotas": [
    { "scope_type": "organization|group|database", "scope": "default", "storage_bytes": 10737418240, "rows": null, "write_ops_per_minute": null }
  ],
  "usage": [
    { "scope_type": "organization|group|database", "scope": "default", "storage_bytes": 0, "rows": null, "updated_at": "2026-09-12T00:00:00Z" }
  ]
}
```

実ファイルは §9.6 の通り `databases.json`、`organizations.json`、`groups.json`、`locations.json`、`quotas.json`、`usage.json` に分ける。上記 JSON は論理 schema の説明であり、1 ファイルへ統合してはならない。Phase 8 migration 後の `databases.json` は legacy field に加えて `organization`、`group`、`location`、`delete_protection`、`block_reads`、`block_writes`、`allow_attach` を必須 field とする。

**Phase 8 metadata unique constraint 固定表：**

Metadata store は読み込み時と書き込み前の両方で下表を検証する。違反を検出した metadata は起動失敗とし、API 書き込み時の競合は対応する error code を返して既存 metadata を変更しない。

| Resource | Unique key | 競合時 error |
|----------|------------|--------------|
| organization | `id` | `ORG_ALREADY_EXISTS` |
| organization | `slug` | `ORG_ALREADY_EXISTS` |
| group | `(organization, id)` | `GROUP_ALREADY_EXISTS` |
| group | `(organization, name)` | `GROUP_ALREADY_EXISTS` |
| group | `(organization, slug)` | `GROUP_ALREADY_EXISTS` |
| location | `id` | `LOCATION_ALREADY_EXISTS` |
| location | `name` | `LOCATION_ALREADY_EXISTS` |
| database | `name` | `DB_ALREADY_EXISTS` |
| database | `(organization, name)` | `DB_ALREADY_EXISTS` |
| quota | `(scope_type, scope)` | 既存 quota を `PUT` で更新し、新規重複 record は作らない |
| usage | `(scope_type, scope)` | 計測値を上書き更新し、新規重複 record は作らない |
| token | `id` | token id を再生成する。3 回連続衝突した場合は `500 INTERNAL_ERROR` |
| platform token | `name` | `400 INVALID_REQUEST`。Phase 8 では同名 Platform API token の上書き作成を許可しない |

DB 名は Phase 8 でも実ファイル path と hrana 接続 path の互換性を守るため global unique とする。Turso Platform API の path では organization 配下に見えるが、同名 DB を別 organization に作成することは Phase 8 では禁止し、`DB_ALREADY_EXISTS` を返す。

**Phase 8 migration：**

Phase 8 初回起動時に Phase 7 までの metadata を検出した場合、次を 1 回だけ実行する。

1. `organizations.json`、`groups.json`、`locations.json`、`quotas.json`、`usage.json` がなければ初期値を作る
2. 既存 `databases.json` の全 DB に `organization:"default"`、`group:"default"`、`location:"default"`、`delete_protection:false`、`block_reads:false`、`block_writes:false`、`allow_attach:true` を付与する
3. 既存 `tokens.json` の全 token に `organization_scope:null`、`group_scope:null` を付与し、従来の `dbs` claim は維持する
4. migration 中に失敗した場合は起動失敗とし、途中で更新済みの metadata を成功扱いにしない
5. migration は tmp write、fsync、rename の順で行い、全 metadata が整合した後に起動成功とする

**Phase 8 migration / rollback 固定手順：**

| 手順 | 必須処理 |
|------|----------|
| preflight | 既存 `databases.json` と `tokens.json` を読み、JSON parse、必須 field、DB directory 存在を検証する。失敗時は書き込み前に起動失敗 |
| backup | `{data-dir}/meta/migration-backup/phase8-{timestamp}/` に既存 metadata を copy + fsync する。backup 失敗時は起動失敗 |
| write | 新規 metadata は `.tmp` に書き、file fsync、directory fsync、rename、directory fsync の順で確定する |
| commit marker | 全 metadata 更新後に `{data-dir}/meta/phase8-migration.json` を `{"from_phase":7,"completed_at":...,"backup":"..."}` で書く |
| rollback | migration 中断を検出した場合、commit marker がなければ backup から復元して起動失敗にする。commit marker がある場合は rollback せず通常起動 |
| idempotency | commit marker がある状態で再起動した場合、migration を再実行しない |

Phase 8 migration は data loss を避けるため自動削除をしない。不要になった backup の削除 API は Phase 8 対象外であり、手動削除のみ許可する。

**Phase 8 権限優先順位：**

1. Admin token は全 organization/group/location/quota にアクセスできる
2. JWT に `org` claim がある場合、その organization 外の DB/group/quota 操作は `ORG_SCOPE_DENIED`
3. JWT に `grp` claim がある場合、その group 外の DB 操作は `ORG_SCOPE_DENIED`
4. `dbs` claim は DB 単位の最終制限として維持する。`org` / `grp` で許可されても `dbs` が拒否する DB は操作不可
5. `a:"ro"` は Phase 8 以降も書き込み、restore、replication apply、branch create を禁止する

**Phase 8 quota 判定：**

quota は organization、group、database の順にすべて評価する。1 つでも超過する場合は `QUOTA_EXCEEDED` を返し、DB ファイルや metadata を変更してはならない。usage が取得できず安全に判定できない場合は `USAGE_UNAVAILABLE` を返し、成功扱いにしない。

quota 判定で拒否する操作:

- `POST /v2/pipeline` と `POST /{db-name}/v2/pipeline` の write SQL
- `sequence` に含まれる write SQL
- backup restore / PITR restore
- replication apply
- branch create
- import API は本仕様書では未定義のため実装禁止

backup download、read-only SELECT、DB/token/location/org/group/quota の一覧取得は quota 超過時でも許可する。

**Phase 8 snapshot artifact 固定表：**

Turso 互換 snapshot は `tests/snapshots/phase8_turso/` に保存する。各 snapshot は `status`、`content_type`、`body` を必須 field とし、`Date`、`Server`、request id、JWT、UUID、timestamp は比較前に placeholder へ正規化する。`content_type` は JSON response では `application/json` とし、charset の有無で比較結果を変えてはならない。

**Snapshot 比較共通仕様：**

| 項目 | 固定仕様 |
|------|----------|
| JSON key order | 比較前に object key を辞書順へ正規化する。array order は API 契約通り比較する |
| dynamic placeholder | UUID は `<uuid>`、RFC3339 timestamp は `<timestamp>`、JWT/Platform token は `<secret>`、request id は `<request_id>`、hostname の DB/org 部分以外は `<host>` |
| header 比較 | `content-type`、互換に必要な `location`、replication 系 `x-adlaire-*` だけ比較する。`date`、`server`、`content-length` は比較しない |
| status 比較 | HTTP status は必ず比較する。hrana SQL error の場合は HTTP 200 と body 内 error code を比較する |
| body 比較 | error body は `error` と `code` だけ比較する。success body は schema field の過不足を厳密比較する |
| 更新禁止条件 | 実装変更だけで snapshot を更新してはならない。Turso 追従または本仕様変更 commit が先に存在する場合だけ更新可 |
| CI failure | snapshot 差分、未生成 snapshot、placeholder 未正規化、secret 検出はすべて CI failure |
| secret scan | snapshot directory に `Bearer `、`eyJ`、`adlpt_`、admin token 生値、JWT signature 形式があれば failure |

| Snapshot file | 対象 |
|---------------|------|
| `auth.validate.json` | `GET /v1/auth/validate` |
| `auth.api_tokens.create.json` | `POST /v1/auth/api-tokens/{tokenName}` |
| `auth.api_tokens.revoke.json` | `DELETE /v1/auth/api-tokens/{tokenName}` |
| `locations.list.json` | `GET /v1/locations` |
| `organizations.list.json` | `GET /v1/organizations` |
| `organizations.update.json` | `PATCH /v1/organizations/{org}` |
| `organizations.usage.json` | `GET /v1/organizations/{org}/usage` |
| `groups.list.json` | `GET /v1/organizations/{org}/groups` |
| `groups.create.json` | `POST /v1/organizations/{org}/groups` |
| `groups.retrieve.json` | `GET /v1/organizations/{org}/groups/{group}` |
| `groups.configuration.update.json` | `PATCH /v1/organizations/{org}/groups/{group}/configuration` |
| `groups.rotate_tokens.json` | `POST /v1/organizations/{org}/groups/{group}/auth/rotate` |
| `databases.list.json` | `GET /v1/organizations/{org}/databases` |
| `databases.create.json` | `POST /v1/organizations/{org}/databases` |
| `databases.retrieve.json` | `GET /v1/organizations/{org}/databases/{db}` |
| `databases.delete.json` | `DELETE /v1/organizations/{org}/databases/{db}` |
| `databases.configuration.update.json` | `PATCH /v1/organizations/{org}/databases/{db}/configuration` |
| `databases.create_token.json` | `POST /v1/organizations/{org}/databases/{db}/auth/tokens` |
| `databases.rotate_tokens.json` | `POST /v1/organizations/{org}/databases/{db}/auth/rotate` |
| `databases.branch_seed.invalid_request.json` | `POST /v1/organizations/{org}/databases` with `seed.type:"database"` |
| `errors.quota_exceeded_402.json` | `/v1/*` quota 超過時の 402 response |
| `unsupported.audit_logs.json` | `GET /v1/organizations/{org}/audit-logs` |
| `unsupported.group_transfer.json` | `POST /v1/organizations/{org}/groups/{group}/transfer` |
| `unsupported.not_implemented.json` | 501 stub response |
| `routing.validation_errors.json` | 404/405/400 の route/request validation response |

Phase 8 の実装 PR は、上記 API、metadata、migration、権限、quota、エラー契約、snapshot artifact をすべて満たすことを完了条件とする。

**Phase 8 追加テストケース：**

```
TC-8-1: organization CRUD
  （a）POST /admin/v1/organizations {name:"acme"} → 201
  （b）GET /admin/v1/organizations → acme を含む
  （c）GET /admin/v1/organizations/acme → 200
  （d）DELETE /admin/v1/organizations/acme → 204

TC-8-2: group と location の関連
  （a）POST /admin/v1/locations {name:"local"} → 201
  （b）POST /admin/v1/groups {organization:"default", name:"app", location:"local"} → 201
  （c）存在しない organization/location 指定 → 404 ORG_NOT_FOUND / LOCATION_NOT_FOUND

TC-8-3: DB 作成の scope 拡張
  （a）POST /admin/v1/databases {name:"db1", organization:"default", group:"default", location:"default"} → 201
  （b）GET /admin/v1/databases/db1 → organization/group/location を含む
  （c）Phase 7 互換の {name:"db2"} → 201、default scope が付与される

TC-8-4: token scope 拡張
  （a）POST /admin/v1/tokens {access:"rw", organization_scope:"default"} → 201
  （b）scope 外 DB への write → 403 ORG_SCOPE_DENIED
  （c）dbs claim が拒否する DB は org/group scope が許可しても 403

TC-8-5: quota exceeded
  （a）PUT /admin/v1/quotas/database:db1 {storage_bytes:1} → 200
  （b）db1 へ write SQL → 403 QUOTA_EXCEEDED
  （c）SELECT と backup download は quota 超過中も成功

TC-8-6: usage unavailable
  （a）usage snapshot を取得不能状態にする
  （b）quota 判定が必要な write → 503 USAGE_UNAVAILABLE
  （c）GET /admin/v1/usage → 503 USAGE_UNAVAILABLE

TC-8-7: metadata migration
  （a）Phase 7 の databases.json/tokens.json だけが存在する data-dir で起動
  （b）Phase 8 metadata が作成され、既存 DB/token に default scope が付与される
  （c）再起動後も同じ metadata が復元される

TC-8-8: atomicity / rollback
  （a）organizations/groups/locations/quotas の更新中に失敗を注入
  （b）起動成功扱いにせず、partial metadata を成功応答しない

TC-8-9: Turso Platform API 互換 snapshot
  （a）GET /v1/locations → 200 {"locations": {"default":"Self Hosted Default"}}
  （b）POST /v1/organizations/default/groups {name:"app",location:"default"} → 200 {"group":{name,version,uuid,locations,primary,delete_protection}}
  （c）POST /v1/organizations/default/databases {name:"my-db",group:"app"} → 200 {"database":{DbId,Hostname,Name,...}}
  （d）GET /v1/organizations/default/databases?group=app → 200、Turso field casing を維持
  （e）POST /v1/organizations/default/databases/my-db/auth/tokens?expiration=2w&authorization=read-only → 200 {"jwt":"..."}
  （f）uppercase / underscore DB 名 → 400 INVALID_DB_NAME
  （g）Phase 7 legacy DB 名は接続・削除可能だが、Phase 8 新規作成では拒否

TC-8-10: Turso organization API
  （a）GET /v1/organizations → 200 [TursoOrganizationInfo]
  （b）PATCH /v1/organizations/default {overages:false,require_mfa:false} → 200 {"organization":TursoOrganizationInfo}
  （c）PATCH unknown field → 400 INVALID_REQUEST
  （d）GET /v1/organizations/default/usage → 200 {"organization":TursoOrganizationUsage}
  （e）usage 計測不能 → 503 USAGE_UNAVAILABLE

TC-8-11: Turso retrieve API
  （a）GET /v1/organizations/default/groups/app → 200 {"group":TursoGroupInfo}
  （b）GET /v1/organizations/default/databases/my-db → 200 {"database":TursoDatabaseInfo}
  （c）存在しない group → 404 GROUP_NOT_FOUND
  （d）存在しない database → 404 DB_NOT_FOUND

TC-8-12: Turso unsupported API
  （a）GET /v1/organizations/default/plans → 501 NOT_IMPLEMENTED
  （b）POST /v1/organizations/default/members → 501 NOT_IMPLEMENTED
  （c）GET /v1/organizations/default/databases/my-db/stats → 501 NOT_IMPLEMENTED
  （d）POST /v1/upload → 501 NOT_IMPLEMENTED
  （e）POST /v1/organizations/default/databases {name:"seeded",group:"default",seed:{...}} → 400 INVALID_REQUEST
  （f）unsupported API の log に token/body/upload binary が出ない

TC-8-13: Turso route priority / method handling
  （a）POST /v1/organizations/default/databases/my-db/auth/tokens は DB 詳細 route ではなく token 作成 route に一致
  （b）GET /v1/organizations/default/groups/app は groups 一覧 route ではなく group 詳細 route に一致
  （c）GET /v1/locations/ → 404 ENDPOINT_NOT_FOUND
  （d）DELETE /v1/locations → 405 METHOD_NOT_ALLOWED
  （e）未定義 path → 404 ENDPOINT_NOT_FOUND

TC-8-14: Turso request validation
  （a）GET request body あり → 400 INVALID_REQUEST
  （b）POST/PATCH で Content-Type 欠落または JSON object 以外 → 400 INVALID_REQUEST
  （c）unknown query、duplicate query key、unknown body field → 400 INVALID_REQUEST
  （d）percent decode 不能、空 path parameter、path parameter に / または . または .. → 400 INVALID_REQUEST
  （e）Platform token 欠落 → 401 AUTH_REQUIRED、不一致 → 401 AUTH_INVALID

TC-8-15: Phase 8 metadata unique constraints
  （a）organization slug/id 重複 → 409 ORG_ALREADY_EXISTS
  （b）同一 organization 内 group name/slug/id 重複 → 409 GROUP_ALREADY_EXISTS
  （c）location name/id 重複 → 409 LOCATION_ALREADY_EXISTS
  （d）DB name は organization が異なっても global duplicate として 409 DB_ALREADY_EXISTS
  （e）quota/usage は同一 scope を重複追加せず更新する

TC-8-16: Turso snapshot artifact completeness
  （a）tests/snapshots/phase8_turso/*.json が固定表の全 file を含む
  （b）各 snapshot は status/content_type/body を含む
  （c）UUID/timestamp/JWT 等の動的値は placeholder に正規化される
  （d）routing.validation_errors.json が 400/404/405 response を含む

TC-8-17: Turso Platform API token
  （a）GET /v1/auth/validate（Admin token 代用）→ 200 {"exp":-1}
  （b）POST /v1/auth/api-tokens/ci {organization:"default"} → 200 {"name":"ci","id":"tok_<uuid>","token":"<secret>"}
  （c）作成 token で GET /v1/organizations/default/groups → 200
  （d）DELETE /v1/auth/api-tokens/ci → 200 {"token":"ci"}
  （e）失効後 token で GET /v1/auth/validate → 401 AUTH_INVALID

TC-8-18: Turso database delete / auth rotate
  （a）POST /v1/organizations/default/databases {name:"delete-me",group:"default"} → 200
  （b）DELETE /v1/organizations/default/databases/delete-me → 200 {"database":"delete-me"}
  （c）削除済み DB の GET → 404 DB_NOT_FOUND
  （d）POST /v1/organizations/default/databases/my-db/auth/rotate → 200 body なし
  （e）rotate 前に発行した DB token は 401 AUTH_INVALID

TC-8-19: Turso audit logs unsupported
  （a）GET /v1/organizations/default/audit-logs → 501 NOT_IMPLEMENTED
  （b）page/page_size query は受け付けるが success response は返さない
  （c）POST/PATCH/DELETE /v1/organizations/default/audit-logs → 405 METHOD_NOT_ALLOWED
  （d）stub log に token/body が出ない

TC-8-20: Snapshot comparison strictness
  （a）snapshot 比較前に JSON key order と dynamic placeholder が正規化される
  （b）snapshot 内に Bearer/JWT/adlpt_ が残る場合は failure
  （c）status/body schema 差分は failure
  （d）仕様変更 commit なしの snapshot 更新は review failure

TC-8-21: Turso database configuration
  （a）PATCH /v1/organizations/default/databases/my-db/configuration {size_limit:"1mb",delete_protection:true,block_reads:false,block_writes:false,allow_attach:true} → 200 {"size_limit":"1mb","allow_attach":true,"block_reads":false,"block_writes":false,"delete_protection":true}
  （b）delete_protection=true の DB を DELETE /v1/* と DELETE /admin/v1/* で削除しようとすると 403 ORG_SCOPE_DENIED
  （c）size_limit は database quota に反映され、制限超過 write は /v1/* では 402 QUOTA_EXCEEDED、hrana/admin 経由では 403 QUOTA_EXCEEDED
  （d）block_reads=true の DB への read SQL、backup download、export は 403 PERMISSION_DENIED
  （e）block_writes=true の DB への write SQL、restore、replication apply、branch create、import は 403 PERMISSION_DENIED

TC-8-22: Turso group configuration / group auth rotate
  （a）PATCH /v1/organizations/default/groups/app/configuration {delete_protection:true} → 200 {"delete_protection":true}
  （b）delete_protection=true の group 削除と配下 DB 削除は 403 ORG_SCOPE_DENIED
  （c）POST /v1/organizations/default/groups/app/auth/rotate → 200 body なし
  （d）rotate 前に発行した group 配下 DB token と group_scope token は 401 AUTH_INVALID、Platform API token は引き続き有効

TC-8-23: Turso group transfer unsupported / branch seed rejection
  （a）POST /v1/organizations/default/groups/app/transfer → 501 NOT_IMPLEMENTED
  （b）GET/PATCH/DELETE /v1/organizations/default/groups/app/transfer → 405 METHOD_NOT_ALLOWED
  （c）POST /v1/organizations/default/databases {name:"branch-copy",group:"default",seed:{type:"database",name:"my-db"}} → 400 INVALID_REQUEST
  （d）seed upload、CSV import、dump import、remote_encryption はすべて 400 INVALID_REQUEST
  （e）unsupported/stub log に token、body、SQL、upload binary が出ない

TC-8-24: Strict unknown field default
  （a）/admin/v1/*、/v1/*、destructive API、永続化 API の unknown body field は 400 INVALID_REQUEST
  （b）unknown field を無視できるのは hrana wire protocol など該当節で明記された endpoint のみ
  （c）unknown query、duplicate query key、空 query value の拒否は §Phase 8 request validation 固定表と一致する
  （d）TC-8-1〜TC-8-24 の全 snapshot と regression が同時に通る
```

**Phase 8 実装タスク：**

```
T8-1: organization/location/group/quota/usage 型と metadata store を追加
T8-2: Phase 7 metadata から Phase 8 metadata への migration を実装
T8-3: organization/group/location CRUD API を実装
T8-4: quota/usage API と scope parser（organization:{id} / group:{id} / database:{name}）を実装
T8-5: DB/token API に organization/group/location/quota scope を統合
T8-6: JWT org/grp claim と dbs claim の権限優先順位を実装
T8-7: quota 判定を write/restore/replication apply/branch create に接続する
T8-8: Turso Platform API 互換 `/v1/*` route と response wrapper を実装
T8-9: Turso organization list/update/usage API を実装
T8-10: Turso group/database retrieve API を実装
T8-11: Turso unsupported API の 501/400 分類を実装
T8-12: Turso route priority と request validation を固定表通り実装
T8-13: Phase 8 metadata unique constraint を起動時・書き込み前に検証
T8-14: Turso snapshot artifact 生成と正規化比較を実装
T8-15: METHOD_NOT_ALLOWED / ENDPOINT_NOT_FOUND error mapping を実装
T8-16: Turso Platform API token create/validate/revoke を実装
T8-17: Turso database delete と auth rotate を実装
T8-18: audit logs 501 stub と method 分類を実装
T8-19: snapshot 比較 strictness と secret scan を実装
T8-20: strict unknown field default を §9.3 と Phase 8 request validation 固定表通り実装
T8-21: Turso database configuration 更新、quota 接続、delete/block/attach policy を実装
T8-22: Turso group configuration と group auth rotate を実装
T8-23: group transfer 501 stub と branch seed 400 rejection を実装
T8-24: TC-8-1〜TC-8-24 を通す
```
