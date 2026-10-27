# Architecture and current boundaries

The dependency planner represents nodes with caller-provided 32-byte fingerprints and explicit dependencies. It validates missing references and cycles, computes a deterministic dependency-first order, and propagates changes through both the old and new graphs. The union is conservative when edges are removed. A caller can force a node dirty even when its fingerprint is unchanged. Unknown change events fail.

It is a prototype, not an input discovery system. Correct reuse requires complete input observation, sound fingerprints and atomic publication tied to a consistent source snapshot. File additions, directory membership, environment, tools, target properties, Cargo configuration, proc macros and build scripts all need explicit modeling. Nothing here yet implements those obligations. It also has no concurrent scheduler, storage, eviction, sandbox, Rust frontend, code generator, linker, daemon or Cargo interface.

Suggested development sequence: specify input and execution semantics; implement transactional snapshots and content addressing with fault-injection tests; build a Cargo-compatible unit model validated against Cargo; add language semantics and diagnostics with differential Rust tests; then implement code generation and object/link handling and only enable reuse after proving invalidation. Measure edit latency, cold startup, memory, disk and generated-code behavior against pinned upstream revisions.

No component here calls Cargo as a fallback. The separate smoke harness invokes explicitly selected programs as test subjects; it is not a compiler execution path.
