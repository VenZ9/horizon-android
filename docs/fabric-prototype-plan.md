# Java + Fabric Runtime: Prototype & Implementation Plan

This document outlines the phased engineering roadmap for introducing the Java + Fabric
Runtime to Horizon Android while preserving the 26.3 Rust engine.

---

## Phase 0: Feasibility (Completed)
- Architecture and feasibility report produced: `docs/fabric-runtime-feasibility.md`
- Multi-version registry designed: `docs/version-registry.md`
- Automated CI build pipeline established: `.github/workflows/android-apk.yml`
- Initial test suites added: `tests/test_version_registry.py`

---

## Phase 1: Launcher Backend Abstraction
**Objective**: Introduce backend selection and isolated profiles in the native launcher.

### Deliverables:
1. **Launcher Settings UI Enhancement**:
   - Add **Execution Backend** selector to Settings page in `launcher_pages.rs`:
     - Option A: *Horizon Rust Engine (26.3)*
     - Option B: *Minecraft Java + Fabric Runtime*
   - Add **Version Picker** tied to the Multi-Version Registry.
2. **Profile & Instance Isolation**:
   - Backend A profiles stored under `profiles/profile-<n>/`.
   - Backend B instances stored under `instances/fabric-<version>-profile-<n>/`.
   - Completely separate `options.txt`, `saves/`, `resourcepacks/`, and `mods/`.
3. **Automated Tests**:
   - Profile collision avoidance tests.
   - Configuration persistence tests (`android-launcher.json`).

---

## Phase 2: Java Runtime Prototype
**Objective**: Prove that an actual Minecraft Java client runs on Android ARM64 and renders to the screen.

### Deliverables:
1. **Embedded OpenJDK Runtime**:
   - Stage OpenJDK 21/25 binaries for Android ARM64 (`aarch64-linux-android`) in internal storage.
   - Verify JNI invocation or subprocess execution with bionic linker compatibility.
2. **Graphics Integration (Mesa Zink)**:
   - Compile Mesa Zink for Android ARM64 with Vulkan WSI.
   - Bind desktop OpenGL 3.3/4.5 calls to an EGL surface attached to Android `ANativeWindow`.
3. **Input & Audio Bridge**:
   - Route touch motion events to GLFW synthetic cursor and button events.
   - Configure OpenAL Soft with AAudio backend.
4. **Validation Milestone**:
   - Vanilla Minecraft 1.21.4 boots to main menu on an ARM64 Android device.
   - Single-player world loads, renders terrain, and responds to touch controls.

---

## Phase 3: Fabric Runtime
**Objective**: Prove actual Fabric Loader and Mixin functionality with real Java mods.

### Deliverables:
1. **Fabric Loader Integration**:
   - Bootstrap `net.fabricmc.loader.impl.launch.knot.KnotClient`.
   - Configure classpath with Fabric Loader and intermediary mappings.
2. **Mod Discovery & Dependency Resolution**:
   - Parse `fabric.mod.json` inside JARs in `instances/<instance>/mods/`.
   - Validate environment flags and dependency checks (`depends`, `breaks`).
3. **Mixin Verification**:
   - Verify that SpongePowered Mixin tree transformers apply bytecode changes without crashing the JVM.
4. **Validation Milestone**:
   - Launch Minecraft with Fabric API and at least two functional Fabric mods:
     1. One mod requiring Fabric API (e.g. content or HUD mod).
     2. One mod utilizing Mixins (e.g. Lithium / optimization mod).

---

## Phase 4: Multi-Version Support
**Objective**: Support versions progressively from 26.3 backwards toward 1.21.4.

### Deliverables:
1. **26.3 Java + Fabric**:
   - Verify OpenJDK 25 execution on Android ARM64.
   - Validate Fabric Loader for 26.3 snapshots/release.
2. **Backwards Expansion**:
   - Add 1.21.4, 1.21.3, 1.21.1, and 1.20.4 to active registry.
   - Map per-version LWJGL and Java version requirements (Java 17 for 1.20.4, Java 21 for 1.21.x).
3. **Instance Manager**:
   - Support creating, cloning, and deleting separate instances per version.

---

## Phase 5: Integration, Optimization & Diagnostics
**Objective**: Production stabilization, thermal management, and crash diagnostics.

### Deliverables:
1. **Thermal & Memory Budgeting**:
   - Monitor device memory via `ComponentCallbacks2.onTrimMemory()`.
   - Dynamic heap tuning (`-Xmx`) based on physical device RAM.
2. **Unified Crash Reporter**:
   - Capture JVM crash logs (`hs_err_pid.log`) and Minecraft crash reports.
   - Present diagnostic summaries in launcher UI.
3. **Performance Benchmarking**:
   - Measure frame pacing and memory consumption on real Android hardware.
