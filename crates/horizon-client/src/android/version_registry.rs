//! Multi-Version Registry for Horizon Android.
//!
//! Tracks Minecraft versions, Java runtime requirements, protocol versions,
//! download manifests, backend compatibility (Horizon Rust vs Java + Fabric),
//! Fabric Loader availability, installation states, and verification status.
//!
//! Rule: Never mark an unsupported version as playable. World migration
//! support (DataFixers) is distinct from actual game-version execution.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Execution backend for an instance or game session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    /// Horizon clean-room Rust engine (targeting Minecraft Java 26.3).
    #[default]
    HorizonRust,
    /// Genuine Minecraft Java Edition client running under OpenJDK with Fabric Loader.
    JavaFabric,
}

impl Backend {
    pub fn label(self) -> &'static str {
        match self {
            Self::HorizonRust => "Horizon Rust Engine",
            Self::JavaFabric => "Minecraft Java + Fabric Runtime",
        }
    }
}

/// Java runtime version requirement for Minecraft Java Edition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JavaVersion {
    Java8 = 8,
    Java16 = 16,
    Java17 = 17,
    Java21 = 21,
    Java25 = 25,
}

impl JavaVersion {
    pub fn major_version(self) -> u32 {
        self as u32
    }

    pub fn component_name(self) -> &'static str {
        match self {
            Self::Java8 => "jre-legacy",
            Self::Java16 => "java-runtime-alpha",
            Self::Java17 => "java-runtime-gamma",
            Self::Java21 => "java-runtime-delta",
            Self::Java25 => "java-runtime-25",
        }
    }
}

/// Installation state on the local Android storage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum InstallationState {
    NotInstalled,
    Installing { progress: f32, message: String },
    Installed,
    Corrupted { reason: String },
}

/// Verification status of downloaded binaries and game data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "verification", rename_all = "kebab-case")]
pub enum VerificationStatus {
    Unverified,
    Verified { date: String, sha256: String },
    Failed { reason: String },
}

/// Resource and download metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceMetadata {
    pub asset_index: String,
    pub client_jar_sha1: Option<String>,
    pub server_jar_sha1: Option<String>,
}

/// Single version entry in the multi-version registry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VersionEntry {
    /// Minecraft version string (e.g. "26.3", "1.21.4").
    pub minecraft_version: String,
    /// Protocol version (e.g. 777 for 26.3, 769 for 1.21.4).
    pub protocol_version: i32,
    /// Java version required to execute this client.
    pub java_version: JavaVersion,
    /// Official Mojang version manifest URL.
    pub manifest_url: String,
    /// Pinned SHA-1 of the version manifest if available.
    pub manifest_sha1: Option<String>,
    /// Resource metadata.
    pub resource_metadata: ResourceMetadata,
    /// Which execution backends are capable of running this version.
    pub backend_compatibility: Vec<Backend>,
    /// Whether Fabric Loader is available for this version.
    pub fabric_supported: bool,
    /// Recommended or tested Fabric Loader version.
    pub fabric_loader_version: Option<String>,
    /// Mod compatibility notes or limitations.
    pub mod_compatibility_notes: String,
    /// Local installation state.
    pub installation_state: InstallationState,
    /// Cryptographic verification status.
    pub verification_status: VerificationStatus,
    /// Explicit flag indicating if this version is currently verified as playable.
    pub playable: bool,
}

impl VersionEntry {
    /// Checks if this version is verified and playable on the specified backend.
    pub fn is_playable_on(&self, backend: Backend) -> bool {
        self.playable
            && self.backend_compatibility.contains(&backend)
            && matches!(self.installation_state, InstallationState::Installed)
            && matches!(self.verification_status, VerificationStatus::Verified { .. })
    }

    /// Checks whether Fabric mods are supported on this version.
    pub fn supports_fabric(&self) -> bool {
        self.fabric_supported && self.backend_compatibility.contains(&Backend::JavaFabric)
    }
}

/// Multi-Version Registry tracking all known versions and installation state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionRegistry {
    pub versions: Vec<VersionEntry>,
    pub default_version: String,
    pub default_backend: Backend,
}

impl Default for VersionRegistry {
    fn default() -> Self {
        Self::standard_catalog()
    }
}

