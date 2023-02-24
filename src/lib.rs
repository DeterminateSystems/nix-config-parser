//! # nix-config-parser
//!
//! A simple parser for the Nix configuration file format.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use thiserror::Error;

/// A newtype wrapper around a [`String`] that represents a `nix.conf` setting
/// key.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NixConfigKey(pub String);

impl std::fmt::Display for NixConfigKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for NixConfigKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}
impl From<String> for NixConfigKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// A newtype wrapper around a [`String`] that represents a `nix.conf` setting
/// value.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NixConfigValue(pub String);

impl std::fmt::Display for NixConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for NixConfigValue {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}
impl From<String> for NixConfigValue {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// A newtype wrapper around a [`HashMap`], where the key is the name of the Nix
/// setting, and the value is the value of that setting. If the setting accepts
/// a list of values, the value will be space delimited.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NixConfig {
    settings: HashMap<NixConfigKey, NixConfigValue>,
}

impl NixConfig {
    pub fn new() -> Self {
        Self {
            settings: HashMap::new(),
        }
    }

    pub fn settings(&self) -> &HashMap<NixConfigKey, NixConfigValue> {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut HashMap<NixConfigKey, NixConfigValue> {
        &mut self.settings
    }

    pub fn to_settings(&self) -> HashMap<NixConfigKey, NixConfigValue> {
        self.settings.clone()
    }

    pub fn into_settings(self) -> HashMap<NixConfigKey, NixConfigValue> {
        self.settings
    }
}

/// An error that occurred while attempting to parse a `nix.conf` [`Path`] or
/// [`String`].
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

/// Attempt to parse the `nix.conf` at the provided path.
///
/// ```rust
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// std::fs::write(
///     "nix.conf",
///     b"experimental-features = flakes nix-command\nwarn-dirty = false\n",
/// )?;
///
/// let nix_conf = nix_config_parser::parse_nix_config_file(&std::path::Path::new("nix.conf"))?;
///
/// assert_eq!(
///     nix_conf.settings().get(&"experimental-features".into()).unwrap(),
///     &"flakes nix-command".into()
/// );
/// assert_eq!(nix_conf.settings().get(&"warn-dirty".into()).unwrap(), &"false".into());
///
/// std::fs::remove_file("nix.conf")?;
/// # Ok(())
/// # }
/// ```
pub fn parse_nix_config_file(path: &Path) -> Result<NixConfig, ParseError> {
    if !path.exists() {
        return Err(ParseError::FileNotFound(path.to_owned()));
    }

    let contents = std::fs::read_to_string(path)
        .map_err(|e| ParseError::FailedToReadFile(path.to_owned(), e))?;

    self::parse_nix_config_string(contents, Some(path))
}

/// Attempt to parse the `nix.conf` out of the provided [`String`]. The `origin`
/// parameter is [`Option`]al, and only influences potential error messages.
///
/// ```rust
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let nix_conf_string = String::from("experimental-features = flakes nix-command");
/// let nix_conf = nix_config_parser::parse_nix_config_string(nix_conf_string, None)?;
///
/// assert_eq!(
///     nix_conf.settings().get(&"experimental-features".into()).unwrap(),
///     &"flakes nix-command".into()
/// );
/// # Ok(())
/// # }
/// ```
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
                Ok(conf) => settings.settings_mut().extend(conf.into_settings()),
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
        settings
            .settings_mut()
            .insert(NixConfigKey(name.to_string()), NixConfigValue(value));
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

        assert_eq!(map.to_settings().get(&"cores".into()), Some(&"4242".into()));
        assert_eq!(
            map.settings().get(&"experimental-features".into()),
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

        assert_eq!(
            map.settings().get(&NixConfigKey("cores".to_string())),
            Some(&NixConfigValue("4242".to_string()))
        );
        assert_eq!(
            map.settings()
                .get(&NixConfigKey("experimental-features".to_string())),
            Some(&NixConfigValue("flakes nix-command".to_string()))
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

        match parse_nix_config_file(temp_dir.path()) {
            Err(ParseError::FailedToReadFile(path, _)) => assert_eq!(path, temp_dir.path()),
            _ => assert!(
                false,
                "trying to read a dir to a string should have returned ParseError::FailedToReadFile"
            ),
        }
    }
}
