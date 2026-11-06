# Android CI & APK Build Automation

Horizon Android provides automated GitHub Actions workflows for continuous integration,
type checking, and Android APK packaging. The primary pipeline is defined in
`.github/workflows/android-apk.yml`.

## Workflows Overview

| Workflow | Path | Purpose | Triggers |
| --- | --- | --- | --- |
| **Android APK Build** | `.github/workflows/android-apk.yml` | Packages installable APKs (`arm64-v8a`, `x86_64`), generates SHA-256 checksums and build metadata. | Push to `android`, `fabric-runtime`, `multi-version`; pull requests to `android`; version tags (`v*`); manual dispatch |
| **Android source checks** | `.github/workflows/android-source.yml` | Validates public tree boundaries, runs Android unit tests, verifies workflow exports. | Push/PR to `android`; dispatch for native type-checking |
| **Android dev APKs** | `.github/workflows/android-dev.yml` | Maintainer-signed dev channel builds. Requires dedicated keystore secret. | Push to `android` by maintainers |

---

## Workflow Architecture (`android-apk.yml`)

### Triggers

- **Push**: Triggered automatically on pushes to branches:
  - `android` (base branch)
  - `fabric-runtime` (Java + Fabric runtime development)
  - `multi-version` (multi-version compatibility development)
- **Version Tags**: Triggered on `v*` tags (e.g. `v0.8.0`).
- **Pull Requests**: Pull requests targeting `android` run the build to prevent packaging regressions without leaking credentials.
- **Workflow Dispatch**: Can be triggered manually from the GitHub Actions web interface with custom parameter inputs.

### Build Matrix & Target Architecture

- **Primary Target**: Android ARM64 (`arm64-v8a`), mapped to Rust target `aarch64-linux-android`.
- **Secondary Target**: Android x86_64 (`x86_64`), mapped to Rust target `x86_64-linux-android`.
- **Minimum Android**: Android 9.0 (API level 28).
- **Compilation Toolchain**:
  - Rust 1.95.0 (pinned in `rust-toolchain.toml`)
  - Android NDK 28.2.13676358
  - Android SDK Platform 35 / Build Tools 35.0.0
  - Eclipse Temurin OpenJDK 17 (for Android Gradle Plugin 8.11.1 & build tools)
  - Eclipse Temurin OpenJDK 25 (for `horizon-prepare` vanilla data exporters)
  - Gradle 8.14

### Security & License Compliance

- **Minimal Permissions**: The workflow operates under `permissions: contents: read`.
- **Zero Secrets on PRs**: Pull request builds run completely self-contained without requiring repository secrets.
- **Signing Isolation**: Local iteration and CI builds utilize a dedicated development key generated locally via `keytool`. Release signing is isolated to tagged releases with verified secrets.
- **No Redistribution of Minecraft Assets**: In adherence to Mojang's EULA and guidelines, original Minecraft game JARs, assets, and touch textures are **never** bundled or redistributed via Git or CI artifacts. The public build generates internal data tables (`engine-data.zip`) from official Mojang endpoints during build preparation, and game textures/sounds are fetched directly by the client on the user's device.

---

## How to Trigger a Manual Build

1. Open the repository on GitHub: `https://github.com/VenZ9/horizon-android`.
2. Navigate to the **Actions** tab.
3. In the left sidebar, click **Android APK Build**.
4. Click the **Run workflow** dropdown on the right side.
5. Configure the inputs:
   - **Branch**: Select `fabric-runtime` or `android`.
   - **Target ABI**: Select `arm64-v8a` (default) or `x86_64`.
   - **Build Type**: Select `debug` (default) or `release`.
6. Click the green **Run workflow** button.

---

## Downloading and Verifying Artifacts

Once the workflow finishes:

1. Click on the completed workflow run.
2. Scroll to the **Artifacts** section at the bottom of the summary page.
3. Download the artifact ZIP, for example: `horizon-android-arm64-v8a-debug`.
4. Extract the ZIP file. The artifact bundle contains:
   - `horizon-android-arm64-v8a-debug.apk` (or `horizon-android-arm64-v8a-release.apk`)
   - `SHA256SUMS.txt`: Cryptographic SHA-256 hash.
   - `BUILD.json`: Comprehensive build provenance (commit SHA, branch, timestamp, ABI, size).
   - `LICENSE.txt` & `THIRD_PARTY_NOTICES.md`.

### Verifying Checksum

To verify the integrity of the downloaded APK:

```bash
sha256sum -c SHA256SUMS.txt
```

Expected output:
```text
horizon-android-arm64-v8a-debug.apk: OK
```

### Inspecting Build Provenance

`BUILD.json` records build metadata:
```json
{
  "commit": "150a414b09a18c900489d9462a7fc035dae744b7",
  "branch_or_tag": "fabric-runtime",
  "timestamp": "2026-11-06T12:00:00Z",
  "abi": "arm64-v8a",
  "build_type": "debug",
  "apk_file": "horizon-android-arm64-v8a-debug.apk",
  "sha256": "...",
  "size_bytes": 45123456,
  "run_id": "123456789",
  "run_number": "42"
}
```

---

## Reproducing the Build Locally

To run the exact commands executed by CI on a local Linux development workstation:

```bash
# 1. Install prerequisites
rustup toolchain install 1.95.0
rustup target add aarch64-linux-android

# 2. Check public boundaries and release audit
python3 tools/check_public_tree.py
python3 -m unittest discover -s tests -p 'test_android_release_audit.py'

# 3. Prepare private engine tables from official downloads
python3 tools/android_prepare.py --java-home "$JAVA_HOME_25"

# 4. Build native library and package APK
python3 tools/android_build.py --abi arm64-v8a \
  --sdk "$ANDROID_HOME" \
  --java-home "$JAVA_HOME_17" \
  --gradle "$(command -v gradle)" \
  --ndk-version 28.2.13676358 \
  --build-tools-version 35.0.0

# 5. Output APK is placed at:
# .local/android/apk/arm64-v8a/horizon-android.apk
```
