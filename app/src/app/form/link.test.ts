import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { reportLink } from './link.ts'

test('четыре формы ссылки на отчёт: путь провайдера под доменом Longpole, вместо пути проекта — id', () => {
  for (const path of [
    '/gitlab.example.com/g/sub/p/-/pipelines/7',
    '/gitlab.example.com/42/-/pipelines/7',
    '/gitlab.example.com/g/p/-/merge_requests/7',
    '/gitlab.example.com/42/-/merge_requests/7/diffs',
    '/ghe.example.com/o/r/actions/runs/7',
    '/ghe.example.com/42/actions/runs/7/job/99',
    '/ghe.example.com/o/r/pull/7',
    '/ghe.example.com:8443/42/pull/7',
  ]) {
    const host = path.split('/')[1]
    assert.deepEqual(reportLink(path), { host, url: `https://${path.slice(1)}` }, path)
  }
})

test('экран проекта (агрегат), страницы приложения и чужие пути — не ссылка на отчёт', () => {
  for (const path of [
    '/project/gitlab.example.com/42',
    '/report/3',
    '/settings',
    '/',
    '/gitlab.example.com/g/p',
    '/gitlab.example.com/g/p/-/jobs/7',
    '/gitlab.example.com/g/p/-/pipelines/abc',
    '/localhost/g/p/-/pipelines/7',
  ]) {
    assert.equal(reportLink(path), null, path)
  }
})

const repoFile = (path: string) => readFileSync(new URL(`../../../../${path}`, import.meta.url), 'utf8')

/** Как shell, GitLab и выражения GitHub Actions: сам CI в тесте не запускается. */
const expandCiVars = (line: string, vars: Record<string, string>) =>
  line
    .replace(/\$\{\{\s*vars\.(\w+)\s*\}\}/g, (_, name: string) => vars[name])
    .replace(/\$\{(\w+)#([^}]*)\}/g, (_, name: string, prefix: string) => vars[name].replace(prefix, ''))
    .replace(/\$\{?(\w+)\}?/g, (_, name: string) => vars[name])

const longpoleUrl = 'https://longpole.example.com'
const linkPathInCi = (url: string | undefined, vars: Record<string, string>) => {
  assert.ok(url)
  const expanded = expandCiVars(url, { LONGPOLE_URL: longpoleUrl, ...vars })
  assert.ok(expanded.startsWith(`${longpoleUrl}/`), expanded)
  return expanded.slice(longpoleUrl.length)
}

test('шаблон GitLab строит ссылку формы «пайплайн GitLab» по id проекта — в echo и в environment:url', () => {
  const vars = { CI_SERVER_HOST: 'gitlab.example.com', CI_PROJECT_ID: '42', CI_PIPELINE_ID: '7' }
  const template = repoFile('ci/gitlab/longpole.yml')
  for (const url of [/echo "[^"]*: (\S+)"/.exec(template)?.[1], /^\s+url: (\S+)$/m.exec(template)?.[1]]) {
    const path = linkPathInCi(url, vars)
    assert.equal(path, '/gitlab.example.com/42/-/pipelines/7')
    assert.ok(reportLink(path))
  }
})

test('строка GitHub из README строит ссылку формы «run GitHub» по id репозитория', () => {
  const vars = { GITHUB_SERVER_URL: 'https://ghe.example.com', GITHUB_REPOSITORY_ID: '42', GITHUB_RUN_ID: '7' }
  const line = /^- run: .*GITHUB_STEP_SUMMARY.*$/m.exec(repoFile('README.md'))?.[0]
  const path = linkPathInCi(line && /\]\((.+)\)"/.exec(line)?.[1], vars)
  assert.equal(path, '/ghe.example.com/42/actions/runs/7')
  assert.ok(reportLink(path))
})
