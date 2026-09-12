# Getting Started

This guide is for the first integration pass: loading the crate inside a process that already has Unity's IL2CPP runtime available, initializing the cache, and reaching the point where metadata lookups and method calls are reliable.

Use this page for the workflow. Use generated rustdoc for detailed API information, exact signatures, and per-item behavior.

## 1. Add the Crate

```bash
cargo add il2cpp-bridge-rs
```

For local development of this repository, the common commands are:

```bash
make build
make check
make clippy
make doc
```

## 2. Understand the Runtime Model

`init(target_image, callback)` is the required entry point.

On the first call, the crate:

1. resolves IL2CPP exports
2. loads the function pointer table
3. attaches the worker thread to the IL2CPP VM
4. loads assemblies into the cache
5. hydrates class metadata
6. runs queued callbacks

If `init` is called again while initialization is still running, the callback is queued. If it is called after initialization has completed, the callback is executed immediately on a newly spawned thread.

If initialization fails, the state falls back to idle and no queued callback is run.

## 3. Choose the Correct Target Image

`target_image` is the loaded binary used to compute runtime image base and RVA/VA information.

Common values:

| Environment | Common image name |
| --- | --- |
| iOS Unity app | `UnityFramework` |
| Desktop Unity player | `GameAssembly` |
| Custom embedding/setup | The module exporting IL2CPP symbols |

If you are unsure, inspect the loaded process modules and identify the image that contains the IL2CPP exports you want to resolve against.

## 4. Initialize Once

```rust
use il2cpp_bridge_rs::{api, init};

init("GameAssembly", || {
    let asm = api::cache::csharp();
    println!("Assembly loaded: {}", asm.name);
});
```

This is runtime-dependent code. It is meant to show usage shape, not to run outside a live Unity IL2CPP process.

## 5. Start with Cache-Backed Lookups

Once initialization has completed, use `api::cache` helpers to reach the assemblies you care about:

```rust
use il2cpp_bridge_rs::api::cache;

let game = cache::csharp();
let core = cache::coremodule();
let corlib = cache::mscorlib();
let ui = cache::assembly("UnityEngine.UI");
```

The cache is the normal entry point for metadata discovery. Avoid reimplementing assembly enumeration unless you are working on internals.

## 6. Resolve a Class and Method

```rust
let player = game
    .class("PlayerController")
    .expect("class should exist");

let heal = player
    .method(("Heal", ["System.Single"]))
    .expect("Heal(float) should exist");

println!("{}::{} -> RVA 0x{:X}", player.name, heal.name, heal.rva);
```

Method selectors support:

- `"Name"` for simple lookup
- `("Name", ["TypeA", "TypeB"])` for overload disambiguation by parameter type
- `("Name", 2)` for overload disambiguation by parameter count

## 7. Call Methods Carefully

For static methods, call the method directly from the class.

For instance methods, prefer `Object::method(...)` so the returned `Method` already has its `instance` pointer bound.

```rust
use std::ffi::c_void;

if let Some(player_obj) = player.find_objects_of_type(false).into_iter().next() {
    let take_damage = player_obj
        .method(("TakeDamage", ["System.Single"]))
        .expect("instance method should exist");

    let amount: f32 = 10.0;
    unsafe {
        let _: Result<(), _> = take_damage.call(&[
            &amount as *const f32 as *mut c_void,
        ]);
    }
}
```

Managed exceptions are converted into `Err(String)`. Argument count and return-shape mismatches are also validated.

## 8. Know When to Attach Threads

The initialization worker attaches itself to the IL2CPP VM. Outside that flow, you are responsible for thread attachment when doing runtime work on other threads.

Use `api::Thread::attach(true)` for scoped attachment:

```rust
use il2cpp_bridge_rs::api::Thread;

let _thread = Thread::attach(true);
```

If you are building a background worker or callback system around this crate, thread attachment is a first-order concern, not an optional cleanup detail.

## 9. Move On to the Workflow Guides

After you can initialize reliably, the next pages to read are:

- [Core Workflows](core-workflows.md)
- [Platform and Runtime Notes](platform-support.md)
- [API Map](api-reference.md)

For detailed API information while following those guides, build and open rustdoc with `cargo doc --no-deps`.
