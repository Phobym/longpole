const ADD_HOST = '__add__'
const STATUS_DEFAULTS = ['SUCCESS', 'MANUAL']
const SEARCH_DEBOUNCE = 300
const dateFmt = new Intl.DateTimeFormat('ru', { dateStyle: 'short', timeStyle: 'short' })
const rtf = new Intl.RelativeTimeFormat('ru', { numeric: 'auto' })
const STATUS_PILL = { success: 'ok', running: 'run', pending: 'run', manual: 'mid', canceled: 'mid', skipped: 'mid', created: 'mid', failed: 'bad' }

const $ = (id) => document.getElementById(id)

// стандартный идиом MDN для Intl.RelativeTimeFormat: подбирает наибольшую подходящую единицу
const RELATIVE_DIVISIONS = [
  { amount: 60, unit: 'second' },
  { amount: 60, unit: 'minute' },
  { amount: 24, unit: 'hour' },
  { amount: 7, unit: 'day' },
  { amount: 4.34524, unit: 'week' },
  { amount: 12, unit: 'month' },
  { amount: Infinity, unit: 'year' },
]

function relativeTime(iso) {
  let duration = (new Date(iso).getTime() - Date.now()) / 1000
  for (const { amount, unit } of RELATIVE_DIVISIONS) {
    if (Math.abs(duration) < amount) return rtf.format(Math.round(duration), unit)
    duration /= amount
  }
}

