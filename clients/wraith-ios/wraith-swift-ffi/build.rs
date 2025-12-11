// Build script for UniFFI scaffolding generation

fn main() {
    // Generate UniFFI scaffolding
    uniffi::generate_scaffolding("src/wraith.udl").unwrap();
}
