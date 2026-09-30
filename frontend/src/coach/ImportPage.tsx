import { useMutation } from '@tanstack/react-query'
import { useState } from 'react'

import { type Schemas, api, unwrap } from '../api/client'
import { formatSet, plural } from '../shared/format'
import { isUnauthorized, useCoach } from './context'

/** One-off import of an old Telegram plan: paste it, see how it was read. */
export function ImportPage() {
  const { onUnauthorized } = useCoach()
  const [text, setText] = useState('')
  const parse = useMutation({
    mutationFn: async (plan: string) =>
      unwrap(await api.POST('/coach/import/parse', { body: { text: plan } })),
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  return (
    <section className="import">
      <div className="import-intro">
        <h1>Імпорт плану з Telegram</h1>
        <p className="muted">
          Встав старе повідомлення з планом — вправи й підходи розкладуться самі. Знадобиться раз,
          коли переносиш клієнта в застосунок.
        </p>
      </div>

      <div className="import-grid">
        <label className="field">
          <span>Текст плану</span>
          <textarea
            value={text}
            onChange={(event) => setText(event.target.value)}
            rows={18}
            spellCheck={false}
          />
        </label>
        <div className="import-preview">
          {parse.data ? (
            <Preview preview={parse.data} />
          ) : (
            <p className="muted">Тут з’явиться розпізнаний план.</p>
          )}
        </div>
      </div>

      <div className="import-actions">
        <button
          type="button"
          className="button primary"
          disabled={!text.trim() || parse.isPending}
          onClick={() => parse.mutate(text)}
        >
          {parse.isPending ? 'Розбираю…' : 'Розібрати'}
        </button>
        {parse.isError && <p className="error">Не вдалося розібрати план. Спробуй ще раз.</p>}
      </div>
    </section>
  )
}

function Preview({ preview }: { preview: Schemas['ImportPreview'] }) {
  const exerciseCount = preview.exercises.length
  const setCount = preview.exercises.reduce((sum, exercise) => sum + exercise.sets.length, 0)

  return (
    <>
      <p className="preview-summary">
        Розпізнано: {exerciseCount} {plural(exerciseCount, 'вправа', 'вправи', 'вправ')} ·{' '}
        {setCount} {plural(setCount, 'підхід', 'підходи', 'підходів')}
      </p>

      {preview.unparsed.length > 0 && (
        <div className="warning">
          <strong>Не вдалося розпізнати — виправ або видали:</strong>
          {preview.unparsed.map((line, index) => (
            <span key={index}>«{line}»</span>
          ))}
        </div>
      )}

      <ol className="preview-list">
        {preview.exercises.map((exercise, index) => (
          <li key={index} className="preview-item">
            <div className="preview-head">
              <span className="preview-name">{exercise.name}</span>
              <span className={exercise.exercise_id ? 'tag' : 'tag tag-new'}>
                {exercise.exercise_id ? 'з бібліотеки' : 'нова вправа'}
              </span>
            </div>
            <span className={exercise.sets.length ? 'preview-sets' : 'preview-sets error'}>
              {exercise.sets.length
                ? exercise.sets.map(formatSet).join(' · ')
                : 'немає підходів'}
              {exercise.per_side_label && ` · ${exercise.per_side_label}`}
            </span>
            {exercise.note && <span className="preview-note">{exercise.note}</span>}
          </li>
        ))}
      </ol>
    </>
  )
}
