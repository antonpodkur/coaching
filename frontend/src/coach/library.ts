import type { Schemas } from '../api/client'
import { telegramWebApp } from '../app/telegram'
import type { UploadProgress } from './uploads'

export type Exercise = Schemas['Exercise']
export type Measure = Schemas['Measure']

/** How Dasha counts an exercise, as the library offers it. */
export const MEASURES: { value: Measure; label: string; hint: string }[] = [
  { value: 'weight', label: 'З вагою', hint: 'Кілограми × повтори.' },
  {
    value: 'bodyweight',
    label: 'Власна вага',
    hint: 'Лише повтори. Додаткову вагу, наприклад пояс, можна додати в підході.',
  },
  { value: 'time', label: 'На час', hint: 'Секунди або хвилини: планка, велотренажер.' },
]

/** For lists: nothing for the usual weighted exercise, else `власна вага` or `на час`. */
export function measureNote(measure: Measure): string | null {
  return { weight: null, bodyweight: 'власна вага', time: 'на час' }[measure]
}

/** The groups the app offers. The backend stores any short text. */
export const MUSCLE_GROUPS = [
  'Спина',
  'Ноги',
  'Сідниці',
  'Груди',
  'Плечі',
  'Руки',
  'Прес',
  'Мобільність',
]

/** `48` → `0:48`. */
export function formatLength(secs: number): string {
  const minutes = Math.floor(secs / 60)
  const seconds = String(secs % 60).padStart(2, '0')
  return `${minutes}:${seconds}`
}

/**
 * What the library says about an exercise's video. An upload running on this
 * phone knows more than the server, so it wins.
 */
export function videoSummary(
  exercise: Exercise,
  upload: UploadProgress | undefined,
): { text: string; busy: boolean } {
  if (upload) {
    switch (upload.phase) {
      case 'preparing':
        return { text: 'Готую завантаження…', busy: true }
      case 'uploading':
        return { text: `Завантаження ${Math.round(upload.progress * 100)}%`, busy: true }
      case 'finishing':
        return { text: 'Завершую завантаження…', busy: true }
      case 'failed':
        return { text: 'Не вдалося завантажити', busy: true }
    }
  }
  switch (exercise.upload?.status) {
    case 'uploading':
      return { text: 'Завантаження не завершене', busy: true }
    case 'processing':
      return { text: 'Відео обробляється…', busy: true }
    case 'failed':
      return { text: 'Відео не вдалося обробити', busy: true }
  }
  const group = exercise.muscle_group?.toLocaleLowerCase('uk')
  const measure = measureNote(exercise.measure)
  if (exercise.video) {
    const length =
      exercise.video.length_secs != null ? ` ${formatLength(exercise.video.length_secs)}` : ''
    return { text: [`відео${length}`, group, measure].filter(Boolean).join(' · '), busy: false }
  }
  return { text: ['без відео', group, measure].filter(Boolean).join(' · '), busy: false }
}

/** Telegram's own confirm dialog inside the Mini App, the browser's elsewhere. */
export function confirmAction(message: string): Promise<boolean> {
  const webApp = telegramWebApp()
  if (!webApp) return Promise.resolve(window.confirm(message))
  return new Promise((resolve) => webApp.showConfirm(message, resolve))
}

/** Telegram's own alert inside the Mini App, the browser's elsewhere. */
export function alertMessage(message: string) {
  const webApp = telegramWebApp()
  if (webApp) webApp.showAlert(message)
  else window.alert(message)
}
