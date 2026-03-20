#![allow(unused_variables, dead_code)]

mod cli;
mod config;
mod dns;
mod exit;
mod expose;
mod mesh;
mod nat;
mod rosenpass;
mod tunnel;

use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("yaya=info".parse()?))
        .init();

    let cli = cli::Cli::parse();
    cli::run(cli).await
}
