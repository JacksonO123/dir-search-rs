use std::{fs, num};

use crate::lib_error::CreateConfigError;

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

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            search_dirs: Default::default(),
            search_strs: Default::default(),
            search_contents: SearchContents::FileName(false),
            parallel_preference: Default::default(),
        }
    }
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
    FileContents(Option<Vec<String>>, bool),
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
    pub prefix_end_index: Option<usize>,
    pub search_str: &'a str,
    pub replaced_str: String,
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
