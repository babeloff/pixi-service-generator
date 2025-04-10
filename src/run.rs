
use log::{error, warn, info, debug, trace};

use std::fs; 
use std::env;
use serde::Deserialize;

use dirs::home_dir;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use serde::Serialize;

use std::collections::HashMap;

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

fn read_template(template_path: Option<PathBuf>) -> Result<String, Box<dyn std::error::Error>> {
    trace!("Try to read from environment variable: PIXI_SYSTEMD_UNIT_PATH");
    if let Ok(local_path) = env::var("PIXI_SYSTEMD_UNIT_PATH") {
        let path = Path::new(&local_path);
        if path.exists() && path.is_file() {
            return Ok(fs::read_to_string(path)?);
        }
    }
    if let Some(tpath) = template_path {
        let path = Path::new(&tpath);
        if path.exists() && path.is_file() {
            return Ok(fs::read_to_string(path)?);
        }
    }

    trace!("Fallback: read from project-local file: unit.service.tera");
    let cwd = env::current_dir()?;
    let unit_template_path = cwd.join("src/resources/unit.service.tera");
    let template_content = fs::read_to_string(unit_template_path.to_str().unwrap())?;
    debug!("the content of the template {:?}", template_content);
    Ok(template_content)
}

pub fn run(normal_dir: PathBuf, early_dir: PathBuf, late_dir: PathBuf, 
           template_dir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
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

    let tera_content = read_template(template_dir)?;
    let mut tera = Tera::default();
    tera.add_raw_template("template", &tera_content)?;
    debug!("slurped up the unit-file template");
             
    let manifest_path = env::var("PIXI_MANIFEST_PATH").unwrap_or_else(|_| {
                let mut default_path = home_dir().unwrap_or_else(|| PathBuf::from("/"));
                default_path.push(".pixi/manifests/pixi-global.toml");
                default_path.to_string_lossy().into_owned()
            });
    let toml_str = fs::read_to_string(manifest_path)?;
    let manifest: Manifest = toml::from_str(&toml_str)?;

    if !manifest.version.is_integer() {
        warn!("Manifest {version} is not an integer, version 1 assumed", version=manifest.version);
    }
    else if let Some(version) = manifest.version.as_integer() {
        if version == 1 {
            info!("Global manifest version 1");
        }
        else {
            error!("Manifest {version} is not yet supported", version=version);
        }
    }
    else {
        error!("Manifest {version} is none, version 1 assumed", version=manifest.version);
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