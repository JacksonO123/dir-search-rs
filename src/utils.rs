use crate::lib_error::{self, ConfigParseError};
use std::io::Read;
use std::num::{NonZero, NonZeroUsize};
use std::{collections, error, fs, io, num, path, string, thread};

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
    const_to_macro!(SEARCH_DIR, &str, search_dir_m, "search_dir");
    const_to_macro!(SEARCH_CONTENTS, &str, search_contents_m, "search_contents");
    const_to_macro!(SEARCH_STR_INSERT, &str, search_str_insert_m, "{search}");
    const_to_macro!(SEARCH_STR, &str, search_str_m, "search_str");
    const_to_macro!(
        PARALLEL_PREFERENCE,
        &str,
        parallel_preference_m,
        "parallel_preference"
    );
}

#[macro_export]
macro_rules! error_log {
    ($arg:expr) => {
        eprintln!("[ERROR]: {}", $arg)
    };
}

pub struct ParseConfig {
    pub search_dir: String,
    pub search_str: String,
    pub search_contents: SearchContents,
    pub parallel_preference: Option<num::NonZeroUsize>,
}

pub enum SearchContents {
    FileName,
    FileContents,
}

pub fn byte_slice_to_string(slice: &[u8]) -> Result<String, string::FromUtf8Error> {
    String::from_utf8(slice.to_vec())
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
                    return Err(lib_error::ConfigParseError::ExpectedEqDelimiter.into());
                }
                _ => {}
            }
        }
        let eq_pos = pos;
        loop {
            pos += 1;
            if pos >= config_contents.len() || config_contents[pos] == b'\n' {
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

    let search_contents = temp_map.get("search_contents").unwrap_or_else(|| {
        panic!(
            "Expected {} to be configured (file_name | file_contents)",
            config::SEARCH_CONTENTS
        )
    });

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

    let parallel_preference_error_msg = concat!(
        "Invalid ",
        config::parallel_preference_m!(),
        " expected nonzero usize"
    );
    let parallel_preference = temp_map.get(config::PARALLEL_PREFERENCE).map(|value| {
        num::NonZeroUsize::new(value.parse::<usize>().expect(parallel_preference_error_msg))
            .expect(parallel_preference_error_msg)
    });

    let parse_config = ParseConfig {
        search_dir: search_dir.clone(),
        search_str: search_str.to_owned(),
        search_contents,
        parallel_preference,
    };

    Ok(parse_config)
}

pub struct LastRunInfo {
    last_run_search_str_len: usize,
    last_run_results: Vec<fs::DirEntry>,
}

pub fn search_with_config(
    config: &ParseConfig,
    search_str: &str,
    last_run_info_option: Option<LastRunInfo>,
) -> Result<Vec<fs::DirEntry>, Box<dyn error::Error>> {
    let dir_contents: Vec<_> = if let Some(last_run_info) = last_run_info_option
        && last_run_info.last_run_search_str_len < search_str.len()
    {
        last_run_info.last_run_results
    } else {
        fs::read_dir(&config.search_dir)?
            .filter_map(|entry| {
                if let Err(err) = &entry {
                    error_log!(err);
                }
                entry.ok()
            })
            .collect()
    };
    let search_str = config
        .search_str
        .replace(config::SEARCH_STR_INSERT, search_str);
    let search_str = search_str.as_str();

    let res = match config.search_contents {
        SearchContents::FileName => search_file_names(dir_contents, search_str),
        SearchContents::FileContents => search_file_contents(config, dir_contents, search_str),
    };

    match res {
        Ok(res) => Ok(res),
        Err(err) => Err(Box::new(err)),
    }
}

pub fn search_file_names(
    dir_contents: Vec<fs::DirEntry>,
    search_str: &str,
) -> Result<Vec<fs::DirEntry>, io::Error> {
    let mut res_paths: Vec<fs::DirEntry> = vec![];

    for dir_entry in dir_contents {
        let name = dir_entry.file_name();
        let name = match name.to_str() {
            Some(value) => value,
            None => {
                error_log!(lib_error::SearchError::FailedToGetFileName);
                continue;
            }
        };

        if name.contains(search_str) {
            res_paths.push(dir_entry);
        }
    }

    Ok(res_paths)
}

pub fn search_file_contents(
    config: &ParseConfig,
    dir_contents: Vec<fs::DirEntry>,
    search_str: &str,
) -> Result<Vec<fs::DirEntry>, io::Error> {
    if dir_contents.is_empty() {
        return Ok(vec![]);
    }

    let core_count = config.parallel_preference.unwrap_or_else(|| {
        thread::available_parallelism().unwrap_or(num::NonZeroUsize::new(1).unwrap())
    });
    let count_per_core = NonZeroUsize::new(dir_contents.len().div_ceil(core_count.get())).unwrap();
    let chunks = to_owned_chunks(dir_contents, count_per_core);

    let result: Vec<fs::DirEntry> = thread::scope(|s| {
        chunks
            .iter()
            .map(|chunk| s.spawn(|| search_chunk(chunk, search_str)))
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect()
    });

    Ok(result)
}

fn to_owned_chunks<T>(items: Vec<T>, chunk_size: NonZero<usize>) -> Vec<Vec<T>> {
    let mut res: Vec<Vec<T>> = Vec::with_capacity(items.len().div_ceil(chunk_size.get()));
    let mut chunk: Vec<T> = Vec::with_capacity(chunk_size.get());

    for item in items {
        chunk.push(item);
        if chunk.len() == chunk_size.get() {
            res.push(chunk);
            chunk = Vec::with_capacity(chunk_size.get());
        }
    }

    res
}

pub fn search_chunk(chunk: &[fs::DirEntry], search_str: &str) -> Vec<fs::DirEntry> {
    let mut res_paths: Vec<fs::DirEntry> = vec![];
    let mut buf = String::new();

    for dir_entry in chunk {
        let mut file = match fs::File::open(dir_entry.path()) {
            Ok(file) => file,
            Err(err) => {
                error_log!(err);
                continue;
            }
        };

        buf.clear();
        let bytes = match file.read_to_string(&mut buf) {
            Ok(value) => value,
            Err(err) => {
                error_log!(err);
                continue;
            }
        };
        if buf[0..bytes].contains(search_str) {
            res_paths.push(dir_entry);
        }
    }

    res_paths
}

pub fn run_search_from_args(
    search_pattern: &str,
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

    search_with_config(&config, search_pattern, None)
}
