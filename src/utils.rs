use std::io::Read;
use std::num::{NonZero, NonZeroUsize};
use std::{error, fs, io, num, thread};

use crate::lib_error::{CreateConfigError, SearchError};

const SEARCH_STR_INSERT: &str = "{search}";

#[macro_export]
macro_rules! error_log {
    ($arg:expr) => {
        eprintln!("[ERROR]: {}", $arg)
    };
}

pub struct ParseConfig {
    pub search_dirs: Vec<String>,
    pub search_strs: Vec<String>,
    pub search_contents: SearchContents,
    pub parallel_preference: Option<num::NonZeroUsize>,
}

impl ParseConfig {
    pub fn try_new(
        search_dirs: Vec<String>,
        search_strs: Vec<String>,
        search_contents: SearchContents,
        parallel_preference: Option<num::NonZeroUsize>,
    ) -> Result<Self, CreateConfigError> {
        if search_strs.is_empty() {
            return Err(CreateConfigError::MissingSearchStrs);
        }

        if let SearchContents::FileName(from_start) = search_contents
            && from_start
            && search_strs.len() > 1
        {
            return Err(CreateConfigError::TooManySearchStrs);
        }

        Ok(Self {
            search_dirs,
            search_strs,
            search_contents,
            parallel_preference,
        })
    }
}

pub enum SearchContents {
    FileName(bool),
    /// file name filter (contains), search in line
    /// when search in line is enabled, the search will look for the inserted
    /// search string in the string from the end of the pre {search} sentinel
    /// to the end of the line
    ///
    /// example:
    ///   search_strs: vec!["a_test={search}".to_string()]
    /// where:
    ///   search in line = true
    ///   {search} = "my_string"
    /// with the file contents:
    ///   a_test=some text before my_string
    /// will match this file
    /// with search in line = false it will not
    FileContents(Option<String>, bool),
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

pub struct SearchStrData<'a> {
    prefix_end_index: Option<usize>,
    search_str: &'a str,
    replaced_str: String,
}

impl<'a> SearchStrData<'a> {
    pub fn new(raw_search_str: &str, search_str: &'a str) -> Self {
        Self {
            prefix_end_index: raw_search_str.find(SEARCH_STR_INSERT),
            search_str,
            replaced_str: raw_search_str.replace(SEARCH_STR_INSERT, search_str),
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
    let search_strs = config
        .search_strs
        .iter()
        .map(|item| SearchStrData::new(item, search_str))
        .collect::<Vec<_>>();

    let res = match &config.search_contents {
        SearchContents::FileName(from_start) => {
            search_file_names(dir_contents, &search_strs[0].replaced_str, *from_start)
        }
        SearchContents::FileContents(file_filter, search_in_line) => search_file_contents(
            config,
            dir_contents,
            search_strs,
            file_filter,
            *search_in_line,
        ),
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
                error_log!(SearchError::FailedToGetFileName);
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

pub fn search_file_contents<'a>(
    config: &ParseConfig,
    dir_contents: Vec<fs::DirEntry>,
    search_strs: Vec<SearchStrData<'a>>,
    file_filter: &Option<String>,
    search_in_line: bool,
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
            .map(|chunk| s.spawn(|| search_chunk(chunk, &search_strs, search_in_line)))
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

    if !chunk.is_empty() {
        res.push(chunk);
    }

    res
}

pub fn search_chunk<'a>(
    chunk: Vec<fs::DirEntry>,
    search_strs: &Vec<SearchStrData<'a>>,
    search_in_line: bool,
) -> Vec<fs::DirEntry> {
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
        let file_data = buf[0..bytes].to_ascii_lowercase();
        let contains = search_strs.iter().all(|item| {
            if search_in_line
                && let Some(pre_sentinel_end) = item.prefix_end_index
                && pre_sentinel_end > 0
            {
                let pre_search_sentinel_str =
                    &item.replaced_str[0..pre_sentinel_end].to_ascii_lowercase();
                file_data
                    .find(pre_search_sentinel_str)
                    .map(|prefix_index| {
                        let end = file_data[prefix_index..]
                            .find("\n")
                            .map(|found_index| found_index + prefix_index)
                            .unwrap_or(file_data.len());
                        file_data[prefix_index..end].contains(&item.search_str.to_ascii_lowercase())
                    })
                    .unwrap_or(false)
            } else {
                file_data.contains(&item.replaced_str.to_ascii_lowercase())
            }
        });
        if contains {
            res_paths.push(dir_entry);
        }
    }

    res_paths
}
