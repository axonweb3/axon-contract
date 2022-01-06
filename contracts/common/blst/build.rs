use std::{env, path::Path};

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!(
        "cargo:rustc-link-search=native={}",
        Path::new(&dir).join("ckb-lib-blst/build").display()
    );
    println!("cargo:rustc-link-lib=static=bls12_381_sighash_all");
}
