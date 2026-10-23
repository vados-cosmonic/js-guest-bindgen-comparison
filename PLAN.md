# Implementation and measurement plan

- [x] Inspect upstream designs and pin source revisions.
- [x] Create common P2 workloads and correctness oracle (scalars, strings, lists, records, host imports).
- [x] A: adapt the actual js-guest-bindgen Boa and Porffor embeddings to the common world.
- [x] B: reuse componentize-js metadata/JS generation; generate a native Canonical ABI runtime and interface-specific engine shims, with no Canonical ABI work in those shims.
- [x] C: reuse the same componentize-js generation and link real wit-dylib against native engine adapters.
- [x] Build and validate all six P2 components; exercise correctness and ownership/post-return.
- [x] Measure stripped and compressed component sizes, cold initialization, steady-state calls, and import round trips with repeated samples.
- [x] Explain measured results and limitations in README.md and SUMMARY.md, with editable draw.io diagrams and SVGs.

## Guardrails

Only Boa and Porffor execute guest JavaScript. Node is a build tool, not a benchmark engine. Compare A/B/C within each engine; Porffor AOT and Boa interpretation are different execution models. Do not label a proxy as a full component benchmark. P2 calls are synchronous; distinguish JS microtask draining from P3 task/waitable machinery. Retain raw samples, exact revisions, build flags, and generated code. No assumed performance or size winner.

## Initial findings

The componentize-js reboot is tightly coupled to mozjs in its runtime, but src/codegen.rs is engine-independent. Port the P2 runtime contract, reuse that code generator independently, and do not build mozjs. js-guest-bindgen includes usable Boa and Porffor example embeddings; the Porffor example targets P3 by default and must be switched to P2 for this comparison. Both examples copy strings/lists through native memory helpers.

## Correctness milestone

All six cells build, validate, and pass u32 boundary/f64/UTF-8/byte-list/record/import correctness. The runner automatically includes canonical post-return via Wasmtime 50. Pilot runs revealed inactive debug calls in A and Porffor's JS UTF-8 prelude as confounders. Three diagnostic A variants remove only logging and then replace the UTF-8 intrinsic, while retaining JS canonical layout code. Final measurements are separate from pilots, pinned to one CPU, repeated in rotated process order, with all custom sections removed for deployment-size accounting.

## Controls and build parity

All three diagnostic A variants pass the same correctness suite. Final builds use WASI SDK 30 static libc for A/B and the matching shared libc for C. The standard Rust raw linker protocol is bridged to the SDK component linker by `scripts/wasi-static-linker.py`, supplying the SDK reactor startup and excluding Rust's bundled libc search directory.

## Completed experiment

The final dataset contains 27 successful process runs: six primary builds and three controls, each repeated three times. Each process measures 16 warm cases in 15 batches and nine fresh instances for cold behavior. Raw samples, deployment hashes, section sizes, tool versions, and source hashes are retained under `results/`. `SUMMARY.md` records the findings, controls, and limits; the three call-path diagrams include both SVG and editable draw.io files.

Remaining production work is outside this measured comparison: arbitrary-WIT generation of B's typed shims, additional WIT types in C's engine adapter, consistent invalid-JS validation, and a P2 Promise/I/O scheduler. The current results do not establish performance for those paths or for shared prebuilt engine deployment.
