#![allow(missing_docs)]

use std::env;

fn main() {
    if env::var("CI").as_deref() == Ok("true") {
        println!("cargo:rustc-cfg=feature=\"__ci\"");
    }
}
