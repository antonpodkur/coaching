import type Hls from 'hls.js'
import { useEffect, useRef } from 'react'

const HLS_TYPE = 'application/vnd.apple.mpegurl'

/**
 * Plays an HLS stream from Bunny. iPhones play HLS natively; Android's WebView
 * does not, so there hls.js is loaded on demand (it is a large library).
 */
export function VideoPlayer({ src, poster, label }: { src: string; poster?: string; label: string }) {
  const ref = useRef<HTMLVideoElement>(null)

  useEffect(() => {
    const video = ref.current
    if (!video) return
    if (video.canPlayType(HLS_TYPE)) {
      video.src = src
      return
    }
    let player: Hls | undefined
    let cancelled = false
    void import('hls.js').then(({ default: HlsPlayer }) => {
      if (cancelled) return
      if (!HlsPlayer.isSupported()) {
        video.src = src
        return
      }
      player = new HlsPlayer()
      player.loadSource(src)
      player.attachMedia(video)
    })
    return () => {
      cancelled = true
      player?.destroy()
    }
  }, [src])

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
