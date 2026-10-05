import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { useParams } from 'react-router'

import { ApiError, type Schemas, api, unwrap } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { formatDate, localDate } from '../shared/format'
import { kcal, nutritionSummary } from '../shared/nutrition'
import { clientQuery, nutritionQuery } from './clients'
import { isUnauthorized, useCoach } from './context'

/** Dasha sets a client's daily target; the bot tells the client. */
export function ClientNutritionPage() {
  const { id = '' } = useParams()
  const { base, onUnauthorized } = useCoach()
  const client = useQuery(clientQuery(id))
  const nutrition = useQuery(nutritionQuery(id))
  useEffect(() => {
    if (isUnauthorized(client.error) || isUnauthorized(nutrition.error)) onUnauthorized()
  }, [client.error, nutrition.error, onUnauthorized])

  const back = <BackLink to={`${base}/clients/${id}`} label="Клієнт" />
  if (!nutrition.data || !client.data) {
    const failed = nutrition.error ?? client.error
    const missing = failed instanceof ApiError && failed.status === 404
    return (
      <section className="page">
        {back}
        <p className={failed ? 'error' : 'muted'}>
          {failed ? (missing ? 'Такого клієнта немає.' : 'Не вдалося завантажити.') : 'Завантаження…'}
        </p>
      </section>
    )
  }

  const [current, ...earlier] = nutrition.data.targets
  return (
    <section className="page">
      {back}
      <header className="page-title">
        <p className="muted small">{client.data.name}</p>
        <h1>Харчування</h1>
      </header>
      <p className="muted small">
        Норма на кожен день, доки ти її не зміниш. Після збереження бот надішле клієнту нові цифри.
      </p>
      <TargetForm clientId={id} current={current} />
      {earlier.length > 0 && (
        <section className="stack" aria-labelledby="earlier-title">
          <h2 id="earlier-title" className="section-title">
            Раніше
          </h2>
          <ul className="weigh-ins">
            {earlier.map((target) => (
              <li key={target.id}>
                <span>{formatDate(localDate(new Date(target.set_at)))}</span>
                <strong>{nutritionSummary(target)}</strong>
              </li>
            ))}
          </ul>
        </section>
      )}
    </section>
  )
}

/** Whole grams, 0–1000; `undefined` if unreadable. */
function readGrams(text: string): number | undefined {
  const value = text.trim()
  if (value === '') return 0
  return /^\d{1,4}$/.test(value) && Number(value) <= 1000 ? Number(value) : undefined
}

function TargetForm({
  clientId,
  current,
}: {
  clientId: string
  current: Schemas['NutritionTarget'] | undefined
}) {
  const { onUnauthorized } = useCoach()
  const queryClient = useQueryClient()
  const [protein, setProtein] = useState(current ? String(current.protein_g) : '')
  const [fat, setFat] = useState(current ? String(current.fat_g) : '')
  const [carbs, setCarbs] = useState(current ? String(current.carbs_g) : '')
  const [note, setNote] = useState(current?.note ?? '')
  const grams = [readGrams(protein), readGrams(fat), readGrams(carbs)] as const
  const [proteinG, fatG, carbsG] = grams
  const readable = proteinG !== undefined && fatG !== undefined && carbsG !== undefined
  const valid = readable && proteinG + fatG + carbsG > 0
  const changed =
    !current ||
    proteinG !== current.protein_g ||
    fatG !== current.fat_g ||
    carbsG !== current.carbs_g ||
    note.trim() !== (current.note ?? '')

  const save = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.POST('/coach/clients/{id}/nutrition', {
          params: { path: { id: clientId } },
          body: {
            protein_g: proteinG ?? 0,
            fat_g: fatG ?? 0,
            carbs_g: carbsG ?? 0,
            note: note.trim() || null,
          },
        }),
      ),
    onSuccess: (saved) => {
      queryClient.setQueryData<Schemas['NutritionHistory']>(
        nutritionQuery(clientId).queryKey,
        (history) => ({ targets: [saved.target, ...(history?.targets ?? [])] }),
      )
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  const fields = [
    { label: 'Білки, г', value: protein, set: setProtein, bad: proteinG === undefined },
    { label: 'Жири, г', value: fat, set: setFat, bad: fatG === undefined },
    { label: 'Вуглеводи, г', value: carbs, set: setCarbs, bad: carbsG === undefined },
  ]

  return (
    <form
      className="stack"
      aria-label="Норма харчування"
      onSubmit={(event) => {
        event.preventDefault()
        if (valid && changed) save.mutate()
      }}
    >
      <div className="field-row three">
        {fields.map((field) => (
          <label key={field.label} className="field">
            <span>{field.label}</span>
            <input
              inputMode="numeric"
              maxLength={4}
              placeholder="0"
              value={field.value}
              className={field.bad ? 'bad' : undefined}
              onChange={(event) => field.set(event.target.value)}
            />
          </label>
        ))}
      </div>
      {valid ? (
        <p className="kcal-total">{kcal(proteinG, fatG, carbsG)} ккал на день</p>
      ) : readable ? (
        <p className="muted small">Впиши грами — калорії порахуються самі.</p>
      ) : (
        <p className="error">Грами — цілі числа від 0 до 1000.</p>
      )}
      <label className="field">
        <span>Коментар для клієнта</span>
        <textarea
          rows={2}
          maxLength={500}
          placeholder="Наприклад: 2 л води на день, білок — з кожним прийомом їжі"
          value={note}
          onChange={(event) => setNote(event.target.value)}
        />
      </label>
      {save.isError && !isUnauthorized(save.error) && (
        <p className="error">Не вдалося зберегти. Спробуй ще раз.</p>
      )}
      {save.isSuccess && !changed && (
        <p className="notice">
          {save.data.client_notified
            ? 'Збережено. Бот надіслав клієнту нові цифри.'
            : 'Збережено. Клієнт побачить нові цифри в застосунку: бот не може йому написати, доки клієнт не дозволить повідомлення.'}
        </p>
      )}
      <button
        type="submit"
        className="button primary block"
        disabled={!valid || !changed || save.isPending}
      >
        {save.isPending ? 'Зберігаю…' : 'Зберегти й повідомити'}
      </button>
    </form>
  )
}
