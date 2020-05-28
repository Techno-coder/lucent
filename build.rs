use std::path::PathBuf;

fn main() {
	let path: PathBuf = ["tree-sitter-lucent", "src"].iter().collect();
	cc::Build::new().include(&path).file(path.join("parser.c"))
		.file(path.join("scanner.c")).compile("tree-sitter-lucent");
}
