import type { QueryClient } from '@tanstack/react-query'
import { useSyncExternalStore } from 'react'
import { Upload } from 'tus-js-client'

import { ApiError, type Schemas, api, unwrap } from '../api/client'
import { holdClosing } from '../shared/closingGuard'
import { MY_WORKOUTS_KEY } from './workouts'

/**
 * Videos a client is sending Dasha. They live outside React, so a send keeps
 * going while the client moves on to the next exercise.
 */
export interface VideoSend {
  key: string
  /** The `key` of where it goes, e.g. one exercise of a workout, or the gym. */
  target: string
  phase: 'checking' | 'preparing' | 'uploading' | 'finishing' | 'failed'
  /** 0 to 1. */
  progress: number
  /** Why it failed, in words for the client. */
  error?: string
}

/** Where a video goes, and the backend calls that get it there. */
export interface VideoTarget {
  /** Sends are listed under it, e.g. `exercise:<id>` or `gym`. */
  key: string
  /** Creates the video on Bunny and signs its upload. */
  start: () => Promise<Schemas['FormVideoUpload']>
  /** Tells the backend the file is in. */
  uploaded: (id: string) => Promise<unknown>
  /** Frees the slot after a failed send. */
  remove: (id: string) => Promise<unknown>
  /** Refetches the list the video shows up in. */
  refresh: (queryClient: QueryClient) => Promise<void>
}

/** A technique video under one exercise of a workout. */
export function exerciseTarget(workoutId: string, workoutExerciseId: string): VideoTarget {
  return {
    key: `exercise:${workoutExerciseId}`,
    start: async () =>
      unwrap(
        await api.POST('/workout-exercises/{id}/videos', {
          params: { path: { id: workoutExerciseId } },
        }),
      ),
    uploaded: async (id) =>
      unwrap(await api.POST('/form-videos/{id}/uploaded', { params: { path: { id } } })),
    remove: (id) => api.DELETE('/form-videos/{id}', { params: { path: { id } } }),
    refresh: (queryClient) =>
      queryClient.invalidateQueries({ queryKey: [...MY_WORKOUTS_KEY, workoutId] }),
  }
}

/** Longer videos are refused before uploading; Dasha needs a set, not a session. */
export const MAX_VIDEO_SECS = 180
/** Waits between retries after a dropped connection; tus resumes where it stopped. */
const RETRY_DELAYS_MS = [0, 2_000, 5_000, 10_000, 20_000, 30_000, 60_000, 60_000, 120_000]
const CHUNK_BYTES = 8 * 1024 * 1024

const sends = new Map<string, VideoSend>()
const listeners = new Set<() => void>()
let snapshot: VideoSend[] = []

function publish() {
  snapshot = [...sends.values()]
  for (const listener of listeners) listener()
  holdClosing(
    'video-send',
    snapshot.some((send) => send.phase !== 'failed'),
  )
}

function set(key: string, patch: Partial<VideoSend>) {
  const current = sends.get(key)
  if (!current) return
  sends.set(key, { ...current, ...patch })
  publish()
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export function useVideoSends(): VideoSend[] {
  return useSyncExternalStore(subscribe, () => snapshot)
}

export function dismissSend(key: string) {
  sends.delete(key)
  publish()
}

/** The file's length in seconds, or `null` if the phone cannot tell. */
function videoLength(file: File): Promise<number | null> {
  return new Promise((resolve) => {
    const url = URL.createObjectURL(file)
    const video = document.createElement('video')
    const done = (secs: number | null) => {
      URL.revokeObjectURL(url)
      resolve(secs)
    }
    const timer = setTimeout(() => done(null), 5_000)
    video.preload = 'metadata'
    video.onloadedmetadata = () => {
      clearTimeout(timer)
      done(Number.isFinite(video.duration) ? video.duration : null)
    }
    video.onerror = () => {
      clearTimeout(timer)
      done(null)
    }
    video.src = url
  })
}

function failureText(err: unknown): string {
  if (err instanceof ApiError && err.code === 'too_many_videos') {
    return 'Можна надіслати до 3 відео.'
  }
  if (err instanceof ApiError && err.code === 'client_videos_not_configured') {
    return 'Надсилання відео ще не налаштоване.'
  }
  return 'Не вдалося надіслати. Перевір зв’язок і спробуй ще раз.'
}

/**
 * Sends `file` to Dasha: the backend creates the video in the private library
 * and signs the upload, the phone sends the file straight to Bunny, and the
 * backend is told when it is in.
 */
export async function sendVideo(target: VideoTarget, file: File, queryClient: QueryClient) {
  const key = crypto.randomUUID()
  sends.set(key, { key, target: target.key, phase: 'checking', progress: 0 })
  publish()
  const refresh = () => target.refresh(queryClient)

  const secs = await videoLength(file)
  if (secs !== null && secs > MAX_VIDEO_SECS) {
    set(key, {
      phase: 'failed',
      error: 'Відео довше за 3 хвилини. Обріж його в галереї й надішли ще раз.',
    })
    return
  }

  let videoId: string | null = null
  try {
    set(key, { phase: 'preparing' })
    const started = await target.start()
    videoId = started.id
    const { ticket } = started
    set(key, { phase: 'uploading' })

    await new Promise<void>((resolve, reject) => {
      const upload = new Upload(file, {
        endpoint: ticket.endpoint,
        chunkSize: CHUNK_BYTES,
        retryDelays: RETRY_DELAYS_MS,
        // Each send has its own Bunny video, so never resume into an older one.
        storeFingerprintForResuming: false,
        headers: {
          AuthorizationSignature: ticket.signature,
          AuthorizationExpire: String(ticket.expires_at),
          VideoId: ticket.video_id,
          LibraryId: ticket.library_id,
        },
        metadata: { filetype: file.type || 'video/mp4', title: started.title },
        onProgress: (sent, total) => set(key, { progress: total ? sent / total : 0 }),
        onError: reject,
        onSuccess: () => resolve(),
      })
      upload.start()
    })

    set(key, { phase: 'finishing', progress: 1 })
    await target.uploaded(videoId)
  } catch (err) {
    console.error('video send failed', err)
    // Free the slot, so trying again does not count against the three.
    if (videoId) await target.remove(videoId).catch(() => undefined)
    set(key, { phase: 'failed', error: failureText(err) })
    void refresh()
    return
  }
  // The card stays until the list has the video, so the buttons below do not
  // jump up and back down in between.
  await refresh()
  dismissSend(key)
}
