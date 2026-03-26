fn main() {
    println!("cargo:rerun-if-changed=src/bluetooth/rfcomm.m");

    cc::Build::new()
        .file("src/bluetooth/rfcomm.m")
        .flag("-fobjc-arc")
        .compile("rfcomm");

    println!("cargo:rustc-link-lib=framework=IOBluetooth");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
