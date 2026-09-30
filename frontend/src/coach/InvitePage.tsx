import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'

import { api, unwrap } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { InviteCard } from './InviteCard'
import { CLIENTS_KEY, isUnauthorized, useCoach } from './context'

/** Adds a client and hands Dasha their invite link to send in Telegram. */
export function InvitePage() {
  const { base, onUnauthorized } = useCoach()
  const queryClient = useQueryClient()
  const [name, setName] = useState('')

  const create = useMutation({
    mutationFn: async (newName: string) =>
      unwrap(await api.POST('/coach/clients', { body: { name: newName } })),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: CLIENTS_KEY }),
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })
  const created = create.data

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (name.trim()) create.mutate(name)
  }

  const inviteAnother = () => {
    create.reset()
    setName('')
  }

  return (
    <section className="page">
      <BackLink to={base} label="Клієнти" />
      <div className="page-intro">
        <h1>Запросити клієнта</h1>
        <p className="muted">
          Людина відкриває посилання в Telegram і натискає Start. Реєстрація й паролі не потрібні.
        </p>
      </div>

      {created ? (
        <>
          <InviteCard invite={{ name: created.client.name, ...created.invite }} />
          <button type="button" className="link-button" onClick={inviteAnother}>
            Запросити ще когось
          </button>
        </>
      ) : (
        <form className="stack" onSubmit={submit}>
          <label className="field">
            <span>Ім’я</span>
            <input
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Наприклад, Аліна Р."
              maxLength={80}
            />
          </label>
          <button
            type="submit"
            className="button primary block"
            disabled={!name.trim() || create.isPending}
          >
            {create.isPending ? 'Створюю…' : 'Створити посилання'}
          </button>
          {create.isError && !isUnauthorized(create.error) && (
            <p className="error">Не вдалося створити посилання. Спробуй ще раз.</p>
          )}
        </form>
      )}
    </section>
  )
}
