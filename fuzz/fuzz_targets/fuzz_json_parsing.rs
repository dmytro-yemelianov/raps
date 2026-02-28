#![no_main]
use libfuzzer_sys::fuzz_target;

/// Mirrors raps-kernel TokenResponse for fuzzing OAuth token deserialization
#[derive(serde::Deserialize)]
struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}

/// Mirrors raps-kernel ApsErrorResponse for fuzzing API error deserialization
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApsErrorResponse {
    #[serde(alias = "error", alias = "errorCode")]
    pub error_code: Option<String>,
    #[serde(alias = "error_description", alias = "errorDescription")]
    pub description: Option<String>,
    #[serde(alias = "message", alias = "msg")]
    pub detail: Option<String>,
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz RAPS OAuth token response deserialization
        let _ = serde_json::from_str::<TokenResponse>(s);
        // Fuzz RAPS API error response deserialization
        let _ = serde_json::from_str::<ApsErrorResponse>(s);
    }
});
