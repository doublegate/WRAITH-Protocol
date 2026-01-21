// Build script for UniFFI scaffolding generation
// Using proc-macro approach, so minimal build script needed

fn main() {
    // UniFFI proc macros handle scaffolding generation
    // This file can be kept minimal or used for additional build steps
    println!("cargo:rerun-if-changed=src/lib.rs");
}
