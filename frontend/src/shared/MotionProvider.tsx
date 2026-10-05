import { LazyMotion, MotionConfig } from 'motion/react'
import type { ReactNode } from 'react'

import { SPRING } from './motion'

// The engine comes in its own file once the app has started; until then
// everything simply shows, unanimated.
const features = () => import('./motionFeatures').then((module) => module.default)

/** Animations for everything inside, slowed to fades where the phone asks for less motion. */
export function MotionProvider({ children }: { children: ReactNode }) {
  return (
    <LazyMotion features={features} strict>
      <MotionConfig reducedMotion="user" transition={SPRING}>
        {children}
      </MotionConfig>
    </LazyMotion>
  )
}
