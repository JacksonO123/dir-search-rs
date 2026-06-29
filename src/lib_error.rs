use std::{error, fmt, string};

use crate::utils::config::{search_contents_m, search_dir_m, search_str_insert_m, search_str_m};

#[derive(Debug)]
pub enum ConfigParseError {
    MissingSearchDir,
    ExpectedEqDelimiter,
    UnexpectedSearchContentsValue,
    MissingConfigArg,
    SearchStringDoesNotHaveSearchInsert,
}

impl fmt::Display for ConfigParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fmt_string = match self {
            Self::MissingSearchDir => concat!("missing ", search_dir_m!()),
            Self::ExpectedEqDelimiter => "expected eq delimiter",
            Self::UnexpectedSearchContentsValue => {
                concat!("unexpected ", search_contents_m!(), " value")
            }
            Self::MissingConfigArg => "missing config file path",
            Self::SearchStringDoesNotHaveSearchInsert => {
                concat!(
                    search_str_m!(),
                    " does not include have ",
                    search_str_insert_m!()
                )
            }
        };

        write!(f, "{fmt_string}")
    }
}

#[derive(Debug)]
pub enum SearchError {
    FailedToGetFileName,
}

impl error::Error for SearchError {}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fmt_string = match self {
            Self::FailedToGetFileName => "failed to get file name",
        };

        write!(f, "{fmt_string}")
    }
}

#[derive(Debug)]
pub enum LoadConfigError {
    Utf(string::FromUtf8Error),
    Parse(ConfigParseError),
}

impl std::error::Error for LoadConfigError {}

impl From<string::FromUtf8Error> for LoadConfigError {
    fn from(err: string::FromUtf8Error) -> LoadConfigError {
        LoadConfigError::Utf(err)
    }
}

impl From<ConfigParseError> for LoadConfigError {
    fn from(err: ConfigParseError) -> LoadConfigError {
        LoadConfigError::Parse(err)
    }
}

impl fmt::Display for LoadConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Utf(str_err) => str_err.fmt(f),
            Self::Parse(parse_err) => parse_err.fmt(f),
        }
    }
}
