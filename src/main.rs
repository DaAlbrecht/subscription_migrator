use crate::gateway::migrate_gateway;
use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use gateway::migrate_gateway_bulk;
use std::path::PathBuf;
use subscription::{migrate_subscription, migrate_subscription_bulk};

mod gateway;
mod subscription;

#[derive(Parser)]
#[command(name = "Migrator")]
#[command(version = "1.0")]
#[command(
    about = "migrate subscription from xml to yaml, requires NPR_PLANE_URL and PROD_PLANE_URL environment variables"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Migrate a single subscription")]
    Single(SingleArgs),
    #[command(about = "Search all directories within a path for a given prefix")]
    Bulk(BulkArgs),
}

#[derive(Copy, Clone, PartialEq, PartialOrd, ValueEnum)]
enum Kind {
    Subscription,
    Gateway,
}

#[derive(Args)]
struct SingleArgs {
    #[arg(long, short)]
    kind: Kind,
    #[arg(long, short)]
    input_dir: PathBuf,
    #[arg(long, short)]
    output_dir: PathBuf,
    #[arg(long, short, default_value = "false")]
    force: bool,
}

#[derive(Args)]
struct BulkArgs {
    #[arg(long, short)]
    kind: Kind,
    #[arg(long, short, default_value = ".")]
    path: PathBuf,
    #[arg(long, short)]
    name_prefix: String,
    #[arg(long, short, default_value = ".")]
    output_path: PathBuf,
    #[arg(long, short)]
    environments: Environment,
    #[arg(long, short, default_value = "false")]
    force: bool,
}

#[derive(Copy, Clone, PartialEq, PartialOrd, ValueEnum)]
enum Environment {
    All,
    Dev,
    Test,
    Prod,
}

struct Config {
    npr_plane_url: String,
    prod_plane_url: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let non_prod_plane_url = std::env::var("NPR_PLANE_URL");

    let prod_plane_url = std::env::var("PROD_PLANE_URL");

    if non_prod_plane_url.is_err() || prod_plane_url.is_err() {
        return Err(anyhow::anyhow!(
            "Environment variables NPR_PLANE_URL and PROD_PLANE_URL must be set"
        ));
    }

    let non_prod_plane_url = non_prod_plane_url.unwrap();
    let prod_plane_url = prod_plane_url.unwrap();
    let config = Config {
        npr_plane_url: non_prod_plane_url,
        prod_plane_url,
    };

    match cli.command {
        Commands::Single(args) => match args.kind {
            Kind::Subscription => migrate_subscription(args, &config),
            Kind::Gateway => migrate_gateway(args),
        },
        Commands::Bulk(args) => match args.kind {
            Kind::Subscription => migrate_subscription_bulk(args, &config),
            Kind::Gateway => migrate_gateway_bulk(args),
        },
    }
}
