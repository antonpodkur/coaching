import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type ChangeEvent, useEffect, useState } from 'react'

import { ApiError, type Schemas, api, unwrap } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { FormVideoTile } from '../shared/FormVideoTile'
import { PhotoGrid } from '../shared/PhotoGrid'
import { Screen } from '../shared/Screen'
import { holdClosing } from '../shared/closingGuard'
import { confirmAction } from '../shared/dialogs'
import { CameraIcon, PhotoIcon } from '../shared/icons'
import { VideoSendCards } from './VideoSendCards'
import {
  GYM_VIDEOS,
  MAX_PHOTOS,
  MAX_VIDEOS,
  QUESTIONNAIRE_KEY,
  markStarted,
  uploadPhoto,
  useQuestionnaire,
} from './questionnaire'
import { sendVideo, useVideoSends } from './videoSends'

type Questionnaire = Schemas['Questionnaire']
type Sex = Schemas['Sex']

const SEXES: { value: Sex; label: string }[] = [
  { value: 'female', label: 'Жіноча' },
  { value: 'male', label: 'Чоловіча' },
]

/**
 * The questionnaire for Dasha: age, sex, height, and photos and videos of the
 * client's gym. Optional, and only Dasha sees it.
 */
export function QuestionnairePage() {
  const questionnaire = useQuestionnaire()
  const back = <BackLink to="/app" label="Головна" />
  if (!questionnaire.data) {
    return (
      <Screen>
        {back}
        <p className={questionnaire.isError ? 'error' : 'muted'}>
          {questionnaire.isError ? 'Не вдалося завантажити анкету.' : 'Завантаження…'}
        </p>
      </Screen>
    )
  }
  return (
    <Screen>
      {back}
      <header className="exercise-head">
        <h1>Анкета для Даші</h1>
        <p className="muted small">
          Необов’язково, але так Даша точніше складе програму під тебе. Бачить лише вона.
        </p>
      </header>
      <AnswersForm saved={questionnaire.data} />
      <GymSection questionnaire={questionnaire.data} />
    </Screen>
  )
}

/** `1991` → 1991, empty → `null`, anything else → `undefined`. */
function readWhole(text: string): number | null | undefined {
  const value = text.trim()
  if (value === '') return null
  return /^\d{1,4}$/.test(value) ? Number(value) : undefined
}

