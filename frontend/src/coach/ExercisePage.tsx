import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type ChangeEvent, type FormEvent, useEffect, useState } from 'react'
import { useNavigate, useParams } from 'react-router'

import { ApiError, type Schemas, api, unwrap } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { goBackTo } from '../shared/history'
import { PhotoGrid } from '../shared/PhotoGrid'
import { VideoPlayer } from '../shared/VideoPlayer'
import { jpegBody, shrinkPhoto } from '../shared/photos'
import { PhotoIcon, UploadIcon } from '../shared/icons'
import { GroupPicker } from './GroupPicker'
import { MeasurePicker } from './MeasurePicker'
import { UploadCard } from './UploadCard'
import { EXERCISES_KEY, isUnauthorized, useCoach } from './context'
import { confirmAction } from '../shared/dialogs'
import { type Exercise, formatLength } from './library'
import { uploadVideo, useUploads } from './uploads'

/** While Bunny encodes, ask again this often. */
const PROCESSING_POLL_MS = 5_000

/** One exercise: its video, name and group. */
export function ExercisePage() {
  const { id = '' } = useParams()
  const { base, onUnauthorized } = useCoach()

  const exercise = useQuery({
    queryKey: [...EXERCISES_KEY, id],
    queryFn: async () =>
      unwrap(await api.GET('/coach/exercises/{id}', { params: { path: { id } } })),
    refetchInterval: (q) =>
      q.state.data?.upload?.status === 'processing' ? PROCESSING_POLL_MS : false,
  })
  useEffect(() => {
    if (isUnauthorized(exercise.error)) onUnauthorized()
  }, [exercise.error, onUnauthorized])

  const back = <BackLink to={`${base}/exercises`} label="Вправи" />
  if (exercise.isPending) {
    return (
      <section className="page">
        {back}
        <p className="muted">Завантаження…</p>
      </section>
    )
  }
  if (exercise.isError) {
    const missing = exercise.error instanceof ApiError && exercise.error.status === 404
    return (
      <section className="page">
        {back}
        <p className="error">{missing ? 'Такої вправи немає.' : 'Не вдалося завантажити вправу.'}</p>
      </section>
    )
  }
  return (
    <section className="page">
      {back}
      {/* Remount the form when another exercise opens, so it starts from that one. */}
      <ExerciseForm key={exercise.data.id} exercise={exercise.data} />
      <VideoSection exercise={exercise.data} />
      <PhotosSection exercise={exercise.data} />
      <ArchiveButton exercise={exercise.data} />
    </section>
  )
}

function ExerciseForm({ exercise }: { exercise: Exercise }) {
  const { onUnauthorized } = useCoach()
  const queryClient = useQueryClient()
  const [name, setName] = useState(exercise.name)
  const [group, setGroup] = useState(exercise.muscle_group ?? null)
  const [measure, setMeasure] = useState(exercise.measure)
  const [description, setDescription] = useState(exercise.description ?? '')
  const changed =
    name.trim() !== exercise.name ||
    group !== (exercise.muscle_group ?? null) ||
    measure !== exercise.measure ||
    description.trim() !== (exercise.description ?? '')

  const save = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.PATCH('/coach/exercises/{id}', {
          params: { path: { id: exercise.id } },
          body: { name, muscle_group: group ?? '', measure, description },
        }),
      ),
    onSuccess: (saved) => {
      queryClient.setQueryData([...EXERCISES_KEY, exercise.id], saved)
      void queryClient.invalidateQueries({ queryKey: EXERCISES_KEY })
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })
  const taken = save.error instanceof ApiError && save.error.code === 'name_taken'
  const measureInUse = save.error instanceof ApiError && save.error.code === 'measure_in_use'

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (name.trim() && changed) save.mutate()
  }

  return (
    <form className="stack" onSubmit={submit}>
      <label className="field">
        <span>Назва</span>
        <input
          className="title-input"
          value={name}
          onChange={(event) => setName(event.target.value)}
          maxLength={120}
        />
      </label>
      <div className="field">
        <span>Група м’язів</span>
        <GroupPicker value={group} onChange={setGroup} />
      </div>
      <div className="field">
        <span>Як рахувати</span>
        <MeasurePicker value={measure} onChange={setMeasure} />
      </div>
      <label className="field">
        <span>Опис для клієнта</span>
        <textarea
          rows={3}
          maxLength={1000}
          placeholder="Наприклад: широка рукоятка, хват зверху, сидіння на 4-ту позначку"
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />
      </label>
      {/* Always in place, so it is there to find; it wakes up once something changes. */}
      <button
        type="submit"
        className="button primary block"
        disabled={!changed || !name.trim() || save.isPending}
      >
        {save.isPending ? 'Зберігаю…' : save.isSuccess && !changed ? 'Збережено' : 'Зберегти'}
      </button>
      {taken && <p className="error">Вправа з такою назвою вже є.</p>}
      {measureInUse && (
        <p className="error">
          Ця вправа вже є в тренуваннях, тож повтори не можна перетворити на час чи навпаки.
          Додай окрему вправу, наприклад «Планка на час».
        </p>
      )}
      {save.isError && !taken && !measureInUse && !isUnauthorized(save.error) && (
        <p className="error">Не вдалося зберегти. Спробуй ще раз.</p>
      )}
    </form>
  )
}

