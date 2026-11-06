# Horizon Android — Phase 0: Feasibility & Architecture Report
## Multi-Version Execution & Java + Fabric Runtime on Android ARM64

**Author:** Horizon Android Engineering Team  
**Branch:** `fabric-runtime`  
**Base:** `android` (Horizon 0.7.7 / Minecraft Java 26.3 Baseline)  
**Target Architecture:** Android ARM64 (`aarch64-linux-android` / `arm64-v8a`)  
**Status:** Approved Feasibility & Architectural Specification

---

## 1. Executive Summary

Horizon Android currently provides a clean-room Rust implementation of Minecraft Java 26.3 built atop `winit`, `wgpu` (Vulkan/GLES), and sandboxed Wasmtime `.hmod` mods. While this architecture achieves high performance and low memory overhead on Android devices, it cannot execute standard Java bytecode JARs, Fabric Loader, SpongePowered Mixins, or the vast ecosystem of Minecraft Java mods.

This report establishes the technical foundation for a **Dual-Backend Architecture**:
1. **Backend A (Horizon Rust)**: Retains the existing 26.3 Rust engine, NativeActivity, low-overhead Vulkan pipeline, and sandboxed `.hmod` runtime.
2. **Backend B (Minecraft Java + Fabric Runtime)**: Introduces an Android-compatible OpenJDK execution environment capable of launching actual Minecraft Java Edition clients, loading Fabric Loader and Fabric API, and applying runtime Mixins on ARM64 devices.

This document details the runtime architecture, JVM execution options, graphics translation layers, input/audio bridges, lifecycle handling, strict licensing boundaries, memory budgets, and a backwards multi-version strategy spanning from version 26.3 down to 1.21.4.

---

## 2. Existing Engine Architecture Audit

The existing Horizon Android codebase contains the following architectural components:

```
+-----------------------------------------------------------------------+
|                         Android OS (API 28+)                          |
+-----------------------------------------------------------------------+
|  HorizonActivity (NativeActivity)  |  HorizonDocumentsProvider (SAF)  |
+------------------------------------+----------------------------------+
|                    JNI Platform Bridge (Rust)                         |
|   (AAudio, ClipboardManager, Native IME, Android Keystore, Battery)   |
+-----------------------------------------------------------------------+
|                    Horizon Launcher (Vulkan / GLES)                   |
|       (Page::Home, Page::Settings, Page::Mods, Page::Packs, etc.)     |
+-----------------------------------------------------------------------+
|                     Backend A: Horizon Rust Engine                    |
|   - crates/horizon-client (Gameplay, Net, Screens, Integrated Server) |
|   - crates/horizon-client-render (wgpu Vulkan / GLES batch renderer)  |
|   - crates/horizon-world (Worldgen, Level, Ticks, Redstone, Physics)  |
|   - crates/horizon-mods (Wasmtime .hmod sandboxed native runtime)     |
+-----------------------------------------------------------------------+
```

### Key Characteristics:
- **Single Native Library**: The entire application is compiled into `libhorizon_android.so`, loaded by `dev.horizon.android.HorizonActivity`.
- **Render Loop**: Driven by `winit` 0.30 (`android-native-activity`) and `wgpu` 30.0.1.
- **Resources**: Generates private engine tables (`engine-data.zip`) during build preparation (`tools/android_prepare.py`). Original textures and audio are downloaded directly on-device from official Mojang endpoints.
- **Mods**: Only content mods (`.hmod` ZIPs containing JSON/NBT and assets) are executed on Android via Wasmtime. Desktop native mods (`.hnmod`) and Java JAR mods are completely unsupported.

---

## 3. Dual-Backend Architecture Design

To support genuine Java Minecraft and Fabric mods without destabilizing the optimized Rust engine, the launcher must decouple the game presentation from the execution backend.

```
                              [ Unified Launcher UI ]
                                         |
                       +-----------------+-----------------+
                       |                                   |
              [ Select Backend A ]                [ Select Backend B ]
                       |                                   |
             Horizon Rust Backend                Java + Fabric Backend
                       |                                   |
         +-------------+-------------+       +-------------+-------------+
         | Version: 26.3 Pinned      |       | Multi-Version: 26.3->1.21.4|
         | Rendering: wgpu / Vulkan  |       | Rendering: Zink / LWJGL3  |
         | Mods: .hmod (Wasmtime)    |       | Mods: Fabric Loader+Mixin |
         | State: Horizon Profiles   |       | State: Isolated Instances |
         +---------------------------+       +---------------------------+
```

