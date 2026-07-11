use crate::lib_error;
use std::io::Read;
use std::num::{NonZero, NonZeroUsize};
use std::{error, fs, io, num, thread};

const SEARCH_STR_INSERT: &str = "{search}";

#[macro_export]
macro_rules! error_log {
    ($arg:expr) => {
        eprintln!("[ERROR]: {}", $arg)
    };
}

pub struct ParseConfig {
    pub search_dirs: Vec<String>,
    pub search_str: String,
    pub search_contents: SearchContents,
    pub parallel_preference: Option<num::NonZeroUsize>,
}

pub enum SearchContents {
    FileName(bool),
    FileContents(Option<String>),
}

pub struct LastRunInfo {
    pub last_run_search_str_len: usize,
    pub last_run_results: Vec<fs::DirEntry>,
}

impl LastRunInfo {
    pub fn new(last_run_search_str_len: usize, last_run_results: Vec<fs::DirEntry>) -> Self {
        Self {
            last_run_search_str_len,
            last_run_results,
        }
    }
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
        config
            .search_dirs
            .iter()
            .filter_map(|item| -> Option<Vec<fs::DirEntry>> {
                fs::read_dir(item).ok().map(|entries| {
                    entries
                        .filter_map(|entry| {
                            if let Err(err) = &entry {
                                error_log!(err);
                            }
                            entry.ok()
                        })
                        .collect::<Vec<_>>()
                })
            })
            .flatten()
            .collect::<Vec<_>>()
    };
    let search_str = config.search_str.replace(SEARCH_STR_INSERT, search_str);
    let search_str = search_str.as_str();

    let res = match &config.search_contents {
        SearchContents::FileName(from_start) => {
            search_file_names(dir_contents, search_str, *from_start)
        }
        SearchContents::FileContents(file_filter) => {
            search_file_contents(config, dir_contents, search_str, file_filter)
        }
    };

    match res {
        Ok(res) => Ok(res),
        Err(err) => Err(Box::new(err)),
    }
}

pub fn search_file_names(
    dir_contents: Vec<fs::DirEntry>,
    search_str: &str,
    from_start: bool,
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

        if if from_start {
            name.starts_with(search_str)
        } else {
            name.contains(search_str)
        } {
            res_paths.push(dir_entry);
        }
    }

    Ok(res_paths)
}

pub fn search_file_contents(
    config: &ParseConfig,
    dir_contents: Vec<fs::DirEntry>,
    search_str: &str,
    file_filter: &Option<String>,
) -> Result<Vec<fs::DirEntry>, io::Error> {
    let dir_contents = if let Some(file_filter) = file_filter {
        dir_contents
            .into_iter()
            .filter(|item| item.file_name().to_str().unwrap().contains(file_filter))
            .collect()
    } else {
        dir_contents
    };

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
            .into_iter()
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

    res.push(chunk);

    res
}

pub fn search_chunk(chunk: Vec<fs::DirEntry>, search_str: &str) -> Vec<fs::DirEntry> {
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
