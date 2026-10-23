use anyhow::Result;
use std::{fs, path::Path};
use wasm_encoder_old::Section;
mod boa_a;
#[path = "../../../vendor/componentize-codegen/codegen.rs"]
mod componentize;
mod porffor_a;
fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("wit") {
        return porffor_a::generate();
    }
    let out = Path::new("build/generated");
    fs::create_dir_all(out)?;
    boa_a::generate(out)?;
    let mut resolve = wit_parser::Resolve::default();
    let (pkg, _) = resolve.push_path("wit")?;
    let world = resolve.select_world(&[pkg], Some("bench"))?;
    let mut files = wit_bindgen_core::Files::default();
    wit_bindgen_c::Opts::default()
        .build()
        .generate(&mut resolve, world, &mut files)?;
    for (name, bytes) in files.iter() {
        fs::write(out.join(name), bytes)?;
    }
    let mut resolve = wit_parser_old::Resolve::default();
    let (pkg, _) = resolve.push_path("wit")?;
    let world = resolve.select_world(&[pkg], Some("bench"))?;
    let (mut dylib, metadata) = wit_dylib::create_with_metadata(
        &resolve,
        world,
        Some(&mut wit_dylib::DylibOpts {
            interpreter: Some("libengine.so".into()),
            async_: Default::default(),
        }),
    );
    wasm_encoder_old::CustomSection {
        name: "component-type:comparison".into(),
        data: wit_component_old::metadata::encode(
            &resolve,
            world,
            wit_component_old::StringEncoding::UTF8,
            None,
        )?
        .into(),
    }
    .append_to(&mut dylib);
    fs::write(out.join("world.so"), dylib)?;
    let code = componentize::generate(&metadata);
    fs::write(out.join("globals.js"), &code.globals)?;
    fs::write(out.join("async-exports.js"), &code.script)?;
    let mut modules = String::from("const IMPORT_MODULES: &[(&str, &str)] = &[\n");
    let mut aliases = String::new();
    for (i, (name, code)) in code.modules.iter().enumerate() {
        fs::write(out.join(format!("import-{i}.js")), code)?;
        modules.push_str(&format!("({name:?},{code:?}),\n"));
        aliases.push_str(&format!(
            "--alias:{name}={}/import-{i}.js\n",
            fs::canonicalize(out)?.display()
        ));
    }
    modules.push_str("];\n");
    let names: Vec<_> = metadata
        .export_funcs
        .iter()
        .map(|f| {
            use heck::ToLowerCamelCase;
            f.name.to_lower_camel_case()
        })
        .collect();
    modules.push_str(&format!("const EXPORT_NAMES: &[&str] = &{:?};\n", names));
    fs::write(out.join("modules.rs"), modules)?;
    fs::write(out.join("native-aliases.txt"), aliases)?;

    fs::write(out.join("exports.json"), serde_json::to_vec_pretty(&names)?)?;
    Ok(())
}
