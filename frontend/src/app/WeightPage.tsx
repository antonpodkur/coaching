import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'

import { ApiError, api, unwrap } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { Screen } from '../shared/Screen'
import { WeightHistory } from '../shared/WeightHistory'
import { confirmAction } from '../shared/dialogs'
import { formatDate, formatKg, localDate } from '../shared/format'
import type { WeightEntry } from '../shared/weight'
import { MY_WEIGHT_KEY, useMyWeight } from './weight'

/** `82,4` or `82.4` → kg, or `undefined` if unreadable. */
function readKg(text: string): number | undefined {
  const kg = Number(text.trim().replace(',', '.'))
  return text.trim() !== '' && Number.isFinite(kg) ? kg : undefined
}

/** The client logs their weight and sees how it moves. */
export function WeightPage() {
  const queryClient = useQueryClient()
  const weight = useMyWeight()
  const today = localDate(new Date())
  const [kgText, setKgText] = useState('')
  const [date, setDate] = useState(today)
  const kg = readKg(kgText)
  const entries = weight.data ?? []
  const replaces = entries.some((entry) => entry.date === date)

  const save = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.PUT('/me/weight/{date}', { params: { path: { date } }, body: { kg: kg ?? 0 } }),
      ),
    onSuccess: (saved) => {
      // Keep the list sorted, with one entry a day.
      queryClient.setQueryData<WeightEntry[]>(MY_WEIGHT_KEY, (current = []) =>
        [...current.filter((entry) => entry.date !== saved.date), saved].sort((a, b) =>
          a.date.localeCompare(b.date),
        ),
      )
      setKgText('')
      setDate(today)
    },
  })
  const code = save.error instanceof ApiError ? save.error.code : null

  const remove = async (day: string) => {
    if (!(await confirmAction(`Видалити запис за ${formatDate(day)}?`))) return
    await api.DELETE('/me/weight/{date}', { params: { path: { date: day } } }).catch(() => undefined)
    void queryClient.invalidateQueries({ queryKey: MY_WEIGHT_KEY })
  }

  return (
    <Screen>
      <BackLink to="/app" label="Головна" />
      <header className="exercise-head">
        <h1>Вага</h1>
        <p className="muted small">
          Зважуйся зранку, до сніданку — так цифри можна порівнювати. Вага стрибає на кілограм-два
          через воду, тому дивись на лінію, а не на окремі точки.
        </p>
      </header>

      <form
        className="stack"
        aria-label="Записати вагу"
        onSubmit={(event) => {
          event.preventDefault()
          if (kg !== undefined) save.mutate()
        }}
      >
        <div className="field-row">
          <label className="field">
            <span>Вага, кг</span>
            <input
              inputMode="decimal"
              placeholder={entries.at(-1) ? formatKg(entries.at(-1)?.kg ?? 0) : '70'}
              value={kgText}
              className={kgText.trim() !== '' && kg === undefined ? 'bad' : undefined}
              onChange={(event) => setKgText(event.target.value)}
            />
          </label>
          <label className="field">
            <span>Дата</span>
            <input
              type="date"
              value={date}
              max={today}
              onChange={(event) => setDate(event.target.value || today)}
            />
          </label>
        </div>
        {code === 'invalid_weight' && <p className="error">Вага — від 20 до 400 кг.</p>}
        {code === 'invalid_date' && <p className="error">Дата не може бути в майбутньому.</p>}
        {save.isError && !code?.startsWith('invalid_') && (
          <p className="error">Не вдалося записати. Перевір зв’язок і спробуй ще раз.</p>
        )}
        <button
          type="submit"
          className="button primary block"
          disabled={kg === undefined || save.isPending}
        >
          {save.isPending ? 'Записую…' : replaces ? 'Замінити запис за цей день' : 'Записати'}
        </button>
      </form>

      {weight.isPending && <p className="muted">Завантаження…</p>}
      {weight.isError && <p className="error">Не вдалося завантажити записи.</p>}
      {weight.data?.length === 0 && (
        <p className="muted">Ще немає записів. Додай перший — а згодом тут з’явиться графік.</p>
      )}
      <WeightHistory entries={entries} onDelete={(day) => void remove(day)} />
    </Screen>
  )
}
