# Incremental Rust compiler research

This project is at an early research stage. It is **not yet a compiler, Cargo replacement, or Blaze rewrite**. The current code is an in-memory invalidation planner and a small build-oracle smoke harness. Neither is connected to a compiler or production cache.

We aim for a safe, fast incremental toolchain with Cargo compatibility, checked against a pinned Rust compiler. The project code uses stable Rust. See [research](docs/research.md) and [architecture](docs/architecture.md) for scope and open work.

Run `cargo test --workspace --locked`, `cargo fmt --all --check`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`. The smoke harness is described in [its README](tools/build-oracle/README.md).
