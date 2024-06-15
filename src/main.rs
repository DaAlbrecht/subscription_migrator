use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use migrate::parse_xml_file;
use std::path::PathBuf;

mod migrate;

#[derive(Parser)]
#[command(name = "Migrator")]
#[command(version = "1.0")]
#[command(about = "migrate subscription from xml to yaml", long_about = None)]
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

#[derive(Args)]
struct SingleArgs {
    #[arg(long, short)]
    input_dir: PathBuf,
    #[arg(long, short)]
    output_dir: PathBuf,
}

#[derive(Args)]
struct BulkArgs {
    #[arg(long, short, default_value = ".")]
    path: PathBuf,
    #[arg(long, short)]
    name_prefix: String,
    #[arg(long, short, default_value = ".")]
    output_path: PathBuf,
    #[arg(long, short, default_value = "All")]
    environments: Environment,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Environment {
    All,
    Dev,
    Test,
    Prod,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Single(args) => {
            let _ = migrate_single(args.input_dir.to_str().unwrap());
        }
        Commands::Bulk(args) => {
            let _ = migrate_bulk(args.name_prefix.as_str());
        }
    }
}

fn migrate_bulk(prefix: &str) -> Result<()> {
    todo!()
}

fn migrate_single(directory_name: &str) -> Result<()> {
    let directory = PathBuf::from(directory_name);

    if !directory.exists() {
        println!("Directory does not exist");
        return Err(anyhow::anyhow!("Directory does not exist"));
    }

    let file = directory.join("subscription.xml");

    if !file.exists() {
        println!("File does not exist");
        return Err(anyhow::anyhow!("File does not exist"));
    }

    let content = std::fs::read_to_string(file)?;
    println!("{}", content);
    _ = parse_xml_file(content.as_str());

    Ok(())
}
