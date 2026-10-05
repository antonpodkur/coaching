/** A touch screen: a phone or a tablet rather than a computer. */
export function onPhone(): boolean {
  return window.matchMedia('(pointer: coarse)').matches
}

/** Installed on an iPhone's or iPad's home screen, where there is no swipe back. */
export function installedOnIphone(): boolean {
  return (navigator as Navigator & { standalone?: boolean }).standalone === true
}

/** Running as the app installed on a home screen (or a computer), not in a browser tab. */
export function installedApp(): boolean {
  return window.matchMedia('(display-mode: standalone)').matches || installedOnIphone()
}

/** Which phone's install steps apply here. */
export function phoneKind(): 'iphone' | 'android' | 'other' {
  const agent = navigator.userAgent
  // iPads ask for the computer version of sites and say they are Macs.
  const iPad = /Macintosh/.test(agent) && navigator.maxTouchPoints > 1
  if (/iPhone|iPad|iPod/.test(agent) || iPad) return 'iphone'
  if (/Android/.test(agent)) return 'android'
  return 'other'
}

/** A browser on an iPhone other than Safari, whose steps differ from Safari's. */
export function otherBrowserOnIphone(): boolean {
  return /CriOS|FxiOS|EdgiOS|OPiOS|YaBrowser|GSA\//.test(navigator.userAgent)
}
