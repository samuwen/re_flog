mod argparser;
pub mod commands;
pub mod structures;
pub mod utils;

use std::io;

pub fn exit_with_message(message: &str) -> ! {
    println!("{}", message);
    std::process::exit(1)
}

pub fn main() -> Result<(), io::Error> {
    argparser::parse_args()?;
    Ok(())
}
