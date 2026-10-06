//! Списки для формы: проекты, ветки, последние пайплайны.

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use crate::error::Error;
use crate::gitlab::{Connection, Gql, query_as, query_project};

/// Страница списка; `next` — курсор следующей страницы, `null` у последней.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<String>,
}

impl<T> Page<T> {
    fn from_connection<W>(connection: Connection<W>, convert: impl FnMut(W) -> T) -> Self {
        Self {
            items: connection.nodes.into_iter().map(convert).collect(),
            next: connection.page_info.next(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Project {
    /// путь вида `group/sub/project`
    pub full_path: String,
    pub name: String,
    pub last_activity_at: Option<String>,
    pub default_branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Commit {
    pub sha: String,
    pub title: String,
}

/// Пайплайн в списке последних.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Pipeline {
    /// число из gid
    pub id: String,
    pub iid: String,
    /// строчными: `manual`, `running`
    pub status: String,
    pub source: Option<String>,
    pub created_at: String,
    /// мс
    pub duration: Option<i64>,
    pub commit: Option<Commit>,
    pub author: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RootRef {
    root_ref: Option<String>,
}

const PROJECTS_QUERY: &str = "query($search: String, $after: String) {
  projects(membership: true, search: $search, sort: \"latest_activity_desc\", first: 20, after: $after) {
    pageInfo { hasNextPage endCursor }
    nodes { fullPath nameWithNamespace lastActivityAt repository { rootRef } }
  }
}";

#[derive(Deserialize)]
struct ProjectsData {
    projects: Connection<WireProject>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireProject {
    full_path: String,
    name_with_namespace: String,
    last_activity_at: Option<String>,
    repository: Option<RootRef>,
}

impl From<WireProject> for Project {
    fn from(p: WireProject) -> Self {
        Project {
            full_path: p.full_path,
            name: p.name_with_namespace,
            last_activity_at: p.last_activity_at,
            default_branch: p.repository.and_then(|r| r.root_ref),
        }
    }
}

/// Проекты, где пользователь участник, — от недавно активных.
pub async fn list_projects(
    gql: &impl Gql,
    search: &str,
    after: Option<&str>,
) -> Result<Page<Project>, Error> {
    let search = Some(search.trim()).filter(|s| !s.is_empty());
    let data: ProjectsData = query_as(
        gql,
        PROJECTS_QUERY,
        json!({ "search": search, "after": after }),
    )
    .await?;
    Ok(Page::from_connection(data.projects, Project::from))
}

const PROJECT_QUERY: &str = "query($project: ID!) {
  project(fullPath: $project) { fullPath nameWithNamespace lastActivityAt repository { rootRef } }
}";

/// Один проект по пути: проверка доступа при добавлении в «Мои проекты».
pub async fn fetch_project(gql: &impl Gql, path: &str) -> Result<Project, Error> {
    let found: WireProject =
        query_project(gql, PROJECT_QUERY, json!({ "project": path }), path).await?;
    Ok(found.into())
}

const BRANCHES_QUERY: &str = "query($project: ID!, $pattern: String!) {
  project(fullPath: $project) { repository { rootRef branchNames(searchPattern: $pattern, offset: 0, limit: 20) } }
}";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BranchesRepository {
    root_ref: Option<String>,
    branch_names: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct BranchesProject {
    repository: Option<BranchesRepository>,
}

/// Имена веток по подстроке; основная ветка, если попала в выдачу, — первой.
pub async fn list_branches(
    gql: &impl Gql,
    project: &str,
    search: &str,
) -> Result<Vec<String>, Error> {
    // пустой поиск даёт `**`, а шаблон «все ветки» у GitLab — `*`
    let pattern = format!("*{}*", search.trim()).replacen("**", "*", 1);
    let variables = json!({ "project": project, "pattern": pattern });
    let found: BranchesProject = query_project(gql, BRANCHES_QUERY, variables, project).await?;
    let Some(BranchesRepository {
        root_ref,
        branch_names,
    }) = found.repository
    else {
        return Ok(Vec::new());
    };
    let mut names = branch_names.unwrap_or_default();
    let root_at = root_ref.and_then(|root| names.iter().position(|n| *n == root));
    if let Some(i) = root_at {
        names[..=i].rotate_right(1);
    }
    Ok(names)
}

const PIPELINES_QUERY: &str = "query($project: ID!, $ref: String, $after: String) {
  project(fullPath: $project) {
    pipelines(ref: $ref, first: 20, after: $after) {
      pageInfo { hasNextPage endCursor }
      nodes { id iid status source createdAt duration commit { shortId title } user { username } }
    }
  }
}";

#[derive(Deserialize)]
struct PipelinesProject {
    pipelines: Connection<WireRecentPipeline>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireRecentPipeline {
    id: String,
    iid: String,
    status: String,
    source: Option<String>,
    created_at: String,
    /// секунды
    duration: Option<i64>,
    commit: Option<WireCommit>,
    user: Option<WireUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireCommit {
    short_id: String,
    title: String,
}

#[derive(Deserialize)]
struct WireUser {
    username: String,
}

/// Последние пайплайны ветки (или всего проекта при `ref == None`).
pub async fn recent_pipelines(
    gql: &impl Gql,
    project: &str,
    r#ref: Option<&str>,
    after: Option<&str>,
) -> Result<Page<Pipeline>, Error> {
    let variables = json!({ "project": project, "ref": r#ref, "after": after });
    let found: PipelinesProject = query_project(gql, PIPELINES_QUERY, variables, project).await?;
    Ok(Page::from_connection(found.pipelines, |p| Pipeline {
        id: p
            .id
            .rsplit_once('/')
            .map_or(p.id.as_str(), |(_, tail)| tail)
            .into(),
        iid: p.iid,
        status: p.status.to_lowercase(),
        source: p.source,
        created_at: p.created_at,
        duration: p.duration.map(|seconds| seconds.saturating_mul(1000)),
        commit: p.commit.map(|c| Commit {
            sha: c.short_id,
            title: c.title,
        }),
        author: p.user.map(|u| u.username),
    }))
}
