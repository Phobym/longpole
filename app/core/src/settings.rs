//! Настройки приложения: `settings.json` с языком интерфейса.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::json_file;
use crate::schema::Locale;

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    locale: Option<Locale>,
}

pub struct Settings {
    file: PathBuf,
}

impl Settings {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self { file: file.into() }
    }

    /// Выбранный язык, а пока выбора не было — системный.
    pub fn locale(&self) -> Result<Locale, Error> {
        let stored: Stored = json_file::read(&self.file)?;
        Ok(stored.locale.unwrap_or_else(Locale::system))
    }

    pub fn set_locale(&self, locale: Locale) -> Result<(), Error> {
        json_file::write(
            &self.file,
            &Stored {
                locale: Some(locale),
            },
        )
    }
}
