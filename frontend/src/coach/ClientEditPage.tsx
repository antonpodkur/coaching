import { useQuery } from '@tanstack/react-query'
import { type FormEvent, useEffect, useState } from 'react'
import { useNavigate, useParams } from 'react-router'

import { ApiError } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { addMonths, localDate } from '../shared/format'
import { type Client, clientQuery, useUpdateClient } from './clients'
import { isUnauthorized, useCoach } from './context'
import { confirmAction } from '../shared/dialogs'

/** A client's name, how long they have paid for, and the archive. */
export function ClientEditPage() {
  const { id = '' } = useParams()
  const { base, onUnauthorized } = useCoach()
  const client = useQuery(clientQuery(id))
  useEffect(() => {
    if (isUnauthorized(client.error)) onUnauthorized()
  }, [client.error, onUnauthorized])

  const back = <BackLink to={`${base}/clients/${id}`} label={client.data?.name ?? 'Клієнт'} />
  if (client.isPending) {
    return (
      <section className="page">
        {back}
        <p className="muted">Завантаження…</p>
      </section>
    )
  }
  if (client.isError) {
    const missing = client.error instanceof ApiError && client.error.status === 404
    return (
      <section className="page">
        {back}
        <p className="error">{missing ? 'Такого клієнта немає.' : 'Не вдалося завантажити.'}</p>
      </section>
    )
  }
  return (
    <section className="page">
      {back}
      <h1>Дані клієнта</h1>
      {/* Remount when another client opens, so the form starts from them. */}
      <ClientForm key={client.data.id} client={client.data} />
      {client.data.archived ? (
        <ArchivedNotice client={client.data} />
      ) : (
        <ArchiveButton client={client.data} />
      )}
    </section>
  )
}

function ClientForm({ client }: { client: Client }) {
  const [name, setName] = useState(client.name)
  const [paidUntil, setPaidUntil] = useState(client.paid_until ?? '')
  const save = useUpdateClient(client.id)

  const changes = {
    ...(name.trim() !== client.name && { name }),
    ...(paidUntil !== (client.paid_until ?? '') && { paid_until: paidUntil || null }),
  }
  const changed = Object.keys(changes).length > 0

  // Another month from the paid date, or from today once that has passed.
  const addMonth = () => {
    const today = localDate(new Date())
    setPaidUntil(addMonths(paidUntil && paidUntil >= today ? paidUntil : today, 1))
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (name.trim() && changed) save.mutate(changes)
  }

  return (
    <form className="stack" onSubmit={submit}>
      <label className="field">
        <span>Ім’я</span>
        <input value={name} onChange={(event) => setName(event.target.value)} maxLength={80} />
      </label>
      <div className="field">
        <span id="paid-until-label">Оплачено до</span>
        <input
          type="date"
          value={paidUntil}
          onChange={(event) => setPaidUntil(event.target.value)}
          aria-labelledby="paid-until-label"
        />
        <div className="chips wrap">
          <button type="button" className="chip" onClick={addMonth}>
            +1 місяць
          </button>
          {paidUntil && (
            <button type="button" className="chip" onClick={() => setPaidUntil('')}>
              Без дати
            </button>
          )}
        </div>
        <p className="muted small">
          Останній оплачений день. За три дні до нього бот нагадає тобі у вечірньому підсумку.
        </p>
      </div>
      {changed && (
        <button
          type="submit"
          className="button primary block"
          disabled={!name.trim() || save.isPending}
        >
          {save.isPending ? 'Зберігаю…' : 'Зберегти'}
        </button>
      )}
      {!changed && save.isSuccess && <p className="muted small">Збережено.</p>}
      {save.isError && !isUnauthorized(save.error) && (
        <p className="error">Не вдалося зберегти. Спробуй ще раз.</p>
      )}
    </form>
  )
}

function ArchiveButton({ client }: { client: Client }) {
  const { base } = useCoach()
  const navigate = useNavigate()
  const archive = useUpdateClient(client.id, () => void navigate(base, { replace: true }))

  const onClick = async () => {
    const confirmed = await confirmAction(
      `Архівувати «${client.name}»? Застосунок для клієнта закриється, нагадування припиняться. Тренування й звіти залишаться в тебе, клієнта можна повернути.`,
    )
    if (confirmed) archive.mutate({ archived: true })
  }

  return (
    <>
      <button
        type="button"
        className="link-button danger"
        disabled={archive.isPending}
        onClick={() => void onClick()}
      >
        Архівувати клієнта
      </button>
      {archive.isError && !isUnauthorized(archive.error) && (
        <p className="error">Не вдалося архівувати. Спробуй ще раз.</p>
      )}
    </>
  )
}

/** Shown on an archived client's pages, with the way back. */
export function ArchivedNotice({ client }: { client: Client }) {
  const restore = useUpdateClient(client.id)
  return (
    <div className="notice stack">
      <p>Клієнт в архіві: застосунок для нього закритий, бот нічого не надсилає.</p>
      <button
        type="button"
        className="button block"
        disabled={restore.isPending}
        onClick={() => restore.mutate({ archived: false })}
      >
        Повернути з архіву
      </button>
      {restore.isError && !isUnauthorized(restore.error) && (
        <p className="error">Не вдалося повернути. Спробуй ще раз.</p>
      )}
    </div>
  )
}
