use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Tell cargo to look for shared libraries in the native/lib directory
    println!("cargo:rustc-link-search=native={}/native/lib", manifest_dir);

    // Tell cargo to tell rustc to link the rune library
    println!("cargo:rustc-link-lib=static=rune");

    // Link against libc (required by Rune)
    println!("cargo:rustc-link-lib=c");

    // Invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=native/lib/librune.a");
    println!("cargo:rerun-if-changed=native/include/rune.h");
    println!("cargo:rerun-if-changed=build.rs");
}