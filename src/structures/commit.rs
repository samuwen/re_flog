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
    utils::iterable_to_string,
};

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
    message: String,
    author: Author,
    committer: Author,
    parent: Option<Sha>,
    sha: Sha,
    tree_sha: Sha,
}

impl Commit {
    pub fn new_from_disk<R: BufRead + Seek>(
        reader: &mut R,
        count: usize,
        sha: &Sha,
    ) -> Result<Self, io::Error> {
        info!("Reading commit from disk. Total size: {} bytes", count);
        let msg = format!("Invalid sha: {}", sha);
        let mut total = 0;
        let mut commit = Commit::empty();
        let (name, data) = Commit::read_file_line(reader, &msg, &mut total)?;
        commit.add_field(name, data);
        let (name, data) = Commit::read_file_line(reader, &msg, &mut total)?;
        commit.add_field(name, data);
        let (name, data) = Commit::read_file_line(reader, &msg, &mut total)?;
        commit.add_field(name, data);
        if commit.parent.is_some() {
            // need one more read
            let (name, data) = Commit::read_file_line(reader, &msg, &mut total)?;
            commit.add_field(name, data);
        }
        debug!("total read: {} vs total in file: {}", total, count);
        let mut message = String::new();
        reader.read_line(&mut message)?;
        commit.message = message;
        commit.sha = sha.clone();
        Ok(commit)
    }

    pub fn new_from_tree_sha(tree_sha: Sha, parent: &Option<String>) -> Result<Self, io::Error> {
        let mut message = String::new();
        let stdin = stdin();
        stdin.read_line(&mut message)?;
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
            tree_sha,
        };
        if let Some(parent) = parent {
            me.parent = Some(Sha::new_from_str(parent));
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
        if let Some(p_sha) = &self.parent {
            bytes.write(b"\nparent ")?;
            bytes.write(p_sha.buf())?;
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
            message: String::new(),
            author: Author::empty(),
            committer: Author::empty(),
            parent: None,
            sha: Sha::empty(),
            tree_sha: Sha::empty(),
        }
    }

    pub fn print_recursive(&self) {
        println!("commit {}", self.sha);
        println!("Author:\t{}", self.author.to_string_without_date());
        println!("Date:\t{}\n", self.author.to_string_date());
        println!("    {}", self.message);
        if let Some(sha) = &self.parent {
            let next = load_commit_from_sha(sha).unwrap();
            next.print_recursive();
        }
    }

    fn create_heading(size: usize) -> Vec<u8> {
        let heading = format!("commit {}\0", size);
        heading.chars().map(|ch| ch as u8).collect()
    }

    fn read_file_line<R: BufRead>(
        reader: &mut R,
        msg: &str,
        total: &mut usize,
    ) -> Result<(String, String), io::Error> {
        let mut buf = vec![];
        *total += reader.read_until('\n' as u8, &mut buf)?;
        let string = iterable_to_string(&mut buf.iter());
        let split = string.split_once(" ");
        match split {
            Some((start, rest)) => Ok((start.to_string(), rest.to_string())),
            None => {
                exit_with_message(msg);
            }
        }
    }

    fn add_field(&mut self, field_name: String, data: String) {
        let data = data.trim().to_string();
        match field_name.as_str() {
            "tree" => {
                let sha = Sha::new_from_str(&data);
                debug!("Setting tree_sha to {}", sha);
                self.tree_sha = sha;
            }
            "parent" => {
                let sha = Sha::new_from_str(&data);
                debug!("Setting parent to {}", sha);
                self.parent = Some(sha);
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
        if let Some(p_sha) = &self.parent {
            out.push_str("\nparent ");
            out.push_str(&p_sha.to_string());
        }
        let author_string = format!("\nauthor {}", self.author.to_string_with_date());
        out.push_str(&author_string);
        let committer_string = format!("\ncommitter {}", self.committer.to_string_with_date());
        out.push_str(&committer_string);
        out.push_str("\n");
        out.push_str(&self.message);
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
        if let Some(p) = &self.parent {
            println!("parent {}", p);
        }
        println!("author {}", self.author.to_string_with_date());
        println!("committer {}", self.committer.to_string_with_date());
        println!("\n{}", self.message);
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
