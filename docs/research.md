# Initial research and engineering record

Date: 2026-10-27. This is preliminary, not a completed codebase audit.

## Reference

Blaze checkout: https://github.com/indexable-inc/blaze, commit `1bd553f2e3334e17541e4a0cf239a89fe1504d9d`. Inspected README, architecture, benchmark, no-rustc and harness documentation, workspace manifest and repository inventory. The repository includes 5,499 tracked paths in this checkout; no claim is made that every file has been read or reviewed. Upstream declares MIT OR Apache-2.0; examine licenses and attribution of every dependency before reuse.

The architecture combines rust-analyzer's incremental database, MIR lowering to Cranelift, per-function caching and object generation, a daemon and workers, an object/executable splice, Cargo integration, and an optional LLVM backend. Correctness relies on interactions with rustc's unstable ABI and type/layout conventions. Its own no-rustc document distinguishes the end goal from the current implementation: an initial rustc build and other rustc-dependent steps remain. Its documentation and code are changing quickly, so pin any comparison to a recorded commit.

## Evidence and opportunities

The Rust compiler performance survey reports over 3,700 responses and identifies incremental rebuilds, dependency invalidation and linking as key pain points. This is a survey, not a representative causal measurement of Cargo alone. The 2025 State of Rust survey also reports resource use as a continuing problem. References:

- https://blog.rust-lang.org/2025/09/10/rust-compiler-performance-survey-2025-results/
- https://blog.rust-lang.org/2026/03/02/2025-State-Of-Rust-Survey-results/
- https://goals.rust-lang.org/2026/roadmap-fast-builds.html

Upstream's own published Linux x86-64 measurements for a ripgrep edit show 508 ms with Cargo and 56 ms with Blaze; peak memory is 528 MB and 1.7 GB respectively. Those figures describe its hardware, versions and methodology, not this environment. Other upstream measurements include regressions: the biome edit in its table is 2.12 s versus 2.38 s, and a signature change took 108.75 s with fallback. Upstream describes samples, build configuration and limitations in `docs/benchmarks.md`.

## Design obligations

- Stable Rust for the implementation is not proof of stable Rust language compatibility: the frontend, std source, ABI, proc-macro protocol and layout must each be matched to a stated toolchain and target. Latest stable at this date is 1.99.1; track updates through explicit, tested upgrades.
- Build a versioned input model including source, generated inputs, environment, flags, tools, targets, configuration, build scripts, proc macros and transitive dependencies. Incorrect invalidation must fail visibly; caching cannot claim correctness without it.
- A complete implementation needs parsing, expansion, name resolution, type checking, borrow checking, const evaluation, layout, monomorphization, codegen, linking and a Cargo-compatible build model, as well as executable formats, unwinding and debug info. Do not confuse a benchmark harness with any of these.
- Require differential behavior against a pinned rustc reference on success and failure, incremental edit sequences, generics, macros, build scripts, cfg, async, unsafe, ABI, linking and mixed-language cases. Add crash recovery, concurrency, filesystem race and cache-corruption tests. Document platform coverage.
- Compare cold, no-op, warm edits, signatures and dependency changes; record output correctness, unsupported cases, build and readiness latency, run-time performance, peak total process-tree RSS, idle memory, disk usage and artifact size. Pin projects and toolchains, alternate order, repeat samples, publish raw data and confidence intervals. GitHub hosted runners are useful for correctness and trend checks but noisy for performance claims; dedicated machines are needed for defensible comparisons.
- The requirement to outperform every competing tool on every metric is a goal to test, not a fact that can be guaranteed before an implementation and measurements exist. A daemon's latency, memory and cold-start tradeoffs need explicit measurement.

## Current status

An initial dependency invalidation planner and the small smoke harness exist. The harness has self-checked with Cargo as both participants. No compiler rewrite, production safety review, Blaze build, or upstream comparison has been completed.
