fn main() {
    //println!("cargo:rustc-link-arg=-N");
    //println!("cargo:rustc-link-arg=-entry 0");
    //println!("cargo:rustc-link-arg=-Ttext=0x7c00");
    //println!("cargo:rustc-link-arg=--oformat=binary");
    //println!("cargo:rustc-link-arg=--strip-all");
    println!("cargo:rustc-link-arg=--script=loader_link.ld");
}
