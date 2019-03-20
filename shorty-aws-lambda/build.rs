fn main() {
    let os = std::env::consts::OS;

    // according to https://medium.com/@kkostov/rust-aws-lambda-30a1b92d4009
    // on macos linker executable is named "x86_64-linux-musl-gcc" instead of "musl-gcc"
    if os == "macos" {
        std::env::set_var("RUSTC_LINKER", "x86_64-linux-musl-gcc");
    }
}
