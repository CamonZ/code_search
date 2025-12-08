use std::error::Error;

use serde::Serialize;

use super::FileCmd;
use crate::commands::Execute;
use crate::queries::file::{find_functions_in_file, FileWithFunctions};

/// Result of the file command execution
#[derive(Debug, Default, Serialize)]
pub struct FileResult {
    pub project: String,
    pub file_pattern: String,
    pub files: Vec<FileWithFunctions>,
}

impl Execute for FileCmd {
    type Output = FileResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = FileResult {
            project: self.project.clone(),
            file_pattern: self.file.clone(),
            ..Default::default()
        };

        result.files = find_functions_in_file(
            db,
            &self.file,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}
