mod lib_error;
mod utils;

use std::{io::Write, path};

pub fn search_with_config(
    config: &utils::ParseConfig,
    search_pattern: &String,
) -> Result<Vec<path::PathBuf>, Box<dyn std::error::Error>> {
    utils::search_with_config(config, search_pattern)
}

pub fn search_from_args(search_pattern: &String) {
    match utils::run_search_from_args(search_pattern) {
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

#[test]
fn test_search() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = utils::ParseConfig {
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
