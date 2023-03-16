use clap::Args;
use clap::ValueEnum;
use std::process::Command;

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
    #[arg(long, default_value = "loader.bin")]
    loader: String,
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

pub fn run(args: RunArgs) {}

fn run_qemu<'a>(disks: impl IntoIterator<Item = &'a str>, mem: usize, debugger: Debugger) {
    let mut cmd = Command::new("qemu-system-i386");

    let disk_names = ["-hda", "-hdb", "-hdc", "-hdd"];
    for (&name, path) in disk_names.iter().zip(disks.into_iter()) {
        cmd.args([name, path]);
    }

    cmd.args(["-m", &mem.to_string()]);

    match debugger {
        Debugger::None => (),
        Debugger::Gdb => {
            cmd.args(["-s", "-S"]);
        }
    }

    let mut proc = cmd.spawn().expect("Unable to spawn QEMU process");
    proc.wait().unwrap();
}
