use std::{env, path::Path};

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!(
        "cargo:rustc-link-search=native={}",
        Path::new(&dir).join("ckb-lib-secp256k1/build").display()
    );
    println!("cargo:rustc-link-lib=static=ckb-lib-secp256k1");
}
