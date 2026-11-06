# Version Registry Specification

Horizon Android features a unified **Version Registry** enabling the coexistence of the
native Rust engine (Minecraft Java 26.3) and the Java + Fabric runtime across multiple
game versions.

The implementation is located at:
- Rust: `crates/horizon-client/src/android/version_registry.rs`
- Tests: `tests/test_version_registry.py`

---

## 1. Core Principles & Invariants

1. **Explicit Playability Guarantee**: A version is **never** marked as playable until:
   - The version manifest, client JAR, and libraries have been downloaded from official endpoints.
   - Cryptographic checksums (SHA-1 / SHA-256) match the verified pins.
   - The execution backend has successfully booted and passed basic sanity checks.
2. **World Migration vs. Game-Version Execution**:
   - Upstream Horizon includes DataFixers capable of reading and upgrading world saves from 1.8 through 26.2.
   - World upgrade support **must not be confused with arbitrary game-version execution**.
   - Running an actual older client (e.g. 1.21.4) requires Backend B (Java + Fabric runtime).
3. **Backend Isolation**:
   - Backend A (Horizon Rust) and Backend B (Java + Fabric) store configurations, saves, and mods in separated directories.
   - Backend A: `profiles/profile-<n>/`
   - Backend B: `instances/fabric-<version>-profile-<n>/`

---

## 2. Version Matrix

| Minecraft Version | Protocol Version | Required Java Runtime | Backend Compatibility | Fabric Loader | Playable Baseline |
|---|---|---|---|---|---|
| **26.3** | 777 | OpenJDK 25 | Horizon Rust, Java + Fabric | 0.16.10+ | Horizon Rust Verified |
| **1.21.4** | 769 | OpenJDK 21 | Java + Fabric | 0.16.9+ | Target for Phase 2-3 |
| **1.21.3** | 768 | OpenJDK 21 | Java + Fabric | 0.16.9+ | Target for Phase 4 |
| **1.21.1** | 767 | OpenJDK 21 | Java + Fabric | 0.16.9+ | Target for Phase 4 (LTS) |
| **1.20.4** | 765 | OpenJDK 17 | Java + Fabric | 0.15.11+ | Target for Phase 4 |

---

## 3. Schema & Data Model

The version registry is serialized as JSON at `versions/version-registry.json`:

```json
{
  "default_version": "26.3",
  "default_backend": "horizon-rust",
  "versions": [
    {
      "minecraft_version": "26.3",
      "protocol_version": 777,
      "java_version": "java25",
      "manifest_url": "https://piston-meta.mojang.com/v1/packages/26.3/26.3.json",
      "manifest_sha1": "11f1aefe473eefba0af62e3286c33b692807b2eb",
      "resource_metadata": {
        "asset_index": "34",
        "client_jar_sha1": "e877b6a07acd633fb3bb475002175cec036e7b87",
        "server_jar_sha1": "33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c"
      },
      "backend_compatibility": [
        "horizon-rust",
        "java-fabric"
      ],
      "fabric_supported": true,
      "fabric_loader_version": "0.16.10",
      "mod_compatibility_notes": "Horizon Rust runs vanilla 26.3 and .hmod mods. Java backend runs 26.3 with Fabric Loader via OpenJDK 25.",
      "installation_state": {
        "status": "not-installed"
      },
      "verification_status": {
        "verification": "unverified"
      },
      "playable": false
    },
    {
      "minecraft_version": "1.21.4",
      "protocol_version": 769,
      "java_version": "java21",
      "manifest_url": "https://piston-meta.mojang.com/v1/packages/1.21.4/1.21.4.json",
      "manifest_sha1": null,
      "resource_metadata": {
        "asset_index": "19",
        "client_jar_sha1": null,
        "server_jar_sha1": null
      },
      "backend_compatibility": [
        "java-fabric"
      ],
      "fabric_supported": true,
      "fabric_loader_version": "0.16.9",
      "mod_compatibility_notes": "Java + Fabric runtime on OpenJDK 21 with Mesa Zink (OpenGL 3.3 Core).",
      "installation_state": {
        "status": "not-installed"
      },
      "verification_status": {
        "verification": "unverified"
      },
      "playable": false
    }
  ]
}
```

---

## 4. Lifecycle States

```
                [ NotInstalled ]
                       |
                       | (User initiates install)
                       v
                [ Installing ]
                       |
                       +------------------------+
                       |                        | (Checksum mismatch / error)
                       | (Checksums verified)   v
                       v                  [ Corrupted ]
                [ Installed ]
                       |
                       | (Launch verification test)
                       v
                 [ Verified ] (playable = true)
```

1. **NotInstalled**: Manifest metadata known; binaries absent.
2. **Installing**: Resource download in progress; reports percentage.
3. **Installed**: All JARs, libraries, and assets downloaded.
4. **Verified**: Cryptographic hashes validated; client successfully launches.
5. **Corrupted**: File modification or incomplete download detected. Requires re-verification.
