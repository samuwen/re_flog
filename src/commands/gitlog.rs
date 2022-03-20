use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

use clap::ArgEnum;
use log::debug;

use crate::{
    exit_with_message,
    structures::{MediumPrinter, OneLinePrinter, Printer, RefFile, ShortPrinter},
};

#[derive(Clone, Debug, ArgEnum)]
pub enum LogFormat {
    Oneline,
    Short,
    Medium,
    Full,
    Fuller,
    Reference,
    Email,
    Raw,
    Format,  // takes a string
    TFormat, // takes a string
}

pub fn log(print_options: &Option<LogFormat>) -> Result<(), io::Error> {
    let current_branch = get_current_branch()?;
    let ref_file = RefFile::new_from_branch(&current_branch)?;
    debug!("{:?}", ref_file);
    let printer: &dyn Printer = match print_options {
        Some(format) => match format {
            &LogFormat::Oneline => &OneLinePrinter {},
            &LogFormat::Short => &ShortPrinter {},
            _ => &MediumPrinter {},
        },
        None => &MediumPrinter {},
    };
    ref_file.pretty_print(printer);
    Ok(())
}

fn get_current_branch() -> Result<String, io::Error> {
    let file = match File::open(".re_flogged/HEAD") {
        Ok(f) => f,
        Err(_err) => {
            exit_with_message(
                "fatal: not a flog repository (or any of the parent directories): .re_flog",
            );
        }
    };
    let mut reader = BufReader::new(file);
    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    let last_bit = {
        let this = buf.rsplit(" ").next();
        match this {
            Some(val) => val,
            None => {
                exit_with_message(
                    "fatal: not a flog repository (or any of the parent directories): .re_flog",
                );
            }
        }
    };
    Ok(last_bit.to_string())
}
