# nix-config-parser

A simple parser for the Nix configuration file format.

Based off of https://github.com/NixOS/nix/blob/0079d2943702a7a7fbdd88c0f9a5ad677c334aa8/src/libutil/config.cc#L80-L138.

## Usage

```rust
fn main() {
    let nix_conf_string = String::from("experimental-features = flakes nix-command");
    let nix_conf = nix_config_parser::parse_nix_config_string(nix_conf_string, None)
        .expect("failed to parse nix config string");
    assert_eq!(
        nix_conf.get("experimental-features").unwrap(),
        "flakes nix-command"
    );

    std::fs::write(
        "nix.conf",
        b"experimental-features = flakes nix-command\nwarn-dirty = false\n",
    )
    .expect("failed to write to ./nix.conf");
    let nix_conf = nix_config_parser::parse_nix_config_file(&std::path::Path::new("nix.conf"))
        .expect("failed to parse nix config file");
    assert_eq!(
        nix_conf.get("experimental-features").unwrap(),
        "flakes nix-command"
    );
    assert_eq!(nix_conf.get("warn-dirty").unwrap(), "false");
    std::fs::remove_file("nix.conf").expect("failed to remove ./nix.conf");
}
```