### Shared Launcher Services:
- **Account System**: Shares Microsoft OAuth 2.0 / Xbox Live tokens securely stored in Android Keystore (AES-GCM). Tokens are passed as session files to both backends.
- **Virtual Touch Controls**: The four existing touch schemes (Joystick/D-pad + Tap/Crosshair) and customizable HUD buttons are rendered by the launcher overlay and translated into backend-specific input events.
- **Instance Isolation**:
  - Backend A stores profiles under `~/.local/share/horizon/profiles/<profile>/`.
  - Backend B stores Java instances under `~/.local/share/horizon/instances/<instance>/` (standard `.minecraft` layout containing `mods/`, `config/`, `saves/`, `resourcepacks/`).
  - Worlds and configurations are never cross-contaminated.

---

## 4. Android JVM Options & Execution Environment

### 4.1 The ART/Dalvik vs. OpenJDK Dilemma
Android's standard runtime (ART) cannot run Minecraft Java Edition or Fabric for three fundamental reasons:
1. **DEX vs. JVM Bytecode**: ART executes DEX bytecode compiled ahead-of-time or via JIT. Minecraft and Fabric mods are distributed as JVM `.class` bytecode inside JARs.
2. **Missing Desktop Class Libraries**: Android's `core-oj` lacks desktop standard packages (`java.awt.*`, `javax.swing.*`, `javax.sound.*`, `java.lang.management.*`).
3. **Runtime Bytecode Manipulation**: Fabric Loader and SpongePowered Mixin rely on runtime class transformation (via OW2 ASM) and custom `ClassLoader` implementations (`KnotClassLoader`), which invoke `ClassLoader.defineClass(byte[], int, int)`. ART prohibits dynamic definition of JVM bytecode at runtime.

### 4.2 Evaluated OpenJDK Implementations

| Solution | Architecture | Android Compatibility | Licensing | Verdict |
|---|---|---|---|---|
| **Android ART + Dexing** | Mobile DEX | Incompatible with Mixin/ASM; lacks desktop APIs | Apache 2.0 | **Rejected** |
| **Avian / JamVM / TinyVM** | Embedded JVM | Lacks modern Java 17/21/25 features, MethodHandles, and invokeDynamic | BSD / GPL | **Rejected** |
| **GraalVM Native Image** | AOT Binary | Cannot support dynamic mod loading or runtime bytecode injection | GPLv2+CE / UPL | **Rejected** |
| **Mobile OpenJDK (Bionic)** | HotSpot aarch64 | Full Java 17/21/25 support; compiled directly for Android Bionic C library | GPLv2 + Classpath Exception | **Selected** |

### 4.3 JVM Invocation Strategy: JNI vs. Subprocess Exec
Two execution models exist for running HotSpot on Android:

1. **In-Process via `JNI_CreateJavaVM`**:
   - `libjvm.so` is loaded into `libhorizon_android.so` via `dlopen`.
   - The VM runs in the same process space as the NativeActivity.
   - *Risk*: A JVM crash or `SIGSEGV` terminates the entire Android process, preventing crash report generation or clean launcher return. Android 10+ linker namespaces restrict loading non-public system libraries.

2. **Process-Isolated Execution (`fork` / `execve`)**:
   - The launcher spawns an isolated `java` executable located in the app's native library or internal binaries directory (`/data/data/dev.horizon.android/files/runtime/bin/java`).
   - Communication occurs via Unix domain sockets (IPC) and Android `Surface` / `ANativeWindow` passing.
   - *Advantage*: Crash isolation, independent memory management, and ability to restart the JVM without killing the Android activity lifecycle.
   - **Recommended Approach**: Process-isolated execution with an IPC command bridge connecting virtual touch input and audio.

---

## 5. Graphics Compatibility & Rendering Pipeline

### 5.1 The Desktop OpenGL Barrier
- Minecraft Java Edition requires desktop OpenGL:
  - 1.21.4 requires **OpenGL 3.3 Core Profile**.
  - 26.3 requires **OpenGL 4.5 Core Profile** (or Vulkan backend).
- Android GPU hardware (Qualcomm Adreno, ARM Mali, Imagination PowerVR) exposes only **OpenGL ES 3.2** and **Vulkan 1.1 / 1.2 / 1.3**. Desktop GL function pointers (e.g. `glGenVertexArrays`, `glBindBufferRange`, GLSL 330/450 shaders) do not exist natively in mobile vendor drivers.

### 5.2 Translation Architecture: Mesa Zink vs. GL4ES

```
+-------------------------------------------------------------------+
|               Minecraft Java Client (Render Thread)               |
+-------------------------------------------------------------------+
|                LWJGL 3 (org.lwjgl.opengl.GL33)                    |
+-------------------------------------------------------------------+
|                           Mesa / Zink                             |
|          (Desktop OpenGL 4.6 Core -> Vulkan 1.2 API Translation)  |
+-------------------------------------------------------------------+
|                     Custom Android EGL / WSI                      |
|                  (Binds to Android ANativeWindow)                 |
+-------------------------------------------------------------------+
|                    Vulkan Device Driver (libvulkan.so)            |
|                   (Adreno / Mali / Turnip driver)                 |
+-------------------------------------------------------------------+
```

