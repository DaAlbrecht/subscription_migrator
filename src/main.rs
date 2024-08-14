use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use migrate::{parse_xml_file, unify_applilcations, write_to_file, YamlApiSubscription};
use std::path::PathBuf;

mod migrate;

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
        Commands::Single(args) => migrate_single(args, &config),
        Commands::Bulk(args) => migrate_bulk(args, &config),
    }
}

fn migrate_bulk(args: BulkArgs, config: &Config) -> Result<()> {
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
    let yaml_applications = unify_applilcations(&staged_applications, config);
    let files_written = write_to_file(&yaml_applications, args.output_path, args.force)?;
    for file in files_written {
        println!("File written: {:?}", file);
    }

    Ok(())
}

fn migrate_single(args: SingleArgs, config: &Config) -> Result<()> {
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
        .map(|app| {
            let mut yaml_app: YamlApiSubscription = app.into();
            for env in &mut yaml_app.environments {
                match env.environments.iter().any(|e| e.name == "prod") {
                    true => {
                        env.control_plane_url = config.prod_plane_url.to_string();
                    }
                    false => {
                        env.control_plane_url = config.npr_plane_url.to_string();
                    }
                }
            }
            yaml_app
        })
        .collect::<Vec<YamlApiSubscription>>();

    let files_written = write_to_file(&yaml_applications, args.output_dir, args.force)?;
    for file in files_written {
        println!("File written: {:?}", file);
    }

    Ok(())
}