function formatDuration(ms) {
  if (ms == null) return '—'
  const s = ms / 1000
  if (s < 60) return `${Math.round(s)}s`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m}m${String(Math.floor(s % 60)).padStart(2, '0')}s`
  return `${Math.floor(m / 60)}h${String(m % 60).padStart(2, '0')}m`
}

// элементы: экран «Проекты»
const urlInput = $('url')
const buildLinkBtn = $('build-link')
const errorUrl = $('error-url')
const fieldHost = $('field-host')
const hostSelect = $('host-select')
const removeTokenBtn = $('remove-token')
const fieldNewHost = $('field-new-host')
const newHostInput = $('new-host')
const newTokenInput = $('new-token')
const saveTokenBtn = $('save-token')
const tokenHint = $('token-hint')
const projectsPanel = $('projects-panel')
const projectSearchInput = $('project-search')
const projectsStatus = $('projects-status')
const projectsError = $('projects-error')
const projectListEl = $('project-list')
const projectsMoreBtn = $('projects-more')

// элементы: экран «Проект»
const screenProjects = $('screen-projects')
const screenProject = $('screen-project')
const backBtn = $('back-to-projects')
const projectTitle = $('project-title')
const branchInput = $('branch-input')
const branchList = $('branch-list')
const pipelinesStatus = $('pipelines-status')
const pipelinesError = $('pipelines-error')
const pipelineListEl = $('pipeline-list')
const pipelinesMoreBtn = $('pipelines-more')
const statusChecks = [...document.querySelectorAll('.status-cb')]
const statusAny = $('status-any')
const lastInput = $('last')
const buildAggBtn = $('build-aggregate')
const errorLast = $('error-last')
const errorStatuses = $('error-statuses')
const aggregateError = $('aggregate-error')

// история
const historyList = $('history-list')
const historyEmpty = $('history-empty')
const historyClearBtn = $('history-clear')

let savedHosts = []
let currentHost = null

let projectsSeq = 0
let projectsAfter = null
let projectsSearchTimer = null

let currentProject = null
let currentBranch = null
let branchSeq = 0
let branchSearchTimer = null
let pipelinesSeq = 0
let pipelinesAfter = null

// выполняет build, транслирует прогресс через onProgress-колбэк вызывающего и обновляет историю по успеху
async function performBuild(payload, onProgress) {
  const unsubscribe = window.app.onProgress(({ loaded, total }) => onProgress(loaded, total))
  try {
    const res = await window.app.build(payload)
    if (res.ok) await loadHistory()
    return res
  } finally {
    unsubscribe()
  }
}

function progressLabel(loaded, total) {
  return total == null ? 'Загружаю…' : `Загружаю ${loaded} из ${total}…`
}

function showProjectsScreen() {
  screenProject.hidden = true
  screenProjects.hidden = false
}

function showProjectScreen() {
  screenProjects.hidden = true
  screenProject.hidden = false
}

backBtn.addEventListener('click', showProjectsScreen)
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && !screenProject.hidden) showProjectsScreen()
})

// --- ссылка на пайплайн/MR ---

buildLinkBtn.addEventListener('click', async () => {
  errorUrl.hidden = true
  buildLinkBtn.disabled = true
  const label = buildLinkBtn.textContent
  const res = await performBuild({ mode: 'link', url: urlInput.value }, (loaded, total) => {
    buildLinkBtn.textContent = progressLabel(loaded, total)
  })
  buildLinkBtn.disabled = false
  buildLinkBtn.textContent = label
  if (!res.ok) {
    errorUrl.hidden = false
    errorUrl.textContent = res.errors ? res.errors.url : res.error
  }
})

// --- хост и токен ---

function hostHasToken(host) {
  return savedHosts.includes(host)
}

function syncHostSubform() {
  const host = hostSelect.value
  const isAdd = host === ADD_HOST
  const noToken = isAdd || !hostHasToken(host)
  fieldNewHost.hidden = !noToken
  tokenHint.hidden = !noToken
  removeTokenBtn.hidden = isAdd || noToken
  if (noToken && !isAdd) newHostInput.value = host
  projectsPanel.hidden = noToken
}

function refreshProjectsPanel() {
  syncHostSubform()
  currentHost = hostSelect.value === ADD_HOST ? null : hostSelect.value
  if (!currentHost || !hostHasToken(currentHost)) return
  projectSearchInput.value = ''
  loadProjects({ reset: true })
}

hostSelect.addEventListener('change', refreshProjectsPanel)

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
    // хост из истории или ссылки, токен для него ещё не сохранён — временный пункт списка
    const option = document.createElement('option')
    option.value = selected
    option.textContent = selected
    hostSelect.insertBefore(option, addOption)
    hostSelect.value = selected
  } else {
    hostSelect.value = savedHosts.length ? savedHosts[0] : ADD_HOST
  }
  refreshProjectsPanel()
}

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

// --- список проектов ---

function renderProjectRow(project) {
  const li = document.createElement('li')
  const button = document.createElement('button')
  button.type = 'button'
  button.className = 'proj-row'

  const name = document.createElement('span')
  name.className = 'proj-name'
  name.translate = 'no'
  name.textContent = project.name

  const path = document.createElement('span')
  path.className = 'proj-path'
  path.translate = 'no'
  path.textContent = project.fullPath

  const meta = document.createElement('span')
  meta.className = 'proj-meta'
  if (project.defaultBranch) {
    const branch = document.createElement('span')
    branch.className = 'chip'
    branch.translate = 'no'
    branch.textContent = project.defaultBranch
    meta.appendChild(branch)
  }
  if (project.lastActivityAt) {
    const activity = document.createElement('span')
    activity.textContent = `активность ${relativeTime(project.lastActivityAt)}`
    meta.appendChild(activity)
  }

  button.append(name, path, meta)
  button.addEventListener('click', () => openProject(project))
  li.appendChild(button)
  return li
}

async function loadProjects({ reset }) {
  if (reset) { projectsAfter = null; projectListEl.innerHTML = '' }
  const seq = ++projectsSeq
  const host = currentHost
  projectsStatus.textContent = 'Загрузка…'
  projectsError.hidden = true
  const res = await window.app.projects({ host, search: projectSearchInput.value, after: reset ? null : projectsAfter })
  if (seq !== projectsSeq || host !== currentHost) return // устарело: другой запрос или сменился хост
  if (!res.ok) {
    projectsStatus.textContent = ''
    projectsError.hidden = false
    projectsError.textContent = res.error
    projectsMoreBtn.hidden = true
    return
  }
  projectsAfter = res.value.next
  for (const project of res.value.items) projectListEl.appendChild(renderProjectRow(project))
  projectsMoreBtn.hidden = !projectsAfter
  projectsStatus.textContent = projectListEl.children.length ? '' : 'Ничего не найдено'
}

projectSearchInput.addEventListener('input', () => {
  clearTimeout(projectsSearchTimer)
  projectsSearchTimer = setTimeout(() => loadProjects({ reset: true }), SEARCH_DEBOUNCE)
})

projectsMoreBtn.addEventListener('click', () => loadProjects({ reset: false }))

// --- экран проекта: ветки и пайплайны ---

function openProject(project) {
  currentProject = project
  currentBranch = project.defaultBranch || null
  projectTitle.textContent = project.name
  branchInput.value = currentBranch || ''
  showProjectScreen()
  loadBranches('')
  loadPipelines({ reset: true })
}

async function loadBranches(search) {
  if (!currentProject) return
  const seq = ++branchSeq
  const res = await window.app.branches({ host: currentHost, project: currentProject.fullPath, search })
  // ошибка здесь не критична: та же причина проявится и в списке пайплайнов, с видимым сообщением
  if (seq !== branchSeq || !res.ok) return
  branchList.innerHTML = ''
  for (const name of res.value) {
    const option = document.createElement('option')
    option.value = name
    branchList.appendChild(option)
  }
}

branchInput.addEventListener('input', () => {
  clearTimeout(branchSearchTimer)
  branchSearchTimer = setTimeout(() => loadBranches(branchInput.value), SEARCH_DEBOUNCE)
})

branchInput.addEventListener('change', () => {
  const next = branchInput.value || null
  if (next === currentBranch) return
  currentBranch = next
  loadPipelines({ reset: true })
})

function renderPipelineRow(item) {
  const li = document.createElement('li')
  const button = document.createElement('button')
  button.type = 'button'
  button.className = 'pipe-row'

  const iid = document.createElement('span')
  iid.className = 'pipe-id'
  iid.textContent = `#${item.iid}`

  const status = document.createElement('span')
  status.className = `pill ${STATUS_PILL[item.status] ?? 'mid'}`
  status.textContent = item.status

  const title = document.createElement('span')
  title.className = 'pipe-title'
  title.translate = 'no'
  title.textContent = item.commit?.title ?? '—'

  const author = document.createElement('span')
  author.className = 'pipe-author'
  author.translate = 'no'
  author.textContent = item.author ?? '—'

  const time = document.createElement('span')
  time.className = 'pipe-time'
  time.textContent = relativeTime(item.createdAt)

  const duration = document.createElement('span')
  duration.className = 'pipe-duration'
  duration.textContent = formatDuration(item.duration)

  const rowStatus = document.createElement('span')
  rowStatus.className = 'row-status'
  rowStatus.hidden = true
  rowStatus.setAttribute('aria-live', 'polite')

  button.append(iid, status, title, author, time, duration, rowStatus)
  button.addEventListener('click', async () => {
    button.disabled = true
    rowStatus.hidden = false
    rowStatus.classList.remove('row-error')
    rowStatus.textContent = 'Загружаю…'
    const url = `https://${currentHost}/${currentProject.fullPath}/-/pipelines/${item.id}`
    const res = await performBuild({ mode: 'link', url }, (loaded, total) => {
      rowStatus.textContent = progressLabel(loaded, total)
    })
    button.disabled = false
    if (res.ok) {
      rowStatus.hidden = true
    } else {
      rowStatus.classList.add('row-error')
      rowStatus.textContent = res.error ?? 'Не удалось построить отчёт'
    }
  })
  li.appendChild(button)
  return li
}

