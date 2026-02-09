import React, { type JSX } from 'react'
import { CatchErrorBoundary } from './catch-error-boundary'
import type { ErrorInfo } from './error-boundary'

type FallbackComponent<P> = (
  props: P & { children?: React.ReactNode },
  errorInfo: ErrorInfo
) => React.ReactNode

export function catchError<P extends Record<string, any>>(
  fallback: FallbackComponent<P>
): React.ComponentType<P & { children?: React.ReactNode }> {
  function CatchErrorWrapper(
    props: P & { children?: React.ReactNode }
  ): JSX.Element {
    const { children, ...componentProps } = props

    return (
      <CatchErrorBoundary
        fallback={fallback}
        componentProps={componentProps as P}
      >
        {children}
      </CatchErrorBoundary>
    )
  }

  if (process.env.NODE_ENV !== 'production') {
    const name = fallback.name || 'Unknown'
    CatchErrorWrapper.displayName = `catchError(${name})`
  }

  return CatchErrorWrapper
}
