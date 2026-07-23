use galfus_workspace::Workspace;

fn main() {
    let mut workspace = Workspace::new();

    let config = r#"
        [module]
        name = "test-app"
        target = "app"
        entry = "main.gfs"
    "#;
    workspace.load_config(config.as_bytes()).unwrap();

    let format_gfs = r#"
        export fn stringify<T>(value: T): [u8] {
            return "specialized"
        }
    "#;
    workspace.load_module("format.gfs", format_gfs.as_bytes()).unwrap();

    // 1. Initial compilation, format.gfs is reachable and stringify<i32> is specialized
    let main_imports_format = r#"
        import { stringify } from "./format.gfs"
        export fn main(args: [[u8]]): i32 { 
            stringify(42)
            return 0 
        }
    "#;
    workspace.load_module("main.gfs", main_imports_format.as_bytes()).unwrap();
    workspace.check();
    
    println!("--- FIRST COMPILE ---");
    let report1 = workspace.compile().unwrap();
    println!("Graph modules: {}", report1.graph.modules().count());

    // 2. Modify main so format.gfs is unreachable
    let main_empty = r#"
        export fn main(args: [[u8]]): i32 { return 0 }
    "#;
    workspace.load_module("main.gfs", main_empty.as_bytes()).unwrap();
    workspace.check();
    
    println!("--- SECOND COMPILE ---");
    let report2 = workspace.compile().unwrap();
    println!("Graph modules: {}", report2.graph.modules().count());

    // 3. Modify main so format.gfs is reachable again
    // This should trigger the bug if it panics during generic specialization reuse
    workspace.load_module("main.gfs", main_imports_format.as_bytes()).unwrap();
    workspace.check();
    
    println!("--- THIRD COMPILE ---");
    let report3 = workspace.compile().unwrap();
    println!("Graph modules: {}", report3.graph.modules().count());
}
