mod lib_error;
mod utils;

use std::{io, io::Write};

pub use utils::ParseConfig;
pub use utils::SearchContents;
pub use utils::search_with_config;

pub fn search_from_args(search_pattern: &str) -> Result<(), io::Error> {
    match utils::run_search_from_args(search_pattern) {
        Ok(res) => {
            let null_byte: &[u8] = &[0];
            let stdout = io::stdout();
            let lock = stdout.lock();
            let mut writer = io::BufWriter::new(lock);

            for item in res {
                let item = match item.to_str() {
                    Some(value) => value,
                    None => continue,
                };

                _ = writer.write_all(item.as_bytes());
                _ = writer.write_all(null_byte);
            }

            writer.flush()?;

            Ok(())
        }
        Err(e) => {
            crate::error_log!(e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error, path};

    #[test]
    fn test_search_file_names() -> Result<(), Box<dyn error::Error>> {
        use std::fs;

        let dir = std::env::temp_dir().join(format!("dir_search_contents_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;

        let files: &[(&str, &str)] = &[
            ("another-file2.txt", ""),
            ("some-file1.txt", ""),
            ("the-the-file.txt", ""),
        ];
        for (name, contents) in files {
            fs::write(dir.join(name), contents)?;
        }

        let mut config = utils::ParseConfig {
            search_dir: dir.to_str().unwrap().to_string(),
            search_str: "{search}".to_string(),
            search_contents: utils::SearchContents::FileName,
            parallel_preference: None,
        };

        {
            let res = search_with_config(&config, &"the".to_string(), None)?;
            assert_eq!(
                res,
                expect(&dir, &["another-file2.txt", "the-the-file.txt"])
            );

            let res = search_with_config(&config, &"some".to_string(), None)?;
            assert_eq!(res, expect(&dir, &["some-file1.txt"]));
        }

        config.search_str = "m{search}".to_string();

        {
            let res = search_with_config(&config, &"e-".to_string(), None)?;
            assert_eq!(res, expect(&dir, &["some-file1.txt"]));
        }

        Ok(())
    }

    #[test]
    fn test_search_file_contents() -> Result<(), Box<dyn error::Error>> {
        use std::fs;

        let dir = std::env::temp_dir().join(format!("dir_search_contents_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;

        let files: &[(&str, &str)] = &[
            ("a.txt", "the quick brown fox"),
            ("b.txt", "lazy dog sleeps"),
            ("c.txt", "quick response needed"),
            ("d.txt", "nothing interesting here"),
            ("e.txt", "brown sugar recipe"),
            ("f.txt", "unique_token_xyz present"),
        ];
        for (name, contents) in files {
            fs::write(dir.join(name), contents)?;
        }

        let mut config = utils::ParseConfig {
            search_dir: dir.to_str().unwrap().to_string(),
            search_str: "{search}".to_string(),
            search_contents: utils::SearchContents::FileContents,
            parallel_preference: None,
        };

        assert_eq!(
            sorted(search_with_config(&config, "quick", None)?),
            expect(&dir, &["a.txt", "c.txt"]),
        );
        assert_eq!(
            sorted(search_with_config(&config, "brown", None)?),
            expect(&dir, &["a.txt", "e.txt"]),
        );
        assert_eq!(
            sorted(search_with_config(&config, "unique_token_xyz", None)?),
            expect(&dir, &["f.txt"]),
        );
        assert_eq!(
            search_with_config(&config, "zzz_absent_zzz", None)?,
            Vec::<path::PathBuf>::new(),
        );

        config.search_str = "un{search}".to_string();
        assert_eq!(
            sorted(search_with_config(&config, "ique_token_xyz", None)?),
            expect(&dir, &["f.txt"]),
        );

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    fn expect(dir: &path::PathBuf, names: &[&str]) -> Vec<path::PathBuf> {
        sorted(names.iter().map(|n| dir.join(n)).collect())
    }

    fn sorted(mut v: Vec<path::PathBuf>) -> Vec<path::PathBuf> {
        v.sort();
        v
    }
}
