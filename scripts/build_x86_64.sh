# Build kernel and os crate to make bootable kernel image
cargo run

cargo build -p kernel --target x86_64-unknown-none
cp ./target/x86_64-unknown-none/debug/kernel ./build/kernel