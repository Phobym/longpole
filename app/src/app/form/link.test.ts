import assert from 'node:assert/strict'
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
