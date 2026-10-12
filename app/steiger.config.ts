import { defineConfig } from 'steiger'
import fsd from '@feature-sliced/steiger-plugin'
import { readFileSync } from 'node:fs'

type Folder = { type: 'folder'; path: string; children: Array<{ type: 'file'; path: string } | Folder> }
const files = (folder: Folder): string[] => folder.children.flatMap((c) => (c.type === 'file' ? [c.path] : files(c)))

// Флаг поставки читают только транспорт, диалоги и `capabilities` (спека поставки, § 9); остальной код — только `capabilities`. Тесты не в счёт: тест сборки задаёт `VITE_DELIVERY`.
const DELIVERY_READERS = /\/src\/shared\/(api|lib\/dialogs|config)\//
const longpole = {
  meta: { name: 'longpole', version: '1.0.0' },
  ruleDefinitions: [
    {
      name: 'longpole/delivery-import',
      check: (root: Folder) => ({
        diagnostics: files(root)
          .filter((path) => /(?<!\.test)\.tsx?$/.test(path) && !DELIVERY_READERS.test(path) && /config\/delivery['"]|VITE_DELIVERY/.test(readFileSync(path, 'utf8')))
          .map((path) => ({ message: 'DELIVERY читают только shared/api, shared/lib/dialogs и shared/config; здесь — `capabilities` из shared/config', location: { path } })),
      }),
    },
  ],
}

export default defineConfig([
  ...fsd.configs.recommended,
  longpole,
  { rules: { 'longpole/delivery-import': 'error' } },
  // Слайсы отчёта нарезаны спекой (§ 8), а страница у него одна: виджеты `waterfall`, `report-header`, `hotspots`,
  // `detail-panel` и фичи `timeline-navigation`, `group-toggle`, `keys-help` имеют ровно одного потребителя —
  // слияние нарушило бы спеку. Правило не считает ссылки из файлов, для которых оно выключено, поэтому в список
  // входят и слайсы с несколькими потребителями, но только из этих виджетов (`entities/node`, `node-selection`, `tree-switch`, `build-aggregate`).
  // Убрать, когда появится вторая страница, которая их использует.
  // Остальное — слайсы формы (спека, § 7) с одним потребителем после редизайна первого запуска.
  {
    files: [
      './src/widgets/{waterfall,report-header,hotspots,detail-panel,pipelines-list,aggregate-block,language-switch}/**',
      './src/features/{timeline-navigation,group-toggle,keys-help,select-branch,select-workflow,history-actions,choose-theme,build-aggregate,node-selection,tree-switch}/**',
      './src/entities/{pipeline,node}/**',
    ],
    rules: { 'fsd/insignificant-slice': 'off' },
  },
  // `settings` — имя из спеки (настройки приложения, одна сущность); единственное число («setting») звучало бы как одна настройка.
  { files: ['./src/entities/settings/**'], rules: { 'fsd/inconsistent-naming': 'off' } },
])
