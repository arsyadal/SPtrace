use clap::Parser;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:?}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = sptrace::cli::Cli::parse();
    sptrace::execute(cli)
}
