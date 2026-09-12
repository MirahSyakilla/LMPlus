# Core Workflows

This page covers the workflows most adopters need after initialization: metadata lookup, method invocation, field and property access, Unity object access, and metadata dumping.

Use this page for patterns and caveats. Use generated rustdoc for detailed API information on the types and functions mentioned here.

## Find Assemblies and Classes

Start with the cache:

```rust
use il2cpp_bridge_rs::api::cache;

let game = cache::csharp();
let player = game.class("PlayerController").expect("class should exist");
```

`Assembly::class` accepts either:

- `"Namespace.TypeName"`
- `"TypeName"` for the global namespace

Use `cache::assembly("Name")` for non-standard assemblies. The helper accepts names with or without the `.dll` suffix.

## Select Methods Safely

Use the narrowest selector that avoids ambiguity.

```rust
let by_name = player.method("Update");
let by_types = player.method(("TakeDamage", ["System.Single"]));
let by_count = player.method(("TakeDamage", 1));
```

Recommended practice:

- use name-only selectors for methods that are known not to be overloaded
- use parameter type selectors for stable overload resolution
- use parameter count selectors only when type names are inconvenient but overload count is still unambiguous

## Call Static vs Instance Methods

For static methods:

```rust
use std::ffi::c_void;

let factory = player.method("GetInstance").expect("static method should exist");
unsafe {
    let _instance_ptr: Result<*mut c_void, _> = factory.call(&[]);
}
```

For instance methods, bind through `Object::method` whenever possible:

```rust
use std::ffi::c_void;

if let Some(obj) = player.find_objects_of_type(false).into_iter().next() {
    let take_damage = obj
        .method(("TakeDamage", ["System.Single"]))
        .expect("instance method should exist");

    let amount: f32 = 25.0;
    unsafe {
        let _: Result<(), _> = take_damage.call(&[
            &amount as *const f32 as *mut c_void,
        ]);
    }
}
```

`Method::call` handles:

- static vs instance dispatch
- argument count checks
- reference-type vs value-type return handling
- managed exception conversion into `Err(String)`

## Read Fields and Properties

For fields:

```rust
if let Some(obj) = player.find_objects_of_type(false).into_iter().next() {
    let health = obj.field("health").expect("field should exist");
    unsafe {
        let current: f32 = health.get_value().expect("field read should succeed");
        println!("health = {}", current);
    }
}
```

For properties:

```rust
if let Some(obj) = player.find_objects_of_type(false).into_iter().next() {
    let enabled = obj.property("enabled").expect("property should exist");
    unsafe {
        let is_enabled: bool = enabled.get_value().expect("getter should succeed");
        println!("enabled = {}", is_enabled);
    }
}
```

Instance fields and properties require an instance pointer. `Object::field` and `Object::property` provide that automatically.

## Work with Unity Objects

`GameObject` and related wrappers sit on top of the metadata/object layer and are useful once you already have a live runtime object.

```rust
use il2cpp_bridge_rs::structs::GameObject;

if let Ok(player_go) = GameObject::find("Player") {
    let transform = player_go.get_transform().expect("transform should exist");
    println!("Found player object: {:?}", transform);
}
```

Use the wrapper layer when it makes common Unity operations clearer. Drop back to `Object`, `Class`, and `Method` when you need raw flexibility.

## Dump Metadata

The dump helpers generate C#-like pseudo-code from the hydrated cache.

```rust
use il2cpp_bridge_rs::api;

let _dump_dir = api::dump();
let _single_file = api::dump_all();
let _custom_dir = api::dump_to("/tmp/il2cpp_dump");
let _custom_file = api::dump_all_to("/tmp/il2cpp_dump");
```

These functions are useful for:

- offline analysis
- verifying cache hydration
- exploring unfamiliar assemblies
- building higher-level tooling on top of emitted pseudo-code

## Common Failure Shapes

Most workflow failures come from one of these conditions:

- `init` has not completed yet
- the wrong target image name was supplied
- the current thread is not attached to the IL2CPP VM
- a method overload was selected too loosely
- a field or property was accessed without a bound instance
- the expected Rust return type does not match the managed return shape

If you see one of those failure modes, resolve it before adding more wrapper code around the problem.

For detailed API information on `Assembly`, `Class`, `Method`, `Object`, `GameObject`, and related types, refer to rustdoc rather than treating this guide as the reference layer.
