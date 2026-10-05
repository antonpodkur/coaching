import { BottomNote } from './BottomNote'
import { applyUpdate, useUpdateReady } from './appUpdate'

/**
 * Offers a new version that has already downloaded. Logged sets and workouts
 * are saved on the phone, so reloading loses nothing.
 */
export function UpdateNote() {
  const ready = useUpdateReady()
  return (
    <BottomNote show={ready} className="update-note">
      <span>Є нова версія застосунку.</span>
      <button type="button" className="button small primary" onClick={applyUpdate}>
        Оновити
      </button>
    </BottomNote>
  )
}
