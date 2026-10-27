# Build oracle smoke harness

This is an initial testing tool, not a compiler or a rewrite of Blaze. It creates two isolated, dependency-free Cargo projects; builds and runs both after each function-body change; checks output and exit status; and records build wall time in microseconds. The first observation is cold. Subsequent ones are warm edits. An incorrect output, unexpected stderr, failed build, or failed launch aborts the run.

```
cargo run --release -- /absolute/path/to/cargo /absolute/path/to/blaze /tmp/new-oracle-run
```

The candidate must support `build --offline` and place its executable at Cargo's conventional debug output path. The working directory must not already exist; the tool never deletes it. Use a fresh, absolute directory. The harness inherits the calling environment and PATH. The two projects have different paths and build caches; results do not establish statistical performance or equivalence for real workloads. In particular, it does not wait for a daemon to become ready, measure memory or disk, capture fallback behavior, account for toolchain differences, or test the rest of Rust and Cargo. A serious comparison needs pinned revisions, equivalent compiler and linker settings, randomized paired samples, documented readiness and cache state, resource measurements, real projects and broader differential tests.

For a harness self-check, pass the same cargo executable twice. This does not benchmark Blaze.
