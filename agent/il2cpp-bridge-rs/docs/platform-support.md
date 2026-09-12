# Platform and Runtime Notes

This page collects the platform-specific details and runtime caveats that matter when integrating `il2cpp-bridge-rs` into a live Unity process.

Use this page for environment guidance and failure analysis. Use generated rustdoc for detailed API information on the functions and types involved.

## Supported Targets

| Platform | Target triple | Status |
| --- | --- | --- |
| iOS | `aarch64-apple-ios` | Primary target |
| macOS | `aarch64-apple-darwin` | Supported |
| Linux | `x86_64-unknown-linux-gnu` | Supported |
| Android | `aarch64-linux-android` | Supported |
| Windows | `x86_64-pc-windows-msvc` | Supported |

These target triples come directly from the repository `Makefile`.

## Symbol Resolution

The runtime resolves IL2CPP exports dynamically, using platform-specific APIs under `memory::info::symbol` and `memory::info::image`.

| Platform family | Export lookup | Image base lookup |
| --- | --- | --- |
| macOS / iOS | `dlsym` | `dyld` APIs |
| Linux / Android | `dlsym` | `dl_iterate_phdr` |
| Windows | `GetProcAddress` | `GetModuleHandleA` |

## Choosing the Target Image

`init(target_image, ...)` needs the name of the loaded binary used to compute image base and method addresses.

Common values:

- iOS: `UnityFramework`
- many desktop Unity builds: `GameAssembly`
- custom environments: the module exporting the IL2CPP symbols you are resolving

If you get method RVA/VA values that look wrong, the first thing to question is the selected target image.

## Thread Attachment Expectations

The crate attaches the initialization worker thread automatically. Outside that path, you are responsible for attaching any thread that interacts with the IL2CPP runtime.

Use `api::Thread` when:

- dispatching work to your own background threads
- running delayed callbacks after initialization
- polling or invoking methods from an external event loop

If a thread is not attached, lookups and invocations may fail or behave unpredictably.

## Build Commands

Common targets from the `Makefile`:

```bash
make build
make build-release
make check
make clippy
make doc
```

Platform-specific builds:

```bash
make build-ios
make build-macos
make build-linux
make build-android
make build-windows
```

## Common Failure Modes

### Linux: IL2CPP symbols not found

On Linux, Unity may load `GameAssembly.so` with `RTLD_LOCAL`, which prevents `dlsym(RTLD_DEFAULT, ...)` from finding its exports. The crate handles this automatically by promoting the library to global visibility via `dlopen` with `RTLD_GLOBAL` before the first symbol lookup.

If automatic promotion fails (check log output), you can call `promote_library_to_global("GameAssembly")` before `init()` as a manual workaround, or use the raw `libc::dlopen` approach:

```rust
unsafe { libc::dlopen(b"GameAssembly.so\0".as_ptr().cast(), libc::RTLD_NOW | libc::RTLD_GLOBAL); }
```

### Initialization does not complete

Likely causes:

- IL2CPP exports could not be resolved
- the runtime domain was not ready yet
- cache initialization failed repeatedly

### Cache helpers panic

Helpers like `cache::csharp()` and `cache::coremodule()` expect a hydrated cache. If they panic, initialization probably did not complete or the expected assembly is absent.

### Method calls fail with argument or return errors

Likely causes:

- wrong overload selected
- wrong Rust type requested from `Method::call`
- reference-type vs value-type return shape mismatch
- missing bound instance for an instance method

### Object wrappers return null-related errors

Likely causes:

- underlying Unity object no longer exists
- wrapper method was called on the wrong thread
- the runtime returned a valid managed null even though the metadata lookup succeeded

## Dependency Notes

The platform split in `Cargo.toml` is:

- `libc` on Unix targets
- `mach2` on macOS and iOS
- `windows-sys` on Windows

Those dependencies are implementation details, but they matter when debugging platform-specific symbol or image-resolution issues.

If you need the exact signatures for runtime helpers such as `init`, `Thread`, cache accessors, or dump functions, check rustdoc.
