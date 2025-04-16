use log::{debug, error, info, trace, warn};

use serde::Deserialize;
use std::env;
use std::fs;
use std::io;

use serde::Serialize;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};

use std::collections::HashMap;

use crate::config;

#[derive(Debug, Deserialize)]
struct Manifest {
    version: toml::Value,
    envs: HashMap<String, EnvConfig>,
}

#[derive(Debug, Deserialize)]
struct EnvConfig {
    channels: Vec<String>,
    dependencies: HashMap<String, String>,
    exposed: Option<HashMap<String, String>>,
    service: Option<Service>,
}

#[derive(Debug, Deserialize)]
struct Service {
    status: String,
    after: Option<String>,
    #[serde(rename = "exec-start-pre")]
    exec_start_pre: Option<String>,
    #[serde(rename = "exec-start")]
    exec_start: Option<String>,
}

#[derive(Serialize)]
struct TemplateData {
    name: String,
    description: String,
    after: String,
    exec_start_pre: String,
    exec_start: String,
}

fn read_template(template_path: Option<PathBuf>, privilege: config::Privilege) -> Result<String, Box<dyn std::error::Error>> {
    trace!("Try to read systemd unit-file template");
    if let Some(tpath) = template_path {
        let path = Path::new(&tpath);
        if path.exists() && path.is_file() {
            return Ok(fs::read_to_string(path)?);
        }
    }

    trace!("Fallback: read from project-local file: unit.service.tera");
    let cwd = env::current_dir()?;
    let unit_template_path = match privilege {
        config::Privilege::SYSTEM => cwd.join(config::SYSTEM_UNIT_FILE_TEMPLATE),
        config::Privilege::USER => cwd.join(config::USER_UNIT_FILE_TEMPLATE),
        config::Privilege::UNSPEC => cwd.join(config::USER_UNIT_FILE_TEMPLATE),
    };
    let template_content = fs::read_to_string(unit_template_path.to_str().unwrap())?;
    debug!("the content of the template {:?}", template_content);
    Ok(template_content)
}

fn read_manifest(manifest_path: Option<PathBuf>) -> Result<String, io::Error> {
    trace!("Try to read pixi global manifest");
    if let Some(path) = manifest_path {
        match fs::read_to_string(&path) {
            Ok(content) => return Ok(content),
            Err(_) => {
                // Fall through to fallback path
            }
        }
    }

    debug!("Try the manifest fallback path");
    let fallback = dirs::home_dir()
        .map(|home| home.join(".pixi/manifests/pixi-global.toml"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME directory not found"))?;
    fs::read_to_string(fallback)
}

pub fn run(
    normal_dir: PathBuf,
    early_dir: PathBuf,
    late_dir: PathBuf,
    privilege: config::Privilege,
    template_dir: Option<PathBuf>,
    manifest_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running the application!");

    let normal_path = PathBuf::from(normal_dir.clone());
    if let Some(parent) = normal_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let early_path = PathBuf::from(early_dir.clone());
    if let Some(parent) = early_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let late_path = PathBuf::from(late_dir.clone());
    if let Some(parent) = late_path.parent() {
        fs::create_dir_all(parent)?;
    }
    debug!("created the output directories (if they did not exist)");

    let tera_content = read_template(template_dir, privilege)?;
    let mut tera = Tera::default();
    tera.add_raw_template("template", &tera_content)?;
    debug!("slurped up the unit-file template");

    let toml_str = read_manifest(manifest_path)?;
    let manifest: Manifest = toml::from_str(&toml_str)?;

    if !manifest.version.is_integer() {
        warn!(
            "Manifest {version} is not an integer, version 1 assumed",
            version = manifest.version
        );
    } else if let Some(version) = manifest.version.as_integer() {
        if version == 1 {
            info!("Global manifest version 1");
        } else {
            error!("Manifest {version} is not yet supported", version = version);
        }
    } else {
        error!(
            "Manifest {version} is none, version 1 assumed",
            version = manifest.version
        );
    }

    for (name, env) in manifest.envs {
        info!("Environment: {}", name);
        info!("  Channels: {:?}", env.channels);
        info!("  Dependencies: {:?}", env.dependencies);
        info!("  Exposed: {:?}", env.exposed);
        info!("  Service: {:?}", env.service);
        if let Some(service) = env.service {
            info!("{:?}", service.status);
            info!("{:?}", service.after);
            info!("{:?}", service.exec_start_pre);
            info!("{:?}", service.exec_start);
            let data = TemplateData {
                name: name.clone().into(),
                description: name.clone().into(),
                after: "unknown".into(), // service.after,
                exec_start_pre: "missing".into(),
                exec_start: "missing".into(),
            };
            let content = Context::from_serialize(&data)?;
            let rendered = tera.render("template", &content)?;
            let mut output_path = PathBuf::from(normal_dir.clone());
            output_path.push(format!("{}.service", &name));
            fs::write(&output_path, rendered)?;
            info!("Wrote to: {}", output_path.display());
        }
    }

    Ok(())
}
