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
    #[error("file '{0}' included from '{}' not found", .1.as_ref().map(|path| path.display().to_string()).unwrap_or(String::from("<unknown>")))]
    IncludedFileNotFound(PathBuf, Option<PathBuf>),
    #[error("illegal configuration line '{0}' in '{}'", .1.as_ref().map(|path| path.display().to_string()).unwrap_or(String::from("<unknown>")))]
    IllegalConfiguration(String, Option<PathBuf>),
    #[error("failed to read contents of '{0}': {1}")]
    FailedToReadFile(PathBuf, #[source] std::io::Error),
}

pub fn parse_nix_config_file(path: &Path) -> Result<NixConfig, ParseError> {
    if !path.exists() {
        return Err(ParseError::FileNotFound(path.to_owned()));
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|e| ParseError::FailedToReadFile(path.to_owned(), e))?;

    self::parse_nix_config_string(contents, Some(path))
}

// Mostly a carbon copy of AbstractConfig::applyConfig from Nix:
// https://github.com/NixOS/nix/blob/0079d2943702a7a7fbdd88c0f9a5ad677c334aa8/src/libutil/config.cc#L80
// Some things were adjusted to be more idiomatic, as well as to account for the lack of
// `try { ... } catch (SpecificErrorType &) { }`
pub fn parse_nix_config_string(
    contents: String,
    origin: Option<&Path>,
) -> Result<NixConfig, ParseError> {
    let mut settings = NixConfig::new();

    for line in contents.lines() {
        let mut line = line;

        // skip comments
        if let Some(pos) = line.find('#') {
            line = &line[..pos];
        }

        if line.is_empty() {
            continue;
        }

        let tokens = line.split(&[' ', '\t', '\n', '\r']).collect::<Vec<_>>();

        if tokens.is_empty() {
            continue;
        }

        if tokens.len() < 2 {
            return Err(ParseError::IllegalConfiguration(
                line.to_owned(),
                origin.map(ToOwned::to_owned),
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
                    origin.map(ToOwned::to_owned),
                ));
            }

            let include_path = PathBuf::from(tokens[1]);
            match self::parse_nix_config_file(&include_path) {
                Ok(conf) => settings.extend(conf),
                Err(_) if ignore_missing => {}
                Err(_) if !ignore_missing => {
                    return Err(ParseError::IncludedFileNotFound(
                        include_path,
                        origin.map(ToOwned::to_owned),
                    ));
                }
                _ => unreachable!(),
            }

            continue;
        }

        if tokens[1] != "=" {
            return Err(ParseError::IllegalConfiguration(
                line.to_owned(),
                origin.map(ToOwned::to_owned),
            ));
        }

        let name = tokens[0];
        let value = tokens[2..].join(" ");
        settings.insert(name.to_string(), value);
    }

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_from_string() {
        let res = parse_nix_config_string(
            "cores = 4242\nexperimental-features = flakes nix-command".into(),
            None,
        );

        assert!(res.is_ok());

        let map = res.unwrap();

        assert_eq!(map.get("cores"), Some(&"4242".into()));
        assert_eq!(
            map.get("experimental-features"),
            Some(&"flakes nix-command".into())
        );
    }

    #[test]
    fn parses_config_from_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir
            .path()
            .join("recognizes_existing_different_files_and_fails_to_merge");

        std::fs::write(
            &test_file,
            "cores = 4242\nexperimental-features = flakes nix-command",
        )
        .unwrap();

        let res = parse_nix_config_file(&test_file);

        assert!(res.is_ok());

        let map = res.unwrap();

        assert_eq!(map.get("cores"), Some(&"4242".into()));
        assert_eq!(
            map.get("experimental-features"),
            Some(&"flakes nix-command".into())
        );
    }

    #[test]
    fn errors_on_invalid_config() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir.path().join("does-not-exist");

        match parse_nix_config_string("bad config".into(), None) {
            Err(ParseError::IllegalConfiguration(_, _)) => (),
            _ => assert!(
                false,
                "bad config should have returned ParseError::IllegalConfiguration"
            ),
        }

        match parse_nix_config_file(&test_file) {
            Err(ParseError::FileNotFound(path)) => assert_eq!(path, test_file),
            _ => assert!(
                false,
                "nonexistent path should have returned ParseError::FileNotFound"
            ),
        }

        match parse_nix_config_string(format!("include {}", test_file.display()), None) {
            Err(ParseError::IncludedFileNotFound(path, _)) => assert_eq!(path, test_file),
            _ => assert!(
                false,
                "nonexistent include path should have returned ParseError::IncludedFileNotFound"
            ),
        }

        match parse_nix_config_file(&temp_dir.path()) {
            Err(ParseError::FailedToReadFile(path, _)) => assert_eq!(path, temp_dir.path()),
            _ => assert!(
                false,
                "trying to read a dir to a string should have returned ParseError::FailedToReadFile"
            ),
        }
    }
}
