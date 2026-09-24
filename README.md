# pipeline-trace

Показывает, на что уходит время GitLab-пайплайна: водопад стейджей и джоб в стиле трейсов Sentry, критический путь и p50/p90 по многим пайплайнам. Результат — один HTML-файл без внешних запросов.

## Установка

Нужен Node 22+.

```bash
git clone <url этого репозитория> && cd pipeline-trace && npm link
```

Токен берётся из `glab` (`glab auth login --hostname <host>`). Вместо этого можно задать `GITLAB_TOKEN` вместе с `GITLAB_HOST=<host>`: на другие хосты `GITLAB_TOKEN` не отправляется. Нужен доступ `read_api`.

## Запуск

```bash
# один пайплайн или пайплайн MR
pipeline-trace https://gitlab.example.com/group/project/-/pipelines/1025721
pipeline-trace https://gitlab.example.com/group/project/-/merge_requests/7632

# агрегат по последним 50 пайплайнам master
pipeline-trace --project group/project --ref master --last 50 --host gitlab.example.com

# агрегат по пайплайнам всех MR
pipeline-trace --project group/project --source merge_request_event --last 50 --host gitlab.example.com
```

Флаги: `--status success,manual|any` (по умолчанию `success,manual`), `--out файл.html`, `--no-open`.

Пайплайн, который остановился на ручной джобе, GitLab помечает `manual`. Если на ветке есть короткие служебные пайплайны в статусе `success` (например, Publish-коммиты), они смешаются с полными. Чтобы их отсечь, укажи `--status manual`.

## Как читать отчёт

- Строка стейджа: `длина` — от старта его первой джобы до конца последней; `до конца` — от создания пайплайна до конца стейджа.
- Жирные строки с обводкой — критический путь. Клик по стейджу пересчитывает путь до его конца. Пунктир — ожидание между джобами пути.
- Серый сегмент перед полоской — очередь раннера. Бледные полоски — прошлые попытки.
- В агрегате полоска идёт от p50 старта до p50 конца, линия после неё — до p90 конца. «крит. 80%» — джоба была на критическом пути в 80% пайплайнов.

## Разработка

```bash
npm test
```
