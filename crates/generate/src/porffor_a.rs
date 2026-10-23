//! Generates the JS bindings for a WIT world with js-guest-bindgen, and the C
//! core exports/imports that connect them to the component model.
//!
//! The C is emitted through `Porffor.c`, so porffor compiles it together with
//! the JS. Core exports call the JS bindings via `porf_call_dynamic`.
//!
//! Usage: porffor <wit> --source <module.js> --out-dir <dir> [--world <name>]

use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use js_guest_bindgen::{CoreFn, CoreTy, GuestOpts, guest_bindgen};
use wit_parser::Resolve;

macro_rules! uwriteln {
    ($dst:expr, $($arg:tt)*) => {
        writeln!($dst, $($arg)*).unwrap()
    };
}

/// Intrinsics provided by `prelude.js`, since porffor has no wasm memory object
const OVERRIDDEN_INTRINSICS: &[&str] = &[
    "WebAssemblyRuntimeError",
    "dataView",
    "memoryView",
    "memoryWrite",
    "TEXT_DECODER_UTF8",
    "_utf8AllocateAndEncode",
];

const PRELUDE_C: &str = include_str!("prelude.c");
const PRELUDE_JS: &str = include_str!("prelude.js");

pub fn generate() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let (mut wit, mut source, mut out_dir, mut world) = (None, None, None, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--source" => source = args.next(),
            "--out-dir" => out_dir = args.next(),
            "--world" => world = args.next(),
            _ if wit.is_none() => wit = Some(arg),
            _ => bail!("unexpected argument {arg}"),
        }
    }
    let usage = "usage: porffor <wit> --source <module.js> --out-dir <dir> [--world <name>]";
    let wit = PathBuf::from(wit.context(usage)?);
    let source = std::fs::canonicalize(source.context(usage)?).context("--source")?;
    let out_dir = PathBuf::from(out_dir.context(usage)?);

    let mut resolve = Resolve::default();
    let (pkg, _) = resolve.push_path(&wit)?;
    let world = resolve.select_world(&[pkg], world.as_deref())?;
    let opts = GuestOpts {
        overridden_intrinsics: OVERRIDDEN_INTRINSICS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        on_bind: Some("$wit_register".into()),
        ..GuestOpts::default()
    };
    let bindings = guest_bindgen(&resolve, world, &opts)?;

    let mut c = String::from(PRELUDE_C);
    let mut js = String::new();

    // exports: core function the host calls -> JS binding via wit_cb[i]
    uwriteln!(c, "static jsval wit_cb[{}];", bindings.exports.len().max(1));
    let mut register = String::from("function $wit_register() {\n  let wit_f;\n");
    for (i, (name, item)) in bindings.exports.iter().enumerate() {
        let canon = item.resource.canon_string(&item.name);
        let core_name = if item.iface {
            format!("{name}#{canon}")
        } else {
            canon
        };
        let CoreFn {
            params,
            ret,
            retptr,
            retsize,
            ..
        } = &item.func;
        let mut args: Vec<String> = (0..params.len())
            .map(|n| format!("WIT_ARG(a{n})"))
            .collect();
        uwriteln!(
            c,
            "__attribute__((export_name(\"{core_name}\")))\n{} wit_export_{i}({}) {{\n  WIT_ENTER();",
            c_ret(ret)?,
            c_params(params)?
        );
        if *retptr {
            uwriteln!(c, "  int32_t rp = (int32_t)(uintptr_t)malloc({retsize});");
            args.push("WIT_ARG(rp)".into());
        }
        uwriteln!(
            c,
            "  jsbits argv[] = {{ {} }};\n  jsval r = porf_call_dynamic(wit_cb[{i}], JV_UNDEFINED, JV_UNDEFINED, {}, argv);\n  WIT_LEAVE();",
            if args.is_empty() {
                "0".to_string()
            } else {
                args.join(", ")
            },
            args.len()
        );
        match ret {
            _ if *retptr => uwriteln!(c, "  (void)r;\n  return rp;\n}}"),
            Some(ty) => uwriteln!(
                c,
                "  wit_free_tracked();\n  return ({}){}r.val;\n}}",
                c_ty(ty)?,
                if matches!(ty, CoreTy::I32) {
                    "(int64_t)"
                } else {
                    ""
                }
            ),
            None => uwriteln!(c, "  (void)r;\n  wit_free_tracked();\n}}"),
        }
        if *retptr {
            uwriteln!(
                c,
                "__attribute__((export_name(\"cabi_post_{core_name}\")))\nvoid wit_post_{i}(int32_t rp) {{\n  free((void*)(uintptr_t)rp);\n  wit_free_tracked();\n}}"
            );
        }
        uwriteln!(
            register,
            "  wit_f = {};\n  Porffor.c`wit_cb[{i}] = WIT_JV(wit_f);`;",
            item.binding_name
        );
    }
    register.push_str("}\n");

    // imports: JS binding -> C stub -> core import. Bindings use the export ABI,
    // so the stub provides the return area and hands it back to JS.
    for (i, ((_, item), binding)) in bindings
        .imports
        .iter()
        .zip(&bindings.import_bindings)
        .enumerate()
    {
        let module = item.core_module.as_deref().unwrap();
        let canon = item.resource.canon_string(&item.name);
        let CoreFn {
            params,
            ret,
            retptr,
            retsize,
            ..
        } = &item.func;
        uwriteln!(
            c,
            "__attribute__((import_module(\"{module}\"), import_name(\"{canon}\")))\nextern {} wit_import_raw_{i}({});",
            c_ret(ret)?,
            c_params(params)?
        );
        let js_params = &params[..params.len() - *retptr as usize];
        let mut call_args: Vec<String> = (0..js_params.len()).map(|n| format!("a{n}")).collect();
        let stub_ret = if *retptr { "int32_t" } else { c_ret(ret)? };
        uwriteln!(
            c,
            "static {stub_ret} wit_import_{i}({}) {{",
            c_params(js_params)?
        );
        if *retptr {
            uwriteln!(c, "  int32_t rp = (int32_t)(uintptr_t)malloc({retsize});");
            call_args.push("rp".into());
        }
        let call = format!("wit_import_raw_{i}({})", call_args.join(", "));
        match ret {
            _ if *retptr => uwriteln!(c, "  {call};\n  wit_free_tracked();\n  return rp;\n}}"),
            Some(ty) => uwriteln!(
                c,
                "  {} r = {call};\n  wit_free_tracked();\n  return r;\n}}",
                c_ty(ty)?
            ),
            None => uwriteln!(c, "  {call};\n  wit_free_tracked();\n}}"),
        }
        let js_args: Vec<String> = (0..js_params.len()).map(|n| format!("a{n}")).collect();
        let c_args: Vec<String> = js_params
            .iter()
            .enumerate()
            .map(|(n, ty)| js_to_c(ty, &format!("a{n}")))
            .collect::<Result<_>>()?;
        let c_call = format!("wit_import_{i}({})", c_args.join(", "));
        if *retptr || ret.is_some() {
            uwriteln!(
                js,
                "function $import_{binding}({}) {{ let r = 0; Porffor.c`WIT_SET(r, (f64){c_call});`; return r; }}",
                js_args.join(", ")
            );
        } else {
            uwriteln!(
                js,
                "function $import_{binding}({}) {{ Porffor.c`{c_call};`; }}",
                js_args.join(", ")
            );
        }
    }

    // [resource-new], [resource-rep] and [resource-drop] intrinsics
    for (i, ((module, name, nret), binding)) in bindings
        .resource_imports
        .iter()
        .zip(&bindings.resource_bindings)
        .enumerate()
    {
        let ret = if *nret == 0 { "void" } else { "int32_t" };
        uwriteln!(
            c,
            "__attribute__((import_module(\"{module}\"), import_name(\"{name}\")))\nextern {ret} wit_rsc_{i}(int32_t);"
        );
        if *nret == 0 {
            uwriteln!(
                js,
                "function $resource_{binding}(h) {{ Porffor.c`wit_rsc_{i}(WIT_I32(h));`; }}"
            );
        } else {
            uwriteln!(
                js,
                "function $resource_{binding}(h) {{ let r = 0; Porffor.c`WIT_SET(r, (f64)wit_rsc_{i}(WIT_I32(h)));`; return r; }}"
            );
        }
    }

    // wit.js holds everything, wit-imports/*.js re-export each imported interface
    std::fs::create_dir_all(out_dir.join("wit-imports"))?;
    let mut wit_js = format!(
        "Porffor.c`{}`;\n\n{PRELUDE_JS}\n{js}\n{register}\n",
        c.replace('`', "\\`")
    );
    wit_js.push_str(&bindings.js_bindings);
    let mut aliases = String::new();
    for (specifier, list) in &bindings.import_modules {
        let locals: Vec<&str> = list
            .split(", ")
            .map(|s| s.split(" as ").next().unwrap())
            .collect();
        uwriteln!(wit_js, "\nexport {{ {} }};", locals.join(", "));
        let file = out_dir
            .join("wit-imports")
            .join(format!("{}.js", sanitize(specifier)));
        std::fs::write(&file, format!("export {{ {list} }} from '../wit.js';\n"))?;
        uwriteln!(
            aliases,
            "--alias:{specifier}={}",
            std::fs::canonicalize(&file)?.display()
        );
    }
    std::fs::write(out_dir.join("wit.js"), wit_js)?;
    std::fs::write(out_dir.join("esbuild-aliases.txt"), aliases)?;
    std::fs::write(
        out_dir.join("main.js"),
        format!(
            "import {{ bindExports }} from './wit.js';\nimport * as $source_mod from '{}';\nbindExports($source_mod);\n",
            source.display()
        ),
    )?;
    Ok(())
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn c_ty(ty: &CoreTy) -> Result<&'static str> {
    Ok(match ty {
        CoreTy::I32 => "int32_t",
        CoreTy::F32 => "float",
        CoreTy::F64 => "double",
        CoreTy::I64 => bail!("64-bit core values are not supported yet"),
    })
}

fn c_ret(ty: &Option<CoreTy>) -> Result<&'static str> {
    ty.as_ref().map_or(Ok("void"), c_ty)
}

fn c_params(params: &[CoreTy]) -> Result<String> {
    if params.is_empty() {
        return Ok("void".into());
    }
    let params: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(n, ty)| Ok(format!("{} a{n}", c_ty(ty)?)))
        .collect::<Result<_>>()?;
    Ok(params.join(", "))
}

fn js_to_c(ty: &CoreTy, local: &str) -> Result<String> {
    Ok(match ty {
        CoreTy::I32 => format!("WIT_I32({local})"),
        CoreTy::F32 => format!("(float)WIT_NUM({local})"),
        CoreTy::F64 => format!("WIT_NUM({local})"),
        CoreTy::I64 => bail!("64-bit core values are not supported yet"),
    })
}
