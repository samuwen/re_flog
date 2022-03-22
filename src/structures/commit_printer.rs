use log::debug;

use super::Commit;

pub trait Printer {
    fn print_commit(&self, commit: &Commit);
}

pub struct OneLinePrinter {}

impl Printer for OneLinePrinter {
    fn print_commit(&self, commit: &Commit) {
        let msg = format!("{} {}\n", commit.sha(), commit.message().get(0).unwrap());
        print!("{}", msg);
    }
}

pub struct MediumPrinter {}

impl Printer for MediumPrinter {
    fn print_commit(&self, commit: &Commit) {
        debug!("{:?}", commit);
        println!("commit {}", commit.sha());
        println!("Author:\t{}", commit.author().to_string_without_date());
        println!("Date:\t{}\n", commit.author().to_string_date());
        for msg in commit.message().iter() {
            println!("    {}\n", msg);
        }
    }
}

pub struct ShortPrinter {}

impl Printer for ShortPrinter {
    fn print_commit(&self, commit: &Commit) {
        println!("commit {}", commit.sha());
        println!("Author:\t{}\n", commit.author().to_string_without_date());
        for msg in commit.message().iter() {
            println!("    {}\n", msg);
        }
    }
}
