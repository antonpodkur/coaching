/** Photos are shrunk to this on the longer side: plenty to see a machine, a few hundred KB. */
const PHOTO_SIDE = 1600
const PHOTO_QUALITY = 0.85

/**
 * A photo as a JPEG at most `PHOTO_SIDE` px on the longer side. Redrawing it
 * also drops what the camera stored with it, such as the location.
 */
export async function shrinkPhoto(file: File): Promise<Blob> {
  const url = URL.createObjectURL(file)
  try {
    const image = new Image()
    image.src = url
    await image.decode()
    const scale = Math.min(1, PHOTO_SIDE / Math.max(image.naturalWidth, image.naturalHeight))
    const canvas = document.createElement('canvas')
    canvas.width = Math.round(image.naturalWidth * scale)
    canvas.height = Math.round(image.naturalHeight * scale)
    const context = canvas.getContext('2d')
    if (!context) throw new Error('no canvas')
    context.drawImage(image, 0, 0, canvas.width, canvas.height)
    return await new Promise((resolve, reject) =>
      canvas.toBlob(
        (blob) => (blob ? resolve(blob) : reject(new Error('could not encode the photo'))),
        'image/jpeg',
        PHOTO_QUALITY,
      ),
    )
  } finally {
    URL.revokeObjectURL(url)
  }
}

/** Request options that send a JPEG as the body itself, not as JSON. */
export function jpegBody(jpeg: Blob) {
  return {
    body: jpeg as unknown as string,
    bodySerializer: (body: unknown) => body,
    headers: { 'Content-Type': 'image/jpeg' },
  }
}
