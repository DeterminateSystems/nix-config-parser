# nix-config-parser

A simple parser for the Nix configuration file format.

Based off of https://github.com/NixOS/nix/blob/0079d2943702a7a7fbdd88c0f9a5ad677c334aa8/src/libutil/config.cc#L80-L138.

## Usage

### Read from a file

```rust
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    std::fs::write(
       "nix.conf",
       b"experimental-features = flakes nix-command\nwarn-dirty = false\n",
    )?;

    let nix_conf = nix_config_parser::NixConfig::parse_file(&std::path::Path::new("nix.conf"))?;

    assert_eq!(
       nix_conf.settings().get(&"experimental-features".into()).unwrap(),
       &"flakes nix-command".into()
    );
    assert_eq!(nix_conf.settings().get(&"warn-dirty".into()).unwrap(), &"false".into());

    std::fs::remove_file("nix.conf")?;

    Ok(())
}
```

### Read from a string

```rust
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let nix_conf_string = String::from("experimental-features = flakes nix-command");
    let nix_conf = nix_config_parser::NixConfig::parse_string(nix_conf_string, None)?;

    assert_eq!(
        nix_conf.settings().get(&"experimental-features".into()).unwrap(),
        &"flakes nix-command".into()
    );

    Ok(())
}
```
