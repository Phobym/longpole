const ADD_HOST = '__add__'
const STATUS_DEFAULTS = ['SUCCESS', 'MANUAL']
const dateFmt = new Intl.DateTimeFormat('ru', { dateStyle: 'short', timeStyle: 'short' })
// повторяет URL_RE из src/cli-args.mjs: рендерер — обычный <script>, а не модуль,
// импортировать оттуда напрямую нельзя
const LINK_RE = /^https?:\/\/([^/]+)\/(.+?)\/-\/(pipelines|merge_requests)\/(\d+)/

const $ = (id) => document.getElementById(id)

const modeLinkBtn = $('mode-link')
const modeAggBtn = $('mode-aggregate')
const urlInput = $('url')
const fieldHost = $('field-host')
const hostSelect = $('host-select')
const removeTokenBtn = $('remove-token')
const fieldNewHost = $('field-new-host')
const newHostInput = $('new-host')
const newTokenInput = $('new-token')
const saveTokenBtn = $('save-token')
const projectInput = $('project')
const refInput = $('ref')
const sourceSelect = $('source')
const lastInput = $('last')
const statusChecks = [...document.querySelectorAll('.status-cb')]
const statusAny = $('status-any')
const form = $('build-form')
const submitBtn = $('submit')
const formError = $('form-error')
const historyList = $('history-list')
const historyEmpty = $('history-empty')

const FIELD_ERROR_IDS = { url: 'error-url', host: 'error-host', project: 'error-project', last: 'error-last', statuses: 'error-statuses' }
const FIELD_INPUT = { url: urlInput, host: hostSelect, project: projectInput, last: lastInput, statuses: statusChecks[0] }

let mode = 'link'
let unsubscribeProgress = null
let savedHosts = []

function setMode(next) {
  mode = next
  modeLinkBtn.setAttribute('aria-pressed', String(mode === 'link'))
  modeAggBtn.setAttribute('aria-pressed', String(mode === 'aggregate'))
  for (const node of document.querySelectorAll('.link-only')) node.hidden = mode !== 'link'
  for (const node of document.querySelectorAll('.aggregate-only')) node.hidden = mode !== 'aggregate'
  if (mode === 'aggregate') syncHostSubform()
  else syncLinkHost()
  clearErrors()
}

modeLinkBtn.addEventListener('click', () => setMode('link'))
modeAggBtn.addEventListener('click', () => setMode('aggregate'))

function clearErrors() {
  formError.hidden = true
  formError.textContent = ''
  for (const [field, errorId] of Object.entries(FIELD_ERROR_IDS)) {
    const errorEl = $(errorId)
    errorEl.hidden = true
    errorEl.textContent = ''
    const input = FIELD_INPUT[field]
    if (input) { input.removeAttribute('aria-invalid'); input.removeAttribute('aria-describedby') }
  }
}

function showErrors(errors) {
  let focused = null
  for (const [field, message] of Object.entries(errors)) {
    const errorId = FIELD_ERROR_IDS[field]
    const errorEl = errorId && $(errorId)
    if (!errorEl) continue
    errorEl.hidden = false
    errorEl.textContent = message
    const input = FIELD_INPUT[field]
    if (input) {
      input.setAttribute('aria-invalid', 'true')
      input.setAttribute('aria-describedby', errorId)
      if (!focused) focused = input
    }
  }
  if (focused) focused.focus()
}

function showFormError(message) {
  formError.hidden = false
  formError.textContent = message
}

// select показывает «+ Добавить хост…» отдельным пунктом: раскрывает поля хоста и токена
function syncHostSubform() {
  const isAdd = hostSelect.value === ADD_HOST
  fieldNewHost.hidden = !isAdd
  removeTokenBtn.hidden = isAdd || !hostSelect.value
}

hostSelect.addEventListener('change', syncHostSubform)

async function loadHosts(selected) {
  const res = await window.app.hosts()
  savedHosts = res.ok ? res.value : []
  hostSelect.innerHTML = ''
  for (const host of savedHosts) {
    const option = document.createElement('option')
    option.value = host
    option.textContent = `${host} — токен сохранён`
    hostSelect.appendChild(option)
  }
  const addOption = document.createElement('option')
  addOption.value = ADD_HOST
  addOption.textContent = '+ Добавить хост…'
  hostSelect.appendChild(addOption)
  if (selected && savedHosts.includes(selected)) {
    hostSelect.value = selected
  } else if (selected) {
    // хост из истории, токен для него ещё не сохранён — покажем как временный пункт
    const option = document.createElement('option')
    option.value = selected
    option.textContent = selected
    hostSelect.insertBefore(option, addOption)
    hostSelect.value = selected
  } else {
    hostSelect.value = savedHosts.length ? savedHosts[0] : ADD_HOST
  }
  // в режиме «Ссылка» видимость субформы держит syncLinkHost — синхронизация здесь её перезатирала бы
  if (mode === 'aggregate') syncHostSubform()
}

