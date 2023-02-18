sh ./scripts/build_x86_64.sh

qemu-system-x86_64 -drive format=raw,file=build/bios/pintos.img -serial file:"qemu_log.txt" -device isa-debug-exit