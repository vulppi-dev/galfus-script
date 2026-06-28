use crate::{compile_file_to_gfb, load_gfb_file};
use std::fs;
use std::path::Path;

#[test]
fn test_gfb_compilation_and_loading_golden() {
    let temp_dir = Path::new("gfb_test_temp");
    fs::create_dir_all(temp_dir).unwrap();

    let source_path = temp_dir.join("input.gfs");
    let output_path = temp_dir.join("output.gfb");

    let code = r#"
        struct Point {
            x: int32,
            y: int32,
        }

        fn compute(a: int32, b: int32): int32 {
            var pt = new(Point) { x: a, y: b };
            return pt.x + pt.y
        }
    "#;

    fs::write(&source_path, code).unwrap();

    // Compile to .gfb
    compile_file_to_gfb(&source_path, &output_path).unwrap();
    assert!(output_path.exists());

    // Load from .gfb
    let loaded = load_gfb_file(&output_path).unwrap();

    // Assert correctness
    assert!(!loaded.name.is_empty());
    assert!(!loaded.functions.is_empty());

    let compute_func = loaded
        .functions
        .iter()
        .find(|f| f.name == "compute")
        .unwrap();
    assert_eq!(compute_func.param_count, 2);
    assert_eq!(compute_func.local_count, 5); // pt + MIR temps

    assert!(!loaded.struct_layouts.is_empty());
    let pt_layout = &loaded.struct_layouts[0];
    assert_eq!(pt_layout.name, "Point");
    assert_eq!(pt_layout.fields.len(), 2);
    assert_eq!(pt_layout.fields[0].name, "x");
    assert_eq!(pt_layout.fields[1].name, "y");

    // Clean up temporary files/folders
    let _ = fs::remove_file(&source_path);
    let _ = fs::remove_file(&output_path);
    let _ = fs::remove_dir(temp_dir);
}