async function loadPipelines({ reset }) {
  if (reset) { pipelinesAfter = null; pipelineListEl.innerHTML = '' }
  const seq = ++pipelinesSeq
  const project = currentProject
  pipelinesStatus.textContent = 'Загрузка…'
  pipelinesError.hidden = true
  const res = await window.app.pipelines({ host: currentHost, project: project.fullPath, ref: currentBranch, after: reset ? null : pipelinesAfter })
  if (seq !== pipelinesSeq || project !== currentProject) return
  if (!res.ok) {
    pipelinesStatus.textContent = ''
    pipelinesError.hidden = false
    pipelinesError.textContent = res.error
    pipelinesMoreBtn.hidden = true
    return
  }
  pipelinesAfter = res.value.next
  for (const item of res.value.items) pipelineListEl.appendChild(renderPipelineRow(item))
  pipelinesMoreBtn.hidden = !pipelinesAfter
  pipelinesStatus.textContent = pipelineListEl.children.length ? '' : 'Ничего не найдено'
}

pipelinesMoreBtn.addEventListener('click', () => loadPipelines({ reset: false }))

// --- агрегат ---

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

function updateAggregateLabel() {
  buildAggBtn.textContent = `Агрегат по ${lastInput.value || 0} пайплайнам`
}

lastInput.addEventListener('input', updateAggregateLabel)

