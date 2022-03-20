use std::{
    fmt,
    fs::{create_dir_all, metadata, File},
    io::{self, BufRead, Read, Seek, Write},
    path::{Ancestors, Path},
};

use derive_getters::Getters;
use log::*;

use crate::{
    exit_with_message,
    structures::{compress, Blob},
    utils::iterable_to_string,
};

use super::{GitObject, IndexEntry, Sha};

#[derive(Clone, Debug, Getters, PartialEq)]
pub struct TreeNode {
    mode: String,
    name: String,
    sha: Sha,
    node_type: TreeNodeType,
    nodes: Vec<TreeNode>,
}

impl TreeNode {
    fn new_from_reader<R: BufRead>(reader: &mut R) -> Self {
        let (mode, node_type) = TreeNode::read_mode(reader);
        let name = TreeNode::read_filename(reader);
        let sha = TreeNode::read_sha(reader);
        Self {
            mode,
            name,
            sha,
            node_type,
            nodes: vec![],
        }
    }

    fn empty_tree() -> Self {
        Self {
            mode: String::from("40000 "),
            name: String::new(),
            sha: Sha::empty(),
            node_type: TreeNodeType::Tree,
            nodes: vec![],
        }
    }

    fn new_from_data(mode: String, name: String, sha: Sha, node_type: TreeNodeType) -> Self {
        let mut name = name;
        name.push(0 as char); // null byte needs to be here, unsure if it is stripped from index?
        Self {
            mode,
            name,
            sha,
            node_type,
            nodes: vec![],
        }
    }

    fn add_node(&mut self, new_node: TreeNode) {
        let found = self.nodes.iter().position(|test_node| {
            test_node.name() == new_node.name() && test_node.node_type() == new_node.node_type()
        });
        match found {
            Some(e) => {
                // merge nodes
                let mut to_keep = self.nodes.remove(e);
                new_node.nodes().iter().for_each(|n| {
                    to_keep.add_node(n.clone());
                });
                self.nodes.insert(e, to_keep);
            }
            None => {
                debug!("Adding node: {}", new_node);
                self.nodes.push(new_node);
            }
        }
    }

    fn read_mode<R: BufRead>(reader: &mut R) -> (String, TreeNodeType) {
        let mut buf = vec![];
        reader.read_until(' ' as u8, &mut buf).unwrap();
        let node_type = match buf[0] as char == '1' {
            true => TreeNodeType::Blob,
            false => TreeNodeType::Tree,
        };
        let as_string = iterable_to_string(&mut buf.iter());
        (as_string, node_type)
    }

    fn read_filename<R: BufRead>(reader: &mut R) -> String {
        let mut name_buf = vec![];
        reader.read_until(0, &mut name_buf).unwrap();
        iterable_to_string(&mut name_buf.iter())
    }

