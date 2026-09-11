// ─── リクエスト ───────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct PipelineRequest {
    pub baton:    Option<String>,
    pub requests: Vec<StreamRequest>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamRequest {
    Execute  { stmt: Stmt },
    Sequence { sql: String },
    Close,
}

#[derive(Debug, serde::Deserialize)]
pub struct Stmt {
    pub sql:        String,
    pub args:       Vec<Value>,
    #[serde(default)]
    pub named_args: Vec<NamedArg>,
    #[serde(default)]
    pub want_rows:  bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct NamedArg {
    pub name:  String,
    pub value: Value,
}

// ─── hrana Value ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Value {
    Null,
    Integer { value: String },
    Real    { value: f64    },
    Text    { value: String },
    Blob    { value: String }, // base64
}

// ─── レスポンス ───────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct PipelineResponse {
    pub baton:    Option<String>,
    pub base_url: Option<String>,
    pub results:  Vec<StreamResult>,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamResult {
    Ok    { response: StreamResponse },
    Error { error: HranaError         },
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamResponse {
    Execute  { result: StmtResult },
    Sequence,
    Close,
}

#[derive(Debug, serde::Serialize)]
pub struct StmtResult {
    pub cols:              Vec<Col>,
    pub rows:              Vec<Vec<Value>>,
    pub rows_affected:     u64,
    pub last_insert_rowid: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct Col {
    pub name:     Option<String>,
    pub decltype: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct HranaError {
    pub message: String,
    pub code:    String,
}
