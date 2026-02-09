'use client'

import { catchError } from 'next/error'

function ErrorFallback(props, { error, reset, retry }) {
  return (
    <>
      <p id="error-boundary-message">{error.message}</p>
      <p id="error-boundary-title">{props.title}</p>
      <button id="reset" onClick={() => reset()}>
        Reset
      </button>
      <button id="retry" onClick={() => retry()}>
        Retry
      </button>
    </>
  )
}

export default catchError(ErrorFallback)
