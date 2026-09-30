import type { QueryClient } from '@tanstack/react-query'
import { useSyncExternalStore } from 'react'
import { Upload } from 'tus-js-client'

import { ApiError, api, unwrap } from '../api/client'
import { telegramWebApp } from '../app/telegram'
import { EXERCISES_KEY } from './context'

/**
 * Video uploads in flight. They live outside React so that an upload keeps
 * going while Dasha moves between screens, and every screen can show it.
 */
export interface UploadProgress {
  exerciseId: string
  exerciseName: string
  phase: 'preparing' | 'uploading' | 'finishing' | 'failed'
  /** 0 to 1. */
  progress: number
  /** Set when the server has no Bunny settings, so retrying cannot help. */
  notConfigured?: boolean
}

/** Waits between retries after a dropped connection; tus resumes where it stopped. */
const RETRY_DELAYS_MS = [0, 2_000, 5_000, 10_000, 20_000, 30_000, 60_000, 60_000, 120_000]
/** Smaller requests lose less when a gym's signal drops mid-upload. */
const CHUNK_BYTES = 8 * 1024 * 1024

const uploads = new Map<string, UploadProgress>()
/** Kept so a failed upload can be retried without picking the file again. */
const files = new Map<string, File>()
const listeners = new Set<() => void>()
let snapshot: UploadProgress[] = []

function publish() {
  snapshot = [...uploads.values()]
  for (const listener of listeners) listener()
  // Ask before Telegram closes the app in the middle of an upload.
  const webApp = telegramWebApp()
  const busy = snapshot.some((upload) => upload.phase !== 'failed')
  if (busy) webApp?.enableClosingConfirmation()
  else webApp?.disableClosingConfirmation()
}

function set(exerciseId: string, patch: Partial<UploadProgress>) {
  const current = uploads.get(exerciseId)
  if (!current) return
  uploads.set(exerciseId, { ...current, ...patch })
  publish()
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export function useUploads(): UploadProgress[] {
  return useSyncExternalStore(subscribe, () => snapshot)
}

export function dismissUpload(exerciseId: string) {
  uploads.delete(exerciseId)
  files.delete(exerciseId)
  publish()
}

export function retryUpload(exerciseId: string, queryClient: QueryClient) {
  const upload = uploads.get(exerciseId)
  const file = files.get(exerciseId)
  if (!upload || !file) return
  void uploadVideo({ id: exerciseId, name: upload.exerciseName }, file, queryClient)
}

/**
 * Uploads `file` as the exercise's video: the backend creates the video on
 * Bunny and signs the upload, the phone sends the file straight to Bunny, and
 * the backend is told when it is done so it can follow the encoding.
 */
export async function uploadVideo(
  exercise: { id: string; name: string },
  file: File,
  queryClient: QueryClient,
) {
  const running = uploads.get(exercise.id)
  if (running && running.phase !== 'failed') return
  files.set(exercise.id, file)
  uploads.set(exercise.id, {
    exerciseId: exercise.id,
    exerciseName: exercise.name,
    phase: 'preparing',
    progress: 0,
  })
  publish()

  const refresh = () => queryClient.invalidateQueries({ queryKey: EXERCISES_KEY })
  try {
    const ticket = unwrap(
      await api.POST('/coach/exercises/{id}/video-upload', {
        params: { path: { id: exercise.id } },
      }),
    )
    void refresh()
    set(exercise.id, { phase: 'uploading' })

    await new Promise<void>((resolve, reject) => {
      const upload = new Upload(file, {
        endpoint: ticket.endpoint,
        chunkSize: CHUNK_BYTES,
        retryDelays: RETRY_DELAYS_MS,
        // Each upload has its own Bunny video, so never resume into an older one.
        storeFingerprintForResuming: false,
        headers: {
          AuthorizationSignature: ticket.signature,
          AuthorizationExpire: String(ticket.expires_at),
          VideoId: ticket.video_id,
          LibraryId: ticket.library_id,
        },
        metadata: { filetype: file.type || 'video/mp4', title: exercise.name },
        onProgress: (sent, total) => set(exercise.id, { progress: total ? sent / total : 0 }),
        onError: reject,
        onSuccess: () => resolve(),
      })
      upload.start()
    })

    set(exercise.id, { phase: 'finishing', progress: 1 })
    unwrap(
      await api.POST('/coach/exercises/{id}/video-uploaded', {
        params: { path: { id: exercise.id } },
      }),
    )
    dismissUpload(exercise.id)
  } catch (err) {
    console.error('video upload failed', err)
    const notConfigured = err instanceof ApiError && err.code === 'video_not_configured'
    set(exercise.id, { phase: 'failed', notConfigured })
  } finally {
    void refresh()
  }
}
