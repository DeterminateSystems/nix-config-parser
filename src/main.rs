use std::path::PathBuf;

use nix_config_parser::parse_nix_config_file;

fn main() {
    let file = std::env::args().nth(1).expect("no file");
    let path = PathBuf::from(file);
    let settings = parse_nix_config_file(&path);

    match settings {
        Ok(settings) => {
            eprintln!("{settings:?}");
        }
        Err(settings) => {
            eprintln!("{settings}");
        }
    }
}
