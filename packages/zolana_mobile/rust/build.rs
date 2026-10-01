//! Android devices with 16 KB memory pages load only libraries whose segments
//! are 16 KB aligned, and Google Play requires it. NDK r28 aligns by default;
//! older NDKs link with 4 KB pages unless told otherwise.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    }
}