1. **Mesa Zink (Vulkan-based Desktop GL)**:
   - Implements full desktop OpenGL 4.6 on top of Vulkan using SPIR-V shader translation.
   - Fully compatible with modern Minecraft (1.21.4 through 26.3) and modern shader mods (Sodium / Iris / Lithium).
   - *Performance*: Fast on modern Vulkan 1.2+ hardware.
   - *Licensing*: MIT License (compatible with Horizon).

2. **GL4ES (OpenGL ES Translation)**:
   - Translates OpenGL 1.x/2.1 calls to OpenGL ES 2.0/3.0.
   - Excellent for legacy versions (1.8 - 1.16), but incomplete for OpenGL 3.3+ Core Profiles required by modern Minecraft versions.

**Conclusion**: Zink is the required rendering driver for modern Minecraft (1.21.4+), with an EGL-based surface bridge bound to Android's `ANativeWindow`.

---

## 6. Fabric Loader & Mixin Feasibility

### 6.1 Fabric Bootstrap Mechanism
Fabric Loader operates as a meta-launcher:
1. **Entry Point**: `net.fabricmc.loader.impl.launch.knot.KnotClient`.
2. **KnotClassLoader**: Custom classloader inheriting from `URLClassLoader`. Intercepts every `.class` loading request.
3. **Mixin Transformation**: Passes class byte arrays through SpongePowered Mixin tree transformers before defining the class in the VM.
4. **Mod Discovery**: Reads `fabric.mod.json` inside each JAR in `mods/`. Resolves environment (`*`, `client`, `server`) and dependency graphs (`depends`, `recommends`, `breaks`).

### 6.2 ARM64 & Android Feasibility
- **Pure Java**: Fabric Loader and Mixin are written entirely in platform-agnostic Java.
- **HotSpot Compatibility**: As long as the underlying VM is a standard OpenJDK HotSpot implementation with full reflection and Unsafe capabilities, KnotClassLoader and Mixin function identically on ARM64 Linux as on desktop x86_64.
- **Verification Requirement**: A test suite must verify:
  1. Bootstrapping KnotClient on Android.
  2. Mod dependency resolution using official Fabric JSON schemas.
  3. Transformation of Minecraft bytecode via a sample Mixin mod.

---

## 7. Input, Touch & Audio Systems

### 7.1 Input Translation (Touch to GLFW)
Minecraft Java listens for input via GLFW callbacks:
- `glfwSetKeyCallback`
- `glfwSetMouseButtonCallback`
- `glfwSetCursorPosCallback`
- `glfwSetScrollCallback`

**Android Bridge**:
The launcher's virtual touch controls capture motion events from `NativeActivity` and write synthetic GLFW event packets to a shared memory ring-buffer:
- Joystick / D-pad -> Synthetic KeyPress (`W`, `A`, `S`, `D`) or analog player movement packets.
- Screen Drag -> Delta camera movement fed into `glfwSetCursorPosCallback`.
- Tap / Hold -> Synthetic MouseButton (`GLFW_MOUSE_BUTTON_LEFT`, `GLFW_MOUSE_BUTTON_RIGHT`).
- Text Input -> Android IME JNI bridge injecting UTF-32 characters via `glfwSetCharCallback`.

### 7.2 Audio Subsystem
- Minecraft Java uses OpenAL Soft for 3D positional audio.
- OpenAL Soft supports multiple backends on Android:
  - OpenSL ES (available on all Android versions).
  - AAudio (low-latency audio backend for Android 8.1+ / API 27+).
- OpenAL Soft is dynamically configured via `alsoft.conf` to direct output to AAudio.

---

## 8. Memory, Storage & Android Lifecycle

### 8.1 Memory Footprint & Low Memory Killer (LMK)

| Component | Minimum RAM | Recommended RAM | Mitigation / Tuning |
|---|---|---|---|
| Android OS & System UI | 1.0 GB | 1.5 GB | Background service suppression |
| Horizon Launcher | 100 MB | 150 MB | Releases renderer assets during game launch |
| OpenJDK HotSpot JVM | 300 MB | 500 MB | Compressed oops (`-XX:+UseCompressedOops`), G1GC |
| Minecraft 1.21.4 Client | 1.5 GB | 2.5 GB | Chunk render distance capped to 8-12 on mobile |
| Fabric API + 10 Mods | 300 MB | 500 MB | Explicit heap sizing (`-Xmx2560M -Xms1024M`) |
| **Total System RAM** | **3.2 GB** | **5.1 GB** | Devices with < 4GB RAM require low-memory flags |

