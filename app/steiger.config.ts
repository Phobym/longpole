import { defineConfig } from 'steiger'
import fsd from '@feature-sliced/steiger-plugin'

export default defineConfig([
  ...fsd.configs.recommended,
  // Слайсы отчёта нарезаны спекой (§ 8): `timeline-navigation`, `node-selection`, `group-toggle`,
  // `entities/node`, `widgets/waterfall` до S8b имеют одного потребителя — слияние их нарушило бы спеку.
  { rules: { 'fsd/insignificant-slice': 'off' } },
])
