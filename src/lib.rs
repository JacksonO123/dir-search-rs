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
