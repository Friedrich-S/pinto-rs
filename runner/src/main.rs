use crate::run::RunArgs;
use clap::Parser;
use clap::Subcommand;

mod disks;
mod run;

#[derive(Parser, Debug)]
#[command(name = "Pinto-rs Runner", version)]
struct Args {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    Run {
        #[command(flatten)]
        args: RunArgs,
    },
}

fn main() {
    let args = Args::parse();

    match args.mode {
        Mode::Run { args } => {
            run::run(args);
        }
    }
}
