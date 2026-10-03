//! Файлы данных приложения: `history.json`, `settings.json`, список хостов.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{Error, ErrorCode};

fn storage_error(path: &Path, cause: impl std::fmt::Display) -> Error {
    Error::new(ErrorCode::Storage).with("detail", format!("{}: {cause}", path.display()))
}

/// Содержимое файла; нет файла — значение по умолчанию, битый файл — ошибка, а не потеря данных.
pub(crate) fn read<T: DeserializeOwned + Default>(path: &Path) -> Result<T, Error> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| storage_error(path, e)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(storage_error(path, e)),
    }
}

/// Через временный файл рядом: сбой посреди записи не оставляет половину JSON.
pub(crate) fn write<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), Error> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| storage_error(path, e))?;
    }
    let json = serde_json::to_vec_pretty(value).map_err(|e| storage_error(path, e))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json)
        .and_then(|()| fs::rename(&tmp, path))
        .map_err(|e| storage_error(path, e))
}

/// Замок ничего не охраняет в памяти, поэтому отравление (паника в другом потоке) не повод падать дальше.
pub(crate) fn lock(mutex: &Mutex<()>) -> MutexGuard<'_, ()> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
