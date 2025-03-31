use crate::error::SgleamError;

const PROMPT: &str = "> ";
const HISTORY_FILE: &str = ".sgleam_history";

pub struct ReplReader {}

impl ReplReader {
    pub fn new() -> Result<ReplReader, SgleamError> {
        panic!()
    }
}

impl Iterator for ReplReader {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        panic!()
    }
}
