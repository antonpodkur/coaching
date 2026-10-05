import { useState } from 'react'

import { initials } from './format'

/**
 * A client's photo in a circle, or their initials when there is none or it
 * does not load (a signed link that ran out while offline, for one).
 */
export function Avatar({
  name,
  url,
  large = false,
}: {
  name: string
  url?: string | null
  large?: boolean
}) {
  const [failed, setFailed] = useState<string | null>(null)
  const photo = url && failed !== url ? url : null
  return (
    <span className={large ? 'avatar large' : 'avatar'} aria-hidden="true">
      {photo ? <img src={photo} alt="" onError={() => setFailed(photo)} /> : initials(name)}
    </span>
  )
}
