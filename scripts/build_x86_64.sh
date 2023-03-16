RUSTFLAGS="" \
    cargo +nightly build -Z build-std=core -Z build-std-features=compiler-builtins-mem \
    -p loader --release --target ./targets/i386-boot-sector.json
RUSTFLAGS="-C link-arg=-Tkernel_link.x" \
    cargo +nightly build -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem \
    -p kernel --target ./targets/i386-unknown-none.json
    
cp ./target/i386-unknown-none/debug/kernel ./build/kernel
objcopy -I elf32-i386 -O binary ./target/i386-boot-sector/release/loader ./build/loader.bin