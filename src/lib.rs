use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use thiserror::Error;

pub type NixConfig = HashMap<String, String>;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("file '{0}' not found")]
    FileNotFound(PathBuf),
    #[error("file '{0}' included from '{1}' not found")]
    IncludedFileNotFound(PathBuf, PathBuf),
    #[error("illegal configuration line '{0}' in '{1}'")]
    IllegalConfiguration(String, PathBuf),
    #[error("failed to read contents of '{0}': {1}")]
    FailedToReadFile(PathBuf, #[source] std::io::Error),
}

pub fn parse_nix_config_file(path: &Path) -> Result<NixConfig, ParseError> {
    if !path.exists() {
        return Err(ParseError::FileNotFound(path.to_owned()));
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|e| ParseError::FailedToReadFile(path.to_owned(), e))?;

    self::parse_nix_config_string(contents, path)
}

// Mostly a carbon copy of AbstractConfig::applyConfig from Nix:
// https://github.com/NixOS/nix/blob/0079d2943702a7a7fbdd88c0f9a5ad677c334aa8/src/libutil/config.cc#L80
// Some things were adjusted to be more idiomatic, as well as to account for the lack of
// `try { ... } catch (SpecificErrorType &) { }`
pub fn parse_nix_config_string(contents: String, origin: &Path) -> Result<NixConfig, ParseError> {
    let mut settings = NixConfig::new();

    for line in contents.lines() {
        let mut line = line;

        // skip comments
        if let Some(pos) = line.find('#') {
            line = &line[..pos];
        }

        let tokens = line.split_ascii_whitespace().collect::<Vec<_>>();

        if tokens.is_empty() {
            continue;
        }

        if tokens.len() < 2 {
            return Err(ParseError::IllegalConfiguration(
                line.to_owned(),
                origin.to_owned(),
            ));
        }

        let mut include = false;
        let mut ignore_missing = false;
        if tokens[0] == "include" {
            include = true;
        } else if tokens[0] == "!include" {
            include = true;
            ignore_missing = true;
        }

        if include {
            if tokens.len() != 2 {
                return Err(ParseError::IllegalConfiguration(
                    line.to_owned(),
                    origin.to_owned(),
                ));
            }

            let include_path = PathBuf::from(tokens[1]);
            match self::parse_nix_config_file(&include_path) {
                Ok(conf) => settings.extend(conf),
                Err(_) if ignore_missing => {}
                Err(_) if !ignore_missing => {
                    return Err(ParseError::IncludedFileNotFound(
                        include_path,
                        origin.to_owned(),
                    ));
                }
                _ => unreachable!(),
            }

            continue;
        }

        if tokens[1] != "=" {
            return Err(ParseError::IllegalConfiguration(
                line.to_owned(),
                origin.to_owned(),
            ));
        }

        let name = tokens[0];
        let value = tokens[2..].join(" ");
        settings.insert(name.to_string(), value);
    }

    Ok(settings)
}
