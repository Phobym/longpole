const { contextBridge, ipcRenderer } = require('electron')

contextBridge.exposeInMainWorld('app', {
  build: (form) => ipcRenderer.invoke('build', form),
  hosts: () => ipcRenderer.invoke('hosts'),
  setToken: (host, token) => ipcRenderer.invoke('setToken', host, token),
  removeToken: (host) => ipcRenderer.invoke('removeToken', host),
  history: () => ipcRenderer.invoke('history'),
  removeHistory: (at) => ipcRenderer.invoke('removeHistory', at),
  clearHistory: () => ipcRenderer.invoke('clearHistory'),
  onProgress: (fn) => {
    const listener = (_event, progress) => fn(progress)
    ipcRenderer.on('progress', listener)
    return () => ipcRenderer.off('progress', listener)
  },
})
