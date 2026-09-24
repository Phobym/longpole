import { app, BrowserWindow, dialog, ipcMain, Menu, safeStorage, shell } from 'electron'
import { copyFile, mkdtemp, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import { createClient, resolveHost, resolveToken } from './core/gitlab.mjs'
import { buildReport, defaultFileName } from './core/report.mjs'
import { render } from './core/render.mjs'
import { parseFormRequest } from './request.mjs'
import { createTokenStore } from './tokens.mjs'
import { createHistory } from './history.mjs'

const here = fileURLToPath(new URL('.', import.meta.url))
const secure = { contextIsolation: true, sandbox: true, nodeIntegration: false }
const reports = new Map()
let tokens
let history
let reportsDir
let formWindow

// на Linux без keyring safeStorage «шифрует» открытым текстом — это не шифрование
const crypto = {
  isEncryptionAvailable: () => safeStorage.isEncryptionAvailable()
    && !(process.platform === 'linux' && safeStorage.getSelectedStorageBackend() === 'basic_text'),
  encryptString: (s) => safeStorage.encryptString(s),
  decryptString: (b) => safeStorage.decryptString(b),
}

function lockDown(win) {
  win.webContents.on('will-navigate', (event) => event.preventDefault())
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('https:')) shell.openExternal(url)
    return { action: 'deny' }
  })
}

function createFormWindow() {
  formWindow = new BrowserWindow({
    width: 1280, height: 860, minWidth: 900, minHeight: 640, title: 'pipeline-trace',
    webPreferences: { ...secure, preload: join(here, 'preload.cjs') },
  })
  lockDown(formWindow)
  formWindow.loadFile(join(here, 'ui', 'index.html'))
  formWindow.on('closed', () => { formWindow = null })
}

function openReport(file, name, title) {
  const win = new BrowserWindow({ width: 1440, height: 900, title, webPreferences: secure })
  const id = win.webContents.id
  lockDown(win)
  reports.set(id, { file, name })
  win.on('page-title-updated', (event) => event.preventDefault())
  win.on('closed', () => reports.delete(id))
  win.loadFile(file)
}

async function saveFocusedReport() {
  const win = BrowserWindow.getFocusedWindow()
  const entry = win && reports.get(win.webContents.id)
  if (!entry) return
  const { canceled, filePath } = await dialog.showSaveDialog(win, {
    defaultPath: entry.name, filters: [{ name: 'HTML', extensions: ['html'] }],
  })
  if (!canceled && filePath) await copyFile(entry.file, filePath)
}

// ошибки возвращаются значением: иначе Electron добавляет к тексту «Error invoking remote method…»
function handle(channel, fn) {
  ipcMain.handle(channel, async (event, ...args) => {
    try {
      return { ok: true, value: await fn(event, ...args) }
    } catch (err) {
      return { ok: false, error: err.message }
    }
  })
}

function registerIpc() {
  handle('hosts', () => tokens.hosts())
  handle('history', () => history.list())
  handle('setToken', async (_event, host, token) => tokens.set(await resolveHost(String(host).trim()), token))
  handle('removeToken', (_event, host) => tokens.remove(host))
  ipcMain.handle('build', async (event, form) => {
    const parsed = parseFormRequest(form)
    if (!parsed.ok) return { ok: false, errors: parsed.errors }
    try {
      const token = (await tokens.get(parsed.host)) ?? (await resolveToken(parsed.host))
      const gql = createClient({ host: parsed.host, token })
      const { report, suffix } = await buildReport(parsed.request, {
        gql, host: parsed.host, onProgress: (p) => event.sender.send('progress', p),
      })
      const name = defaultFileName(parsed.request.project, suffix)
      const file = join(reportsDir, `${Date.now()}-${name}`)
      await writeFile(file, await render(report))
      const label = `${parsed.request.project} · ${report.meta.label}`
      await history.add({ host: parsed.host, form, request: parsed.request, label })
      openReport(file, name, label)
      return { ok: true }
    } catch (err) {
      return { ok: false, error: err.message }
    }
  })
}

function buildMenu() {
  const isMac = process.platform === 'darwin'
  Menu.setApplicationMenu(Menu.buildFromTemplate([
    ...(isMac ? [{ role: 'appMenu' }] : []),
    { label: 'Файл', submenu: [
      { label: 'Новый отчёт', accelerator: 'CmdOrCtrl+N', click: () => (formWindow ? formWindow.focus() : createFormWindow()) },
      { label: 'Сохранить отчёт…', accelerator: 'CmdOrCtrl+S', click: saveFocusedReport },
      { type: 'separator' },
      isMac ? { role: 'close', label: 'Закрыть окно' } : { role: 'quit', label: 'Выход' },
    ] },
    { role: 'editMenu', label: 'Правка' },
    { role: 'viewMenu', label: 'Вид' },
    { role: 'windowMenu', label: 'Окно' },
  ]))
}

app.whenReady().then(async () => {
  const data = app.getPath('userData')
  tokens = createTokenStore({ file: join(data, 'tokens.json'), crypto })
  history = createHistory({ file: join(data, 'history.json') })
  reportsDir = await mkdtemp(join(tmpdir(), 'pipeline-trace-'))
  registerIpc()
  buildMenu()
  createFormWindow()
  if (process.env.PIPELINE_TRACE_SMOKE) {
    formWindow.webContents.once('did-finish-load', async () => {
      const ok = await formWindow.webContents.executeJavaScript('typeof window.app?.build === "function"')
      console.log(ok ? 'smoke: ok' : 'smoke: bridge missing')
      app.exit(ok ? 0 : 1)
    })
  }
  app.on('activate', () => { if (!BrowserWindow.getAllWindows().length) createFormWindow() })
})

app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit() })
