//! A component implemented in JS (`../../../app.js`) running on the Boa engine.
//!
//! build.rs generates the JS bindings with js-component-bindgen and the core
//! exports/imports in Rust. The bindings reach linear memory and the core
//! imports through the native `__wit_*` functions registered here.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;

use boa_engine::builtins::promise::PromiseState;
use boa_engine::module::{ModuleLoader, ModuleRequest, Referrer};
use boa_engine::object::builtins::{JsArray, JsArrayBuffer, JsPromise, JsTypedArray};
use boa_engine::{
    Context, JsArgs, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Module,
    NativeFunction, Source, js_string,
};

include!(concat!(env!("BINDINGS_DIR"), "/bindings.rs"));

const PRELUDE_JS: &str = include_str!("prelude.js");
const WIT_JS: &str = include_str!(concat!(env!("BINDINGS_DIR"), "/wit.js"));
const APP_JS: &str = include_str!("../../../packages/app/app.js");

// wasi-libc's allocator, which the host's allocations via cabi_realloc also use
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(ptr: *mut c_void);
}

pub(crate) unsafe fn free(ptr: *mut u8) {
    unsafe { libc_free(ptr.cast()) }
}

thread_local! {
    static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) };
    static CALLBACKS: RefCell<Vec<Option<JsObject>>> = RefCell::new(vec![None; EXPORT_COUNT]);
    // allocations made by JS lowering, freed once the host is done with them
    static TRACKED: RefCell<Vec<*mut u8>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn free_tracked() {
    TRACKED.with_borrow_mut(|t| t.drain(..).for_each(|p| unsafe { free(p) }));
}

fn alloc_tracked(size: usize) -> *mut u8 {
    let p = unsafe { malloc(size.max(1)) }.cast::<u8>();
    TRACKED.with_borrow_mut(|t| t.push(p));
    p
}

pub(crate) fn call_export(i: usize, args: &[f64]) -> f64 {
    CONTEXT.with_borrow_mut(|context| {
        let context = context.get_or_insert_with(|| init().unwrap_or_else(|e| panic!("init: {e}")));
        let callback = CALLBACKS
            .with_borrow(|c| c[i].clone())
            .expect("export not bound");
        let args: Vec<JsValue> = args.iter().map(|&a| JsValue::from(a)).collect();
        match callback.call(&JsValue::undefined(), &args, context) {
            Ok(v) => v.to_number(context).unwrap_or(0.0),
            Err(e) => panic!("uncaught exception: {e}"),
        }
    })
}

/// Resolves module specifiers exactly, without treating them as paths
#[derive(Default)]
struct Loader(RefCell<HashMap<String, Module>>);

impl ModuleLoader for Loader {
    async fn load_imported_module(
        self: Rc<Self>,
        _referrer: Referrer,
        request: ModuleRequest,
        _context: &RefCell<&mut Context>,
    ) -> JsResult<Module> {
        let specifier = request.specifier().to_std_string_escaped();
        self.0.borrow().get(&specifier).cloned().ok_or_else(|| {
            JsNativeError::typ()
                .with_message(format!("unknown module {specifier}"))
                .into()
        })
    }
}

fn init() -> JsResult<Context> {
    let loader = Rc::new(Loader::default());
    let mut context = Context::builder().module_loader(loader.clone()).build()?;
    let natives: [(&str, usize, NativeFunction); 11] = [
        ("__wit_register", 2, NativeFunction::from_fn_ptr(register)),
        ("__wit_import", 1, NativeFunction::from_fn_ptr(import)),
        ("__wit_resource", 2, NativeFunction::from_fn_ptr(resource)),
        ("__wit_alloc", 1, NativeFunction::from_fn_ptr(alloc)),
        ("__wit_free", 1, NativeFunction::from_fn_ptr(dealloc)),
        ("__wit_load", 2, NativeFunction::from_fn_ptr(load)),
        ("__wit_store", 3, NativeFunction::from_fn_ptr(store)),
        ("__wit_read", 2, NativeFunction::from_fn_ptr(read)),
        ("__wit_write", 2, NativeFunction::from_fn_ptr(write)),
        (
            "__wit_utf8_decode",
            1,
            NativeFunction::from_fn_ptr(utf8_decode),
        ),
        (
            "__wit_utf8_encode",
            1,
            NativeFunction::from_fn_ptr(utf8_encode),
        ),
    ];
    for (name, length, f) in natives {
        context.register_global_callable(JsString::from(name), length, f)?;
    }

    let mut parse = |src: &str| Module::parse(Source::from_bytes(src), None, &mut context);
    let wit = parse(&format!("{PRELUDE_JS}\n{WIT_JS}"))?;
    let app = parse(APP_JS)?;
    let mut modules = loader.0.borrow_mut();
    for (specifier, src) in IMPORT_MODULES {
        modules.insert(specifier.to_string(), parse(src)?);
    }
    modules.insert("wit".into(), wit.clone());
    drop(modules);

    let promise = app.load_link_evaluate(&mut context);
    settle(&promise, &mut context)?;
    let bind = wit
        .namespace(&mut context)
        .get(js_string!("bindExports"), &mut context)?;
    let app_ns = app.namespace(&mut context);
    bind.as_callable().expect("bindExports").call(
        &JsValue::undefined(),
        &[app_ns.into()],
        &mut context,
    )?;
    Ok(context)
}

