import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { Link } from 'react-router'

import { api, unwrap } from '../api/client'
import { Avatar } from '../shared/Avatar'
import { formatDay, formatToday } from '../shared/format'
import { InstallCard } from '../shared/InstallCard'
import { PushCard } from '../shared/PushCard'
import { PersonIcon, PlusIcon, SearchIcon } from '../shared/icons'
import { InviteCard, type ShownInvite } from './InviteCard'
import { ARCHIVED_KEY, type Client, payment, planned } from './clients'
import { CLIENTS_KEY, isUnauthorized, useCoach } from './context'

/** Search appears once the list no longer fits on a phone screen. */
const SEARCH_FROM = 7

/** Dasha's home screen: everyone she coaches, and who still has to join. */
export function ClientsPage() {
  const { base, coach, onUnauthorized } = useCoach()
  const queryClient = useQueryClient()
  const [query, setQuery] = useState('')
  const [invite, setInvite] = useState<ShownInvite | null>(null)

  const clients = useQuery({
    queryKey: CLIENTS_KEY,
    queryFn: async () => unwrap(await api.GET('/coach/clients')),
  })
  useEffect(() => {
    if (isUnauthorized(clients.error)) onUnauthorized()
  }, [clients.error, onUnauthorized])

  const reinvite = useMutation({
    mutationFn: async (client: Client) => {
      const link = unwrap(
        await api.POST('/coach/clients/{id}/invite', { params: { path: { id: client.id } } }),
      )
      return { name: client.name, ...link }
    },
    onSuccess: (link) => {
      setInvite(link)
      window.scrollTo({ top: 0, behavior: 'smooth' })
      void queryClient.invalidateQueries({ queryKey: CLIENTS_KEY })
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  // Clients with a report or a video Dasha has not opened come first.
  const fresh = (client: Client) => client.unseen_reports > 0 || client.new_videos > 0
  const all = [...(clients.data ?? [])].sort((a, b) => Number(fresh(b)) - Number(fresh(a)))
  const needle = query.trim().toLocaleLowerCase('uk')
  const shown = needle
    ? all.filter((client) => client.name.toLocaleLowerCase('uk').includes(needle))
    : all

  return (
    <section className="page">
      <div className="page-top">
        <p className="muted small">{formatToday()}</p>
        <Link className="icon-button" to={`${base}/profile`} aria-label="Профіль">
          {coach.avatar_url ? <Avatar name={coach.name} url={coach.avatar_url} /> : <PersonIcon />}
        </Link>
      </div>
      <header className="page-head">
        <h1>
          Клієнти {clients.data && <span className="count">{all.length}</span>}
        </h1>
        <Link className="button primary" to={`${base}/invite`}>
          <PlusIcon />
          Запросити
        </Link>
      </header>

      {invite && <InviteCard invite={invite} onClose={() => setInvite(null)} />}
      {!invite && (
        <>
          <InstallCard />
          <PushCard forCoach />
        </>
      )}
      {reinvite.isError && !isUnauthorized(reinvite.error) && (
        <p className="error">Не вдалося створити посилання. Спробуй ще раз.</p>
      )}

      {all.length >= SEARCH_FROM && (
        <label className="search">
          <SearchIcon />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Пошук за ім’ям"
            aria-label="Пошук клієнта"
          />
        </label>
      )}

      {clients.isPending && <p className="muted">Завантаження…</p>}
      {clients.isError && !isUnauthorized(clients.error) && (
        <p className="error">Не вдалося завантажити клієнтів.</p>
      )}
      {clients.data?.length === 0 && (
        <p className="muted">
          Поки нікого. Запроси першу людину: вона відкриє посилання в Telegram і одразу
          потрапить у застосунок.
        </p>
      )}
      {needle && shown.length === 0 && <p className="muted">Нікого не знайдено.</p>}

      <ul className="client-list">
        {shown.map((client) => (
          <li key={client.id} className="client-row">
            <Link className="client-link" to={`${base}/clients/${client.id}`}>
              <Avatar name={client.name} url={client.avatar_url} />
              <span className="client-text">
                <span className="client-name">{client.name}</span>
                <ClientStatus client={client} />
              </span>
            </Link>
            {!client.joined && (
              <button
                type="button"
                className="button small"
                disabled={reinvite.isPending}
                onClick={() => reinvite.mutate(client)}
              >
                Нове посилання
              </button>
            )}
          </li>
        ))}
      </ul>

      {clients.isSuccess && !needle && <Archive />}
    </section>
  )
}

/** Archived clients, loaded only when Dasha asks for them. */
function Archive() {
  const { base, onUnauthorized } = useCoach()
  const [open, setOpen] = useState(false)
  const archived = useQuery({
    queryKey: ARCHIVED_KEY,
    queryFn: async () =>
      unwrap(await api.GET('/coach/clients', { params: { query: { archived: true } } })),
    enabled: open,
  })
  useEffect(() => {
    if (isUnauthorized(archived.error)) onUnauthorized()
  }, [archived.error, onUnauthorized])

  return (
    <section className="stack archive" aria-label="Архів">
      <button
        type="button"
        className="link-button"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        {open ? 'Сховати архів' : 'Архів'}
      </button>
      {open && archived.isPending && <p className="muted">Завантаження…</p>}
      {open && archived.isError && !isUnauthorized(archived.error) && (
        <p className="error">Не вдалося завантажити архів.</p>
      )}
      {open && archived.data?.length === 0 && <p className="muted">В архіві нікого.</p>}
      {open && (
        <ul className="client-list">
          {archived.data?.map((client) => (
            <li key={client.id} className="client-row">
              <Link className="client-link" to={`${base}/clients/${client.id}`}>
                <Avatar name={client.name} url={client.avatar_url} />
                <span className="client-text">
                  <span className="client-name">{client.name}</span>
                  <span className="client-status">В архіві</span>
                </span>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

function ClientStatus({ client }: { client: Client }) {
  if (client.unseen_reports > 0) {
    return (
      <span className="client-status warn">
        {client.unseen_reports === 1 ? 'Новий звіт' : `Нових звітів: ${client.unseen_reports}`}
      </span>
    )
  }
  if (client.new_videos > 0) {
    return (
      <span className="client-status warn">
        {client.new_videos === 1 ? 'Нове відео' : `Нових відео: ${client.new_videos}`}
      </span>
    )
  }
  const paid = payment(client.paid_until)
  if (paid?.due) {
    return (
      <span className="client-status warn">
        {paid.text.charAt(0).toUpperCase() + paid.text.slice(1)}
      </span>
    )
  }
  if (client.joined) {
    const plan = planned(client)
    return <span className={plan.warn ? 'client-status warn' : 'client-status'}>{plan.text}</span>
  }
  if (!client.invite_expires_at) return <span className="client-status">Без запрошення</span>
  const expired = new Date(client.invite_expires_at) < new Date()
  return expired ? (
    <span className="client-status warn">Запрошення минуло</span>
  ) : (
    <span className="client-status">Запрошення до {formatDay(client.invite_expires_at)}</span>
  )
}
