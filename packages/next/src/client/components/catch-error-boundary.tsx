'use client'

import React, { type JSX } from 'react'
import {
  ErrorBoundary,
  type ErrorComponent,
  type ErrorInfo,
} from './error-boundary'

type FallbackComponent<P> = (
  props: P & { children?: React.ReactNode },
  errorInfo: ErrorInfo
) => React.ReactNode

export function CatchErrorBoundary<P extends Record<string, any>>({
  fallback,
  componentProps,
  children,
}: {
  fallback: FallbackComponent<P>
  componentProps: P
  children?: React.ReactNode
}): JSX.Element {
  const errorComponent: ErrorComponent = (errorInfo) =>
    fallback(componentProps as P, errorInfo)

  return (
    <ErrorBoundary errorComponent={errorComponent}>{children}</ErrorBoundary>
  )
}
