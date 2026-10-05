import { useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'
import { useParams } from 'react-router'

import { ApiError } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { FormVideoTile } from '../shared/FormVideoTile'
import { PhotoGrid } from '../shared/PhotoGrid'
import { formatAge } from '../shared/format'
import { SEX_WORD, clientQuery, questionnaireQuery } from './clients'
import { isUnauthorized, useCoach } from './context'

/** What a client told Dasha about themself, and their gym's photos and videos. */
export function ClientQuestionnairePage() {
  const { id = '' } = useParams()
  const { base, onUnauthorized } = useCoach()
  const client = useQuery(clientQuery(id))
  const questionnaire = useQuery(questionnaireQuery(id))
  useEffect(() => {
    if (isUnauthorized(client.error) || isUnauthorized(questionnaire.error)) onUnauthorized()
  }, [client.error, questionnaire.error, onUnauthorized])

  const back = <BackLink to={`${base}/clients/${id}`} label="Клієнт" />
  const answers = questionnaire.data
  if (!answers || !client.data) {
    const failed = questionnaire.error ?? client.error
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

  const facts = [
    {
      label: 'Вік',
      value: answers.birth_year != null && `${formatAge(answers.birth_year)} (${answers.birth_year})`,
    },
    { label: 'Стать', value: answers.sex && SEX_WORD[answers.sex] },
    { label: 'Зріст', value: answers.height_cm != null && `${answers.height_cm} см` },
  ]
  const gymEmpty = answers.photos.length === 0 && answers.videos.length === 0

  return (
    <section className="page">
      {back}
      <header className="page-title">
        <p className="muted small">{client.data.name}</p>
        <h1>Анкета</h1>
      </header>

      <dl className="facts">
        {facts.map((fact) => (
          <div key={fact.label} className="fact">
            <dt>{fact.label}</dt>
            <dd className={fact.value ? undefined : 'muted'}>{fact.value || 'не вказано'}</dd>
          </div>
        ))}
      </dl>

      <section className="stack" aria-labelledby="gym-title">
        <h2 id="gym-title" className="section-title">
          Зал
        </h2>
        {gymEmpty && <p className="muted">Фото й відео залу ще немає.</p>}
        <PhotoGrid photos={answers.photos} />
        {answers.videos.length > 0 && (
          <div className="form-videos">
            {answers.videos.map((video, index) => (
              <FormVideoTile key={video.id} video={video} label={`Відео ${index + 1}`} />
            ))}
          </div>
        )}
      </section>
    </section>
  )
}
