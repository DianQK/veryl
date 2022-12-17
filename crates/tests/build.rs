use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    println!("cargo:rerun-if-changed=../../testcases");

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_test = Path::new(&out_dir).join("test.rs");
    let mut out_test = File::create(&out_test).unwrap();

    let mut test_list = String::new();
    test_list.push_str("const TESTCASES: ");

    let mut testcases = Vec::new();

    for entry in WalkDir::new("../../testcases") {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            testcases.push(
                entry
                    .path()
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
            let file = entry.path().file_stem().unwrap().to_string_lossy();
            let _ = writeln!(out_test, "#[test]");
            let _ = writeln!(out_test, "fn test_{}() {{", file);
            let _ = writeln!(out_test, "    test(\"{}\");", file);
            let _ = writeln!(out_test, "}}");
        }
    }

    let _ = writeln!(out_test, "const TESTCASES: [&str; {}] = [", testcases.len());
    for testcase in testcases {
        let _ = writeln!(out_test, "    \"{}\",", testcase);
    }
    let _ = writeln!(out_test, "];");
}
