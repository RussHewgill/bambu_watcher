// extern crate embed_resource;
use dotenvy::dotenv;
use std::env;

fn main() {
    dotenv().ok();

    for (key, value) in env::vars() {
        println!("cargo:rustc-env={}={}", key, value);
    }
    //     embed_resource::compile("tray.rc", embed_resource::NONE);
}