buildAggBtn.addEventListener('click', async () => {
  errorLast.hidden = true
  errorStatuses.hidden = true
  aggregateError.hidden = true
  buildAggBtn.disabled = true
  const label = buildAggBtn.textContent
  const payload = {
    mode: 'aggregate', host: currentHost, project: currentProject.fullPath,
    ref: currentBranch || '', source: '', last: lastInput.value, statuses: collectStatuses(),
  }
  const res = await performBuild(payload, (loaded, total) => {
    buildAggBtn.textContent = progressLabel(loaded, total)
  })
  buildAggBtn.disabled = false
  buildAggBtn.textContent = label
  if (!res.ok) {
    if (res.errors) {
      if (res.errors.last) { errorLast.hidden = false; errorLast.textContent = res.errors.last }
      if (res.errors.statuses) { errorStatuses.hidden = false; errorStatuses.textContent = res.errors.statuses }
    } else {
      aggregateError.hidden = false
      aggregateError.textContent = res.error
    }
  }
})

// --- история ---

function openHistoryEntry(entry) {
  if (entry.form.mode === 'link') {
    showProjectsScreen()
    urlInput.value = entry.form.url ?? ''
    urlInput.focus()
    return
  }
  currentHost = entry.host
  // имя проекта в истории не хранится — путь используется и как заголовок
  currentProject = { fullPath: entry.form.project, name: entry.form.project, defaultBranch: entry.form.ref || null }
  currentBranch = entry.form.ref || null
  projectTitle.textContent = currentProject.name
  branchInput.value = currentBranch || ''
  lastInput.value = entry.form.last ?? 50
  updateAggregateLabel()
  setStatuses(entry.form.statuses ?? STATUS_DEFAULTS)
  showProjectScreen()
  loadBranches('')
  loadPipelines({ reset: true })
}

async function loadHistory() {
  const res = await window.app.history()
  const entries = res.ok ? res.value : []
  historyList.innerHTML = ''
  historyEmpty.hidden = entries.length > 0
  historyClearBtn.hidden = entries.length === 0
  for (const entry of entries) {
    const li = document.createElement('li')
    li.className = 'hist-row'
    const button = document.createElement('button')
    button.type = 'button'
    button.className = 'hist-item'
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
    button.addEventListener('click', () => openHistoryEntry(entry))

    // отдельная кнопка, а не вложенная: клик по ней не должен ещё и открывать запись
    const remove = document.createElement('button')
    remove.type = 'button'
    remove.className = 'hist-remove'
    remove.setAttribute('aria-label', `Удалить из истории: ${entry.label}`)
    remove.textContent = '×'
    remove.addEventListener('click', async () => {
      const removeRes = await window.app.removeHistory(entry.at)
      if (removeRes.ok) await loadHistory()
    })

    li.append(button, remove)
    historyList.appendChild(li)
  }
}

historyClearBtn.addEventListener('click', async () => {
  if (!confirm('Очистить историю запросов?')) return
  const res = await window.app.clearHistory()
  if (res.ok) await loadHistory()
})

updateAggregateLabel()
loadHosts()
loadHistory()
