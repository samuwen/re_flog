use crate::{commands::*, exit_with_message, structures::Sha};
use clap::{ArgGroup, Parser, Subcommand};
use flexi_logger::{colored_detailed_format, Duplicate, Logger};
use std::path::PathBuf;
// use log::*;

fn halp_str() -> &'static str {
    "halp me"
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Add file contents to the index
    Add { pathspec: Vec<PathBuf> },
    #[clap(group(
        ArgGroup::new("mode")
            .required(true)
    ))]
    /// Provide content or type and size information for repository objects
    CatFile {
        /// pretty prints object's content
        #[clap(short = 'p', group = "mode")]
        pretty: bool,
        /// show object's type
        #[clap(short = 't', group = "mode")]
        type_print: bool,
        sha: Sha,
    },
    /// Record changes to the repository
    Commit {
        #[clap(short = 'm')]
        messages: Option<Vec<String>>,
    },
    /// Create a new commit object
    CommitTree {
        sha: Sha,
        #[clap(short = 'm')]
        message: Option<Vec<String>>,
        #[clap(short = 'p')]
        parent: Option<Vec<Sha>>,
    },
    /// Compute object ID and optionally creates a blob from a file
    HashObject {
        /// The file to hash
        file: String,
        /// Whether or not to write output
        #[clap(short = 'w')]
        write: bool,
    },
    /// Initializes this directory as a re_Flog (git) directory
    Init { destination: Option<String> },
    /// Shows the commit logs
    Log {
        #[clap(arg_enum, long = "pretty")]
        pretty: Option<LogFormat>,
    },
    /// Show information about files in the index and the working tree
    LsFiles {
        #[clap(long)]
        stage: bool,
    },
    /// Reads tree information into the index
    ReadTree { sha: Sha },
    /// Register file contents in the working tree to the index
    UpdateIndex {
        /// If a specified file isn’t in the index already then it’s added. Default behaviour is to ignore new files.
        #[clap(long)]
        add: bool,
        /// If a specified file is in the index but is missing then it’s removed. Default behavior is to ignore removed file.
        #[clap(long)]
        remove: bool,
        files: Vec<PathBuf>,
    },
    /// Update the object name stored in a ref safely
    UpdateRef {
        #[clap(help = halp_str())]
        r#ref: String,
        new_value: Sha,
    },
    /// Create a tree object from the current index
    WriteTree {
        /// Normally flog write-tree ensures that the objects referenced by the directory exist in the object database. This option disables this check.
        #[clap(long = "missing-ok")]
        missing: bool,
        /// Writes a tree object that represents a subdirectory <prefix>. This can be used to write the tree object for a subproject that is in the named subdirectory.
        prefix: Option<String>,
    },
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(subcommand)]
    command: Command,
    #[clap(short = 'd')]
    debug: bool,
}

pub fn parse_args() -> Result<(), std::io::Error> {
    let args = Args::parse();
    let d_level = match args.debug {
        true => "debug",
        false => "error",
    };
    Logger::try_with_str(d_level)
        .unwrap()
        .duplicate_to_stdout(Duplicate::All)
        .format(colored_detailed_format)
        .start()
        .expect("Failed to start logger");
    match &args.command {
        Command::Add { pathspec: files } => {
            update_index_add(files)?;
        }
        Command::CatFile {
            pretty,
            type_print,
            sha,
        } => {
            if *pretty {
                cat_file_pretty_print(sha)?;
            }
            if *type_print {
                cat_file_print_type(sha)?;
            }
        }
        Command::Commit { messages } => {
            commit(messages)?;
        }
        Command::CommitTree {
            sha,
            message,
            parent,
        } => {
            commit_tree(sha, message, parent)?;
        }
        Command::HashObject { file, write } => {
            if *write {
                return hash_and_write_to_db(file);
            }
            hash_object(file, true)?;
        }
        Command::Init { destination } => {
            let root_dir = match destination {
                Some(dir) => dir,
                None => ".",
            };
            init(root_dir)?;
        }
        Command::Log { pretty } => {
            log(pretty)?;
        }
        Command::LsFiles { stage } => {
            if *stage {
                ls_files_staging()?;
            }
        }
        Command::ReadTree { sha } => {
            read_tree(sha)?;
        }
        Command::UpdateIndex { add, remove, files } => {
            if *add {
                update_index_add(files)?;
            } else if *remove {
                update_index_remove(files)?;
            } else {
                exit_with_message("error: file cannot add to the index - missing --add option?")
            }
        }
        Command::UpdateRef { r#ref, new_value } => {
            update_ref_basic(r#ref, new_value)?;
        }
        Command::WriteTree { missing, prefix } => {
            if let Some(_) = prefix {
                unimplemented!();
            }
            write_tree(*missing)?;
        }
    }
    Ok(())
}
