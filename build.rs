use std::fs;

fn main() {
    println!("Precompiling handlebars frontend templates...");

    // Get list of all files in templates_frontend
    let files = fs::read_dir("templates_frontend").unwrap();
    let mut args = Vec::new();
    for file in files {
        args.push(String::from(file.unwrap().path().to_str().unwrap()));
    }

    args.push("-f".to_string());
    args.push("static/js/precompiled_templates.js".to_string());

    // Precompile handlebars templates
    let res = std::process::Command::new("handlebars")
        .args(args)
        .output();

    match res{
        Ok(res) => {
            if !res.status.success() {
                panic!("Failed to precompile handlebars frontend templates: {} {}", String::from_utf8_lossy(&res.stdout), String::from_utf8_lossy(&res.stderr));
            }
        },
        Err(e) => {
            panic!("Failed to precompile handlebars frontend templates: {}", e);
        },
    }

    println!("Compiling typescript to javascript with tsc...");
    // Compile typescript to javascript with tsc
    let res = std::process::Command::new("tsc")
        .args(&["--module", "system", "--lib", "es2015,dom,dom.Iterable", "--target", "es6", "--outFile", "static/js/editor.js", "typescript/Editor.ts"])
        .output()
        .expect("Failed to compile typescript to javascript with tsc");

    println!("cargo:rerun-if-changed=typescript");
    println!("cargo:rerun-if-changed=templates_frontend");
    if !res.status.success() {
        panic!("Failed to compile typescript to javascript with tsc: {}", String::from_utf8_lossy(&res.stdout));
    }
}