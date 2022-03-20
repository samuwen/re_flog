use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

use log::{debug, info};

use crate::exit_with_message;

use super::{check_file_is_of_kind, load_commit_from_sha, Printer, Sha};

#[derive(Clone, Debug)]
pub struct RefFile {
    sha: Sha,
    name: String,
}

impl RefFile {
    pub fn new(reff: &String, new_value: &String) -> Self {
        let sha = Sha::new_from_str(new_value);
        if !check_file_is_of_kind(&sha, "commit") {
            let message = format!("fatal: update_ref failed for ref '{}': cannot update ref '{}': trying to write non-commit object {} to branch '{}'", reff, reff, new_value, reff);
            exit_with_message(&message);
        }
        Self {
            sha,
            name: reff.clone(),
        }
    }

    pub fn new_from_branch(branch: &String) -> Result<Self, io::Error> {
        let branch = branch.trim();
        info!("New ref file from branch: {}", branch);
        let path = Path::join(Path::new(".re_flogged"), Path::new(branch));
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        reader.read_line(&mut buf)?;
        let sha = Sha::new_from_str(&buf.trim());
        Ok(Self {
            name: branch.to_owned(),
            sha,
        })
    }

    pub fn write(&self) -> Result<(), io::Error> {
        let path = Path::join(Path::new(".re_flogged"), Path::new(&self.name));
        debug!("Writing new ref to: {}", path.display());
        let mut file = File::create(path)?;
        file.write(self.sha.to_string().as_bytes())?;
        Ok(())
    }

    pub fn pretty_print(&self, printer: &dyn Printer) {
        let head = load_commit_from_sha(&self.sha).unwrap();
        head.print_recursive(printer);
    }
}
