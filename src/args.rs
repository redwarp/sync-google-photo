#[derive(clap::Parser)]
pub struct Cli {
    #[clap(short, long)]
    pub configure: bool,
}
