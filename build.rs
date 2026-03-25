fn main() {
    println!("cargo:rerun-if-changed=src/rfcomm.m");

    cc::Build::new()
        .file("src/rfcomm.m")
        .flag("-fobjc-arc")
        .compile("rfcomm");

    println!("cargo:rustc-link-lib=framework=IOBluetooth");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
