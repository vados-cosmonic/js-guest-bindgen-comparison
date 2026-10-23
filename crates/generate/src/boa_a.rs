//! Generates the JS bindings for `../../wit` with js-component-bindgen, and the
//! Rust core exports/imports that connect them to the component model.

use std::fmt::Write as _;

use anyhow::{Result, bail};
use js_guest_bindgen::{CoreFn, CoreTy, GuestOpts, guest_bindgen};
use wit_parser::Resolve;

macro_rules! uwriteln {
    ($dst:expr, $($arg:tt)*) => {
        writeln!($dst, $($arg)*).unwrap()
    };
}

/// Intrinsics provided by `src/prelude.js`, since Boa cannot view linear memory
const OVERRIDDEN_INTRINSICS: &[&str] = &[
    "WebAssemblyRuntimeError",
    "dataView",
    "memoryView",
    "memoryWrite",
    "TEXT_DECODER_UTF8",
    "_utf8AllocateAndEncode",
];

pub fn generate(out_dir: &std::path::Path) -> Result<()> {
    println!("cargo::rerun-if-changed=../../wit");

    let mut resolve = Resolve::default();
    let (pkg, _) = resolve.push_path("wit")?;
    let world = resolve.select_world(&[pkg], Some("bench"))?;
    let opts = GuestOpts {
        overridden_intrinsics: OVERRIDDEN_INTRINSICS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        on_bind: Some("$wit_register".into()),
        ..GuestOpts::default()
    };
    let bindings = guest_bindgen(&resolve, world, &opts)?;

    let mut rs = String::new();
    let mut js = String::new();

    // exports: core function the host calls -> JS binding registered as callback i
    let mut register = String::from("function $wit_register() {\n");
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
        let mut args: Vec<String> = (0..params.len()).map(|n| format!("a{n} as f64")).collect();
        uwriteln!(
            rs,
            "#[unsafe(export_name = \"{core_name}\")]\nextern \"C\" fn wit_export_{i}({}){} {{",
            rs_params(params)?,
            rs_ret(ret)?
        );
        if *retptr {
            uwriteln!(rs, "    let rp = crate::alloc_ret({retsize});");
            args.push("rp as f64".into());
        }
        uwriteln!(
            rs,
            "    let r = crate::call_export({i}, &[{}]);",
            args.join(", ")
        );
        match ret {
            _ if *retptr => uwriteln!(rs, "    let _ = r;\n    rp\n}}"),
            Some(ty) => uwriteln!(
                rs,
                "    crate::free_tracked();\n    {}\n}}",
                from_f64(ty, "r")?
            ),
            None => uwriteln!(rs, "    let _ = r;\n    crate::free_tracked();\n}}"),
        }
        if *retptr {
            uwriteln!(
                rs,
                "#[unsafe(export_name = \"cabi_post_{core_name}\")]\nextern \"C\" fn wit_post_{i}(rp: i32) {{\n    unsafe {{ crate::free(rp as *mut u8) }};\n    crate::free_tracked();\n}}"
            );
        }
        uwriteln!(register, "  __wit_register({i}, {});", item.binding_name);
    }
    register.push_str("}\n");

    // imports: JS binding -> call_import(i) -> core import. Bindings use the
    // export ABI, so the return area is provided here and handed back to JS.
    let mut dispatch =
        String::from("pub(crate) fn call_import(i: usize, a: &[f64]) -> f64 {\n    match i {\n");
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
            rs,
            "#[link(wasm_import_module = \"{module}\")]\nunsafe extern \"C\" {{\n    #[link_name = \"{canon}\"]\n    fn wit_import_{i}({}){};\n}}",
            rs_params(params)?,
            rs_ret(ret)?
        );
        let js_params = &params[..params.len() - *retptr as usize];
        let mut call_args: Vec<String> = js_params
            .iter()
            .enumerate()
            .map(|(n, ty)| from_f64(ty, &format!("a[{n}]")))
            .collect::<Result<_>>()?;
        uwriteln!(dispatch, "        {i} => {{");
        if *retptr {
            uwriteln!(
                dispatch,
                "            let rp = crate::alloc_ret({retsize});"
            );
            call_args.push("rp".into());
        }
        let call = format!("unsafe {{ wit_import_{i}({}) }}", call_args.join(", "));
        match ret {
            _ if *retptr => uwriteln!(
                dispatch,
                "            {call};\n            crate::free_tracked();\n            rp as f64"
            ),
            Some(_) => uwriteln!(
                dispatch,
                "            let r = {call};\n            crate::free_tracked();\n            r as f64"
            ),
            None => uwriteln!(
                dispatch,
                "            {call};\n            crate::free_tracked();\n            0.0"
            ),
        }
        dispatch.push_str("        }\n");
        uwriteln!(
            js,
            "function $import_{binding}(...args) {{ return __wit_import({i}, ...args); }}"
        );
    }
    dispatch.push_str("        _ => unreachable!(),\n    }\n}\n");
    rs.push_str(&dispatch);

    // [resource-new], [resource-rep] and [resource-drop] intrinsics
    let mut dispatch =
        String::from("pub(crate) fn call_resource(i: usize, h: i32) -> f64 {\n    match i {\n");
    for (i, ((module, name, nret), binding)) in bindings
        .resource_imports
        .iter()
        .zip(&bindings.resource_bindings)
        .enumerate()
    {
        let ret = if *nret == 0 { "" } else { " -> i32" };
        uwriteln!(
            rs,
            "#[link(wasm_import_module = \"{module}\")]\nunsafe extern \"C\" {{\n    #[link_name = \"{name}\"]\n    fn wit_resource_{i}(h: i32){ret};\n}}"
        );
        if *nret == 0 {
            uwriteln!(
                dispatch,
                "        {i} => {{\n            unsafe {{ wit_resource_{i}(h) }};\n            0.0\n        }}"
            );
        } else {
            uwriteln!(
                dispatch,
                "        {i} => (unsafe {{ wit_resource_{i}(h) }}) as f64,"
            );
        }
        uwriteln!(
            js,
            "function $resource_{binding}(h) {{ return __wit_resource({i}, h); }}"
        );
    }
    dispatch.push_str("        _ => unreachable!(),\n    }\n}\n");
    rs.push_str(&dispatch);

    // one module for the bindings, plus a re-export module per imported interface
    let mut wit_js = format!("{js}\n{register}\n{}", bindings.js_bindings);
    let mut modules = String::from("pub(crate) const IMPORT_MODULES: &[(&str, &str)] = &[\n");
    for (specifier, list) in &bindings.import_modules {
        let locals: Vec<&str> = list
            .split(", ")
            .map(|s| s.split(" as ").next().unwrap())
            .collect();
        uwriteln!(wit_js, "\nexport {{ {} }};", locals.join(", "));
        uwriteln!(
            modules,
            "    ({specifier:?}, \"export {{ {list} }} from 'wit';\"),"
        );
    }
    modules.push_str("];\n");
    rs.push_str(&modules);
    uwriteln!(
        rs,
        "pub(crate) const EXPORT_COUNT: usize = {};",
        bindings.exports.len()
    );

    std::fs::write(out_dir.join("wit.js"), wit_js)?;
    std::fs::write(out_dir.join("bindings.rs"), rs)?;
    Ok(())
}

fn rs_ty(ty: &CoreTy) -> Result<&'static str> {
    Ok(match ty {
        CoreTy::I32 => "i32",
        CoreTy::F32 => "f32",
        CoreTy::F64 => "f64",
        CoreTy::I64 => bail!("64-bit core values are not supported yet"),
    })
}

fn rs_ret(ty: &Option<CoreTy>) -> Result<String> {
    Ok(match ty {
        Some(ty) => format!(" -> {}", rs_ty(ty)?),
        None => String::new(),
    })
}

fn rs_params(params: &[CoreTy]) -> Result<String> {
    let params: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(n, ty)| Ok(format!("a{n}: {}", rs_ty(ty)?)))
        .collect::<Result<_>>()?;
    Ok(params.join(", "))
}

fn from_f64(ty: &CoreTy, value: &str) -> Result<String> {
    Ok(match ty {
        CoreTy::I32 => format!("{value} as i64 as i32"),
        CoreTy::F32 => format!("{value} as f32"),
        CoreTy::F64 => value.to_string(),
        CoreTy::I64 => bail!("64-bit core values are not supported yet"),
    })
}
