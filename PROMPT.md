OK, I want you to compare these two projects:

1. https://github.com/dicej/componentize-js/tree/main
  - depends on: https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wit-dylib
2. https://github.com/MendyBerger/js-guest-bindgen

js-guest-bindgen is more minimal than componentize-js and has more per-language adapters, but my question is whether there are size/performance benefits to using js-guest-bindgen over the machinery in componentize-js (based on wit-dylib) and having the lifting/lowering/etc happen on the JS side is actually beneficial.

I already know that P3 support isn't consistent/present for many languages in js-guest-bindgen, and I already know that wit-dylib *should* produce slightly slower/larger binaries as it's very general, but it's not clear that this actually will turn into significant performance/size benefits.

The idea is that we want to compare for P2 components three different approaches:

A. js-guest-bindgen
B. componentize-js's WIT generation machinery & runtime generation machinery with native per-js-engine shims (the shims in the engine should NOT do the lifting/lowering, that should be on the runtime side, rutnime shims should be native and specific to the WIT interface not general like wit-dylib) 
C. componentize-js's WIT generation machinery & runtime generation machinery with wit-dylib as the target. (wit-dylib is more general than the approach in B,)

To make this work, we will need considerable thinking and architectural planning along with implementation for each of the different engines. For now, let's target Boa and Porffor as the engines to build for and check results against. We should not be using starlingmonkey or qjs or any other engines that may be out there.

Work on this in a git monorepo style project in ~/workspace/js-guest-bindgen-comparison, make sure to explain architecture, differences and perceived benefits/drawbacks of each approach in the README. You can also keep track of progress and the plan in PLAN.md there. Try to main separate subfolders for separate projects as necessary, and reuse over package/crate boundaries where you can in the monorepo.

At the end, I want a SUMMARY.md which has:
1. consicse summary of approaches, with diagrams (use draw.io style and include the SVGs) with pros/cons
1.1. Call path explanation/diagram for each approach (which piece of code receives the actual call, where does lifting/lowering happen, which part is the core engine running, how does the async loop pump, where is the result handled, etc)
2. Size differences between each approach
3. Performance differences between each approach
4. This prompt in PROMPT.md
