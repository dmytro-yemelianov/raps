#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz JSON parsing — exercises API response parsing paths
        let _ = serde_json::from_str::<serde_json::Value>(s);
    }
});
