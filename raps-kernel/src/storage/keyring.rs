// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Storage backend type

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    /// File-based storage (default)
    File,
    /// OS keychain storage (Windows Credential Manager, macOS Keychain, Linux Secret Service)
    Keychain,
}

impl StorageBackend {
    /// Determine storage backend from environment variable
    pub fn from_env() -> Self {
        std::env::var("RAPS_USE_KEYCHAIN")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(StorageBackend::Keychain),
                _ => None,
            })
            .unwrap_or(StorageBackend::File)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_backend_from_env_default() {
        unsafe {
            std::env::remove_var("RAPS_USE_KEYCHAIN");
        }
        let backend = StorageBackend::from_env();
        assert_eq!(backend, StorageBackend::File);
    }

    #[test]
    fn test_storage_backend_from_env_keychain_true() {
        unsafe {
            std::env::set_var("RAPS_USE_KEYCHAIN", "true");
        }
        let backend = StorageBackend::from_env();
        assert_eq!(backend, StorageBackend::Keychain);
        unsafe {
            std::env::remove_var("RAPS_USE_KEYCHAIN");
        }
    }

    #[test]
    fn test_storage_backend_from_env_keychain_1() {
        unsafe {
            std::env::set_var("RAPS_USE_KEYCHAIN", "1");
        }
        let backend = StorageBackend::from_env();
        assert_eq!(backend, StorageBackend::Keychain);
        unsafe {
            std::env::remove_var("RAPS_USE_KEYCHAIN");
        }
    }

    #[test]
    fn test_storage_backend_from_env_keychain_yes() {
        unsafe {
            std::env::set_var("RAPS_USE_KEYCHAIN", "yes");
        }
        let backend = StorageBackend::from_env();
        assert_eq!(backend, StorageBackend::Keychain);
        unsafe {
            std::env::remove_var("RAPS_USE_KEYCHAIN");
        }
    }

    #[test]
    fn test_storage_backend_from_env_keychain_on() {
        unsafe {
            std::env::set_var("RAPS_USE_KEYCHAIN", "on");
        }
        let backend = StorageBackend::from_env();
        assert_eq!(backend, StorageBackend::Keychain);
        unsafe {
            std::env::remove_var("RAPS_USE_KEYCHAIN");
        }
    }

    #[test]
    fn test_storage_backend_from_env_keychain_false() {
        unsafe {
            std::env::set_var("RAPS_USE_KEYCHAIN", "false");
        }
        let backend = StorageBackend::from_env();
        assert_eq!(backend, StorageBackend::File);
        unsafe {
            std::env::remove_var("RAPS_USE_KEYCHAIN");
        }
    }

    #[test]
    fn test_storage_backend_equality() {
        assert_eq!(StorageBackend::File, StorageBackend::File);
        assert_eq!(StorageBackend::Keychain, StorageBackend::Keychain);
        assert_ne!(StorageBackend::File, StorageBackend::Keychain);
    }
}
