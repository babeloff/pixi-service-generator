use std::path::{Path, PathBuf};
use std::fs;
use std::os::unix::fs::symlink;
use log::{error, warn, info, debug, trace};

/*
The purpose of init is to provide symlinks
from the generator directories to this executable.

The symlinks need to exist in the directories described in:
https://www.freedesktop.org/software/systemd/man/latest/systemd.generator.html

*/

fn create_symlink(source_path: PathBuf, target_path: PathBuf) -> std::io::Result<()> {
    let target = Path::new(&target_path);
    let source = Path::new(&source_path);

    symlink(source, target)?;

    Ok(())
}

pub fn initialize(source_path: PathBuf)  -> Result<(), Box<dyn std::error::Error>>  {
    info!("Initialization process started!");

    let system_paths: [PathBuf; 4] = [
        PathBuf::from("/run/systemd/system-generators/"),
        PathBuf::from("/etc/systemd/system-generators/"),
        PathBuf::from("/usr/local/lib/systemd/system-generators/"),
        PathBuf::from("/usr/lib/systemd/system-generators/"),
    ];
    for target_path in &system_paths {
        debug!("{}", target_path.display());
        let rc = create_symlink(source_path.clone(), target_path.to_path_buf());
        if rc.is_ok() {
            break;
        };
    }
    let user_paths: [PathBuf; 4] = [
        PathBuf::from("/run/systemd/user-generators/"),
        PathBuf::from("/etc/systemd/user-generators/"),
        PathBuf::from("/usr/local/lib/systemd/user-generators/"),
        PathBuf::from("/usr/lib/systemd/user-generators/"),
    ];
    for target_path in &user_paths {
        debug!("{}", target_path.display());
        let rc = create_symlink(source_path.clone(), target_path.to_path_buf());
        if rc.is_ok() {
            break;
        };
    }

    Ok(())
}
