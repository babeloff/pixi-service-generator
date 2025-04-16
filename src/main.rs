use clap::{Parser, ValueEnum};
use log::warn;
use std::fs; 
use std::env;
use std::path::{Path, PathBuf};

use log::{error, info, debug };

mod config;
mod install;
mod run;

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
#[derive(Parser, Debug)]
#[command(name = "systemd-pixi-generator")]
#[command(about = "A systemd generator for pixi global environments")]
#[command(long_about = None)]
struct Cli {
    // positional parameters
    #[arg(required = false, num_args = 0..=3)]
    dirs: Vec<PathBuf>,

    // Verbose mode (flag/keyword argument)
    #[arg(short, long )]
    verbose: bool,

    // Mode of operation
    #[arg(long, value_enum, default_value_t = Mode::Run)]
    mode: Mode,

    // Manifest Path 
    #[arg(required = false, short = 'm', long = "manifest")]
    manifest_path: Option<PathBuf>,

    // Template Path 
    #[arg(required = false, short = 't', long = "template")]
    template_path: Option<PathBuf>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
enum Mode {
    Run,
    Init,
}

fn grab_base(argv0: String) -> Result<(PathBuf, String), Box<dyn std::error::Error>> {

    debug!("Get just the filename component (i.e., what the user typed)");
    let alias = Path::new(&argv0)
        .file_name()
        .map(|os_str| os_str.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("unknown"));

    debug!("Optionally resolve symlink to canonical path");
    let resolved_path = fs::canonicalize(&argv0)?;
    let real_base = {
        resolved_path
            .file_name()
            .map(|os_str| os_str.to_string_lossy().into_owned())
            .unwrap_or_default()
    };

    info!("Alias (invoked as): {}", alias);
    info!("Resolved to binary: {}", real_base);

    // debug!("grab the symlink name");
    // Result<Vec<String>, Box<dyn std::error::Error>>
    // let args: Vec<String> = env::args().skip(1).collect();

    Ok((resolved_path, real_base.to_string()))
}

/*
https://www.freedesktop.org/software/systemd/man/latest/systemd.generator.html#Environment
*/
fn check_base(name: String) -> config::Privilege {
    warn!("name check {}", name);
    match env::var("SYSTEMD_SCOPE") {
        Ok(scope) => 
            if scope == name {
                info!("System level");
                config::Privilege::SYSTEM
            }
            else if scope == name { 
                info!("User level");
                config::Privilege::USER
            }
            else {
                warn!("SYSTEMD_SCOPE is not a valid value {}", scope);
                config::Privilege::UNSPEC
            },
        Err(_) => {
            warn!("SYSTEMD_SCOPE is not set");
            config::Privilege::UNSPEC
        } 
    }
}

fn check_normal_dir(normal_dir: PathBuf) -> bool {
    warn!("normal dir check {}", normal_dir.into_os_string().into_string().unwrap());
    true
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    debug!("The first argument is the executable path (possibly a symlink)");
    let argv0 = env::args().next().unwrap_or_else(|| String::from(""));

    let (resolved_path, real_base) = grab_base(argv0)?;
    let privilege = check_base(real_base);
    let cwd = env::current_dir()?;

    let cli = Cli::parse();

    debug!("Assign default (cwd) if not enough args are given");
    let mut paths = cli.dirs.clone();
    while paths.len() < 3 {
       paths.push(cwd.clone());
    }

    let [normal_dir, early_dir, late_dir] = 
        <[PathBuf; 3]>::try_from(paths).expect("Always 3 paths now");

    debug!("Validate paths exist and are directories");
    for (label, path) in [("normal_dir", &normal_dir),
                          ("early_dir", &early_dir),
                          ("late_dir", &late_dir)] {
        if !path.exists() {
            error!("Error: path '{}' ({}) does not exist", 
                   label, path.display());
            std::process::exit(1);
        }
    }
    check_normal_dir(normal_dir.clone());
    info!("directories: normal={:?}, early={:?}, late={:?}", 
             normal_dir, early_dir, late_dir);


    let _ = match cli.mode {
        Mode::Init => install::initialize(resolved_path),
        Mode::Run => run::run(normal_dir, early_dir, late_dir, 
                    privilege, cli.template_path, 
                    cli.manifest_path),
    };
         
    Ok(())
}
