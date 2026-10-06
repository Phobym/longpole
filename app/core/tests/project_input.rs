//! Разбор ввода диалога «Добавить проект»: ссылки, ssh-URL, папка с клоном.

use pipeline_trace_core::error::{Error, ErrorCode};
use pipeline_trace_core::request::{ProjectRef, parse_project_input};
use tempfile::TempDir;

fn r(host: &str, path: &str) -> Result<ProjectRef, Error> {
    Ok(ProjectRef {
        host: host.into(),
        path: path.into(),
    })
}

#[test]
fn https_ссылки_на_репозиторий_с_git_и_слэшем() {
    for input in [
        "https://gitlab.example.com/group/project",
        "https://gitlab.example.com/group/project/",
        "https://gitlab.example.com/group/project.git",
        "  http://gitlab.example.com/group/project.git/ ",
    ] {
        assert_eq!(
            parse_project_input(input),
            r("gitlab.example.com", "group/project"),
            "{input}"
        );
    }
    assert_eq!(
        parse_project_input("https://h.example/g/sub/p"),
        r("h.example", "g/sub/p")
    );
}

#[test]
fn ссылка_на_пайплайн_и_mr_даёт_проект() {
    assert_eq!(
        parse_project_input("https://h.example/g/p/-/pipelines/123"),
        r("h.example", "g/p")
    );
    assert_eq!(
        parse_project_input("https://h.example/g/p/-/merge_requests/7/pipelines"),
        r("h.example", "g/p")
    );
}

#[test]
fn ссылка_на_подстраницу_проекта_режет_хвост_дефис() {
    for (input, path) in [
        ("https://h.example/g/p/-/tree/main", "g/p"),
        ("https://h.example/g/sub/p/-/jobs/1", "g/sub/p"),
        ("https://h.example/g/p/-/pipelines", "g/p"),
    ] {
        assert_eq!(parse_project_input(input), r("h.example", path), "{input}");
    }
}

#[test]
fn https_с_userinfo_и_токеном_не_попадает_в_хост() {
    assert_eq!(
        parse_project_input("https://oauth2:secret@gitlab.example.com/group/project.git"),
        r("gitlab.example.com", "group/project")
    );
}

#[test]
fn ssh_url_scp_и_с_портом_без_порта_в_хосте() {
    assert_eq!(
        parse_project_input("git@gitlab.example.com:group/project.git"),
        r("gitlab.example.com", "group/project")
    );
    assert_eq!(
        parse_project_input("ssh://git@gitlab.example.com/group/project.git"),
        r("gitlab.example.com", "group/project")
    );
    assert_eq!(
        parse_project_input("ssh://git@gitlab.example.com:2222/group/project"),
        r("gitlab.example.com", "group/project")
    );
}

#[test]
fn мусор_это_project_input_invalid() {
    for input in [
        "",
        "foo",
        "https://h.example/",
        "https://h.example/single",
        "https://h.example/g/../p",
        "ftp://h/g/p",
        "git@h.example:/abs/path",
    ] {
        assert_eq!(
            parse_project_input(input).unwrap_err().code,
            ErrorCode::ProjectInputInvalid,
            "{input:?}"
        );
    }
}

fn repo(config: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();
    std::fs::write(dir.path().join(".git").join("config"), config).unwrap();
    dir
}

#[test]
fn папка_с_клоном_remote_origin() {
    let dir = repo(
        "[core]\n\trepositoryformatversion = 0\n[remote \"upstream\"]\n\turl = git@other:x/y.git\n[remote \"origin\"]\n\turl = git@gitlab.example.com:group/project.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n",
    );
    assert_eq!(
        parse_project_input(dir.path().to_str().unwrap()),
        r("gitlab.example.com", "group/project")
    );
}

#[test]
fn worktree_с_файлом_git_читает_общий_config() {
    let main = repo("[remote \"origin\"]\n\turl = https://gitlab.example.com/group/project.git\n");
    let wt_git = main.path().join(".git").join("worktrees").join("wt");
    std::fs::create_dir_all(&wt_git).unwrap();
    std::fs::write(wt_git.join("commondir"), "../..\n").unwrap();
    let wt = TempDir::new().unwrap();
    std::fs::write(
        wt.path().join(".git"),
        format!("gitdir: {}\n", wt_git.display()),
    )
    .unwrap();
    assert_eq!(
        parse_project_input(wt.path().to_str().unwrap()),
        r("gitlab.example.com", "group/project")
    );
}

#[test]
fn папка_без_git_или_без_origin_это_project_dir_no_remote() {
    let plain = TempDir::new().unwrap();
    assert_eq!(
        parse_project_input(plain.path().to_str().unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ProjectDirNoRemote
    );
    let no_origin = repo("[remote \"upstream\"]\n\turl = git@h:x/y.git\n");
    assert_eq!(
        parse_project_input(no_origin.path().to_str().unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ProjectDirNoRemote
    );
}
