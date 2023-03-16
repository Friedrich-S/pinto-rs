sh ./scripts/build_x86_64.sh

#qemu-system-i386 -drive format=raw,file=build/bios/pintos.img \
#    -nographic -serial file:"qemu_log.txt" -device isa-debug-exit

cargo run -p runner