use std::{error, fmt};

#[derive(Debug)]
pub enum SearchError {
    FailedToGetFileName,
}

impl error::Error for SearchError {}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fmt_string = match self {
            Self::FailedToGetFileName => "failed to get file name",
        };

        write!(f, "{fmt_string}")
    }
}
