//! Boa engine operations for native B/C runtimes. Canonical ABI stays in C/Wasm.
use boa_engine::builtins::promise::PromiseState;
use boa_engine::module::{ModuleLoader, ModuleRequest, Referrer};
use boa_engine::object::builtins::{JsArray, JsArrayBuffer, JsTypedArray, JsUint8Array};
use boa_engine::{
    Context, JsArgs, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Module,
    NativeFunction, Source, js_string,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, c_char, c_void},
    rc::Rc,
};
include!(concat!(env!("BINDINGS_DIR"), "/modules.rs"));
thread_local! {
    static CONTEXT: RefCell<Option<Context>> = const {RefCell::new(None)};
    static ACTIVE: std::cell::Cell<*mut Context> = const {std::cell::Cell::new(std::ptr::null_mut())};
    static VALUES: RefCell<Vec<JsValue>> = const {RefCell::new(Vec::new())};
    static CALLBACKS: RefCell<Vec<JsObject>> = const {RefCell::new(Vec::new())};
}
unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn runtime_import(i: usize, a: *const u32, n: usize) -> u32;
}
fn put(v: JsValue) -> u32 {
    VALUES.with_borrow_mut(|vs| {
        let i = vs.len();
        vs.push(v);
        i as u32
    })
}
fn get(i: u32) -> JsValue {
    VALUES.with_borrow(|vs| vs[i as usize].clone())
}
fn with_context<T>(f: impl FnOnce(&mut Context) -> T) -> T {
    let active = ACTIVE.get();
    if !active.is_null() {
        return f(unsafe { &mut *active });
    }
    CONTEXT.with_borrow_mut(|slot| f(slot.as_mut().expect("engine initialized")))
}
#[derive(Default)]
struct Loader(RefCell<HashMap<String, Module>>);
impl ModuleLoader for Loader {
    async fn load_imported_module(
        self: Rc<Self>,
        _: Referrer,
        req: ModuleRequest,
        _: &RefCell<&mut Context>,
    ) -> JsResult<Module> {
        self.0
            .borrow()
            .get(&req.specifier().to_std_string_escaped())
            .cloned()
            .ok_or_else(|| JsNativeError::typ().with_message("missing module").into())
    }
}
fn import(_: &JsValue, args: &[JsValue], cx: &mut Context) -> JsResult<JsValue> {
    let i = args.get_or_undefined(0).to_u32(cx)? as usize;
    let a = JsArray::from_object(args.get_or_undefined(1).as_object().unwrap().clone())?;
    let mark = engine_mark();
    let vals = (0..a.length(cx)?)
        .map(|i| a.get(i, cx).map(put))
        .collect::<JsResult<Vec<_>>>()?;
    let prev = ACTIVE.replace(cx);
    let result = unsafe { runtime_import(i, vals.as_ptr(), vals.len()) };
    ACTIVE.set(prev);
    let result = get(result);
    engine_reset(mark);
    Ok(result)
}
#[unsafe(no_mangle)]
pub extern "C" fn engine_init() {
    if CONTEXT.with_borrow(|c| c.is_some()) {
        return;
    }
    let loader = Rc::new(Loader::default());
    let mut cx = Context::builder()
        .module_loader(loader.clone())
        .build()
        .unwrap();
    cx.register_global_callable(
        js_string!("_componentizeJsCallImport"),
        2,
        NativeFunction::from_fn_ptr(import),
    )
    .unwrap();
    cx.eval(Source::from_bytes(include_str!(concat!(
        env!("BINDINGS_DIR"),
        "/globals.js"
    ))))
    .unwrap();
    for (name, code) in IMPORT_MODULES {
        let m = Module::parse(Source::from_bytes(code), None, &mut cx).unwrap();
        loader.0.borrow_mut().insert(name.to_string(), m);
    }
    let source = format!(
        "{}\n{}",
        include_str!("../../../packages/app/app.js"),
        include_str!(concat!(env!("BINDINGS_DIR"), "/async-exports.js"))
    );
    let app = Module::parse(Source::from_bytes(&source), None, &mut cx).unwrap();
    let p = app.load_link_evaluate(&mut cx);
    cx.run_jobs().unwrap();
    match p.state() {
        PromiseState::Fulfilled(_) => (),
        PromiseState::Rejected(e) => panic!("module: {}", JsError::from_opaque(e)),
        _ => panic!("module initialization remained pending"),
    }
    let ns = app.namespace(&mut cx);
    let fs = EXPORT_NAMES
        .iter()
        .map(|name| {
            ns.get(JsString::from(*name), &mut cx)
                .unwrap()
                .as_callable()
                .unwrap()
                .clone()
        })
        .collect();
    CALLBACKS.with_borrow_mut(|f| *f = fs);
    CONTEXT.with_borrow_mut(|c| *c = Some(cx));
}
#[unsafe(no_mangle)]
pub extern "C" fn engine_mark() -> u32 {
    VALUES.with_borrow(|v| v.len() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn engine_reset(mark: u32) {
    VALUES.with_borrow_mut(|v| v.truncate(mark as usize));
}
#[unsafe(no_mangle)]
pub extern "C" fn engine_number(x: f64) -> u32 {
    put(x.into())
}
#[unsafe(no_mangle)]
pub extern "C" fn engine_to_number(x: u32) -> f64 {
    with_context(|cx| get(x).to_number(cx).unwrap())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_string(p: *const u8, n: usize) -> u32 {
    let bytes = if n == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(p, n) }
    };
    put(JsString::from(std::str::from_utf8(bytes).expect("valid UTF-8")).into())
}
unsafe fn copy_out(bytes: &[u8], out: *mut *mut u8) -> usize {
    let p = unsafe { malloc(bytes.len().max(1)) }.cast::<u8>();
    assert!(!p.is_null());
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
        *out = p;
    };
    bytes.len()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_to_string(x: u32, out: *mut *mut u8) -> usize {
    let s = with_context(|cx| get(x).to_string(cx).unwrap().to_std_string_lossy());
    unsafe { copy_out(s.as_bytes(), out) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_bytes(p: *const u8, n: usize) -> u32 {
    let bytes = if n == 0 {
        vec![]
    } else {
        unsafe { std::slice::from_raw_parts(p, n) }.to_vec()
    };
    with_context(|cx| put(JsUint8Array::from_iter(bytes, cx).unwrap().into()))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_to_bytes(x: u32, out: *mut *mut u8) -> usize {
    with_context(|cx| {
        let arr = JsTypedArray::from_object(get(x).as_object().unwrap().clone()).unwrap();
        let (offset, len) = (arr.byte_offset(cx).unwrap(), arr.byte_length(cx).unwrap());
        let b = JsArrayBuffer::from_object(arr.buffer(cx).unwrap().as_object().unwrap().clone())
            .unwrap();
        let data = b.data().unwrap();
        unsafe { copy_out(&data[offset..offset + len], out) }
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn engine_object() -> u32 {
    with_context(|cx| put(JsObject::with_object_proto(cx.intrinsics()).into()))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_set(o: u32, key: *const c_char, v: u32) {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap();
    with_context(|cx| {
        get(o)
            .as_object()
            .unwrap()
            .set(JsString::from(key), get(v), true, cx)
            .unwrap();
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_get(o: u32, key: *const c_char) -> u32 {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap();
    with_context(|cx| {
        put(get(o)
            .as_object()
            .unwrap()
            .get(JsString::from(key), cx)
            .unwrap())
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_call(i: usize, p: *const u32, n: usize) -> u32 {
    let args = unsafe { std::slice::from_raw_parts(p, n) }
        .iter()
        .map(|i| get(*i))
        .collect::<Vec<_>>();
    let f = CALLBACKS.with_borrow(|fs| fs[i].clone());
    with_context(|cx| put(f.call(&JsValue::undefined(), &args, cx).unwrap()))
}
