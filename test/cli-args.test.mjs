import { test } from 'node:test'
import assert from 'node:assert/strict'
import { parseCliArgs } from '../src/cli-args.mjs'

test('ссылка на пайплайн даёт режим pipeline с числовым id', () => {
  const args = parseCliArgs(['https://gitlab.litres.io/platform/react/monorepo/-/pipelines/1025721'])
  assert.deepEqual(args, {
    mode: 'pipeline', host: 'gitlab.litres.io', project: 'platform/react/monorepo',
    pipelineId: '1025721', out: null, open: true,
  })
})

test('ссылка на MR даёт режим mr, хвост ссылки игнорируется', () => {
  const args = parseCliArgs(['https://gitlab.litres.io/g/p/-/merge_requests/7632/pipelines', '--no-open'])
  assert.equal(args.mode, 'mr')
  assert.equal(args.mrIid, '7632')
  assert.equal(args.project, 'g/p')
  assert.equal(args.open, false)
})

test('агрегат: статусы по умолчанию success,manual в верхнем регистре', () => {
  const args = parseCliArgs(['--project', 'g/p', '--ref', 'master', '--last', '20'])
  assert.deepEqual(args, {
    mode: 'aggregate', host: null, project: 'g/p', ref: 'master', source: null,
    last: 20, statuses: ['SUCCESS', 'MANUAL'], out: null, open: true,
  })
})

test('агрегат: --status any снимает фильтр', () => {
  assert.equal(parseCliArgs(['--project', 'g/p', '--status', 'any']).statuses, null)
})

test('агрегат: --source и --host передаются как есть', () => {
  const args = parseCliArgs(['--project', 'g/p', '--source', 'merge_request_event', '--host', 'h.example'])
  assert.equal(args.source, 'merge_request_event')
  assert.equal(args.host, 'h.example')
  assert.equal(args.last, 50)
})

test('ошибки: нет ссылки и проекта, неверная ссылка, неверный --last', () => {
  assert.throws(() => parseCliArgs([]), /Нужна ссылка или --project/)
  assert.throws(() => parseCliArgs(['https://h/g/p/-/jobs/1']), /Не похоже на ссылку/)
  assert.throws(() => parseCliArgs(['--project', 'g/p', '--last', '0']), /--last/)
})

test('--help возвращает текст использования', () => {
  assert.match(parseCliArgs(['--help']).usage, /pipeline-trace <url/)
})

test('неизвестный флаг даёт русскую ошибку со справкой', () => {
  assert.throws(() => parseCliArgs(['--projekt', 'g/p']), (err) => err instanceof Error && /Неверные аргументы/.test(err.message) && /Использование:/.test(err.message))
})
