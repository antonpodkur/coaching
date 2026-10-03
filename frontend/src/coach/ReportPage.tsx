import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { Link, useNavigate, useParams } from 'react-router'

import { ApiError, api, unwrap } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { CopyIcon } from '../shared/icons'
import { ReportCard } from './ReportCard'
import { useBackTarget } from './backTarget'
import { WORKOUTS_KEY, isUnauthorized, useCoach } from './context'
import { resultsQuery } from './results'

/** One workout's full report: every exercise and set next to the plan. */
export function ReportPage() {
  const { id = '' } = useParams()
  const { base, onUnauthorized } = useCoach()
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const results = useQuery(resultsQuery(id))
  useEffect(() => {
    if (isUnauthorized(results.error)) onUnauthorized()
  }, [results.error, onUnauthorized])

  const copy = useMutation({
    mutationFn: async (clientId: string) =>
      unwrap(await api.POST('/coach/workouts', { body: { client_id: clientId, copy_from: id } })),
    onSuccess: (workout) => {
      void queryClient.invalidateQueries({ queryKey: WORKOUTS_KEY })
      void navigate(`${base}/workouts/${workout.id}`)
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  const clientId = results.data?.client_id
  const backTarget = useBackTarget({
    to: clientId ? `${base}/clients/${clientId}` : base,
    label: 'Клієнт',
  })
  const back = <BackLink to={backTarget.to} label={backTarget.label} />
  if (!results.data) {
    const missing = results.error instanceof ApiError && results.error.status === 404
    return (
      <section className="page">
        {back}
        <p className={results.isError ? 'error' : 'muted'}>
          {results.isError ? (missing ? 'Такого тренування немає.' : 'Не вдалося завантажити.') : 'Завантаження…'}
        </p>
      </section>
    )
  }

  return (
    <section className="page">
      {back}
      <ReportCard
        results={results.data}
        actions={
          <div className="stack">
            {clientId && (
              <button
                type="button"
                className="button primary block"
                disabled={copy.isPending}
                onClick={() => copy.mutate(clientId)}
              >
                <CopyIcon />
                Скопіювати в наступне тренування
              </button>
            )}
            <Link className="button block" to={`${base}/workouts/${id}`}>
              Відкрити в конструкторі
            </Link>
            {copy.isError && !isUnauthorized(copy.error) && (
              <p className="error">Не вдалося створити копію.</p>
            )}
          </div>
        }
      />
    </section>
  )
}
