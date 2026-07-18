use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_WORKSPACE_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
fn load_workspace_reads_all_nested_source_files() {
    let workspace_root = std::env::current_dir()
        .expect("current directory")
        .join(".tmp")
        .join(format!(
            "runner-load-workspace-{}",
            NEXT_WORKSPACE_ID.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(workspace_root.join("nested")).expect("temporary workspace");
    std::fs::write(
        workspace_root.join("galfus.toml"),
        "[module]\nname = \"runner-test\"\ntarget = \"app\"\nentry = \"main.gfs\"\n",
    )
    .expect("configuration");
    std::fs::write(
        workspace_root.join("main.gfs"),
        "export fn main(args: [[u8]]): i32 { return 0 }",
    )
    .expect("entry source");
    std::fs::write(
        workspace_root.join("nested/helper.gfs"),
        "export fn helper(): i32 { return 1 }",
    )
    .expect("nested source");

    let mut workspace = load_workspace(workspace_root.as_path()).expect("loads workspace");
    assert!(workspace.check().is_valid);
    assert_eq!(
        workspace.compile().expect("compiles workspace").graph.len(),
        2
    );

    std::fs::remove_dir_all(workspace_root).expect("remove temporary workspace");
}
