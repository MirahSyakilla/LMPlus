# il2cpp-bridge-rs

<a href="https://crates.io/crates/il2cpp-bridge-rs"><img src="https://img.shields.io/crates/v/il2cpp-bridge-rs?style=for-the-badge" alt="Crates.io"></a>
<a href="#supported-targets"><img src="https://img.shields.io/badge/platform-iOS%20%C2%B7%20macOS%20%C2%B7%20Linux%20%C2%B7%20Android%20%C2%B7%20Windows-222?style=for-the-badge&logoColor=white" alt="Platform"></a>

Poke at Unity's IL2CPP runtime from Rust. Resolve classes, call methods, dump metadata, grab Unity objects — without the usual pointer soup.

Built for native plugins, injected libraries, and tooling that already lives inside a running Unity process. Not a mod loader.

## Install

```bash
cargo add il2cpp-bridge-rs
```

## Quick look

```rust
use il2cpp_bridge_rs::{api, init};

init("GameAssembly", || {
    let player = api::cache::csharp()
        .class("PlayerController")
        .expect("class exists");

    let take_damage = player
        .method(("TakeDamage", ["System.Single"]))
        .expect("method exists");

    println!("{}::{} @ 0x{:X}", player.name, take_damage.name, take_damage.rva);
});
```

## Supported targets

iOS (`aarch64-apple-ios`) · macOS (`aarch64-apple-darwin`) · Linux (`x86_64-unknown-linux-gnu`) · Android (`aarch64-linux-android`) · Windows (`x86_64-pc-windows-msvc`)

## Docs

- [Getting Started](docs/getting-started.md)
- [Core Workflows](docs/core-workflows.md)
- [Platform Notes](docs/platform-support.md)
- [Architecture](docs/architecture.md)
- [API Map](docs/api-reference.md)

Or run `cargo doc --no-deps` for the real source of truth.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Disclaimer

For educational and research use. Learn about IL2CPP, reverse engineering, and native interop — responsibly.

## License

[MIT](LICENSE)
