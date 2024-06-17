use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use migrate::{parse_xml_file, unify_applilcations, write_to_file, YamlApiSubscription};
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
    #[arg(long, short, default_value = "false")]
    force: bool,
}

#[derive(Args)]
struct BulkArgs {
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Environment {
    All,
    Dev,
    Test,
    Prod,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Single(args) => migrate_single(args),
        Commands::Bulk(args) => migrate_bulk(args),
    }
}

fn migrate_bulk(args: BulkArgs) -> Result<()> {
    let directories = std::fs::read_dir(&args.path)?;
    let matching_paths = directories
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.as_ref().unwrap();
            let path = entry.path();
            let is_matching = path.is_dir()
                && path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(&args.name_prefix);
            if is_matching {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<PathBuf>>();

    let mut staged_applications = Vec::new();
    for mut path in matching_paths {
        path = path.join("subscribe.xml");
        let file = std::fs::File::open(path)?;
        let applications = parse_xml_file(&file)?;
        staged_applications.extend(applications);
    }
    let yaml_applications = unify_applilcations(&staged_applications);
    let files_written = write_to_file(&yaml_applications, args.output_path, args.force)?;
    for file in files_written {
        println!("File written: {:?}", file);
    }

    Ok(())
}

fn migrate_single(args: SingleArgs) -> Result<()> {
    let directory = args.input_dir;

    if !directory.exists() {
        println!("Directory does not exist");
        return Err(anyhow::anyhow!("Directory {:?} does not exist", directory));
    }

    let file_path = directory.join("subscribe.xml");

    if !file_path.exists() {
        return Err(anyhow::anyhow!(
            "subscribe.xml does not exist in the directory {:?}",
            directory
        ));
    }

    let file = std::fs::File::open(file_path)?;

    let xml_applications = parse_xml_file(&file)?;
    let yaml_applications = xml_applications
        .into_iter()
        .map(|app| app.into())
        .collect::<Vec<YamlApiSubscription>>();

    let files_written = write_to_file(&yaml_applications, args.output_dir, args.force)?;
    for file in files_written {
        println!("File written: {:?}", file);
    }

    Ok(())
}
