use std::{
    io::{self, Write},
    path::Path,
    process::{Command, Output},
};

fn setup() {
    let output = Command::new("target/debug/re_flog")
        .arg("init")
        .output()
        .expect("Failed to start");
    write_output(&output);
}

fn teardown() {
    let output = Command::new("rm")
        .arg("-rf")
        .arg(".re_flogged")
        .output()
        .expect("Failed to start");
    write_output(&output);
}

fn write_output(output: &Output) {
    // println!("status: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
}

fn update_index(path: &Path) {
    let output = Command::new("target/debug/re_flog")
        .arg("update-index")
        .arg("--add")
        .arg(path)
        .output()
        .expect("Failed to start");
    write_output(&output);
}

fn write_tree() -> String {
    let output = Command::new("target/debug/re_flog")
        .arg("write-tree")
        .output()
        .expect("Failed to start");
    write_output(&output);
    output.stdout.iter().map(|&b| b as char).collect()
}

#[test]
fn write_tree_flow() {
    setup();
    let p = Path::new("boop/README.md");
    update_index(p);
    let p = Path::new("boop/STUFF.md");
    update_index(p);
    let written_tree = write_tree();
    assert_eq!(
        written_tree.trim(),
        "0c372d2495259d37c63f554c3bdafd410dae4ae7"
    );
    teardown();
}
