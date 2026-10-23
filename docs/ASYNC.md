# Jobs and async behavior in this P2 experiment

The measured WIT functions are synchronous. Every export returns a plain JS value, every host import returns synchronously, and there are no timers or I/O operations. Thus there is **no async loop pumping during a timed call**. Reporting such a loop as part of any of these call paths would be misleading.

| Engine / path | Initialization | Timed export | Return and cleanup |
|---|---|---|---|
| Boa A | Parse modules, load/link/evaluate, `Context::run_jobs`, bind generated JS callbacks | Native core wrapper calls generated binding, which calls the app | JS binding writes result; native wrapper returns; Wasmtime runs post-return |
| Boa B/C | Parse the componentize-generated modules and app, load/link/evaluate, `Context::run_jobs`, cache exported functions | Typed native B shim or C's generic `export_call` calls the app | B native ABI runtime or C world.so lowers result; Wasmtime runs post-return |
| Porffor A | Lazily call the compiled program's main to register generated binding callbacks | Compiled binding and app execute as Wasm | Compiled binding writes result; native wrapper returns; post-return frees allocations |
| Porffor B/C | Lazily run compiled main and root native callback values | Native B shim or C's `export_call` invokes the compiled app | Native B runtime or C world.so returns canonical values; post-return cleans up |

For an application that returns a Promise through a synchronous P2 export, all three architectures would need an additional *runtime-owned* scheduler. The architectural placement is the same: after the engine call and before lowering the final export result. It must retain argument/result roots, drain microtasks, wait on registered P2 WASI pollables when necessary, resolve I/O promises, drain again, and only return once the outer promise settles. A rejected promise must become a defined failure/trap, and a pending promise with no possible progress must not become a busy loop. Cleanup must still occur exactly once.

Boa exposes `Context::run_jobs` for job draining. Porffor's source includes `__Porffor_promise_runJobs` and a native microtask-draining integration in its compiler runtime, but this experiment does not connect that to a WASI P2 poll driver. Its AOT event-loop integration would require an explicit embedding contract, rather than assuming Node's event loop exists inside the guest.

This is different from the P3 path in componentize-js, which coordinates task state, waitable sets, callbacks, streams/futures and task-return. Those hooks are neither enabled nor benchmarked here. Extending this comparison to P2 Promise/I/O workloads requires implementing the same scheduler policy for A/B/C, validating rejection and deadlock behavior, and then measuring scheduler costs separately from binding costs. The current results cannot answer that performance question.
