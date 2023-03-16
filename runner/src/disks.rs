use std::collections::HashMap;
use std::io::Write;

pub const LOADER_SIZE: usize = 314;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Kernel,
    Filesys,
    Scratch,
    Swap,
}

impl Role {
    pub const ORDER: &[Self] = &[Self::Kernel, Self::Filesys, Self::Scratch, Self::Swap];
}

pub fn assemble_disk(output: &mut impl Write, sections: &HashMap<Role, Vec<u8>>, loader: Option<&[u8; LOADER_SIZE]>) {}
