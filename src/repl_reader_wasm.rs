use crate::error::SgleamError;
use std::io::Write;

const PROMPT: &str = "> ";

pub struct ReplReader {}

impl ReplReader {
    pub fn new() -> Result<ReplReader, SgleamError> {
        Ok(ReplReader {})
    }
}

impl Iterator for ReplReader {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        print!("{PROMPT}");
        std::io::stdout().flush().expect("Flush stdout.");

        let mut input = String::new();

        if std::io::stdin().read_line(&mut input).expect("Read stdin") == 0 {
            None
        } else {
            Some(input)
        }
    }
}