function VideoSection({ exercise }: { exercise: Exercise }) {
  const queryClient = useQueryClient()
  const upload = useUploads().find((item) => item.exerciseId === exercise.id)
  const serverUpload = exercise.upload

  const pick = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) void uploadVideo(exercise, file, queryClient)
  }

  return (
    <section className="video-section" aria-labelledby="video-title">
      <h2 id="video-title" className="section-title">
        Відео техніки
      </h2>

      {exercise.video && (
        <>
          <VideoPlayer
            src={exercise.video.hls_url}
            poster={exercise.video.thumbnail_url}
            label={`Техніка: ${exercise.name}`}
          />
          {exercise.video.length_secs != null && (
            <p className="muted small">{formatLength(exercise.video.length_secs)}</p>
          )}
        </>
      )}

      {upload ? (
        <UploadCard upload={upload} />
      ) : (
        serverUpload && <ServerUploadNote upload={serverUpload} replacing={!!exercise.video} />
      )}

      {!exercise.video && !upload && !serverUpload && (
        <p className="muted">
          Відео ще немає. Клієнти бачитимуть вправу без відео, доки ти його не додаси.
        </p>
      )}

      {!upload && (
        <label className="button block file-button">
          <UploadIcon />
          {exercise.video ? 'Замінити відео' : 'Додати відео'}
          <input type="file" accept="video/*" onChange={pick} />
        </label>
      )}
      <p className="muted small">
        Порада: на iPhone у Параметрах → Камера → Формати обери «Найсумісніший». Такі відео
        завантажуються швидше.
      </p>
    </section>
  )
}

/** Photos per exercise, as many as clients need to recognise the setup. */
const MAX_PHOTOS = 5

function photoFailure(err: unknown): string {
  if (err instanceof ApiError && err.code === 'too_many_photos') return `До ${MAX_PHOTOS} фото на вправу.`
  if (err instanceof ApiError && err.code === 'photos_not_configured') return 'Фото ще не налаштовані.'
  return 'Не вдалося додати фото. Спробуй ще раз.'
}

/** Photos clients see under the video: the machine, the handle, the starting position. */
function PhotosSection({ exercise }: { exercise: Exercise }) {
  const { onUnauthorized } = useCoach()
  const queryClient = useQueryClient()
  const [uploading, setUploading] = useState(0)
  const [error, setError] = useState<string | null>(null)
  const key = [...EXERCISES_KEY, exercise.id]
  const slots = MAX_PHOTOS - exercise.photos.length - uploading

  const add = async (event: ChangeEvent<HTMLInputElement>) => {
    const files = [...(event.target.files ?? [])].slice(0, Math.max(0, slots))
    event.target.value = ''
    if (files.length === 0) return
    setError(null)
    setUploading((count) => count + files.length)
    for (const file of files) {
      try {
        const jpeg = await shrinkPhoto(file)
        const photo = unwrap(
          await api.POST('/coach/exercises/{id}/photos', {
            params: { path: { id: exercise.id } },
            ...jpegBody(jpeg),
          }),
        )
        queryClient.setQueryData<Exercise>(key, (current) =>
          current ? { ...current, photos: [...current.photos, photo] } : current,
        )
      } catch (err) {
        if (isUnauthorized(err)) onUnauthorized()
        setError(photoFailure(err))
      } finally {
        setUploading((count) => count - 1)
      }
    }
  }

  const remove = async (photoId: string) => {
    if (!(await confirmAction('Видалити це фото? Клієнти його більше не побачать.'))) return
    await api
      .DELETE('/coach/exercise-photos/{id}', { params: { path: { id: photoId } } })
      .catch(() => undefined)
    void queryClient.invalidateQueries({ queryKey: key })
  }

  return (
    <section className="stack" aria-labelledby="photos-title">
      <h2 id="photos-title" className="section-title">
        Фото
      </h2>
      <p className="muted small">
        Тренажер, рукоятка, стартова позиція — клієнт побачить їх під відео.
      </p>
      <PhotoGrid photos={exercise.photos} pending={uploading} onDelete={(id) => void remove(id)} />
      {slots > 0 && (
        <label className="button block file-button">
          <PhotoIcon />
          Додати фото
          <input type="file" accept="image/*" multiple onChange={(event) => void add(event)} />
        </label>
      )}
      {error && <p className="error">{error}</p>}
    </section>
  )
}

function ServerUploadNote({
  upload,
  replacing,
}: {
  upload: Schemas['VideoUpload']
  replacing: boolean
}) {
  const text = {
    uploading: 'Завантаження почалося, але не завершилось. Обери відео ще раз.',
    processing: replacing
      ? 'Нове відео обробляється, зазвичай кілька хвилин. Доти клієнти бачать попереднє.'
      : 'Відео обробляється, зазвичай кілька хвилин.',
    failed: 'Bunny не зміг обробити це відео. Спробуй інший файл.',
  }[upload.status]
  return <p className={upload.status === 'processing' ? 'notice' : 'notice danger'}>{text}</p>
}

function ArchiveButton({ exercise }: { exercise: Exercise }) {
  const { base, onUnauthorized } = useCoach()
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const archive = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.PATCH('/coach/exercises/{id}', {
          params: { path: { id: exercise.id } },
          body: { archived: true },
        }),
      ),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: EXERCISES_KEY })
      goBackTo(navigate, `${base}/exercises`)
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  const onClick = async () => {
    const confirmed = await confirmAction(
      `Прибрати «${exercise.name}» з бібліотеки? У старих тренуваннях вона залишиться.`,
    )
    if (confirmed) archive.mutate()
  }

  return (
    <>
      <button
        type="button"
        className="link-button danger"
        disabled={archive.isPending}
        onClick={() => void onClick()}
      >
        Прибрати з бібліотеки
      </button>
      {archive.isError && !isUnauthorized(archive.error) && (
        <p className="error">Не вдалося прибрати. Спробуй ще раз.</p>
      )}
    </>
  )
}
