fn main() {
    // Link Windows Security API library
    #[cfg(windows)]
    {
        println!("cargo:rustc-link-lib=Advapi32");
    }
}
