use super::Commit;

pub trait Printer {
    fn print_commit(&self, commit: &Commit);
}

pub struct OneLinePrinter {}

impl Printer for OneLinePrinter {
    fn print_commit(&self, commit: &Commit) {
        let msg = format!("{} {}", commit.sha(), commit.message());
        print!("{}", msg);
    }
}

pub struct MediumPrinter {}

impl Printer for MediumPrinter {
    fn print_commit(&self, commit: &Commit) {
        println!("commit {}", commit.sha());
        println!("Author:\t{}", commit.author().to_string_without_date());
        println!("Date:\t{}\n", commit.author().to_string_date());
        println!("    {}", commit.message());
    }
}

pub struct ShortPrinter {}

impl Printer for ShortPrinter {
    fn print_commit(&self, commit: &Commit) {
        println!("commit {}", commit.sha());
        println!("Author:\t{}", commit.author().to_string_without_date());
        println!("    {}", commit.message());
    }
}
