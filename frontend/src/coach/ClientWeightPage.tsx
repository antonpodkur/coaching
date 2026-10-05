import { useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'
import { useParams } from 'react-router'

import { ApiError } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { WeightHistory } from '../shared/WeightHistory'
import { clientQuery, weightQuery } from './clients'
import { isUnauthorized, useCoach } from './context'

/** How a client's weight moves, from what they log. */
export function ClientWeightPage() {
  const { id = '' } = useParams()
  const { base, onUnauthorized } = useCoach()
  const client = useQuery(clientQuery(id))
  const weight = useQuery(weightQuery(id))
  useEffect(() => {
    if (isUnauthorized(client.error) || isUnauthorized(weight.error)) onUnauthorized()
  }, [client.error, weight.error, onUnauthorized])

  const back = <BackLink to={`${base}/clients/${id}`} label="Клієнт" />
  if (!weight.data || !client.data) {
    const failed = weight.error ?? client.error
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

  return (
    <section className="page">
      {back}
      <header className="page-title">
        <p className="muted small">{client.data.name}</p>
        <h1>Вага</h1>
      </header>
      {weight.data.length === 0 ? (
        <p className="muted">Клієнт ще не записував вагу.</p>
      ) : (
        <WeightHistory entries={weight.data} />
      )}
    </section>
  )
}
