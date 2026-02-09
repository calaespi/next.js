export function catchError(): never {
  throw new Error(
    '`catchError` can only be used in Client Components. Add the "use client" directive to use it.'
  )
}
