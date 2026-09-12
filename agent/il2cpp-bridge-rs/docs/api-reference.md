# API Map

This page is a navigation aid, not a duplicate of rustdoc. Use generated rustdoc for exact signatures, return shapes, and item-level semantics.

## Start Here

These are the main entry points for most integrations:

- `init`
- `api::cache`
- `api::Thread`
- `api::invoke_method`
- dump helpers under `api`
- wrappers such as `api::Application` and `api::Time`
- metadata and object types under `structs`

## Initialization and Runtime Access

- `init(target_image, callback)` initializes symbol loading and cache hydration
- `api::Thread` manages IL2CPP thread attachment outside the initialization worker
- `api::Internals` resolves or registers internal calls

## Cache and Metadata

Use `api::cache` to reach hydrated assemblies and metadata:

- `assembly(name)`
- `csharp()`
- `mscorlib()`
- `coremodule()`
- `class_from_ptr(ptr)`
- `method_from_ptr(ptr)`
- `ensure_hydrated()`

Core metadata/object wrappers live under `structs`:

- `Assembly`
- `Class`
- `Method`
- `Field`
- `Property`
- `Object`
- `Type`
- `Image`

## Invocation and Object Access

The common call flow is:

1. find an `Assembly`
2. resolve a `Class`
3. resolve a `Method`
4. call through `Method::call<T>` or `api::invoke_method`

For object-specific access:

- use `Object::method` for instance-bound method lookup
- use `Object::field` for instance-bound field access
- use `Object::property` for instance-bound property access

## Unity-Oriented Wrappers

Wrappers help with common Unity tasks without changing the core runtime model:

- `api::Application`
- `api::Time`
- `GameObject`
- `Transform`
- `MonoBehaviour`
- rendering, physics, animation, and scene wrappers under `structs::components`

## Dump Helpers

The pseudo-code dump helpers are under `api`:

- `dump`
- `dump_all`
- `dump_to`
- `dump_all_to`
- `dump_assembly`

## Canonical Reference

Build the generated docs locally with:

```bash
cargo doc --no-deps
```

Use rustdoc as the canonical reference layer. Use the markdown guides for workflow explanations, caveats, and integration strategy.
