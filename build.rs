fn main() {
    println!("cargo:rerun-if-changed=targets/x86_64.ld");
    println!("cargo:rustc-link-arg=-Ttargets/x86_64.ld");
}
