use std::{
    fs::{create_dir_all, File},
    io::{self, stdin, BufRead, Seek, Write},
};

use chrono::{DateTime, Local};
use derive_getters::Getters;
use log::{debug, info};

use crate::{
    exit_with_message,
    structures::{compress, load_commit_from_sha},
    utils::iterable_to_string_no_truncate,
};

use super::commit_printer::Printer;
use super::{GitObject, Sha};

#[derive(Clone, Debug, Getters)]
pub struct Author {
    name: String,
    email: String,
    date: DateTime<Local>,
}

impl Author {
    pub fn new(name: String, email: String, date: DateTime<Local>) -> Self {
        Self { name, email, date }
    }

    pub fn empty() -> Self {
        Self {
            name: String::new(),
            email: String::new(),
            date: Local::now(),
        }
    }

    pub fn to_string_with_date(&self) -> String {
        format!("{} {} {}", self.name, self.email, self.date.format("%s %z"))
    }

    pub fn to_string_without_date(&self) -> String {
        format!("{} {}", self.name, self.email)
    }

    pub fn to_string_date(&self) -> String {
        // Date:   Tue Mar 15 20:13:58 2022 -0700
        format!("{}", self.date.format("%a %b %d %H:%M:%S %Y %z"))
    }
}

#[derive(Clone, Debug, Getters)]
pub struct Commit {
    message: Vec<String>,
    author: Author,
    committer: Author,
    parent: Option<Vec<Sha>>,
    sha: Sha,
    tree_sha: Sha,
}

impl Commit {
    pub fn new_from_disk<R: BufRead + Seek>(
        reader: &mut R,
        count: usize,
        sha: &Sha,
    ) -> Result<Self, io::Error> {
        // tree b708e1ba3e49f514ef252db6cc8733dca2b7471d
        // parent 406b00943149ced320d43c489e6bca6ef423f8b4
        // author Mark Chaitin <markchaitin@gmail.com> 1647742703 -0700
        // committer Mark Chaitin <markchaitin@gmail.com> 1647742703 -0700
        // added a second file
        info!("Reading commit from disk. Total size: {} bytes", count);
        let mut total = 0;
        let mut commit = Commit::empty();
        while let Some((name, data)) = Commit::read_file_line(reader, &mut total) {
            commit.add_field(name, data);
        }
        debug!("total read: {} vs total in file: {}", total, count);
        let message = Commit::read_messages(reader, &mut total)?;
        commit.message = message;
        commit.sha = sha.clone();
        Ok(commit)
    }

    pub fn new_from_tree_sha(
        tree_sha: &Sha,
        message: &Option<Vec<String>>,
        parent: &Option<Vec<Sha>>,
    ) -> Result<Self, io::Error> {
        let message = match message {
            Some(msg) => msg.clone(),
            None => {
                let mut message = String::new();
                let stdin = stdin();
                stdin.read_line(&mut message)?;
                vec![message]
            }
        };
        debug!("{:?}", message);
        let dt = Local::now();
        let author = Author::new(
            String::from("Mark Chaitin"),
            String::from("<markchaitin@gmail.com>"),
            dt,
        );
        let committer = author.clone();
        let mut me = Self {
            message,
            author,
            committer,
            parent: None,
            sha: Sha::empty(),
            tree_sha: tree_sha.clone(),
        };
        if let Some(parents) = parent {
            me.parent = Some(parents.clone());
        };
        me.hash()?;
        me.pretty_print();
        Ok(me)
    }

    pub fn hash(&mut self) -> Result<(), io::Error> {
        info!("Hashing new commit object");
        let mut bytes = vec![];
        bytes.write(b"tree ")?;
        bytes.write(self.tree_sha.buf())?;
        if let Some(parents) = &self.parent {
            for parent in parents.iter() {
                bytes.write(b"\nparent ")?;
                bytes.write(parent.buf())?;
            }
        }
        let author_string = format!("\nauthor {}", self.author.to_string_with_date());
        bytes.write(author_string.as_bytes())?;
        let committer_string = format!("\ncommitter {}", self.committer.to_string_with_date());
        bytes.write(committer_string.as_bytes())?;
        let mut heading = Commit::create_heading(bytes.len());
        let mut all_out = vec![];
        all_out.write(&mut heading)?;
        all_out.write(&mut bytes)?;
        self.sha = Sha::new_hash(all_out);
        Ok(())
    }

    pub fn empty() -> Self {
        Self {
            message: vec![],
            author: Author::empty(),
            committer: Author::empty(),
            parent: None,
            sha: Sha::empty(),
            tree_sha: Sha::empty(),
        }
    }

    pub fn print_recursive(&self, printer: &dyn Printer) {
        printer.print_commit(self);
        if let Some(parent_shas) = &self.parent {
            for sha in parent_shas {
                let next = load_commit_from_sha(sha).unwrap();
                next.print_recursive(printer);
            }
        }
    }