fn settle(promise: &JsPromise, context: &mut Context) -> JsResult<()> {
    context.run_jobs()?;
    match promise.state() {
        PromiseState::Rejected(e) => Err(JsError::from_opaque(e)),
        _ => Ok(()),
    }
}

fn num(args: &[JsValue], i: usize, context: &mut Context) -> JsResult<f64> {
    args.get_or_undefined(i).to_number(context)
}

fn addr(args: &[JsValue], i: usize, context: &mut Context) -> JsResult<usize> {
    Ok(num(args, i, context)? as u32 as usize)
}

/// Copies the bytes viewed by a typed array
fn typed_array_bytes(value: &JsValue, context: &mut Context) -> JsResult<Vec<u8>> {
    let object = value
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("expected a typed array"))?;
    let array = JsTypedArray::from_object(object.clone())?;
    let (offset, length) = (array.byte_offset(context)?, array.byte_length(context)?);
    let buffer = array.buffer(context)?;
    let buffer = JsArrayBuffer::from_object(buffer.as_object().unwrap().clone())?;
    let data = buffer
        .data()
        .ok_or_else(|| JsNativeError::typ().with_message("detached buffer"))?;
    Ok(data[offset..offset + length].to_vec())
}

fn register(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let i = num(args, 0, context)? as usize;
    let f = args.get_or_undefined(1).as_object().map(|o| o.clone());
    CALLBACKS.with_borrow_mut(|c| c[i] = f);
    Ok(JsValue::undefined())
}

fn import(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let i = num(args, 0, context)? as usize;
    let a = (1..args.len())
        .map(|n| num(args, n, context))
        .collect::<JsResult<Vec<_>>>()?;
    Ok(call_import(i, &a).into())
}

fn resource(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let i = num(args, 0, context)? as usize;
    let h = num(args, 1, context)? as i64 as i32;
    Ok(call_resource(i, h).into())
}

fn alloc(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let size = addr(args, 0, context)?;
    Ok((alloc_tracked(size) as u32).into())
}

fn dealloc(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    unsafe { free(addr(args, 0, context)? as *mut u8) };
    Ok(JsValue::undefined())
}

fn load(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let kind = num(args, 0, context)? as u32;
    let p = addr(args, 1, context)?;
    let v = unsafe {
        match kind {
            0 => (p as *const i8).read_unaligned() as f64,
            1 => (p as *const u8).read_unaligned() as f64,
            2 => (p as *const i16).read_unaligned() as f64,
            3 => (p as *const u16).read_unaligned() as f64,
            4 => (p as *const i32).read_unaligned() as f64,
            5 => (p as *const u32).read_unaligned() as f64,
            6 => (p as *const f32).read_unaligned() as f64,
            _ => (p as *const f64).read_unaligned(),
        }
    };
    Ok(v.into())
}

fn store(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let kind = num(args, 0, context)? as u32;
    let p = addr(args, 1, context)?;
    let v = num(args, 2, context)?;
    unsafe {
        match kind {
            0 | 1 => (p as *mut u8).write_unaligned(v as i64 as u8),
            2 | 3 => (p as *mut u16).write_unaligned(v as i64 as u16),
            4 | 5 => (p as *mut u32).write_unaligned(v as i64 as u32),
            6 => (p as *mut f32).write_unaligned(v as f32),
            _ => (p as *mut f64).write_unaligned(v),
        }
    }
    Ok(JsValue::undefined())
}

fn read(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let (p, n) = (addr(args, 0, context)?, addr(args, 1, context)?);
    let buffer = JsArrayBuffer::new(n, context)?;
    if n > 0 {
        let src = unsafe { std::slice::from_raw_parts(p as *const u8, n) };
        buffer.data_mut().unwrap().copy_from_slice(src);
    }
    Ok(buffer.into())
}

fn write(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let p = addr(args, 0, context)?;
    let bytes = typed_array_bytes(args.get_or_undefined(1), context)?;
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len()) };
    Ok(JsValue::undefined())
}

fn utf8_decode(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let bytes = typed_array_bytes(args.get_or_undefined(0), context)?;
    Ok(JsString::from(String::from_utf8_lossy(&bytes).as_ref()).into())
}

fn utf8_encode(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let s = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_lossy();
    let p = alloc_tracked(s.len());
    unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), p, s.len()) };
    let pair = [JsValue::from(p as u32), JsValue::from(s.len() as u32)];
    Ok(JsArray::from_iter(pair, context).into())
}

/// Return area for a call; freed by post-return (exports) or by the bindings (imports)
pub(crate) fn alloc_ret(size: usize) -> i32 {
    unsafe { malloc(size.max(1)) as i32 }
}
