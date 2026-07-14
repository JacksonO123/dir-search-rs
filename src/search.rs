use core::num;
use std::{
    error, fs,
    io::{self, Read},
    num::NonZero,
    thread,
};

use crate::{
    error_log, lib_error,
    utils::{self},
};

pub fn search_with_config(
    config: &utils::ParseConfig,
    search_str: &str,
    last_run_info_option: Option<utils::LastRunInfo>,
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
        .map(|item| utils::SearchStrData::new(item, search_str))
        .collect::<Vec<_>>();

    let res = match &config.search_contents {
        utils::SearchContents::FileName(from_start) => {
            search_file_names(dir_contents, &search_strs[0].replaced_str, *from_start)
        }
        utils::SearchContents::FileContents(file_filter, search_in_line) => search_file_contents(
            config,
            dir_contents,
            search_strs,
            file_filter.as_ref(),
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

pub fn search_file_contents<'a>(
    config: &utils::ParseConfig,
    dir_contents: Vec<fs::DirEntry>,
    search_strs: Vec<utils::SearchStrData<'a>>,
    file_filters: Option<&Vec<String>>,
    search_in_line: bool,
) -> Result<Vec<fs::DirEntry>, io::Error> {
    let dir_contents = if let Some(file_filters) = file_filters {
        dir_contents
            .into_iter()
            .filter(|item| {
                file_filters
                    .iter()
                    .any(|filter| item.file_name().to_str().unwrap().contains(filter))
            })
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
    let count_per_core =
        num::NonZeroUsize::new(dir_contents.len().div_ceil(core_count.get())).unwrap();
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
    search_strs: &Vec<utils::SearchStrData<'a>>,
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
