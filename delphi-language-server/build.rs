fn main() {
    let pascal_parser_dir = "../tree-sitter-pascal";
    let mut config = cc::Build::new();

    config.include(pascal_parser_dir);
    config.file(format!("{}/parser.c", pascal_parser_dir));

    // Configure the build
    if cfg!(target_env = "msvc") {
        config.compiler("cl");
    }
    config.opt_level(2);
    config.compile("tree-sitter-pascal");

    // Tell cargo to rerun this script if the parser changes
    println!("cargo:rerun-if-changed=tree-sitter-pascal/parser.c");
    println!("cargo:rerun-if-changed=tree-sitter-pascal/tree_sitter/parser.h");
}
