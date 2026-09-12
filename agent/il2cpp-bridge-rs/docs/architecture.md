# Architecture

This crate is intentionally layered so most users can stay in cache/object/wrapper APIs while contributors can reason about the lower-level runtime boundary.

This page explains how the crate is organized. Use generated rustdoc for detailed API information on the public modules, structs, and methods referenced here.

## Layered View

```text
init
  -> runtime symbol loading
  -> thread attachment
  -> assembly cache load
  -> class hydration
  -> user callbacks

api
  -> raw IL2CPP bindings
  -> cache helpers
  -> method invocation
  -> thread management
  -> dump helpers
  -> small Unity wrappers

structs
  -> metadata wrappers (Assembly, Class, Method, Field, Property, Type)
  -> object wrappers (Object, GameObject, Transform, MonoBehaviour, ...)
  -> collection and math helpers

memory
  -> symbol resolution
  -> image base lookup
  -> raw memory read/write
```

## Initialization Flow

`init` is the front door for the crate.

During initialization the crate:

1. resolves exported IL2CPP functions
2. loads the FFI function table
3. attaches the worker thread to the IL2CPP VM
4. enumerates assemblies into a global cache
5. hydrates classes, methods, fields, and properties
6. runs queued callbacks

If symbol resolution or cache initialization fails, the state resets so initialization can be attempted again.

## Cache Design

The cache exists to make repeated metadata lookup cheap and predictable.

- assemblies are stored as `Arc<Assembly>`
- classes are stored in a global lookup map
- methods are indexed for fast reuse
- class hydration is guarded so it only runs once per successful initialization cycle

For most users, `api::cache` is the only cache surface they should touch directly.

## Invocation Model

Invocation is split across two layers:

- `api::invoke_method` is the raw pointer-level helper
- `Method::call<T>` is the normal user-facing path

`Method::call<T>` validates the argument count, handles instance binding, differentiates reference vs value-type returns, and converts managed exceptions into `Err(String)`.

## Thread Model

IL2CPP is thread-sensitive. The crate exposes `api::Thread` as the explicit mechanism for attaching and detaching threads outside the initialization path.

This is a design choice, not just a convenience wrapper. The thread boundary is one of the main correctness constraints in the entire crate.

## Wrapper Philosophy

Wrappers under `api::wrappers` and `structs::components` exist to make common Unity tasks less repetitive:

- `Application` and `Time` expose familiar Unity concepts
- `GameObject`, `Transform`, `MonoBehaviour`, and related types layer on top of `Object` and `Method`

These wrappers should stay thin. If a behavior belongs to core runtime or metadata semantics, it should live in the lower layer first.

When you need exact signatures or item-level semantics for any layer described above, consult rustdoc rather than this architecture overview.
