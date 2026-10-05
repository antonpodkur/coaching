import type { Transition } from 'motion/react'

/**
 * How things move in the app: small and quick, settling like a spring rather
 * than stopping dead. Mostly only how big something looks and how see-through
 * it is change, so nothing moves under a finger; a card folding away after a
 * tap on it is the exception. Phones set to reduce motion get fades only.
 */

/** Most movement: quick, settled, no visible bounce. */
export const SPRING: Transition = { type: 'spring', stiffness: 520, damping: 40 }
/** Things that pop, like a ticked set: a small overshoot. */
export const POP: Transition = { type: 'spring', stiffness: 700, damping: 17 }
/** iPhone's own easing, for things that slide a long way. */
export const EASE_IOS = [0.32, 0.72, 0, 1] as const
