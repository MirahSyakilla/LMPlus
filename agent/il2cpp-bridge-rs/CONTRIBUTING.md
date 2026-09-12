# Contributing to `il2cpp-bridge-rs`

This crate sits close to raw IL2CPP APIs, so documentation quality and behavioral clarity matter as much as code changes. Treat every contribution as something another engineer will have to trust inside a live Unity process.

## Local Workflow

Clone the repository and validate changes before opening a PR:

```bash
git clone https://github.com/Batchhh/il2cpp-bridge-rs.git
cd il2cpp-bridge-rs
make build
make check
make clippy
```

If you touch rustdoc or public-facing markdown, also run:

```bash
make doc
```

Platform-specific build targets are available through the `Makefile` when you need to validate a specific target triple.

## What Good Changes Look Like

- Public API changes come with matching rustdoc updates and any required guide changes under `docs/`.
- Unsafe code is kept narrow and justified by the IL2CPP/runtime boundary it is handling.
- New wrappers follow existing project conventions instead of introducing a parallel style.
- Behavior changes are described in the PR clearly enough that users can understand migration impact.

## Public API and Wrapper Boundaries

Use these rough boundaries when deciding how to shape a change:

- Core API changes affect initialization, cache behavior, invocation, metadata hydration, memory access, or thread management.
- Wrapper changes build on the existing core APIs to make common Unity tasks easier without changing the lower-level runtime contract.
- Documentation-only changes are valid contributions when they improve correctness, onboarding, or explain runtime caveats more clearly.

If a change modifies exported behavior, update both the item rustdoc and the relevant guide page. Do not rely on README-only documentation for anything users need in day-to-day integration work.

## Documentation Expectations

The documentation strategy for this repository is:

- `README.md` explains what the crate is, who it is for, and how to get to a first successful flow.
- `docs/*.md` explain workflows, platform/runtime caveats, and contributor-relevant architecture.
- rustdoc is the canonical source for signatures and per-item semantics.

If you add or change a public function, type, or workflow, update the layer that owns it.

## Pull Requests

Before opening a PR:

1. Make sure the crate builds and checks cleanly on at least one relevant target.
2. Update docs for any public API, workflow, or behavior change.
3. Keep the PR focused. Separate wrapper additions, internal refactors, and unrelated doc rewrites unless they must land together.
4. Explain runtime assumptions, safety considerations, and validation steps in the PR description.

## Issue Reports

Use the GitHub issue templates for:

- bugs and regressions
- feature requests
- questions about usage or runtime behavior

When reporting runtime issues, include the target platform, the Unity/IL2CPP environment shape if known, and the failing API or workflow.

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
