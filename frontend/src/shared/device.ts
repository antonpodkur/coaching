/** A touch screen: a phone or a tablet rather than a computer. */
export function onPhone(): boolean {
  return window.matchMedia('(pointer: coarse)').matches
}

/** Installed on an iPhone's or iPad's home screen, where there is no swipe back. */
export function installedOnIphone(): boolean {
  return (navigator as Navigator & { standalone?: boolean }).standalone === true
}
