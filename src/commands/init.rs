use log::*;
use std::ffi::OsStr;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::Write;
use std::path::Path;

/// Initializes repo for flogging
pub fn init<P: AsRef<OsStr>>(root_path: P) -> Result<(), io::Error> {
    debug!("Initializing new repo");
    let path = Path::new(&root_path).join(".re_flogged");
    if path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "This is already a re-flogged repository",
        ));
    }
    create_dir_all(Path::join(&path, "hooks"))?;
    create_dir_all(Path::join(&path, "objects/info"))?;
    create_dir_all(Path::join(&path, "objects/pack"))?;
    create_dir_all(Path::join(&path, "refs/heads"))?;
    create_dir_all(Path::join(&path, "refs/tags"))?;
    create_dir_all(Path::join(&path, "info"))?;
    let write_root_file = |name: &str, content: &str| -> Result<(), io::Error> {
        let mut file = File::create(Path::join(&path, name))?;
        file.write(content.as_bytes())?;
        println!("Wrote {}", name);
        Ok(())
    };
    write_root_file("HEAD", "ref: refs/heads/main\n")?;
    write_root_file(
        "description",
        "Unnamed repository; edit this file 'description' to name the repository.",
    )?;
    write_root_file(
        "config",
        "[core]
	bare = false
	repositoryformatversion = 0
	filemode = true
	logallrefupdates = true",
    )?;
    let print_path = Path::canonicalize(&path)?;
    println!(
        "Initialized empty Flog repository in {}",
        print_path.display()
    );
    Ok(())
}
