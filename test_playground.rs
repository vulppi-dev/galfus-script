use galfus_playground::Playground;

fn main() {
    let mut pg = Playground::new();
    pg.set_config(b"[module]\nname = \"playground\"\ntarget = \"app\"\nentry = \"src/main.gfs\"\n").unwrap();
    let code = "import \"text\"\nimport \"format\"\nfn main(): null { return }";
    pg.set_source("src/main.gfs", code.as_bytes()).unwrap();
    let check = pg.check();
    println!("isValid: {}, diagnostics: {}", check.is_valid, check.diagnostics);
}
