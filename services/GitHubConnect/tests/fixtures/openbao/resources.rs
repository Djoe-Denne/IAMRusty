use serde::Serialize;

pub const BOOTSTRAP_TOKEN: &str = "synthetic-bao-bootstrap-sentinel";
pub const OAUTH_PATH: &str = "/v1/secret/data/github-connect/oauth";
pub const HMAC_PATH: &str = "/v1/secret/data/github-connect/iam";

#[derive(Serialize)]
pub struct Lookup {
    pub data: LookupData,
}
#[derive(Serialize)]
pub struct LookupData {
    pub policies: Vec<&'static str>,
    pub ttl: u64,
}
