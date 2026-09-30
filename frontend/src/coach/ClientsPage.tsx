import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useEffect, useState } from 'react'

import { ApiError, type Schemas, api, unwrap } from '../api/client'

type Client = Schemas['CoachClient']
type ShownInvite = Schemas['InviteLink'] & { name: string }

const CLIENTS_KEY = ['coach-clients']

function formatDay(iso: string) {
  return new Date(iso).toLocaleDateString('uk-UA', { day: 'numeric', month: 'long' })
}

/** Clients and their invite links. */
export function ClientsPage({ onUnauthorized }: { onUnauthorized: () => void }) {
  const queryClient = useQueryClient()
  const [name, setName] = useState('')
  const [invite, setInvite] = useState<ShownInvite | null>(null)

  const clients = useQuery({
    queryKey: CLIENTS_KEY,
    queryFn: async () => unwrap(await api.GET('/coach/clients')),
  })

  const onError = (err: Error) => {
    if (err instanceof ApiError && err.status === 401) onUnauthorized()
  }
  useEffect(() => {
    if (clients.error instanceof ApiError && clients.error.status === 401) onUnauthorized()
  }, [clients.error, onUnauthorized])

  const create = useMutation({
    mutationFn: async (newName: string) =>
      unwrap(await api.POST('/coach/clients', { body: { name: newName } })),
    onSuccess: (created) => {
      setInvite({ name: created.client.name, ...created.invite })
      setName('')
      void queryClient.invalidateQueries({ queryKey: CLIENTS_KEY })
    },
    onError,
  })

  const reinvite = useMutation({
    mutationFn: async (client: Client) => {
      const link = unwrap(
        await api.POST('/coach/clients/{id}/invite', { params: { path: { id: client.id } } }),
      )
      return { name: client.name, ...link }
    },
    onSuccess: (link) => {
      setInvite(link)
      void queryClient.invalidateQueries({ queryKey: CLIENTS_KEY })
    },
    onError,
  })

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (name.trim()) create.mutate(name)
  }

  return (
    <section className="clients">
      <h1>Клієнти</h1>

      <form className="add-client" onSubmit={submit}>
        <label className="field">
          <span>Нова клієнтка чи клієнт</span>
          <input
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Ім’я, як ти його бачиш у Telegram"
            maxLength={80}
          />
        </label>
        <button type="submit" className="button primary" disabled={!name.trim() || create.isPending}>
          Додати
        </button>
      </form>
      {create.isError && <p className="error">Не вдалося додати. Спробуй ще раз.</p>}

      {invite && <InviteCard invite={invite} onClose={() => setInvite(null)} />}

      {clients.isPending && <p className="muted">Завантаження…</p>}
      {clients.data?.length === 0 && (
        <p className="muted">Поки нікого. Додай першу людину — отримаєш посилання-запрошення.</p>
      )}
      <ul className="client-list">
        {clients.data?.map((client) => (
          <li key={client.id} className="client-row">
            <span className="client-name">{client.name}</span>
            <ClientStatus client={client} />
            <button
              type="button"
              className="button small"
              disabled={reinvite.isPending}
              onClick={() => reinvite.mutate(client)}
            >
              Нове посилання
            </button>
          </li>
        ))}
      </ul>
    </section>
  )
}

function ClientStatus({ client }: { client: Client }) {
  if (client.joined) return <span className="tag tag-ok">у застосунку</span>
  if (!client.invite_expires_at) return <span className="tag">без запрошення</span>
  const expired = new Date(client.invite_expires_at) < new Date()
  return expired ? (
    <span className="tag tag-warn">запрошення минуло</span>
  ) : (
    <span className="tag">запрошення до {formatDay(client.invite_expires_at)}</span>
  )
}

function InviteCard({ invite, onClose }: { invite: ShownInvite; onClose: () => void }) {
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(invite.url)
      setCopied(true)
    } catch {
      setCopied(false)
    }
  }

  return (
    <div className="invite-card" role="status">
      <strong>Запрошення для {invite.name}</strong>
      <div className="invite-link">
        <input readOnly value={invite.url} aria-label="Посилання-запрошення" onFocus={(e) => e.target.select()} />
        <button type="button" className="button primary small" onClick={copy}>
          {copied ? 'Скопійовано' : 'Скопіювати'}
        </button>
      </div>
      <p className="muted">
        Надішли його в Telegram. Посилання одноразове й діє до {formatDay(invite.expires_at)}.
      </p>
      <button type="button" className="link-button" onClick={onClose}>
        Закрити
      </button>
    </div>
  )
}
