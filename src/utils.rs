use crate::lib_error::{self, ConfigParseError};
use std::{collections, error, fs, path, string};

macro_rules! const_to_macro {
    ($const_name:ident, $const_type: ty, $macro_name:ident, $value:expr) => {
        macro_rules! $macro_name {
            () => {
                $value
            };
        }

        pub(crate) use $macro_name;

        pub const $const_name: $const_type = $value;
    };
}

pub mod config {
    const_to_macro!(SEARCH_DIR, &'static str, search_dir_m, "search_dir");
    const_to_macro!(
        SEARCH_CONTENTS,
        &'static str,
        search_contents_m,
        "search_contents"
    );
    const_to_macro!(
        SEARCH_STR_INSERT,
        &'static str,
        search_str_insert_m,
        "{search}"
    );
    const_to_macro!(SEARCH_STR, &'static str, search_str_m, "search_str");
}

pub struct ParseConfig {
    pub search_dir: String,
    pub search_str: String,
    pub search_contents: SearchContents,
}

pub enum SearchContents {
    FileName,
    FileContents,
}

pub fn byte_slice_to_string(slice: &[u8]) -> Result<String, string::FromUtf8Error> {
    Ok(String::from_utf8(slice.to_vec())?)
}

pub fn load_config(config_contents: Vec<u8>) -> Result<ParseConfig, lib_error::LoadConfigError> {
    let mut temp_map = collections::HashMap::<String, String>::new();

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
                    return Err(lib_error::ConfigParseError::ExpectedEqDelimiter.into());
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

        let key = byte_slice_to_string(&config_contents[start..eq_pos])?;
        let value = byte_slice_to_string(&config_contents[eq_pos + 1..pos])?;

        temp_map.insert(key.clone(), value.clone());

        pos += 1;
    }

    let search_dir = match temp_map.get(config::SEARCH_DIR) {
        Some(value) => value,
        None => return Err(ConfigParseError::MissingSearchDir.into()),
    };

    let search_contents = temp_map.get("search_contents").expect(
        format!(
            "Expected {} to be configured (file_name | file_contents)",
            config::SEARCH_CONTENTS
        )
        .as_str(),
    );

    let search_contents = match search_contents.as_str() {
        "file_name" => SearchContents::FileName,
        "file_contents" => SearchContents::FileContents,
        _ => {
            return Err(lib_error::ConfigParseError::UnexpectedSearchContentsValue.into());
        }
    };

    let search_str = match temp_map.get(config::SEARCH_STR) {
        Some(value) => {
            if !value.contains(config::SEARCH_STR_INSERT) {
                return Err(
                    lib_error::ConfigParseError::SearchStringDoesNotHaveSearchInsert.into(),
                );
            }

            value
        }
        None => config::SEARCH_STR_INSERT,
    };

    let parse_config = ParseConfig {
        search_dir: search_dir.clone(),
        search_str: search_str.to_owned(),
        search_contents,
    };

    Ok(parse_config)
}

pub fn search_with_config(
    config: &ParseConfig,
    search_str: &str,
) -> Result<Vec<path::PathBuf>, Box<dyn error::Error>> {
    let dir_contents: Vec<_> = fs::read_dir(&config.search_dir)?.collect();
    let mut res_paths: Vec<path::PathBuf> = vec![];
    let search_str = config
        .search_str
        .replace(config::SEARCH_STR_INSERT, search_str);

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
                    None => return Err(Box::new(lib_error::SearchError::FailedToGetFileName)),
                };
                if name.contains(&search_str) {
                    res_paths.push(dir_entry.path().to_owned());
                }
            }
        }
    }

    Ok(res_paths)
}

pub fn run_search_from_args(
    search_pattern: &String,
) -> Result<Vec<path::PathBuf>, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut config_file: Option<String> = None;

    for (i, arg) in args.iter().enumerate() {
        if arg == "-c" {
            if args.len() <= i + 1 {
                return Err(Box::<lib_error::LoadConfigError>::new(
                    lib_error::ConfigParseError::MissingConfigArg.into(),
                ));
            }

            let after = args[i + 1].clone();
            config_file = Some(after);
        }
    }

    let config_contents = fs::read(config_file.expect("Expected config file path"))?;
    let config = load_config(config_contents)?;

    Ok(search_with_config(&config, search_pattern)?)
}
