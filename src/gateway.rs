use core::panic;
use std::{fs, path::PathBuf};

use crate::{BulkArgs, SingleArgs};
use anyhow::{anyhow, Result};
use xml::{reader::XmlEvent, EventReader};

struct Api {
    name: String,
    version: String,
    roles: String,
    context: String,
    description: String,
    api_defintion: String,
}

pub fn migrate_gateway(args: SingleArgs) -> Result<()> {
    let dir = args.input_dir;

    if !dir.exists() {
        return Err(anyhow!("input dir: {:?}, does not exist", dir));
    }

    let apis = dir
        .read_dir()?
        .filter_map(|entry| {
            let entry = if let Ok(entry) = entry {
                entry
            } else {
                return None;
            };

            let file_ending = entry.file_name();
            if file_ending.to_str().unwrap().ends_with(".xml") {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect::<Vec<PathBuf>>();

    for api in apis {
        let file = fs::File::open(api)?;
        _ = parse_api_config_file(file);
    }

    todo!();
}

pub fn migrate_gateway_bulk(args: BulkArgs) -> Result<()> {
    todo!()
}

fn parse_api_config_file(src: impl std::io::Read) -> Result<()> {
    let parser = EventReader::new(src);

    for event in parser {
        match event {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "name" {
                    println!("{name}");
                }
            }
            _ => panic!(),
        };
    }

    todo!()
}