### 8.2 Storage & Scoped Storage Compliance
- Android 11+ enforces Scoped Storage. Code execution (`execve`) and dynamic library loading (`dlopen`) are strictly forbidden on external storage (`/sdcard` or Storage Access Framework volumes).
- **Architecture**:
  - The JVM binary, native libraries (`libzink.so`, `liblwjgl.so`), and game assets reside in app-private internal storage: `/data/data/dev.horizon.android/files/`.
  - World imports/exports and screenshot galleries interface with external storage through the existing `HorizonDocumentsProvider` (SAF).

### 8.3 Lifecycle & Background Transitions
- When the user presses Home or switches apps:
  1. `onPause` / `onStop` triggers surface destruction.
  2. The Java render thread must pause Vulkan presentation and wait on a condition variable to avoid Vulkan `VK_ERROR_DEVICE_LOST`.
  3. Single-player worlds pause their internal server loop.
  4. Audio mixer (`AAudioStream`) is paused to prevent audio stutter or battery drain.

---

## 9. Intellectual Property & Licensing Compliance

In strict compliance with `CONTRIBUTING.md` and repository guidelines:

```
+----------------------------+-----------------------+------------------------+
| Component                  | Upstream License      | Permissibility Status   |
+----------------------------+-----------------------+------------------------+
| Horizon Android (Fork)     | MIT                   | Primary Project License|
| OpenJDK Runtime            | GPLv2 + Classpath Exc.| Permitted (Runtime Exc)|
| Mesa Zink / Gallium        | MIT                   | Permitted              |
| LWJGL 3                    | BSD-3-Clause          | Permitted              |
| Fabric Loader              | Apache-2.0            | Permitted              |
| Fabric API                 | Apache-2.0            | Permitted              |
| SpongePowered Mixin        | MIT                   | Permitted              |
| OW2 ASM                    | BSD-3-Clause          | Permitted              |
| PojavLauncher Codebase     | GPL-3.0               | PROHIBITED (Off-limits)|
| Iris / Sodium (some forks) | LGPL-3.0 / PolyForm   | PROHIBITED (Off-limits)|
+----------------------------+-----------------------+------------------------+
```

> [!CAUTION]
> Under no circumstances may GPL-3.0 or LGPL code from third-party mobile wrappers be copied, ported, or adapted into this repository. All integration shims, GLFW translation layers, and IPC protocols must be written clean-room under MIT / Apache-2.0 terms.

---

## 10. Technical Blockers & Risk Mitigation

1. **SELinux W^X Protections on Android 14+**:
   - *Risk*: Android restricts `mprotect` memory from being simultaneously writable and executable, which HotSpot's JIT compiler requires.
   - *Mitigation*: Ensure target SDK and manifest configure standard isolated execution permissions, or provide fallback flags (`-Xint` interpreter mode / dual-mapping pages).
2. **GPU Driver Bugs in Mobile Vulkan**:
   - *Risk*: Certain older Mali GPU drivers exhibit Vulkan spirv validation errors in Zink.
   - *Mitigation*: Diagnostic detection of GPU hardware renderer with fallback profile options in launcher settings.
3. **Process Termination by Android LMK**:
   - *Risk*: Heavy modpacks exceeding available RAM trigger immediate SIGKILL without saving world state.
   - *Mitigation*: Implement aggressive auto-saving, memory warning listeners via `ComponentCallbacks2.onTrimMemory()`, and low-memory warnings in the launcher before launch.

---

## 11. Multi-Version Roadmap Strategy

The version progression commences with 26.3 and moves backwards to 1.21.4:

```
+-----------------------------------------------------------------------------+
| Stage 1: Minecraft 26.3 (Protocol 777, Java 25)                             |
| - Verify OpenJDK 25 runtime execution on ARM64                              |
| - Validate Vanilla 26.3 client startup & world load                         |
| - Validate Fabric Loader for 26.3 snapshot / release                        |
+-----------------------------------------------------------------------------+
                                      |
                                      v
+-----------------------------------------------------------------------------+
| Stage 2: Minecraft 1.21.4 (Protocol 769, Java 21)                           |
| - Add OpenJDK 21 runtime support                                            |
| - Integrate Fabric Loader 0.16+ and Fabric API for 1.21.4                   |
| - Validate Mixin mods (e.g. Lithium, performance mods, content mods)        |
+-----------------------------------------------------------------------------+
                                      |
                                      v
+-----------------------------------------------------------------------------+
| Stage 3: Backwards Extension (1.21.0 -> 1.20.4)                             |
| - Progressively verify older LTS targets                                    |
| - Map version-specific JVM requirements and LWJGL native builds             |
+-----------------------------------------------------------------------------+
```