function AnswersForm({ saved }: { saved: Questionnaire }) {
  const queryClient = useQueryClient()
  const [yearText, setYearText] = useState(saved.birth_year?.toString() ?? '')
  const [sex, setSex] = useState<Sex | null>(saved.sex ?? null)
  const [heightText, setHeightText] = useState(saved.height_cm?.toString() ?? '')
  const birthYear = readWhole(yearText)
  const heightCm = readWhole(heightText)
  const changed =
    birthYear !== (saved.birth_year ?? null) ||
    sex !== (saved.sex ?? null) ||
    heightCm !== (saved.height_cm ?? null)

  const save = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.PUT('/me/questionnaire', {
          body: { birth_year: birthYear ?? null, sex, height_cm: heightCm ?? null },
        }),
      ),
    onSuccess: (questionnaire) => {
      queryClient.setQueryData(QUESTIONNAIRE_KEY, questionnaire)
      markStarted(queryClient)
    },
  })
  const code = save.error instanceof ApiError ? save.error.code : null

  return (
    <form
      className="stack"
      aria-labelledby="about-title"
      onSubmit={(event) => {
        event.preventDefault()
        if (changed && birthYear !== undefined && heightCm !== undefined) save.mutate()
      }}
    >
      <h2 id="about-title" className="section-title">
        Про тебе
      </h2>
      <div className="field-row">
        <label className="field">
          <span>Рік народження</span>
          <input
            inputMode="numeric"
            maxLength={4}
            placeholder="1995"
            value={yearText}
            className={birthYear === undefined ? 'bad' : undefined}
            onChange={(event) => setYearText(event.target.value)}
          />
        </label>
        <label className="field">
          <span>Зріст, см</span>
          <input
            inputMode="numeric"
            maxLength={3}
            placeholder="170"
            value={heightText}
            className={heightCm === undefined ? 'bad' : undefined}
            onChange={(event) => setHeightText(event.target.value)}
          />
        </label>
      </div>
      <div className="field">
        <span>Стать</span>
        <div className="chips" role="radiogroup" aria-label="Стать">
          {SEXES.map((option) => (
            <button
              key={option.value}
              type="button"
              role="radio"
              className="chip"
              aria-checked={sex === option.value}
              aria-pressed={sex === option.value}
              // Tapping the chosen one again clears it.
              onClick={() => setSex(sex === option.value ? null : option.value)}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>
      {code === 'invalid_birth_year' && <p className="error">Перевір рік народження.</p>}
      {code === 'invalid_height' && <p className="error">Зріст — у сантиметрах, від 100 до 250.</p>}
      {save.isError && !code?.startsWith('invalid_') && (
        <p className="error">Не вдалося зберегти. Спробуй ще раз.</p>
      )}
      <button
        type="submit"
        className="button primary block"
        disabled={!changed || birthYear === undefined || heightCm === undefined || save.isPending}
      >
        {save.isPending ? 'Зберігаю…' : save.isSuccess && !changed ? 'Збережено' : 'Зберегти'}
      </button>
    </form>
  )
}

function photoFailure(err: unknown): string {
  if (err instanceof ApiError && err.code === 'too_many_photos') {
    return `Можна додати до ${MAX_PHOTOS} фото.`
  }
  if (err instanceof ApiError && err.code === 'photos_not_configured') {
    return 'Фото ще не налаштовані.'
  }
  return 'Не вдалося додати фото. Перевір зв’язок і спробуй ще раз.'
}

/** Photos and videos of the gym, so Dasha knows what the client can train with. */
function GymSection({ questionnaire }: { questionnaire: Questionnaire }) {
  const queryClient = useQueryClient()
  const [uploading, setUploading] = useState(0)
  const [photoError, setPhotoError] = useState<string | null>(null)
  const sends = useVideoSends().filter((send) => send.target === GYM_VIDEOS.key)
  const { photos, videos } = questionnaire
  const photoSlots = MAX_PHOTOS - photos.length - uploading
  const videoCount =
    videos.filter((video) => video.status !== 'failed').length +
    sends.filter((send) => send.phase !== 'failed').length

  // Closing the app now would drop the photos still on their way.
  useEffect(() => {
    holdClosing('photo-upload', uploading > 0)
    return () => holdClosing('photo-upload', false)
  }, [uploading])

  const addPhotos = async (event: ChangeEvent<HTMLInputElement>) => {
    const files = [...(event.target.files ?? [])].slice(0, Math.max(0, photoSlots))
    event.target.value = ''
    if (files.length === 0) return
    setPhotoError(null)
    setUploading((count) => count + files.length)
    // One at a time, which gym Wi-Fi copes with better.
    for (const file of files) {
      try {
        const photo = await uploadPhoto(file)
        queryClient.setQueryData<Questionnaire>(QUESTIONNAIRE_KEY, (current) =>
          current ? { ...current, photos: [...current.photos, photo] } : current,
        )
        markStarted(queryClient)
      } catch (err) {
        console.error('photo upload failed', err)
        setPhotoError(photoFailure(err))
      } finally {
        setUploading((count) => count - 1)
      }
    }
  }

  const addVideo = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (!file) return
    markStarted(queryClient)
    void sendVideo(GYM_VIDEOS, file, queryClient)
  }

  const remove = async (id: string, what: string) => {
    if (!(await confirmAction(`Видалити ${what}? Даша його більше не побачить.`))) return
    await api.DELETE('/me/gym/{id}', { params: { path: { id } } }).catch(() => undefined)
    void queryClient.invalidateQueries({ queryKey: QUESTIONNAIRE_KEY })
  }

  return (
    <section className="stack" aria-labelledby="gym-title">
      <h2 id="gym-title" className="section-title">
        Твій зал
      </h2>
      <p className="muted small">
        Сфотографуй тренажери, стійки й гантелі — Даша побачить, з чим ти працюєш.
      </p>
      <PhotoGrid
        photos={photos}
        pending={uploading}
        onDelete={(id) => void remove(id, 'це фото')}
      />
      {photoSlots > 0 && (
        <label className="button block file-button">
          <PhotoIcon />
          Додати фото
          <input type="file" accept="image/*" multiple onChange={(event) => void addPhotos(event)} />
        </label>
      )}
      {photoError && <p className="error">{photoError}</p>}

      {(videos.length > 0 || sends.length > 0) && (
        <div className="form-videos">
          {videos.map((video, index) => (
            <FormVideoTile
              key={video.id}
              video={video}
              label={`Відео ${index + 1}`}
              onDelete={() => void remove(video.id, 'це відео')}
            />
          ))}
          <VideoSendCards sends={sends} />
        </div>
      )}
      {videoCount < MAX_VIDEOS && (
        <label className="button block file-button">
          <CameraIcon />
          Додати відео
          <input type="file" accept="video/*" onChange={addVideo} />
        </label>
      )}
      <p className="muted small">
        До {MAX_PHOTOS} фото й {MAX_VIDEOS} відео до 3 хвилин.
      </p>
    </section>
  )
}
