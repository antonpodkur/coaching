import type Hls from 'hls.js'
import { useEffect, useRef } from 'react'

const HLS_TYPE = 'application/vnd.apple.mpegurl'

/**
 * Plays an HLS stream from Bunny. iPhones play HLS natively; Android's WebView
 * does not, so there hls.js is loaded on demand (it is a large library).
 *
 * `autoPlay` is for a player that appears because of a tap: it starts at once,
 * so there is no second tap to aim. Where the phone refuses, the controls stay.
 */
export function VideoPlayer({
  src,
  poster,
  label,
  autoPlay = false,
}: {
  src: string
  poster?: string
  label: string
  autoPlay?: boolean
}) {
  const ref = useRef<HTMLVideoElement>(null)

  useEffect(() => {
    const video = ref.current
    if (!video) return
    const start = () => {
      if (autoPlay) video.play().catch(() => undefined)
    }
    if (video.canPlayType(HLS_TYPE)) {
      video.src = src
      start()
      return
    }
    let player: Hls | undefined
    let cancelled = false
    void import('hls.js').then(({ default: HlsPlayer }) => {
      if (cancelled) return
      if (!HlsPlayer.isSupported()) {
        video.src = src
        start()
        return
      }
      player = new HlsPlayer()
      player.on(HlsPlayer.Events.MANIFEST_PARSED, start)
      player.loadSource(src)
      player.attachMedia(video)
    })
    return () => {
      cancelled = true
      player?.destroy()
    }
  }, [src, autoPlay])

  return (
    <video
      ref={ref}
      className="video"
      controls
      playsInline
      preload="metadata"
      poster={poster}
      aria-label={label}
    />
  )
}