    fn read_sha<R: Read>(reader: &mut R) -> Sha {
        let mut sha_buf = [0; 20];
        reader.read_exact(&mut sha_buf).unwrap();
        Sha::new_from_bytes(sha_buf)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, io::Error> {
        let mut out = vec![];
        let mode_bytes = self.mode.as_bytes();
        out.write(&mode_bytes)?;
        let name = self.name.clone();
        out.write(&name.as_bytes())?;
        out.write(self.sha.buf())?;
        Ok(out)
    }

    fn write(&mut self) -> Result<(), io::Error> {
        for node in self.nodes.iter_mut() {
            node.write()?;
        }
        if self.node_type == TreeNodeType::Blob {
            return Ok(());
        }
        debug!("Writing node");
        info!("{}", self);
        let mut data = vec![];
        for node in self.nodes.iter() {
            debug!("{}", node);
            data.write(&node.to_bytes()?)?;
        }
        let heading = Tree::create_heading(data.len());
        let mut output = vec![];
        output.write(&heading)?;
        output.write(&data)?;
        self.sha = Sha::new_hash(&output);
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
        Ok(())
    }

    fn process_ancestor(
        &mut self,
        ancestor: &Path,
        prev_node: &Option<Self>,
        missing_ok: bool,
    ) -> Option<Self> {
        debug!("ancestor: {:?}", ancestor);
        if let Some(name) = ancestor.file_name() {
            let name = name.to_str().unwrap().to_string();
            if ancestor.is_file() {
                let path = ancestor.canonicalize().unwrap();
                let blob = Blob::new_from_raw_file(&path);
                let obj_path = blob.sha().to_path();
                if let Err(e) = metadata(obj_path) {
                    if let io::ErrorKind::NotFound = e.kind() {
                        if !missing_ok {
                            error!(
                                "Failed to create tree with invalid object reference: {} {}",
                                blob.sha(),
                                ancestor.display()
                            );
                            let msg = format!(
                                "error: invalid object {} {} for '{}'\nfatal: flog-write-tree: error building trees",
                                blob.mode(),
                                blob.sha(),
                                ancestor.display()
                            );
                            exit_with_message(&msg);
                        }
                        warn!(
                            "Creating tree with invalid object reference: {} {}",
                            blob.sha(),
                            ancestor.display()
                        );
                    }
                }
                let mode = format!("{} ", blob.mode());
                return Some(Self::new_from_data(
                    mode,
                    name,
                    blob.sha().clone(),
                    TreeNodeType::Blob,
                ));
            } else {
                let mut node = Self::new_from_data(
                    String::from("40000 "),
                    name,
                    Sha::empty(),
                    TreeNodeType::Tree,
                );
                let p_node = prev_node.as_ref().unwrap();
                node.add_node(p_node.clone());
                return Some(node);
            }
        }
        None
    }

    fn create_subtree_nodes(
        &mut self,
        ancestors: &mut Ancestors,
        prev_node: Option<Self>,
        missing_ok: bool,
    ) {
        // if there's a next we're not done
        if let Some(ancestor) = ancestors.next() {
            if let Some(node) = self.process_ancestor(ancestor, &prev_node, missing_ok) {
                return self.create_subtree_nodes(ancestors, Some(node), missing_ok);
            } else {
                let node = prev_node.unwrap();
                if let TreeNodeType::Tree = node.node_type() {
                    self.add_node(node);
                }
            }
        }
    }
}

impl fmt::Display for TreeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "0{}{} {}\t{}",
            self.mode,
            self.node_type.get_string(),
            self.sha,
            self.name
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TreeNodeType {
    Blob,
    Tree,
}

impl TreeNodeType {
    fn get_string(&self) -> String {
        match self {
            &TreeNodeType::Blob => "blob".to_string(),
            &TreeNodeType::Tree => "tree".to_string(),
        }
    }
}

#[derive(Clone, Debug, Getters, PartialEq)]
pub struct Tree {
    root: TreeNode,
}

impl Tree {
    pub fn new_from_disk<R: BufRead + Seek>(
        reader: &mut R,
        count: usize,
        sha: Sha,
    ) -> Result<Self, io::Error> {
        info!("Reading tree from disk. Total size: {} bytes", count);
        let mut root = TreeNode::new_from_data(
            String::from("40000 "),
            String::new(),
            sha,
            TreeNodeType::Tree,
        );
        let mut bytes_read = 0;
        let mut before = reader.stream_position().unwrap();
        while bytes_read < count {
            let new_node = TreeNode::new_from_reader(reader);
            let after = reader.stream_position().unwrap();
            bytes_read += (after - before) as usize;
            before = after;
            root.add_node(new_node);
        }
        Ok(Self { root })
    }

    pub fn empty() -> Self {
        Self {
            root: TreeNode::empty_tree(),
        }
    }

    pub fn new_from_path_string(&mut self, path_string: &str, missing_ok: bool) {
        debug!("New tree from path: {}", path_string);
        let path = Path::new(path_string);
        let mut ancestors = path.ancestors();
        self.root
            .create_subtree_nodes(&mut ancestors, None, missing_ok)
    }

    pub fn add_blob_from_index(&mut self, entry: &IndexEntry) {
        info!("Adding blob from index");
        let mode = format!("{:o} ", entry.mode());
        let tree_node = TreeNode::new_from_data(
            mode,
            iterable_to_string(&mut entry.file_name().iter()),
            entry.file_sha().clone(),
            TreeNodeType::Blob,
        );
        self.root.add_node(tree_node);
    }

    pub fn add_tree_from_index(&mut self, entry: &IndexEntry, missing_ok: bool) {
        info!("Adding tree from index");
        let name = iterable_to_string(&mut entry.file_name().iter());
        self.new_from_path_string(&name, missing_ok);
    }

    fn create_heading(size: usize) -> Vec<u8> {
        let heading = format!("tree {}\0", size);
        heading.chars().map(|ch| ch as u8).collect()
    }
}

impl GitObject for Tree {
    fn write_to_disk(&mut self) -> Result<(), io::Error> {
        debug!("Write to disk called");
        self.root.write()?;
        println!("{}", self.root.sha());
        Ok(())
    }

    fn pretty_print(&self) {
        self.root
            .nodes()
            .iter()
            .for_each(|node| println!("{}", node));
    }

    fn print_type(&self) {
        println!("tree");
    }

    fn get_sha(&self) -> &Sha {
        self.root.sha()
    }
}