// в режиме «Ссылка» хост определяется из URL, а не выбирается — своя логика вместо syncHostSubform
async function syncLinkHost() {
  const host = mode === 'link' ? (LINK_RE.exec(urlInput.value.trim())?.[1] ?? null) : null
  fieldHost.hidden = !host
  if (!host) {
    fieldNewHost.hidden = true
    return
  }
  await loadHosts(host)
  removeTokenBtn.hidden = true
  fieldNewHost.hidden = savedHosts.includes(host)
  if (!fieldNewHost.hidden) newHostInput.value = host
}

urlInput.addEventListener('input', syncLinkHost)

saveTokenBtn.addEventListener('click', async () => {
  $('error-new-host').hidden = true
  const host = newHostInput.value.trim()
  const token = newTokenInput.value
  const res = await window.app.setToken(host, token)
  if (!res.ok) {
    $('error-new-host').hidden = false
    $('error-new-host').textContent = res.error
    return
  }
  newTokenInput.value = ''
  await loadHosts(host)
})

removeTokenBtn.addEventListener('click', async () => {
  const host = hostSelect.value
  if (!host || host === ADD_HOST) return
  if (!confirm(`Удалить токен для ${host}?`)) return
  const res = await window.app.removeToken(host)
  if (!res.ok) { alert(res.error); return }
  await loadHosts()
})

function setStatuses(values) {
  const any = !values || values.includes('ANY')
  statusAny.checked = any
  for (const cb of statusChecks) {
    cb.checked = !any && values.includes(cb.value)
    cb.disabled = any
  }
}

statusAny.addEventListener('change', () => {
  for (const cb of statusChecks) { cb.checked = false; cb.disabled = statusAny.checked }
})
for (const cb of statusChecks) {
  cb.addEventListener('change', () => { if (cb.checked) statusAny.checked = false })
}

function collectStatuses() {
  return statusAny.checked ? ['ANY'] : statusChecks.filter((cb) => cb.checked).map((cb) => cb.value)
}

function buildFormPayload() {
  if (mode === 'link') return { mode: 'link', url: urlInput.value }
  return {
    mode: 'aggregate',
    host: hostSelect.value === ADD_HOST ? '' : hostSelect.value,
    project: projectInput.value,
    ref: refInput.value,
    source: sourceSelect.value,
    last: lastInput.value,
    statuses: collectStatuses(),
  }
}

function fillForm(entry) {
  setMode(entry.form.mode === 'link' ? 'link' : 'aggregate')
  if (entry.form.mode === 'link') {
    urlInput.value = entry.form.url ?? ''
    syncLinkHost()
  } else {
    projectInput.value = entry.form.project ?? ''
    refInput.value = entry.form.ref ?? ''
    sourceSelect.value = entry.form.source ?? ''
    lastInput.value = entry.form.last ?? 50
    setStatuses(entry.form.statuses ?? STATUS_DEFAULTS)
    loadHosts(entry.host)
  }
}

async function loadHistory() {
  const res = await window.app.history()
  const entries = res.ok ? res.value : []
  historyList.innerHTML = ''
  historyEmpty.hidden = entries.length > 0
  for (const entry of entries) {
    const li = document.createElement('li')
    const button = document.createElement('button')
    button.type = 'button'
    const label = document.createElement('span')
    label.className = 'hist-label'
    label.translate = 'no'
    label.textContent = entry.label
    const meta = document.createElement('span')
    meta.className = 'hist-meta'
    const host = document.createElement('span')
    host.translate = 'no'
    host.textContent = entry.host
    const time = document.createElement('span')
    time.textContent = dateFmt.format(new Date(entry.at))
    meta.append(host, time)
    button.append(label, meta)
    button.addEventListener('click', () => fillForm(entry))
    li.appendChild(button)
    historyList.appendChild(li)
  }
}

function setProgress(loaded, total) {
  submitBtn.textContent = total == null ? 'Загружаю…' : `Загружаю ${loaded} из ${total}…`
}

form.addEventListener('submit', async (event) => {
  event.preventDefault()
  clearErrors()
  submitBtn.disabled = true
  setProgress(0, null)
  unsubscribeProgress = window.app.onProgress(({ loaded, total }) => setProgress(loaded, total))
  try {
    const res = await window.app.build(buildFormPayload())
    if (res.ok) {
      await loadHistory()
    } else if (res.errors) {
      showErrors(res.errors)
    } else {
      showFormError(res.error)
    }
  } finally {
    unsubscribeProgress?.()
    unsubscribeProgress = null
    submitBtn.disabled = false
    submitBtn.textContent = 'Построить отчёт'
  }
})

setMode('link')
loadHosts()
loadHistory()
