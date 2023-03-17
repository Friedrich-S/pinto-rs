use crate::disks::assemble_disk;
use crate::disks::DiskAlign;
use crate::disks::DiskFormat;
use crate::disks::DiskPart;
use crate::disks::Role;
use crate::disks::LOADER_SIZE;
use clap::Args;
use clap::ValueEnum;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

#[derive(Args, Debug)]
pub struct RunArgs {
    /// The simulator to use when running PintOS.
    #[arg(long, default_value = "qemu")]
    sim: Simulator,
    /// What debugger to use if any.
    #[arg(long, default_value = "none")]
    debugger: Debugger,
    /// The amount of physical memory to make available to PintOS in MB.
    #[arg(long, default_value = "4")]
    mem: usize,
    /// The file to use as the bootstrap loader
    #[arg(long, default_value = "loader_alt.bin")]
    loader: String,
    /// A space separated list of arguments to pass to the kernel.
    #[arg(long, default_value = "")]
    args: String,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum Simulator {
    Qemu,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum Debugger {
    None,
    Gdb,
}

pub fn run(args: RunArgs) {
    let mut disks = Vec::new();

    // Keep a list of temporary files we created so that they remain available for
    // the emulator but get still removed when the program exists.
    let mut tmp_files = Vec::new();
    find_disks(&mut disks, &args, &mut tmp_files);

    match args.sim {
        Simulator::Qemu => run_qemu(disks.iter().map(|v| v.as_str()), args.mem, args.debugger),
    }
}

fn find_disks(disks: &mut Vec<String>, args: &RunArgs, tmp_files: &mut Vec<NamedTempFile>) {
    // Find the filesys disk
    if let Some(path) = find_file("filesys.dsk") {
        disks.push(path);
    }

    // Find the swap disk
    if let Some(path) = find_file("swap.dsk") {
        disks.push(path);
    }

    // Build the boot disk
    let kernel = find_file("kernel.bin").expect("Cannot find kernel");
    let mut parts = HashMap::new();
    let kernel_len = std::fs::metadata(&kernel).unwrap().len() as usize;
    parts.insert(
        Role::Kernel,
        DiskPart {
            path: kernel,
            offset: 0,
            bytes: kernel_len,
        },
    );
    let mut boot_disk = tempfile::Builder::new().suffix(".dsk").tempfile().unwrap();
    let loader = {
        let data = std::fs::read(&find_file(&args.loader).expect("Cannot find loader")).unwrap();
        assert!(data.len() == LOADER_SIZE || data.len() == 512);
        data
    };

    assemble_disk(
        boot_disk.as_file_mut(),
        &parts,
        Some(loader[..LOADER_SIZE].try_into().unwrap()),
        None,
        DiskAlign::Bochs,
        DiskFormat::Partitioned,
        &args.args.split(' ').collect::<Vec<_>>(),
    );
    disks.insert(0, boot_disk.path().to_str().unwrap().to_owned());

    tmp_files.push(boot_disk);
}

fn find_file(name: &str) -> Option<String> {
    let path = PathBuf::from(name);

    if path.exists() {
        return Some(path.to_str().unwrap().to_owned());
    }

    let path = Path::new("build").join(path);
    if path.exists() {
        return Some(path.to_str().unwrap().to_owned());
    }

    None
}

fn run_qemu<'a>(disks: impl IntoIterator<Item = &'a str>, mem: usize, debugger: Debugger) {
    let mut cmd = Command::new("qemu-system-i386");

    let disk_names = ["-hda", "-hdb", "-hdc", "-hdd"];
    for (&name, path) in disk_names.iter().zip(disks.into_iter()) {
        cmd.args([name, path]);
    }

    cmd.args(["-m", &mem.to_string()]);
    cmd.args(["-serial", "file:qemu_log.txt"]);
    cmd.args(["-device", "isa-debug-exit"]);
    cmd.args(["-nographic"]);

    match debugger {
        Debugger::None => (),
        Debugger::Gdb => {
            cmd.args(["-s", "-S"]);
        }
    }

    let mut proc = cmd.spawn().expect("Unable to spawn QEMU process");
    proc.wait().unwrap();
}
