import { defineConfig } from 'steiger'
import fsd from '@feature-sliced/steiger-plugin'

export default defineConfig([
  ...fsd.configs.recommended,
  // Слайсы отчёта нарезаны спекой (§ 8), а страница у него одна: виджеты `waterfall`, `report-header`, `hotspots`,
  // `detail-panel` и фичи `timeline-navigation`, `group-toggle`, `keys-help` имеют ровно одного потребителя —
  // слияние нарушило бы спеку. Правило не считает ссылки из файлов, для которых оно выключено, поэтому в список
  // входят и слайсы с несколькими потребителями, но только из этих виджетов (`entities/{node,report}`, `node-selection`, `tree-switch`).
  // Убрать, когда появится вторая страница, которая их использует.
  {
    files: ['./src/widgets/{waterfall,report-header,hotspots,detail-panel}/**', './src/features/{timeline-navigation,group-toggle,keys-help,node-selection,tree-switch}/**', './src/entities/{node,report}/**'],
    rules: { 'fsd/insignificant-slice': 'off' },
  },
])
