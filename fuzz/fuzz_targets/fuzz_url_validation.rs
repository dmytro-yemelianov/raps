#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz URL parsing — exercises the same path used by raps-kernel http.rs
        let _ = url::Url::parse(s);
    }
});
