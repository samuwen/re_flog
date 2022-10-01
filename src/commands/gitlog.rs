use std::io::{self};

use clap::ArgEnum;
use log::debug;

use crate::{
    structures::{MediumPrinter, OneLinePrinter, Printer, RefFile, ShortPrinter},
    utils::get_current_branch,
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
