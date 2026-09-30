/** Ukrainian plural: plural(21, 'підхід', 'підходи', 'підходів') → 'підхід'. */
export function plural(n: number, one: string, few: string, many: string): string {
  const mod10 = n % 10
  const mod100 = n % 100
  if (mod10 === 1 && mod100 !== 11) return one
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return few
  return many
}

/** `29 вересня`. */
export function formatDay(iso: string): string {
  return new Date(iso).toLocaleDateString('uk-UA', { day: 'numeric', month: 'long' })
}

/** `Вівторок, 29 вересня`. */
export function formatToday(): string {
  const today = new Date().toLocaleDateString('uk-UA', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  })
  return today.charAt(0).toUpperCase() + today.slice(1)
}

/** `Максим К.` → `МК`. */
export function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word.charAt(0))
    .join('')
    .toUpperCase()
}

export function formatKg(kg: number): string {
  return kg.toLocaleString('uk-UA', { maximumFractionDigits: 2 })
}

/** `79 × 8-10` for weighted sets, `× 17` for bodyweight. */
export function formatSet(set: { kg?: number | null; reps_min: number; reps_max: number }): string {
  const reps = set.reps_min === set.reps_max ? `${set.reps_min}` : `${set.reps_min}-${set.reps_max}`
  return set.kg == null ? `× ${reps}` : `${formatKg(set.kg)} × ${reps}`
}
