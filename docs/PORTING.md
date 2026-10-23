# Porting choices and audit trail

The vendored componentize-js generator is an unmodified copy of upstream `src/codegen.rs` and `src/globals.js`, at the revision in `upstream.lock.json`. It runs without compiling the upstream mozjs runtime. Its generated imported-interface modules and globals are used by both native backends. Its generated async export wrappers are retained by the Boa module loader but never selected for synchronous P2 exports; Porffor's build only needs the synchronous exports.

The P2 `Interpreter` / `Call` contract is ported to `native/common/runtime_c.c`. This is a port for the comparison WIT world, not a claim to port componentize-js's entire engine-specific runtime. The actual Canonical ABI code in C is the output of wit-dylib 0.254.0, the version selected by componentize-js's manifest. The separate wasm-tools main checkout records what was inspected; it is not silently substituted for that dependency. `native/common/wit_dylib.h` matches 0.254.0.

B replaces wit-dylib's generic stack-machine callbacks with interface-specific native shims in `native/common/shims_b.c`, and uses wit-bindgen-c's generated `bench.c` as the specialized native Canonical ABI backend. The shims consume typed native values and call engine operations; they do not compute canonical memory offsets, flatten tuples, allocate return areas, or implement post-return. `bench.c` does those things. That is an additional backend in this experiment, not an upstream componentize-js feature. For this fixed experimental world, the shim source is explicit and inspectable. Extending the world requires extending these shims; it is not a production arbitrary-WIT generator.

Boa uses its Rust native embedding API. Porffor compiles JavaScript to C and then Wasm; its native adapter calls compiled engine builtins and uses its pinned object representation. The per-engine operations are shared between B and C. Both copy strings and byte lists rather than advertising zero-copy buffers. Values are rooted across native calls and released at call completion. B uses static linking; C uses the real shared-everything library path, so C's measured size includes shared libc and dynamic-linking scaffolding.

The A adapters are derived from js-guest-bindgen's examples. Local corrections:

- Preserve all u32 bit patterns when Porffor returns a raw core i32 (cast through int64_t).
- Return f32/f64 directly from the Boa wrapper, avoiding the example's intermediate integer cast.
- Give Porffor inline-C byte-buffer locals unique names so bundling does not rename the JS identifier without updating the inline-C text.
- Use P2 for Porffor, matching Boa and the requested comparison.

Porffor C requires a shared `__c_longjmp` exception tag used by both the engine and WASI SDK's `libsetjmp.so`; `native/porffor/longjmp.S` defines it. This is the standard LLVM Wasm exception tag, not a P3 feature. No alternative guest JavaScript engine is built or run.

C callbacks for WIT kinds outside the tested world trap explicitly in `unsupported_c.c`; B's typed shims support only the declared world. Therefore the measurements say nothing about resources, variants, options/results, 64-bit integers, futures/streams, or general event-loop workloads. The scalar, f64, UTF-8, byte-list, record, and imported-call paths are real implementations.

A also overrides the generated `WebAssemblyRuntimeError` intrinsic with `Error`: neither engine embeds a WebAssembly JavaScript API, and the comparison only needs the error constructor, not nested Wasm execution. B/C install `_componentizeJsCallImport` on the JS global object, as the componentize-js generated modules expect. C explicitly exports its Canonical ABI allocator; relying on libc.so's weak allocator alias alone creates a linker-trampoline cycle in the Porffor build.

Wasmtime 50 automatically calls canonical post-return inside `TypedFunc::call`. The runner measures that full call; it does not time only the low-level return. No snapshotting is used in any of the six builds. Thus cold results measure lazy engine initialization, not the snapshot-enabled upstream componentize-js distribution.

The static Boa link is normalized through `scripts/wasi-static-linker.py`: it preserves Rust's rlibs and export selection but uses WASI SDK 30's static libc and reactor startup. C uses the same SDK's shared libc. This removes a bundled-libc-version difference from A/B/C while retaining the static versus shared deployment distinction.

Diagnostic A variants retain the exact application and WIT world. The `nolog` variants remove 18 generated debug-call statements using an AST walk. The Porffor `native-utf8` variant also reuses B/C's native UTF-8 routines, with the A prelude's allocation ownership retained. The JS binding still flattens values, reads/writes canonical memory and return areas, and performs the WIT import/export conversions. These controls address implementation overhead, not a switch to B.

C's reusable-interpreter advantage is architectural, not demonstrated by these artifacts: the Boa library embeds the application text, and Porffor compiles the application into its native library. A deployable prebuilt Boa runtime would load the application separately (or snapshot it); Porffor already needs per-application AOT compilation, reducing the value of keeping its engine library independent of WIT.
