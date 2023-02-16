fn main() {
    // Save the bootable image in the build directory
    let img_path = env!("BIOS_PATH");
    std::fs::create_dir_all("./build/bios/").unwrap();
    std::fs::copy(img_path, "./build/bios/pintos.img").unwrap();
}
