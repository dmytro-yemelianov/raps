#![no_main]
use libfuzzer_sys::fuzz_target;

/// Mirrors raps-kernel ProfilesData for fuzzing deserialization
#[derive(serde::Deserialize)]
struct ProfileConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub base_url: Option<String>,
    pub callback_url: Option<String>,
    pub da_nickname: Option<String>,
    #[serde(default)]
    pub use_keychain: bool,
}

#[derive(serde::Deserialize)]
struct ProfilesData {
    pub active_profile: Option<String>,
    #[serde(default)]
    pub profiles: std::collections::HashMap<String, ProfileConfig>,
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz RAPS profile JSON deserialization — same shape as profiles.json
        let _ = serde_json::from_str::<ProfilesData>(s);
    }
});
