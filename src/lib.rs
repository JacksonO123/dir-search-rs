use std::{fs, io::Write, path};

use crate::utils::ParseConfig;
mod lib_error;
mod utils;

pub fn search_with_config(
    config: &ParseConfig,
    search_pattern: &String,
) -> Result<Vec<path::PathBuf>, Box<dyn std::error::Error>> {
    utils::search_with_config(config, search_pattern)
}

pub fn search_from_args(search_pattern: &String) {
    match run_search_from_args(search_pattern) {
        Ok(res) => {
            let null_byte: &[u8] = &[0];

            for item in res {
                let mut stdout = std::io::stdout();
                let item = match item.to_str() {
                    Some(value) => value,
                    None => continue,
                };
                _ = stdout.write_all(item.as_bytes());
                _ = stdout.write_all(&null_byte);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_search_from_args(
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
    let config = utils::load_config(config_contents)?;

    Ok(utils::search_with_config(&config, search_pattern)?)
}

#[test]
fn test_search() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ParseConfig {
        search_dir: "data".to_string(),
        search_str: "{search}".to_string(),
        search_contents: utils::SearchContents::FileName,
    };

    fn path_buf_from_vec(vec: Vec<&str>) -> path::PathBuf {
        let mut buf = path::PathBuf::new();
        for item in vec {
            buf.push(item);
        }
        buf
    }

    fn to_result(result_layout: Vec<Vec<&str>>) -> Vec<path::PathBuf> {
        let mut res = vec![];

        for path_layout in result_layout {
            res.push(path_buf_from_vec(path_layout));
        }

        res
    }

    {
        let res = search_with_config(&config, &"the".to_string())?;
        let expected_res = to_result(vec![
            vec!["data", "another-file2.txt"],
            vec!["data", "the-the-file.txt"],
        ]);
        assert_eq!(expected_res, res);

        let res = search_with_config(&config, &"some".to_string())?;
        let expected_res = to_result(vec![vec!["data", "some-file1.txt"]]);
        assert_eq!(expected_res, res);
    }

    config.search_str = "m{search}".to_string();

    {
        let res = search_with_config(&config, &"e-".to_string())?;
        let expected_res = to_result(vec![vec!["data", "some-file1.txt"]]);
        assert_eq!(expected_res, res);
    }

    Ok(())
}
