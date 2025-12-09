// build.rs

use std::env;
use std::path::PathBuf;

extern crate bindgen;

fn main() {
    // 1. Tell cargo to link the compiled libtinyfseq library.

    // 2. Locate the header file using the Nix environment variable.
    let tinyfseq_include_dir = env::var("TINYFSEQ_INCLUDE_DIR")
        .expect("TINYFSEQ_INCLUDE_DIR must be set by Nix shell");
    let tinyfseq_lib_dir = env::var("TINYFSEQ_LIB_DIR")
        .expect("TINYFSEQ_LIB_DIR must be set by Nix shell");

    println!("cargo:rustc-link-search=native={}", tinyfseq_lib_dir);
    println!("cargo:rustc-link-lib=static=tinyfseq");
    println!("cargo:rerun-if-changed=tinyfseq.h");

    // 3. Generate the FFI bindings using bindgen.
    let bindings = bindgen::Builder::default()
        // The path to the header file
        .header(format!("{}/tinyfseq.h", tinyfseq_include_dir)) // Use tinyfseq.h as specified in nix/packages.nix
        // Set the search path for C headers (redundant but safe with BINDGEN_EXTRA_CLANG_ARGS set in Nix)
        .clang_arg(format!("-I{}", tinyfseq_include_dir)) 
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_type("TFError")
        .allowlist_type("TFCompressionType")
        .allowlist_type("TFHeader")
        .allowlist_type("TFCompressionBlock")
        .allowlist_type("TFVarHeader")
        .allowlist_type("TFChannelRange")
        .allowlist_function("TFError_string")
        .allowlist_function("TFHeader_read")
        .allowlist_function("TFCompressionBlock_read")
        .allowlist_function("TFVarHeader_read")
        .allowlist_function("TFChannelRange_read")
        // Finish the builder and generate the bindings.
        .generate()
        .expect("Unable to generate bindings for libtinyfseq.h");

    // 4. Write the bindings to a file in the $OUT_DIR
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("tinyfseq_bindings.rs"))
        .expect("Couldn't write bindings!");
}
