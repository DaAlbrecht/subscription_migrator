use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::PathBuf,
};

use anyhow::Result;
use serde::Serialize;
use xml::{reader::XmlEvent, EventReader};

use crate::{BulkArgs, Config, SingleArgs};

#[derive(Debug, Default, Clone)]
pub(crate) struct XmlApplication {
    name: String,
    ///Maybe not needed
    token_type: String,
    apis: Vec<XmlSubscription>,
    token_validity: Option<i32>,
    refresh_token_validity: Option<i32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct XmlSubscription {
    api_name: String,
    api_version: String,
    env: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct YamlApiSubscription {
    pub environments: Vec<YamlEnvironment>,
    #[serde(rename = "subscriptions")]
    subscription: YamlSubscription,
}

#[derive(Debug, Serialize)]
pub(crate) struct YamlEnvironment {
    #[serde(rename = "controlPlaneUrl")]
    pub control_plane_url: String,
    #[serde(rename = "environment")]
    pub environments: Vec<YamlEnvironmentName>,
}

#[derive(Debug, Serialize)]
pub(crate) struct YamlEnvironmentName {
    pub name: String,
}

#[derive(Debug, Serialize)]
struct YamlSubscription {
    application: YamlApplication,
}

#[derive(Debug, Serialize)]
struct YamlApplication {
    name: String,
    description: String,
    apis: Vec<YamlApi>,
    #[serde(rename = "accessTokenTTL")]
    #[serde(skip_serializing_if = "Option::is_none")]
    access_token_ttl: Option<i32>,
    #[serde(rename = "refreshTokenTTL")]
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token_ttl: Option<i32>,
}

#[derive(Debug, Serialize)]
struct YamlApi {
    name: String,
    version: String,
}

impl From<XmlApplication> for YamlApiSubscription {
    fn from(app: XmlApplication) -> Self {
        let mut environments = Vec::new();
        let non_prod_envs: HashSet<String> = app
            .apis
            .iter()
            .filter(|sub| sub.env.iter().any(|env| env != "prod"))
            .flat_map(|sub| sub.env.clone())
            .collect();

        let prod_envs: HashSet<String> = app
            .apis
            .iter()
            .filter(|sub| sub.env.iter().any(|env| env == "prod"))
            .flat_map(|sub| sub.env.clone())
            .collect();

        let yaml_prod_names = prod_envs
            .iter()
            .map(|env| YamlEnvironmentName { name: env.clone() })
            .collect::<Vec<_>>();

        let yaml_non_prod_names = non_prod_envs
            .iter()
            .map(|env| YamlEnvironmentName { name: env.clone() });

        let yaml_env_non_prod = YamlEnvironment {
            control_plane_url: "".to_string(),
            environments: yaml_non_prod_names.collect(),
        };

        let yaml_env_prod = YamlEnvironment {
            control_plane_url: "".to_string(),
            environments: yaml_prod_names,
        };

        if !non_prod_envs.is_empty() {
            environments.push(yaml_env_non_prod);
        }
        if !prod_envs.is_empty() {
            environments.push(yaml_env_prod);
        }

        let access_token_ttl = app.token_validity;
        let refresh_token_ttl = app.refresh_token_validity;

        let apis = app
            .apis
            .iter()
            .map(|sub| YamlApi {
                name: sub.api_name.clone(),
                version: sub.api_version.clone(),
            })
            .collect::<Vec<_>>();

        let description = format!("{}-subscription", app.name);

        let app = YamlApplication {
            name: app.name,
            description,
            apis,
            access_token_ttl,
            refresh_token_ttl,
        };

        let subscription = YamlSubscription { application: app };

        YamlApiSubscription {
            environments,
            subscription,
        }
    }
}

pub(crate) fn migrate_subscription(args: SingleArgs, config: &Config) -> Result<()> {
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

pub(crate) fn migrate_subscription_bulk(args: BulkArgs, config: &Config) -> Result<()> {
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

pub(crate) fn parse_xml_file(file: impl Read) -> Result<Vec<XmlApplication>> {
    let parser = EventReader::new(file);
    let mut app = XmlApplication::default();
    let mut applications = Vec::new();
    let mut subscriptions = Vec::new();

    for event in parser {
        match event {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if name.local_name.as_str() == "application" {
                    app = parse_application(&attributes);
                }
                if name.local_name.as_str() == "subscription" {
                    let sub = parse_subscription(&attributes);
                    subscriptions.push(sub);
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name.as_str() == "application" {
                    app.apis.clone_from(&subscriptions);
                    applications.push(app.clone());
                    subscriptions.clear();
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error: {:?}", e));
            }
            _ => {}
        }
    }

    Ok(applications)
}

fn parse_application(attributes: &[xml::attribute::OwnedAttribute]) -> XmlApplication {
    let mut name = String::new();
    let mut token_type = String::new();
    let mut token_validity = None;
    let mut refresh_token_validity = None;

    for attr in attributes {
        match attr.name.local_name.as_str() {
            "name" => name.clone_from(&attr.value),
            "tokenType" => token_type.clone_from(&attr.value),
            "tokenValidity" => token_validity = Some(attr.value.parse().unwrap()),
            "userTokenValidity" => token_validity = Some(attr.value.parse().unwrap()),
            "refreshTokenValidity" => refresh_token_validity = Some(attr.value.parse().unwrap()),
            _ => {}
        }
    }

    XmlApplication {
        name,
        token_type,
        apis: Vec::new(),
        token_validity,
        refresh_token_validity,
    }
}

fn parse_subscription(attributes: &[xml::attribute::OwnedAttribute]) -> XmlSubscription {
    let mut api_name = String::new();
    let mut api_version = String::new();
    let mut env = Vec::new();

    for attr in attributes {
        match attr.name.local_name.as_str() {
            "apiName" => api_name.clone_from(&attr.value),
            "apiVersion" => api_version.clone_from(&attr.value),
            "environment" => env.push(attr.value.clone()),
            _ => {}
        }
    }

    XmlSubscription {
        api_name,
        api_version,
        env,
    }
}

pub fn write_to_file(
    applications: &[YamlApiSubscription],
    base_path: PathBuf,
    force: bool,
) -> Result<Vec<PathBuf>> {
    let mut files_written = Vec::new();
    for app in applications {
        let dir_name = format!("{}-{}", app.subscription.application.name, "subscription");
        let mut project_path = base_path.join(dir_name);

        if project_path.exists() && !force {
            return Err(anyhow::anyhow!("Directory already exists"));
        }

        std::fs::create_dir_all(&project_path)?;

        project_path = project_path.join("subscription.yaml");

        std::fs::write(project_path.clone(), serde_yaml::to_string(&app)?)?;
        files_written.push(project_path);
    }
    Ok(files_written)
}

pub fn unify_applilcations(
    applications: &[XmlApplication],
    config: &Config,
) -> Vec<YamlApiSubscription> {
    let mut app_map = HashMap::new();

    for app in applications {
        app_map
            .entry(app.name.clone())
            .or_insert_with(|| XmlApplication {
                name: app.name.clone(),
                token_type: app.token_type.clone(),
                token_validity: app.token_validity,
                refresh_token_validity: app.refresh_token_validity,
                apis: Vec::new(),
            })
            .apis
            .extend(app.apis.clone());
    }

    let mut yaml_api_subs = Vec::new();

    for app in app_map.values() {
        let mut yaml_apis = Vec::new();
        let mut env_set = HashSet::new();
        let mut name_set = HashSet::new();
        let mut version_map = HashMap::new();
        for sub in &app.apis {
            name_set.insert(sub.api_name.clone());
            version_map
                .entry(sub.api_name.clone())
                .or_insert_with(HashSet::new)
                .insert(sub.api_version.clone());
            for env in &sub.env {
                env_set.insert(env.clone());
            }
        }

        for name in name_set {
            for version in version_map.get(&name).unwrap() {
                let yaml_api = YamlApi {
                    name: name.clone(),
                    version: version.clone(),
                };
                yaml_apis.push(yaml_api);
            }
        }
        let yaml_app = YamlApplication {
            name: app.name.clone(),
            description: format!("{}-subscription", app.name),
            apis: yaml_apis,
            access_token_ttl: app.token_validity,
            refresh_token_ttl: app.refresh_token_validity,
        };

        let yaml_sub = YamlSubscription {
            application: yaml_app,
        };

        let mut environments = Vec::new();

        let non_prod_envs: HashSet<String> = env_set
            .iter()
            .filter(|env| env.as_str() != "prod")
            .cloned()
            .collect();

        let prod_envs: HashSet<String> = env_set
            .iter()
            .filter(|env| env.as_str() == "prod")
            .cloned()
            .collect();

        let yaml_non_prod_names = non_prod_envs
            .iter()
            .map(|env| YamlEnvironmentName { name: env.clone() });

        let yaml_prod_names = prod_envs
            .iter()
            .map(|env| YamlEnvironmentName { name: env.clone() });

        let yaml_env_non_prod = YamlEnvironment {
            control_plane_url: config.npr_plane_url.clone(),
            environments: yaml_non_prod_names.collect(),
        };

        let yaml_env_prod = YamlEnvironment {
            control_plane_url: config.prod_plane_url.clone(),
            environments: yaml_prod_names.collect(),
        };

        if !non_prod_envs.is_empty() {
            environments.push(yaml_env_non_prod);
        }

        if !prod_envs.is_empty() {
            environments.push(yaml_env_prod);
        }

        let yaml_api_sub = YamlApiSubscription {
            environments,
            subscription: yaml_sub,
        };

        yaml_api_subs.push(yaml_api_sub);
    }

    yaml_api_subs
}