impl VersionRegistry {
    /// Creates the standard curated registry catalog.
    ///
    /// Primary target: 26.3, followed by progressive backwards support to 1.21.4.
    pub fn standard_catalog() -> Self {
        let versions = vec![
            // Target 1: Minecraft 26.3 (Current Horizon baseline)
            VersionEntry {
                minecraft_version: "26.3".into(),
                protocol_version: 777,
                java_version: JavaVersion::Java25,
                manifest_url: "https://piston-meta.mojang.com/v1/packages/26.3/26.3.json".into(),
                manifest_sha1: Some("11f1aefe473eefba0af62e3286c33b692807b2eb".into()),
                resource_metadata: ResourceMetadata {
                    asset_index: "34".into(),
                    client_jar_sha1: Some("e877b6a07acd633fb3bb475002175cec036e7b87".into()),
                    server_jar_sha1: Some("33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c".into()),
                },
                backend_compatibility: vec![Backend::HorizonRust, Backend::JavaFabric],
                fabric_supported: true,
                fabric_loader_version: Some("0.16.10".into()),
                mod_compatibility_notes: "Horizon Rust runs vanilla 26.3 and .hmod mods. Java backend runs 26.3 with Fabric Loader via OpenJDK 25.".into(),
                installation_state: InstallationState::NotInstalled,
                verification_status: VerificationStatus::Unverified,
                playable: false,
            },
            // Target 2: Minecraft 1.21.4
            VersionEntry {
                minecraft_version: "1.21.4".into(),
                protocol_version: 769,
                java_version: JavaVersion::Java21,
                manifest_url: "https://piston-meta.mojang.com/v1/packages/1.21.4/1.21.4.json".into(),
                manifest_sha1: None,
                resource_metadata: ResourceMetadata {
                    asset_index: "19".into(),
                    client_jar_sha1: None,
                    server_jar_sha1: None,
                },
                backend_compatibility: vec![Backend::JavaFabric],
                fabric_supported: true,
                fabric_loader_version: Some("0.16.9".into()),
                mod_compatibility_notes: "Java + Fabric runtime on OpenJDK 21 with Mesa Zink (OpenGL 3.3 Core).".into(),
                installation_state: InstallationState::NotInstalled,
                verification_status: VerificationStatus::Unverified,
                playable: false,
            },
            // Backwards Target 3: Minecraft 1.21.3
            VersionEntry {
                minecraft_version: "1.21.3".into(),
                protocol_version: 768,
                java_version: JavaVersion::Java21,
                manifest_url: "https://piston-meta.mojang.com/v1/packages/1.21.3/1.21.3.json".into(),
                manifest_sha1: None,
                resource_metadata: ResourceMetadata {
                    asset_index: "18".into(),
                    client_jar_sha1: None,
                    server_jar_sha1: None,
                },
                backend_compatibility: vec![Backend::JavaFabric],
                fabric_supported: true,
                fabric_loader_version: Some("0.16.9".into()),
                mod_compatibility_notes: "Java + Fabric runtime on OpenJDK 21.".into(),
                installation_state: InstallationState::NotInstalled,
                verification_status: VerificationStatus::Unverified,
                playable: false,
            },
            // Backwards Target 4: Minecraft 1.21.1 (Major modding LTS)
            VersionEntry {
                minecraft_version: "1.21.1".into(),
                protocol_version: 767,
                java_version: JavaVersion::Java21,
                manifest_url: "https://piston-meta.mojang.com/v1/packages/1.21.1/1.21.1.json".into(),
                manifest_sha1: None,
                resource_metadata: ResourceMetadata {
                    asset_index: "17".into(),
                    client_jar_sha1: None,
                    server_jar_sha1: None,
                },
                backend_compatibility: vec![Backend::JavaFabric],
                fabric_supported: true,
                fabric_loader_version: Some("0.16.9".into()),
                mod_compatibility_notes: "Extensive Fabric mod ecosystem. OpenJDK 21 required.".into(),
                installation_state: InstallationState::NotInstalled,
                verification_status: VerificationStatus::Unverified,
                playable: false,
            },
            // Backwards Target 5: Minecraft 1.20.4
            VersionEntry {
                minecraft_version: "1.20.4".into(),
                protocol_version: 765,
                java_version: JavaVersion::Java17,
                manifest_url: "https://piston-meta.mojang.com/v1/packages/1.20.4/1.20.4.json".into(),
                manifest_sha1: None,
                resource_metadata: ResourceMetadata {
                    asset_index: "16".into(),
                    client_jar_sha1: None,
                    server_jar_sha1: None,
                },
                backend_compatibility: vec![Backend::JavaFabric],
                fabric_supported: true,
                fabric_loader_version: Some("0.15.11".into()),
                mod_compatibility_notes: "OpenJDK 17 required.".into(),
                installation_state: InstallationState::NotInstalled,
                verification_status: VerificationStatus::Unverified,
                playable: false,
            },
        ];

        Self {
            versions,
            default_version: "26.3".into(),
            default_backend: Backend::HorizonRust,
        }
    }

