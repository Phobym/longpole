# Ссылка «Посмотреть в Longpole» в GitLab и GitHub

Тикет: [#72](https://github.com/Phobym/longpole/issues/72), карта [#71](https://github.com/Phobym/longpole/issues/71).
Дата обращения ко всем источникам: 2026-10-12. Источники — только docs.gitlab.com и
docs.github.com (актуальные версии страниц на эту дату). Что документация прямо не
говорит, помечено «не документировано».

Вопрос: какими штатными механизмами показать ссылку на отчёт Longpole рядом с конкретным
пайплайном (GitLab) или run (GitHub Actions) и что каждый требует.

## 1. Вывод

- **Без кода в CI** ссылку рядом с конкретным пайплайном даёт только одно: сервер
  Longpole слушает вебхук Pipeline events и сам ставит **внешний commit status** с
  `target_url` на отчёт (GitLab: `POST /projects/:id/statuses/:sha` с `pipeline_id`;
  GitHub: `POST /repos/{owner}/{repo}/statuses/{sha}`). Цена: нужен **сервисный токен**
  (GitLab — роль Developer + scope `api`; GitHub — `Commit statuses: write`), а в #71
  общего сервисного токена по умолчанию нет. Плюс в GitLab статус становится джобой в
  стадии `external` внутри самого пайплайна — это видимый побочный эффект.
- **С одной строкой в CI** (без токена) — ссылка в логе джобы или `environment:url`
  с `action: access`; на GitHub — Markdown-ссылка в `GITHUB_STEP_SUMMARY`. Id пайплайна
  берётся из `CI_PIPELINE_ID`/`CI_PROJECT_ID` или `github.run_id`.
- **Бейджи** id пайплайна не подставляют (есть `%{commit_sha}`, `%{project_id}`), видны
  только на Overview проекта — годятся для «отчёт по последнему пайплайну ветки по
  умолчанию», не для конкретного пайплайна.
- **Комментарий в MR/PR** и **Checks API** требуют токена с записью; `CI_JOB_TOKEN` в
  GitLab не умеет ни статусы, ни заметки; Checks API — только для GitHub App.

Рекомендация для пилота (GitLab self-managed): основной путь — сниппет в
`.gitlab-ci.yml` (строка в логе и/или `environment:url`, токен не нужен); вебхук +
внешний статус — опция для администратора, который готов завести сервисный токен.
Это отдельное решение в карте #71, т.к. ломает рамку «нет общего сервисного токена».

## 2. Сводная таблица

| Механизм | Где видна ссылка | Код в CI | Токен и права | Id пайплайна без CI | Источник |
|---|---|---|---|---|---|
| GitLab: бейдж проекта/группы | Overview проекта, под описанием | нет | нет (добавляет Maintainer; групповой — Owner) | **нет**: плейсхолдера id пайплайна нет; есть `%{project_id}`, `%{commit_sha}`, `%{default_branch}` | [gl-badges], [gl-perm] |
| GitLab: `environment:url` | Environments, Deployments (кнопка); MR — при условиях деплоя в default branch | да (`environment:` у джобы) | нет; создавать окружения — Developer+ | нет; в CI — предопределённые переменные, но **не** переменные из `script` (динамика — через `artifacts:reports:dotenv`) | [gl-env], [gl-yaml] |
| GitLab: внешний commit status (`target_url`) | джоба в стадии `external` пайплайна; MR и коммит рядом с CI-джобами | нет, если ставит сервер по вебхуку | Developer («Create commit status»), scope `api`; `CI_JOB_TOKEN` **не** подходит | **да**: вебхук отдаёт `object_attributes.id`, статус привязывается параметром `pipeline_id` | [gl-commits], [gl-ext], [gl-perm], [gl-jobtok], [gl-hook] |
| GitLab: строка в логе джобы | лог джобы | да (`echo`) | нет | нет; в CI — `CI_SERVER_URL`, `CI_PROJECT_ID`, `CI_PIPELINE_ID` | [gl-vars], [gl-logs] (кликабельность URL в логе не документирована) |
| GitLab: `artifacts:expose_as` | MR-виджет, ссылка на артефакт | да | нет | — (ведёт на артефакт, не на внешний URL; пути без переменных, ≤10 джоб на MR) | [gl-yaml] |
| GitLab: комментарий в MR | лента MR | да (или сервер по вебхуку) | Planner+ («Comment»), scope `api`; `CI_JOB_TOKEN` **не** умеет создавать заметки | да, если пишет сервер по вебхуку | [gl-notes], [gl-perm], [gl-jobtok] |
| GitLab: вебхук Pipeline events сам по себе | нигде (только доставка события) | нет | настроить — Maintainer | да (`object_attributes.id`, `url`) | [gl-hook], [gl-perm] |
| GitHub: workflow status badge | где вставлен (обычно README) | нет | нет; в приватном репо снаружи не виден | **нет**: только `branch`, `event` | [gh-badge] |
| GitHub: commit status (`target_url`) | PR (статусы коммитов), ссылка из UI | нет, если ставит сервер по вебхуку | push-доступ; fine-grained `Commit statuses: write`; OAuth `repo:status`; в Actions — `permissions: statuses: write` | да (по SHA из вебхука `workflow_run`) | [gh-status], [gh-fgpat], [gh-syntax], [gh-hooks] |
| GitHub: Checks API (`details_url`, кнопки) | вкладка Checks PR, аннотации | нет, если ставит App | **только GitHub App** (пользователи/OAuth Apps не могут создавать) | да | [gh-checks], [gh-about-checks] |
| GitHub: `environment.url` | деплойменты (`environment_url`); место в UI на прочитанных страницах не описано | да | `deployments` создаёт сам Actions | нет; в CI — выражение, доступен контекст `github`, `steps` | [gh-syntax], [gh-env] |
| GitHub: job summary (`GITHUB_STEP_SUMMARY`) | страница summary workflow run | да (`echo` в файл) | нет | нет; в CI — `github.run_id` | [gh-cmds] |
| GitHub: комментарий в PR | лента PR | да (или сервер) | `Issues: write` или `Pull requests: write` | да, если пишет сервер | [gh-fgpat] |

## 3. Детали по механизмам

### GitLab

**Бейджи.** Плейсхолдеры: `%{project_path}`, `%{project_title}`, `%{project_name}`,
`%{project_id}`, `%{project_namespace}`, `%{group_name}`, `%{gitlab_server}`,
`%{gitlab_pages_domain}`, `%{default_branch}`, `%{commit_sha}`, `%{latest_tag}` — id
пайплайна и джобы нет [gl-badges]. Показываются на Overview проекта; групповые — у каждого
проекта группы. Добавляют Maintainer/Owner, групповые — Owner [gl-badges], [gl-perm].
Вывод: бейдж может вести на `…/report?project=%{project_id}&sha=%{commit_sha}`, и Longpole
сам найдёт пайплайн по SHA — но это «последний коммит ветки по умолчанию», не текущий
пайплайн пользователя.

**`environment:url`.** Значение — текст или CI/CD-переменные (предопределённые, проекта,
группы, инстанса, из `.gitlab-ci.yml`); «You can't use variables defined in a `script`
section» [gl-yaml]. Динамический URL — через `artifacts:reports:dotenv`, GitLab раскрывает
`environment:url` после завершения джобы [gl-env]. Видна кнопкой в Environments и
Deployments; в MR — если MR в итоге мержится в default branch и та тоже деплоит в
окружение [gl-env]. `environment:action: access` (а также `prepare`, `verify`) «does not
trigger deployments» [gl-yaml] — так можно не засорять историю деплоев. Минус: семантически
это «окружение», а не отчёт; в MR появляется не всегда.

**Внешний commit status.** `POST /projects/:id/statuses/:sha`, параметры `state`
(обязателен), `name`/`context`, `ref`, `description`, `target_url` (≤255 символов),
`pipeline_id` («Use in case of several pipelines on the same SHA»), `coverage`
[gl-commits]. Если подходящего пайплайна нет, создаётся новый с `CI_PIPELINE_SOURCE:
external`; при превышении лимита джоб — `422`; при гонке на SHA/ref — `409`, повторить
[gl-commits]. В UI статус — джоба в стадии `external`, видна «alongside CI/CD jobs in merge
requests and commit views»; при дублях пайплайнов на один коммит статус может попасть не
туда [gl-ext]. Права: Developer («Create/Update commit status») [gl-perm]; токен — scope
`api` (единственный с записью в REST: «complete read and write access») [gl-scopes].
`CI_JOB_TOKEN` к этому эндпоинту доступа не имеет — в списке разрешённых только GET по
коммитам [gl-jobtok]. Кликабельность `target_url` в UI на прочитанных страницах прямо не
описана.

Побочный эффект: статус меняет состав джоб пайплайна (появляется стадия `external`), и,
если его ставить в `pending`/`failed`, — может влиять на общий статус. Для ссылки на отчёт
разумно ставить `success` после завершения пайплайна. Влияние на итоговый статус уже
завершённого пайплайна не документировано — проверить на пилоте.

**Вебхук Pipeline events.** Поля: `object_attributes.id` (id пайплайна),
`object_attributes.iid`, `object_attributes.url` (страница пайплайна),
`root_pipeline_id`, объект `merge_request` (`iid`, `url`, …) [gl-hook]. Настроить —
Maintainer [gl-perm]. Сам по себе вебхук в UI ничего не пишет; это только триггер для
статуса или комментария.

**Лог джобы.** Предопределённые переменные: `CI_SERVER_URL`, `CI_PROJECT_ID`,
`CI_PIPELINE_ID` (уникален на инстансе), `CI_PIPELINE_IID`, `CI_PIPELINE_URL`,
`CI_MERGE_REQUEST_IID`, `CI_COMMIT_SHA` [gl-vars]. Пример:
`echo "Longpole: $LONGPOLE_URL/p/$CI_PROJECT_ID/$CI_PIPELINE_ID"`. Делают ли логи URL
кликабельными, на странице job logs не сказано [gl-logs].

**`artifacts:expose_as`.** Даёт ссылку в MR-виджете, но только на артефакт: один на джобу,
≤10 джоб на MR, без glob и без переменных в путях [gl-yaml]. Как редирект на внешний URL
не годится (разве что HTML-файл с редиректом — костыль, не рассматриваем).

**Комментарий в MR.** `POST /projects/:id/merge_requests/:merge_request_iid/notes`, `body`
[gl-notes]. Право «Comment and add suggestions» — от Planner [gl-perm]. `CI_JOB_TOKEN`
умеет только читать заметки MR [gl-jobtok] — значит, нужен project/group access token
(`api`). Шумно: комментарий на каждый пайплайн.

### GitHub / GHES

**Workflow badge.** `https://github.com/OWNER/REPOSITORY/actions/workflows/WORKFLOW-FILE/badge.svg`,
параметры только `branch` и `event`; id run нет; в приватном репо снаружи не виден
[gh-badge].

**Commit status.** `POST /repos/{owner}/{repo}/statuses/{sha}`: `state`, `target_url`
(«linked from the GitHub UI»), `description`, `context`; лимит 1000 статусов на SHA и
context; статусы отражаются в PR [gh-status]. Права: push-доступ; fine-grained PAT —
`Commit statuses: write` [gh-fgpat]; в Actions — `permissions: statuses: write`
[gh-syntax]. Сервер может ставить статус по вебхуку `workflow_run` (`completed`)
[gh-hooks]. Вкладку Checks статусы не заполняют [gh-about-checks].

**Checks API.** «Write permission for the REST API to interact with checks is only
available to GitHub Apps. OAuth apps and authenticated users … are not able to create
them» [gh-checks]. Даёт `details_url`, Markdown в `output.summary`, до трёх кнопок
`actions` [gh-checks]. Самый богатый UI, но требует отдельного GitHub App — вне рамок
пилота (GitLab).

**`environment.url`.** Значение может быть выражением (контексты `github`, `steps`, …),
пишется в `environment_url` деплоймента [gh-syntax], [gh-env]. Где именно показывается в
UI, на прочитанных страницах не описано.

**Job summary.** Markdown (GFM) в `$GITHUB_STEP_SUMMARY`, показывается на странице
summary run; 1 MiB на шаг, до 20 шагов на джобу [gh-cmds]. Самый дешёвый путь на GitHub:
`echo "[Посмотреть в Longpole]($URL/…/${{ github.run_id }})" >> $GITHUB_STEP_SUMMARY`.

**Комментарий в PR.** `POST /repos/{owner}/{repo}/issues/{issue_number}/comments`,
`Issues: write` или `Pull requests: write` [gh-fgpat].

## 4. Следствия для карты #71

1. «Отчёт по ссылке» должен принимать адрес, который CI может собрать из своих
   переменных без токена: хост Longpole + `project_id` + `pipeline_id` (GitLab) или
   `owner/repo` + `run_id` (GitHub). Формат URL — решение спеки.
2. Нулевой-CI вариант (вебхук → commit status) требует сервисного токена на стороне
   сервера и хранения секрета вебхука — это противоречит рамке «общего сервисного токена
   по умолчанию нет». Решить: опция администратора или вне объёма.
3. Бейдж — дополнительный вход «отчёт по последнему коммиту ветки по умолчанию» через
   `%{commit_sha}`; Longpole должен уметь найти пайплайн по SHA.

## Источники

- [gl-badges] https://docs.gitlab.com/user/project/badges/
- [gl-env] https://docs.gitlab.com/ci/environments/
- [gl-yaml] https://docs.gitlab.com/ci/yaml/ (`environment:url`, `environment:action`, `artifacts:expose_as`)
- [gl-commits] https://docs.gitlab.com/api/commits/ (Set the pipeline status of a commit)
- [gl-ext] https://docs.gitlab.com/ci/ci_cd_for_external_repos/external_commit_statuses/
- [gl-perm] https://docs.gitlab.com/user/permissions/
- [gl-scopes] https://docs.gitlab.com/security/tokens/access_token_scopes/
- [gl-jobtok] https://docs.gitlab.com/ci/jobs/ci_job_token/
- [gl-hook] https://docs.gitlab.com/user/project/integrations/webhook_events/ (Pipeline events)
- [gl-notes] https://docs.gitlab.com/api/notes/
- [gl-vars] https://docs.gitlab.com/ci/variables/predefined_variables/
- [gl-logs] https://docs.gitlab.com/ci/jobs/job_logs/
- [gh-badge] https://docs.github.com/en/actions/how-tos/monitor-workflows/add-a-status-badge
- [gh-status] https://docs.github.com/en/rest/commits/statuses
- [gh-checks] https://docs.github.com/en/rest/checks/runs
- [gh-about-checks] https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/about-status-checks
- [gh-fgpat] https://docs.github.com/en/rest/authentication/permissions-required-for-fine-grained-personal-access-tokens
- [gh-syntax] https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax (`permissions`, `jobs.<job_id>.environment`)
- [gh-env] https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments
- [gh-cmds] https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands (job summaries)
- [gh-hooks] https://docs.github.com/en/webhooks/webhook-events-and-payloads (`workflow_run`, `check_run`, `check_suite`)
