// extern crate embed_resource;
extern crate winresource;

// use std::path::Path;

// use dotenvy::dotenv;
// use std::env;
use anyhow::Result;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    // dotenv().ok();

    // println!("cargo:rerun-if-changed=assets/gui/test.fl");
    // let g = fl2rust::Generator::default();
    // // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    // g.in_out(
    //     "assets/gui/test.fl",
    //     // out_path.join("src/ui4/test.rs").to_str().unwrap(),
    //     "assets/gui/test.rs",
    // )
    // .expect("Failed to generate rust from fl file!");

    // let g = fl2rust::Generator::default();
    // for entry in std::fs::read_dir(Path::new("assets/gui"))? {
    //     let entry = entry?;
    //     let path = entry.path();
    //     if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("fl") {
    //         println!("cargo:rerun-if-changed={}", entry.path().to_str().unwrap());
    //         g.in_out(
    //             entry.path().to_str().unwrap(),
    //             &format!(
    //                 "assets/gui/{}.rs",
    //                 entry.path().file_stem().unwrap().to_str().unwrap()
    //             ),
    //         )
    //         .expect("Failed to generate rust from fl file!");
    //     }
    // }

    EmitBuilder::builder()
        // .all_build()
        // .all_cargo()
        .all_git()
        .git_sha(true)
        // .all_rustc()
        // .all_sysinfo()
        .emit()?;

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res
            // .set_icon("icon.ico");
            .set_icon_with_id("icon3.ico", "1")
            .compile()
            .unwrap();
    }

    // panic!()
    Ok(())
}
