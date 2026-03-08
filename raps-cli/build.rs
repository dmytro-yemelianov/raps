fn main() {
    // Allow overriding the Ed25519 public key at build time via environment variable.
    // In production CI, set RAPS_MARKETPLACE_ED25519_PUBKEY to the hex-encoded public key.
    // Falls back to a placeholder for development builds.
    println!("cargo:rerun-if-env-changed=RAPS_MARKETPLACE_ED25519_PUBKEY");
}
