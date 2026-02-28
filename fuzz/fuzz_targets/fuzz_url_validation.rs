#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz the RAPS domain allowlist validation (http.rs)
        // This exercises subdomain matching, boundary checks, and edge cases
        let _ = raps_kernel::http::is_allowed_url(s);
    }
});
