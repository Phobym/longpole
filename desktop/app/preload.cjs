const { contextBridge, ipcRenderer } = require('electron')

contextBridge.exposeInMainWorld('app', {
  build: (form) => ipcRenderer.invoke('build', form),
  hosts: () => ipcRenderer.invoke('hosts'),
  setToken: (host, token) => ipcRenderer.invoke('setToken', host, token),
  removeToken: (host) => ipcRenderer.invoke('removeToken', host),
  history: () => ipcRenderer.invoke('history'),
  removeHistory: (at) => ipcRenderer.invoke('removeHistory', at),
  clearHistory: () => ipcRenderer.invoke('clearHistory'),
  projects: (args) => ipcRenderer.invoke('projects', args),
  branches: (args) => ipcRenderer.invoke('branches', args),
  pipelines: (args) => ipcRenderer.invoke('pipelines', args),
  onProgress: (fn) => {
    const listener = (_event, progress) => fn(progress)
    ipcRenderer.on('progress', listener)
    return () => ipcRenderer.off('progress', listener)
  },
})
