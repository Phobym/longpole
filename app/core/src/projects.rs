//! «Мои проекты»: `projects.json`, в порядке добавления, без секретов.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

use crate::error::Error;
use crate::iso::iso;
use crate::json_file;
use crate::request::ProjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedProject {
    pub host: String,
    pub path: String,
    /// `nameWithNamespace` из GitLab
    pub name: String,
    /// ISO 8601
    pub added_at: String,
}

impl SavedProject {
    fn is(&self, project: &ProjectRef) -> bool {
        self.host == project.host && self.path == project.path
    }
}

pub struct Projects {
    file: PathBuf,
    lock: Mutex<()>,
}

impl Projects {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self {
            file: file.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn list(&self) -> Result<Vec<SavedProject>, Error> {
        json_file::read(&self.file)
    }

    /// В конец списка; уже добавленный проект остаётся на месте, обновляется только имя.
    pub fn add(
        &self,
        project: ProjectRef,
        name: &str,
        now: OffsetDateTime,
    ) -> Result<Vec<SavedProject>, Error> {
        let _guard = json_file::lock(&self.lock);
        let mut list = self.list()?;
        match list.iter_mut().find(|p| p.is(&project)) {
            Some(found) => found.name = name.into(),
            None => list.push(SavedProject {
                host: project.host,
                path: project.path,
                name: name.into(),
                added_at: iso(now),
            }),
        }
        self.save(list)
    }

    pub fn remove(&self, project: &ProjectRef) -> Result<Vec<SavedProject>, Error> {
        let _guard = json_file::lock(&self.lock);
        self.save(
            self.list()?
                .into_iter()
                .filter(|p| !p.is(project))
                .collect(),
        )
    }

    fn save(&self, list: Vec<SavedProject>) -> Result<Vec<SavedProject>, Error> {
        json_file::write(&self.file, &list)?;
        Ok(list)
    }
}