    fn create_heading(size: usize) -> Vec<u8> {
        let heading = format!("commit {}\0", size);
        heading.chars().map(|ch| ch as u8).collect()
    }

    fn read_file_line<R: BufRead>(reader: &mut R, total: &mut usize) -> Option<(String, String)> {
        let msg = "Database is corrupt";
        let mut buf = String::new();
        *total += {
            let this = reader.read_line(&mut buf);
            match this {
                Ok(t) => t,
                Err(_e) => exit_with_message(msg),
            }
        };
        let split = buf.split_once(" ");
        if let Some((start, rest)) = split {
            return Some((start.to_string(), rest.to_string()));
        };
        None
    }

    fn read_messages<R: BufRead>(
        reader: &mut R,
        total: &mut usize,
    ) -> Result<Vec<String>, io::Error> {
        let mut buf = vec![];
        *total += reader.read_to_end(&mut buf)?;
        let full_string = iterable_to_string_no_truncate(&mut buf.iter());
        let v = full_string.split("\n\n").map(|s| s.to_string()).collect();
        Ok(v)
    }

    fn add_field(&mut self, field_name: String, data: String) {
        let data = data.trim().to_string();
        match field_name.as_str() {
            "tree" => {
                let sha = data.parse().unwrap();
                debug!("Setting tree_sha to {}", sha);
                self.tree_sha = sha;
            }
            "parent" => {
                let sha = data.parse().unwrap();
                debug!("Setting parent to {}", sha);
                if self.parent.is_some() {
                    let opt = self.parent.clone();
                    let mut v = opt.unwrap();
                    v.push(sha);
                    self.parent = Some(v);
                } else {
                    self.parent = Some(vec![sha]);
                }
            }
            "author" | "committer" => {
                let start_email_index = data.find('<').unwrap();
                let end_email_index = data.find('>').unwrap();
                let name = &data[0..start_email_index - 1];
                let email = &data[start_email_index..=end_email_index];
                let date_string = &data[(end_email_index + 1)..];
                let date = DateTime::parse_from_str(date_string, "%s %z").unwrap();
                let date: DateTime<Local> = DateTime::from(date);
                let val = Author::new(name.to_string(), email.to_string(), date);
                if field_name.as_str() == "author" {
                    debug!("Setting author to: {:?}", val);
                    self.author = val;
                } else {
                    debug!("Setting committer to: {:?}", val);
                    self.committer = val;
                }
            }
            _ => panic!("Unknown field"),
        }
    }
}

impl GitObject for Commit {
    fn write_to_disk(&mut self) -> Result<(), std::io::Error> {
        let mut out = String::new();
        out.push_str("tree ");
        out.push_str(&self.tree_sha.to_string());
        if let Some(parents) = &self.parent {
            for parent in parents.iter() {
                out.push_str("\nparent ");
                out.push_str(&parent.to_string());
            }
        }
        let author_string = format!("\nauthor {}", self.author.to_string_with_date());
        out.push_str(&author_string);
        let committer_string = format!("\ncommitter {}", self.committer.to_string_with_date());
        out.push_str(&committer_string);
        for str in self.message.iter() {
            let msg = format!("\n\n{}", str);
            out.push_str(&msg);
        }
        let mut heading = Commit::create_heading(out.len());
        let mut output = vec![];
        output.write(&mut heading)?;
        output.write(&mut out.as_bytes())?;
        let compressed = compress(&*output);
        let full_path = self.sha.to_path();
        let dir_path = full_path.parent().unwrap();
        debug!("write directory: {:?}", dir_path);
        if !dir_path.exists() {
            info!("Path doesn't exist, creating");
            create_dir_all(&dir_path)?;
        }
        debug!("Full write directory: {:?}", full_path);
        let mut file = File::create(&full_path)?;
        file.write_all(&compressed)?;
        println!("{}", self.sha);
        Ok(())
    }

    fn pretty_print(&self) {
        println!("tree {}", self.tree_sha);
        if let Some(parents) = &self.parent {
            for parent in parents.iter() {
                println!("parent {}", parent);
            }
        }
        println!("author {}", self.author.to_string_with_date());
        println!("committer {}", self.committer.to_string_with_date());
        for msg in self.message.iter() {
            println!("{}", msg);
        }
    }

    fn print_type(&self) {
        println!("commit");
    }

    fn get_sha(&self) -> &super::Sha {
        &self.sha
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::structures::decompress;

    use super::*;

    #[test]
    fn strap() {
        let f =
            File::open(".re_flogged/objects/01/095adb293f7ba296426fb5e5ddab5e65ec13c4").unwrap();
        let decomp = decompress(f);
        let as_string: String = decomp.iter().map(|&b| b as char).collect();
        println!("{}", as_string);
        assert_eq!(true, false);
    }
}
