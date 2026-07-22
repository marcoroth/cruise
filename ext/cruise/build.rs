use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
  let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
  let include_dir = PathBuf::from(&crate_dir).join("include");
  let header_path = include_dir.join("cruise.h");

  let _ = fs::create_dir_all(&include_dir);

  if let Ok(bindings) = cbindgen::generate(&crate_dir) {
    bindings.write_to_file(&header_path);
  }
}