    /// Finds a version entry by Minecraft version ID.
    pub fn get(&self, version: &str) -> Option<&VersionEntry> {
        self.versions.iter().find(|v| v.minecraft_version == version)
    }

    /// Finds a mutable version entry by Minecraft version ID.
    pub fn get_mut(&mut self, version: &str) -> Option<&mut VersionEntry> {
        self.versions.iter().mut_find(|v| v.minecraft_version == version)
    }

    /// Returns list of all versions that are currently playable on a specific backend.
    pub fn playable_versions(&self, backend: Backend) -> Vec<&VersionEntry> {
        self.versions
            .iter()
            .filter(|v| v.is_playable_on(backend))
            .collect()
    }

    /// Marks a version as verified and playable after successful checksum and launch validation.
    pub fn mark_verified(&mut self, version: &str, sha256: String, date: String) -> Result<(), String> {
        let entry = self
            .get_mut(version)
            .ok_or_else(|| format!("Version '{version}' not found in registry"))?;

        entry.installation_state = InstallationState::Installed;
        entry.verification_status = VerificationStatus::Verified { date, sha256 };
        entry.playable = true;
        Ok(())
    }

    /// Serializes registry to JSON format.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes registry from JSON format.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Loads the version registry from an instance/home directory.
    pub fn load_or_default(home: &Path) -> Self {
        let file = home.join("versions/version-registry.json");
        if file.is_file() {
            if let Ok(content) = std::fs::read_to_string(&file) {
                if let Ok(registry) = Self::from_json(&content) {
                    return registry;
                }
            }
        }
        Self::default()
    }

    /// Saves the registry state to disk.
    pub fn save(&self, home: &Path) -> std::io::Result<()> {
        let dir = home.join("versions");
        std::fs::create_dir_all(&dir)?;
        let json = self.to_json().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(dir.join("version-registry.json"), json)
    }
}

trait MutFind<T> {
    fn mut_find<P>(&mut self, predicate: P) -> Option<&mut T>
    where
        P: FnMut(&T) -> bool;
}

impl<T> MutFind<T> for [T] {
    fn mut_find<P>(&mut self, mut predicate: P) -> Option<&mut T>
    where
        P: FnMut(&T) -> bool,
    {
        for elem in self.iter_mut() {
            if predicate(elem) {
                return Some(elem);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unverified_version_is_never_playable() {
        let registry = VersionRegistry::default();
        let entry_26_3 = registry.get("26.3").expect("26.3 must exist");
        assert!(!entry_26_3.is_playable_on(Backend::HorizonRust));
        assert!(!entry_26_3.is_playable_on(Backend::JavaFabric));

        let entry_1_21_4 = registry.get("1.21.4").expect("1.21.4 must exist");
        assert!(!entry_1_21_4.is_playable_on(Backend::JavaFabric));
    }

    #[test]
    fn mark_verified_enables_playability() {
        let mut registry = VersionRegistry::default();
        registry
            .mark_verified("26.3", "dummy_sha256".into(), "2026-11-06".into())
            .expect("mark_verified should succeed");

        let entry = registry.get("26.3").unwrap();
        assert!(entry.is_playable_on(Backend::HorizonRust));
        assert!(entry.is_playable_on(Backend::JavaFabric));
    }

    #[test]
    fn backend_filtering_respects_compatibility() {
        let mut registry = VersionRegistry::default();
        // 1.21.4 only supports JavaFabric backend
        registry
            .mark_verified("1.21.4", "dummy_sha256".into(), "2026-11-06".into())
            .unwrap();

        let entry = registry.get("1.21.4").unwrap();
        assert!(entry.is_playable_on(Backend::JavaFabric));
        assert!(!entry.is_playable_on(Backend::HorizonRust));
    }

    #[test]
    fn json_serialization_round_trips() {
        let registry = VersionRegistry::default();
        let json = registry.to_json().expect("Serialization must succeed");
        let restored = VersionRegistry::from_json(&json).expect("Deserialization must succeed");
        assert_eq!(registry.versions.len(), restored.versions.len());
        assert_eq!(registry.default_version, restored.default_version);
    }
}
