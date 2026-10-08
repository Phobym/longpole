import type { CSSProperties } from 'react'
import { cn } from '../lib/cn'

export type GiraffePose = 'idle' | 'loading' | 'oops'

// шея — полоски трейса снизу вверх, геометрия иконки `app/src-tauri/icons/app-icon.svg` без подложки
const NECK = [
  { x: 18, y: 78, width: 30, opacity: 0.55 },
  { x: 26, y: 67, width: 28, opacity: 0.7 },
  { x: 34, y: 56, width: 26, opacity: 0.85 },
  { x: 42, y: 45, width: 22, opacity: 1 },
] as const
const EYE = '#1c2333'

type Props = { pose?: GiraffePose; size?: number; progress?: { loaded: number; total: number }; className?: string }

/** Прозрачность полоски вне цикла загрузки: доля загруженного или обычная шея. */
function barOpacity(i: number, bar: (typeof NECK)[number], lit: number | null): number {
  return lit !== null ? (i < lit ? 1 : 0.2) : bar.opacity
}

/** Маскот с иконки приложения. Декоративный: смысл несёт текст рядом. `loading` с `progress` зажигает полоски шеи по доле загруженного. */
export function Giraffe({ pose = 'idle', size = 56, progress, className }: Props) {
  const lit = pose === 'loading' && progress && progress.total > 0 ? Math.ceil((NECK.length * progress.loaded) / progress.total) : null
  const cycling = pose === 'loading' && lit === null
  return (
    <svg viewBox="14 8 72 82" width={size} height={Math.round((size * 82) / 72)} aria-hidden className={cn('shrink-0 text-mascot', className)}>
      <g fill="currentColor">
        {/* при reduced-motion — обычная шея вместо погасших полосок */}
        {NECK.map((bar, i) => (
          <rect
            key={bar.y}
            x={bar.x}
            y={bar.y}
            width={bar.width}
            height={9}
            rx={3}
            opacity={cycling ? undefined : barOpacity(i, bar, lit)}
            className={cycling ? 'animate-giraffe-bar opacity-20 motion-reduce:animate-none motion-reduce:opacity-(--bar)' : undefined}
            style={cycling ? ({ animationDelay: `${i * 300}ms`, '--bar': bar.opacity } as CSSProperties) : undefined}
          />
        ))}
        {/* `oops` опускает голову: поворот вокруг верха шеи */}
        <g transform={pose === 'oops' ? 'rotate(20 53 44)' : undefined}>
          <path d="M44 42 C44 33 50 28 58 28 L76 33 C83 35 84 44 77 46 L60 47 C52 48 46 47 44 42 Z" />
          <circle cx="48" cy="17" r="3.6" />
          <circle cx="57" cy="16" r="3.6" />
          <ellipse cx="44" cy="31" rx="6" ry="3" transform="rotate(-25 44 31)" />
          <path d="M50 30 L48 18 M57 29 L57 17" fill="none" stroke="currentColor" strokeWidth="4" strokeLinecap="round" />
          {pose === 'oops' ? (
            <path d="M59.5 35 q2.6 2 5.2 0" fill="none" stroke={EYE} strokeWidth="1.6" strokeLinecap="round" />
          ) : (
            <circle cx="62" cy="35" r="2.6" fill={EYE} />
          )}
        </g>
      </g>
    </svg>
  )
}
