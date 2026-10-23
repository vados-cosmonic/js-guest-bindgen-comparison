use anyhow::{Result, ensure};
use serde::Serialize;
use std::{path::Path, time::Instant};
use wasmtime::component::{Component, ComponentType, Instance, Lift, Linker, Lower, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
#[derive(ComponentType, Lower, Lift, Clone, Debug, PartialEq)]
#[component(record)]
struct Payload {
    count: u32,
    scale: f64,
    label: String,
}
struct State {
    wasi: WasiCtx,
    table: ResourceTable,
}
impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}
fn instance(engine: &Engine, c: &Component) -> Result<(Store<State>, Instance)> {
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    let mut host = linker.instance("comparison:bench/host")?;
    host.func_wrap("ping", |_, (x,): (u32,)| Ok((x.wrapping_add(1),)))?;
    host.func_wrap("echo", |_, (x,): (String,)| Ok((x,)))?;
    let mut store = Store::new(
        engine,
        State {
            wasi: WasiCtxBuilder::new().inherit_stderr().build(),
            table: ResourceTable::new(),
        },
    );
    let i = linker.instantiate(&mut store, c)?;
    Ok((store, i))
}
#[derive(Serialize)]
struct Measurement {
    case: String,
    iterations: usize,
    ns_per_call: Vec<f64>,
    median_ns: f64,
    min_ns: f64,
    max_ns: f64,
}
fn measure(name: &str, mut f: impl FnMut() -> Result<()>) -> Result<Measurement> {
    for _ in 0..10 {
        f()?;
    }
    let t = Instant::now();
    for _ in 0..10 {
        f()?;
    }
    let iterations =
        ((10_000_000.0 / (t.elapsed().as_nanos() as f64 / 10.0)) as usize).clamp(10, 20000);
    let mut ns_per_call = Vec::new();
    for _ in 0..15 {
        let t = Instant::now();
        for _ in 0..iterations {
            f()?;
        }
        ns_per_call.push(t.elapsed().as_nanos() as f64 / iterations as f64);
    }
    let mut sorted = ns_per_call.clone();
    sorted.sort_by(f64::total_cmp);
    Ok(Measurement {
        case: name.into(),
        iterations,
        median_ns: sorted[7],
        min_ns: sorted[0],
        max_ns: sorted[14],
        ns_per_call,
    })
}
#[derive(Serialize)]
struct Report {
    artifact: String,
    wasmtime: String,
    compile_ms: f64,
    instantiate_ns: Vec<f64>,
    first_scalar_ns: Vec<f64>,
    correctness: String,
    measurements: Vec<Measurement>,
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    ensure!(!args.is_empty(), "runner <component> [report.json]");
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_exceptions(true);
    let engine = Engine::new(&config)?;
    let t = Instant::now();
    let component = Component::from_file(&engine, &args[0])?;
    let compile_ms = t.elapsed().as_secs_f64() * 1000.0;
    let (mut instantiate_ns, mut first_scalar_ns) = (vec![], vec![]);
    for _ in 0..9 {
        let t = Instant::now();
        let (mut store, inst) = instance(&engine, &component)?;
        instantiate_ns.push(t.elapsed().as_nanos() as f64);
        let f = inst.get_typed_func::<(u32,), (u32,)>(&mut store, "scalar")?;
        let t = Instant::now();
        let (x,) = f.call(&mut store, (41,))?;
        first_scalar_ns.push(t.elapsed().as_nanos() as f64);
        ensure!(x == 42);
    }
    let (mut store, inst) = instance(&engine, &component)?;
    let scalar = inst.get_typed_func::<(u32,), (u32,)>(&mut store, "scalar")?;
    let floating = inst.get_typed_func::<(f64,), (f64,)>(&mut store, "floating")?;
    let text = inst.get_typed_func::<(String,), (String,)>(&mut store, "text")?;
    let bytes = inst.get_typed_func::<(Vec<u8>,), (Vec<u8>,)>(&mut store, "bytes")?;
    let structured = inst.get_typed_func::<(Payload,), (Payload,)>(&mut store, "structured")?;
    let imports = inst.get_typed_func::<(u32,), (u32,)>(&mut store, "imports")?;
    let import_text = inst.get_typed_func::<(String,), (String,)>(&mut store, "import-text")?;
    for x in [0, 1, 41, 0x7fffffff, 0x80000000, 0xfffffffe, 0xffffffff] {
        let (y,) = scalar.call(&mut store, (x,))?;
        ensure!(y == x.wrapping_add(1), "scalar {x}: {y}");
    }
    for x in [-3.5, 0.0, 0.125, 12345.678] {
        let (y,) = floating.call(&mut store, (x,))?;
        ensure!(y == x * 1.25 + 0.5, "floating {x}: {y}");
    }
    for s in ["", "hello", "héllø 🦀 東京\0last"] {
        for f in [text, import_text] {
            let (out,) = f.call(&mut store, (s.to_string(),))?;
            ensure!(out == s, "UTF8 result: {out:?}");
        }
        let input = Payload {
            count: 0xffffffff,
            scale: -0.25,
            label: s.into(),
        };
        let (out,) = structured.call(&mut store, (input,))?;
        ensure!(
            out == Payload {
                count: 0,
                scale: -0.3125,
                label: s.into()
            },
            "record {out:?}"
        );
    }
    for n in [0, 1, 32, 1024, 16384] {
        let v = (0..n).map(|i| (i % 256) as u8).collect::<Vec<_>>();
        let (out,) = bytes.call(&mut store, (v.clone(),))?;
        ensure!(out == v, "bytes size {n}");
    }
    for n in [0, 1, 10, 100, 1000] {
        let (out,) = imports.call(&mut store, (n,))?;
        ensure!(out == n);
    }
    eprintln!("correctness passed: {}", args[0]);
    let mut measurements = Vec::new();
    measurements.push(measure("scalar", || {
        let r = scalar.call(&mut store, (41,))?;
        std::hint::black_box(r);
        Ok(())
    })?);
    measurements.push(measure("floating", || {
        let r = floating.call(&mut store, (3.125,))?;
        std::hint::black_box(r);
        Ok(())
    })?);
    for n in [0, 32, 1024, 16384] {
        let s = "x".repeat(n);
        measurements.push(measure(&format!("text-{n}"), || {
            let r = text.call(&mut store, (s.clone(),))?;
            std::hint::black_box(r);
            Ok(())
        })?);
        let v = vec![0xA5; n];
        measurements.push(measure(&format!("bytes-{n}"), || {
            let r = bytes.call(&mut store, (v.clone(),))?;
            std::hint::black_box(r);
            Ok(())
        })?);
    }
    let s = "héllø 🦀 東京".repeat(64);
    measurements.push(measure("text-unicode", || {
        let r = text.call(&mut store, (s.clone(),))?;
        std::hint::black_box(r);
        Ok(())
    })?);
    let p = Payload {
        count: 41,
        scale: 1.25,
        label: "hello world".into(),
    };
    measurements.push(measure("record", || {
        let r = structured.call(&mut store, (p.clone(),))?;
        std::hint::black_box(r);
        Ok(())
    })?);
    for n in [1, 10, 100] {
        measurements.push(measure(&format!("imports-{n}"), || {
            let r = imports.call(&mut store, (n,))?;
            std::hint::black_box(r);
            Ok(())
        })?);
    }
    let s = "x".repeat(1024);
    measurements.push(measure("import-text-1024", || {
        let r = import_text.call(&mut store, (s.clone(),))?;
        std::hint::black_box(r);
        Ok(())
    })?);
    let report=Report{artifact:args[0].clone(),wasmtime:"50.0.0".into(),compile_ms,instantiate_ns,first_scalar_ns,correctness:"passed: u32 boundary, f64 fractions, empty and Unicode/NUL strings, byte-list sizes, records, import round trips".into(),measurements};
    let data = serde_json::to_string_pretty(&report)?;
    if let Some(path) = args.get(1) {
        if let Some(dir) = Path::new(path).parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, data)?;
    } else {
        println!("{data}");
    }
    Ok(())
}
