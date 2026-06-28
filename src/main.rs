use std::{collections::HashMap, error, fmt, fs, path, string};

use crate::utils::exists_as_val_in_map;
mod utils;

macro_rules! const_to_macro {
    ($const_name:ident, $const_type: ty, $macro_name:ident, $value:expr) => {
        macro_rules! $macro_name {
            () => {
                $value
            };
        }

        const $const_name: $const_type = $value;
    };
}

const_to_macro!(SEARCH_DIR, &'static str, search_dir_m, "search_dir");
const_to_macro!(
    SEARCH_CONTENTS,
    &'static str,
    search_contents_m,
    "search_contents"
);
const_to_macro!(
    CASE_SENSITIVE,
    &'static str,
    case_sensitive_m,
    "case_sensitive"
);
const_to_macro!(
    START_OF_LINE,
    &'static str,
    start_of_line_m,
    "start_of_line"
);
const_to_macro!(
    SEARCH_STR_INSERT,
    &'static str,
    search_str_insert_m,
    "{search}"
);
const_to_macro!(SEARCH_STR, &'static str, search_str_m, "search_str");

#[derive(Debug)]
enum ConfigParseError {
    ExpectedEqDelimiter,
    UnexpectedSearchContentsValue,
    MissingConfigArg,
    UnexpectedCaseSensitiveValue,
    UnexpectedStartOfLineValue,
    SearchStringDoesNotHaveSearchInsert,
}

impl fmt::Display for ConfigParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fmt_string = match self {
            Self::ExpectedEqDelimiter => "expected eq delimiter",
            Self::UnexpectedSearchContentsValue => {
                concat!("unexpected ", search_contents_m!(), " value")
            }
            Self::MissingConfigArg => "missing config file path",
            Self::UnexpectedCaseSensitiveValue => {
                concat!("unexpected ", case_sensitive_m!(), " value")
            }
            Self::UnexpectedStartOfLineValue => {
                concat!("unexpected ", start_of_line_m!(), " value")
            }
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
enum SearchError {
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
enum LoadConfigError {
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

struct ParseConfig {
    search_dir: String,
    search_str: String,
    search_contents: SearchContents,
    case_sensitive: bool,
    start_of_line: bool,
}

enum SearchContents {
    FileName,
    FileContents,
}

fn main() {
    if let Err(e) = run_proc() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_proc() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut config_file: Option<String> = None;
    let search_str = "has";

    for (i, arg) in args.iter().enumerate() {
        if arg == "-c" {
            if args.len() <= i + 1 {
                return Err(Box::<LoadConfigError>::new(
                    ConfigParseError::MissingConfigArg.into(),
                ));
            }

            let after = args[i + 1].clone();
            config_file = Some(after);
        }
    }

    let config_contents = fs::read(config_file.expect("Expected config file path"))?;
    let config = load_config(config_contents)?;

    let res = search_with_config(&config, search_str)?;

    println!("{:?}", res);

    Ok(())
}

fn load_config(config_contents: Vec<u8>) -> Result<ParseConfig, LoadConfigError> {
    let mut temp_map = HashMap::<String, String>::new();

    let mut pos: usize = 0;
    'outer: while pos < config_contents.len() {
        let start = pos;
        loop {
            pos += 1;

            if pos >= config_contents.len() {
                break 'outer;
            }

            match config_contents[pos] as char {
                '=' => break,
                '\n' => {
                    println!("here");
                    return Err(ConfigParseError::ExpectedEqDelimiter.into());
                }
                _ => {}
            }
        }
        let eq_pos = pos;
        loop {
            pos += 1;
            if pos >= config_contents.len() || config_contents[pos] == '\n' as u8 {
                break;
            }
        }

        let key = utils::byte_slice_to_string(&config_contents[start..eq_pos])?;
        let value = utils::byte_slice_to_string(&config_contents[eq_pos + 1..pos])?;

        temp_map.insert(key.clone(), value.clone());

        pos += 1;
    }

    let search_dir = temp_map
        .get(SEARCH_DIR)
        .expect(format!("Expected {} to be configured", SEARCH_DIR).as_str());

    let search_contents = temp_map.get("search_contents").expect(
        format!(
            "Expected {} to be configured (file_name | file_contents)",
            SEARCH_CONTENTS
        )
        .as_str(),
    );

    let search_contents = match search_contents.as_str() {
        "file_name" => SearchContents::FileName,
        "file_contents" => SearchContents::FileContents,
        _ => {
            return Err(ConfigParseError::UnexpectedSearchContentsValue.into());
        }
    };

    let case_sensitive = exists_as_val_in_map::<String, String, LoadConfigError>(
        &temp_map,
        CASE_SENSITIVE.to_string(),
        "true".to_string(),
        ConfigParseError::UnexpectedCaseSensitiveValue.into(),
    )?;

    let start_of_line = exists_as_val_in_map::<String, String, LoadConfigError>(
        &temp_map,
        START_OF_LINE.to_string(),
        "true".to_string(),
        ConfigParseError::UnexpectedStartOfLineValue.into(),
    )?;

    let search_str = match temp_map.get(SEARCH_STR) {
        Some(value) => {
            if !value.contains(SEARCH_STR_INSERT) {
                return Err(ConfigParseError::SearchStringDoesNotHaveSearchInsert.into());
            }

            value
        }
        None => SEARCH_STR_INSERT,
    };

    let parse_config = ParseConfig {
        search_dir: search_dir.clone(),
        search_str: search_str.to_owned(),
        search_contents,
        case_sensitive,
        start_of_line,
    };

    Ok(parse_config)
}

fn search_with_config(
    config: &ParseConfig,
    search_str: &str,
) -> Result<Vec<path::PathBuf>, Box<dyn error::Error>> {
    let dir_contents: Vec<_> = fs::read_dir(&config.search_dir)?.collect();
    let mut res_paths: Vec<path::PathBuf> = vec![];
    let search_str = config.search_str.replace(SEARCH_STR_INSERT, search_str);

    for dir_entry in dir_contents {
        let dir_entry = dir_entry?;
        match config.search_contents {
            SearchContents::FileContents => {
                let contents = fs::read_to_string(dir_entry.path())?;
                if contents.contains(&search_str) {
                    res_paths.push(dir_entry.path().to_owned());
                }
            }
            SearchContents::FileName => {
                let name = dir_entry.file_name();
                let name = match name.to_str() {
                    Some(value) => value,
                    None => return Err(Box::new(SearchError::FailedToGetFileName)),
                };
                if name.contains(&search_str) {
                    res_paths.push(dir_entry.path().to_owned());
                }
            }
        }
    }

    Ok(res_paths)
}
