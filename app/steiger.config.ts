import { defineConfig } from 'steiger'
import fsd from '@feature-sliced/steiger-plugin'

export default defineConfig([
  ...fsd.configs.recommended,
  // Слайсы отчёта нарезаны спекой (§ 8), а страница у него одна: виджеты `waterfall`, `report-header`, `hotspots`,
  // `detail-panel` и фичи `timeline-navigation`, `group-toggle`, `keys-help` имеют ровно одного потребителя —
  // слияние нарушило бы спеку. Правило не считает ссылки из файлов, для которых оно выключено, поэтому в список
  // входят и слайсы с несколькими потребителями, но только из этих виджетов (`entities/{node,report}`, `node-selection`, `tree-switch`).
  // Убрать, когда появится вторая страница, которая их использует.
  // Форма нарезана так же (спека, § 7): страниц стало четыре, многие слайсы нужны только одной.
  // `language-switch` без потребителя до экрана настроек (задача 11).
  {
    files: [
      './src/widgets/{waterfall,report-header,hotspots,detail-panel}/**',
      './src/features/{timeline-navigation,group-toggle,keys-help,node-selection,tree-switch}/**',
      './src/entities/{node,report}/**',
      './src/widgets/{projects-panel,pipelines-list,aggregate-block,projects-sidebar,app-header,add-project-dialog,language-switch}/**',
      './src/features/{manage-token,select-branch,history-actions,build-aggregate,add-project,choose-theme,build-by-link,search-projects}/**',
      './src/entities/{pipeline,saved-project,settings,history-entry,host,project}/**',
    ],
    rules: { 'fsd/insignificant-slice': 'off' },
  },
  // `settings` — имя из спеки (настройки приложения, одна сущность); единственное число («setting») звучало бы как одна настройка.
  { files: ['./src/entities/settings/**'], rules: { 'fsd/inconsistent-naming': 'off' } },
])
