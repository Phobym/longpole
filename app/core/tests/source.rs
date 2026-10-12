//! Шов `Source`: GitLab через любой `Gql`.
mod fixtures;

use std::sync::Mutex;

use fixtures::fake_gql;
use longpole_core::error::ErrorCode;
use longpole_core::source::{AnySource, Provider, Source, Stamp};
use serde_json::json;

#[tokio::test]
async fn gitlab_отметка_пайплайна_это_числовые_id_и_updated_at() {
    let gql = fake_gql(|v| {
        assert_eq!(v["id"], json!("gid://gitlab/Ci::Pipeline/5"));
        json!({ "project": { "id": "gid://gitlab/Project/12", "pipeline": {
            "id": "gid://gitlab/Ci::Pipeline/5", "updatedAt": "2026-10-07T10:05:00Z"
        } } })
    });
    assert_eq!(
        gql.pipeline_stamp("g/p", "5").await.unwrap(),
        Stamp {
            project_id: "12".into(),
            pipeline_id: "5".into(),
            version: "2026-10-07T10:05:00Z".into(),
        }
    );
}

#[tokio::test]
async fn gitlab_отметка_без_доступа_это_не_найден() {
    let no_project = fake_gql(|_| json!({ "project": null }));
    assert_eq!(
        no_project
            .pipeline_stamp("g/p", "5")
            .await
            .unwrap_err()
            .code,
        ErrorCode::ProjectNotFound
    );
    let no_pipeline =
        fake_gql(|_| json!({ "project": { "id": "gid://gitlab/Project/12", "pipeline": null } }));
    assert_eq!(
        no_pipeline
            .pipeline_stamp("g/p", "5")
            .await
            .unwrap_err()
            .code,
        ErrorCode::PipelineNotFound
    );
}

#[tokio::test]
async fn gitlab_номер_из_ссылки_становится_gid_а_gid_не_меняется() {
    let seen = Mutex::new(Vec::new());
    let gql = fake_gql(|v| {
        seen.lock().unwrap().push(v["id"].clone());
        json!({ "project": { "pipeline": null } })
    });
    let _ = gql.fetch_pipeline("g/p", "5").await;
    let _ = gql
        .fetch_pipeline("g/p", "gid://gitlab/Ci::Pipeline/6")
        .await;
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            json!("gid://gitlab/Ci::Pipeline/5"),
            json!("gid://gitlab/Ci::Pipeline/6")
        ]
    );
}

#[tokio::test]
async fn gitlab_провайдер_и_пустой_список_workflow() {
    let gql = fake_gql(|_| json!({}));
    assert_eq!(gql.provider(), Provider::Gitlab);
    assert_eq!(gql.list_workflows("g/p").await.unwrap(), vec![]);
}

/// Тип хоста из кэша настроек — только Desktop: в Selfhosted провайдер задан конфигурацией инстанса.
#[cfg(feature = "desktop")]
mod тип_хоста {
    use longpole_core::settings::Settings;
    use longpole_core::source::{Provider, resolve_provider_at};
    use serde_json::json;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn проба_один_раз_дальше_кэш() {
        let dir = TempDir::new().unwrap();
        let settings = Settings::new(dir.path().join("settings.json"));
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/meta"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "installed_version": "3.14.0" })),
            )
            .expect(1)
            .mount(&server)
            .await;
        for _ in 0..2 {
            assert_eq!(
                resolve_provider_at("ghe.example", &settings, &server.uri())
                    .await
                    .unwrap(),
                Provider::Github
            );
        }
    }

    #[tokio::test]
    async fn github_com_без_пробы() {
        let dir = TempDir::new().unwrap();
        let settings = Settings::new(dir.path().join("settings.json"));
        // адрес, по которому никто не слушает: проба упала бы с `network`
        assert_eq!(
            resolve_provider_at("github.com", &settings, "http://127.0.0.1:9")
                .await
                .unwrap(),
            Provider::Github
        );
    }

    #[tokio::test]
    async fn сбой_пробы_не_кэшируется() {
        let dir = TempDir::new().unwrap();
        let settings = Settings::new(dir.path().join("settings.json"));
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/meta"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v3/meta"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("x-github-enterprise-version", "3.14.0"),
            )
            .expect(1)
            .mount(&server)
            .await;
        assert!(
            resolve_provider_at("ghe.example", &settings, &server.uri())
                .await
                .is_err()
        );
        for _ in 0..2 {
            assert_eq!(
                resolve_provider_at("ghe.example", &settings, &server.uri())
                    .await
                    .unwrap(),
                Provider::Github
            );
        }
    }
}

#[tokio::test]
async fn клиент_хоста_из_провайдера_хоста_и_токена() {
    let gl = AnySource::new(Provider::Gitlab, "gl.example", "t").unwrap();
    let gh = AnySource::new(Provider::Github, "ghe.example", "t").unwrap();
    assert_eq!((gl.provider(), gl.host()), (Provider::Gitlab, "gl.example"));
    assert_eq!(
        (gh.provider(), gh.host()),
        (Provider::Github, "ghe.example")
    );
}
