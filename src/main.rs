use serde::Deserialize;
use std::fs;
use std::env;
use std::path::PathBuf;
use dirs::home_dir;
use std::path::Path;
use tera::{Context, Tera};
use serde::Serialize;

/*
Generators are invoked with three arguments: 
paths to directories where generators can place their generated unit files or symlinks. 
By default those paths are runtime directories that are included in the search path of systemd,
but a generator may be called with different paths for debugging purposes. 
If only one argument is provided, 
the generator should use the same directory as the three output paths.

normal-dir
    In normal use this is /run/systemd/generator
    in case of the system generators
    and $XDG_RUNTIME_DIR/systemd/generator in case of the user generators. 
    Unit files placed in this directory take precedence over vendor unit configuration
    but not over native user/administrator unit configuration.

early-dir
    In normal use this is /run/systemd/generator.early
    in case of the system generators and $XDG_RUNTIME_DIR/systemd/generator.early 
    in case of the user generators. 
    Unit files placed in this directory override unit files 
    in /usr/, /run/ and /etc/. 
    This means that unit files placed in this directory take precedence over 
    all normal configuration, both vendor and user/administrator.

late-dir
    In normal use this is /run/systemd/generator.late 
    in case of the system generators and $XDG_RUNTIME_DIR/systemd/generator.late 
    in case of the user generators. 
    This directory may be used to extend the unit file tree without 
    overriding any other unit files. 
    Any native configuration files supplied by the vendor 
    or user/administrator take precedence.

Note: generators must not write to other locations or otherwise make changes to system state. 
Generator output is supposed to last only until the next daemon-reload or daemon-reexec; 
if the generator is replaced or masked, its effects should vanish.
*/

fn read_template() -> Result<String, Box<dyn std::error::Error>> {
    // Try to read from environment variable
    if let Ok(env_path) = env::var("PIXI_SYSTEMD_UNIT_PATH") {
        let path = Path::new(&env_path);
        if path.exists() && path.is_file() {
            return Ok(fs::read_to_string(path)?);
        }
    }

    // Fallback: read from project-local file
    let fallback_path = Path::new("unit.service.tera");
    Ok(fs::read_to_string(fallback_path)?)
}


#[derive(Debug, Deserialize)]
struct Manifest {
    envs: std::collections::HashMap<String, EnvConfig>,
}

#[derive(Debug, Deserialize)]
struct EnvConfig {
   channels: Vec<String>,
   dependencies: std::collections::HashMap<String, String>,
   exposed: std::collections::HashMap<String, String>,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let normal_dir = std::env::args().nth(1).expect("no normal directory");
    let early_dir = std::env::args().nth(2).expect("no early directory");
    let late_dir = std::env::args().nth(3).expect("no late directory");

    // https://doc.rust-lang.org/rust-by-example/hello/print.html
    println!("directoreies: normal={:?}, early={:?}, late={:?}", 
             normal_dir, early_dir, late_dir);

    let output_path = PathBuf::from(normal_dir.clone());
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tera_path = read_template()?;
    let tera_content = fs::read_to_string(&tera_path)?;
    let mut tera = Tera::default();
    tera.add_raw_template("template", &tera_content)?;
             
    let manifest_path = env::var("PIXI_MANIFEST_PATH").unwrap_or_else(|_| {
                let mut default_path = home_dir().unwrap_or_else(|| PathBuf::from("/"));
                default_path.push(".pixi/manifests/pixi-global.toml");
                default_path.to_string_lossy().into_owned()
            });
    let toml_str = fs::read_to_string(manifest_path)?;
    let manifest: Manifest = toml::from_str(&toml_str)?;
         
    for (name, env) in manifest.envs {
        println!("Environment: {}", name);
        println!("  Channels: {:?}", env.channels);
        println!("  Dependencies: {:?}", env.dependencies);
        println!("  Exposed: {:?}", env.exposed);
        println!("  Service: {:?}", env.service);
        if let Some(service) = env.service {
            println!("{:?}", service.status);
            println!("{:?}", service.after);
            println!("{:?}", service.exec_start_pre);
            println!("{:?}", service.exec_start);
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
            println!("Wrote to: {}", output_path.display());
        }
    }
         
    Ok(())
}
