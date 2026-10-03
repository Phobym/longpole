import { defineConfig } from 'steiger'
import fsd from '@feature-sliced/steiger-plugin'

export default defineConfig([
  ...fsd.configs.recommended,
  // Слайсы отчёта нарезаны спекой (§ 8): `timeline-navigation`, `node-selection`, `group-toggle`,
  // `entities/{node,report}`, `widgets/waterfall` до S8b имеют одного потребителя — слияние их нарушило бы спеку. Убрать, когда S8b даст им вторых потребителей.
  {
    files: ['./src/entities/{node,report}/**', './src/features/{timeline-navigation,node-selection,group-toggle}/**', './src/widgets/waterfall/**'],
    rules: { 'fsd/insignificant-slice': 'off' },
  },
])
