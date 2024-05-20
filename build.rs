// extern crate embed_resource;
// use dotenvy::dotenv;
// use std::env;
use anyhow::Result;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    // dotenv().ok();

    // for (key, value) in env::vars() {
    //     println!("cargo:rustc-env={}={}", key, value);
    // }
    //     embed_resource::compile("tray.rc", embed_resource::NONE);

    EmitBuilder::builder()
        // .all_build()
        // .all_cargo()
        .all_git()
        .git_sha(true)
        // .all_rustc()
        // .all_sysinfo()
        .emit()?;

    Ok(())
}
