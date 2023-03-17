# Pinto-rs bootloader

This crate contains the loader code for the boot partition. It is very similar to the
original PintOS `loader.S` code. 

## Development

The Kernel is automatically built when running `./scripts/build_x86_64.sh`. During the
compilation there might be a linker error regarding overlapping segments. This implies
that the code has grown too large and doesn't fit into the maximum size of the bootloader
anymore.

`objdump -xsdS -M i8086,intel target/i386-boot-sector/release/loader` can be helpful when
debugging the size of the generated binary. 

### Code Size

The following is a (non exhaustive) list of design choices that were made to reduce the
size of the loader binary. Some of the choices and others not mentioned here are also
documented in the loader code.

1. The code is compiled to a non `code16` target. This saves space with Rust, because the
   the compiler toolchain is not made to optimize for binary size on such a level. It would
   for example frequently emit `calld` instructions taking up 6 bytes each.
2. The code is built with `RUSTFLAGS="-C llvm-args=-align-all-functions=2 -C llvm-args=-align-all-blocks=2"`
   (see `./scripts/build_x86_64.sh`) to force LLVM to remove padding between functions. Previously,
   they used up and wasted a lot of space. In this case we don't care about performance and so it
   should be OK to remove the padding. However, it is not yet confirmed whether this impacts the
   functionality of the code.
3. Some frequently called functions have been set to `inline(never)` to help with code size. However,
   this may not always work and it is generally advised to refrain from using this attribute. LLVM
   does very well without this attribute and fully utilizes its freedom for optimization.