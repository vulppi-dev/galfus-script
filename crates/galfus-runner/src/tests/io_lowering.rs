use crate::workspace::{check_workspace, compile_workspace_to_image};
use galfus_image::Instruction;
use std::fs;

#[test]
fn std_io_println_lowers_builtin_write() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("galfus_io_lowering_{}", std::process::id()));
    let src = root.join("src");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&src)?;
    fs::write(
        root.join("galfus.toml"),
        "[module]\nname = \"io-lowering\"\ntarget = \"app\"\nentry = \"src/main.gfs\"\n",
    )?;
    fs::write(
        src.join("main.gfs"),
        r#"
import { println } from "std/io"

export fn main(args: [[u8]]): i32 {
  println("ok")
  return 0
}
"#,
    )?;

    let check_result = check_workspace(&root)?;
    assert!(
        !check_result.has_errors(),
        "workspace diagnostics: {:?}",
        check_result.diagnostics()
    );
    let io_module = check_result
        .modules()
        .iter()
        .find(|module| module.path().to_string_lossy() == "std/io")
        .expect("std/io module must be checked");
    let io_mir = galfus_ir::builder::MirBuilder::new(
        io_module.graph(),
        io_module.type_result().unwrap(),
        io_module.source().text(),
    )
    .build();
    let io_println = io_mir
        .functions
        .iter()
        .find(|function| function.name == "println")
        .expect("std/io println MIR must be built");
    assert!(
        !io_println.blocks[0].instructions.is_empty(),
        "println MIR: {:#?}",
        io_println
    );
    let image = compile_workspace_to_image(&check_result)?;
    let println = image
        .functions
        .iter()
        .find(|function| function.name == "println")
        .expect("std/io println function must be compiled");
    assert!(
        println
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Write { .. })),
        "println bytecode: {:#?}",
        println.instructions
    );

    fs::remove_dir_all(root)?;
    Ok(())
}
