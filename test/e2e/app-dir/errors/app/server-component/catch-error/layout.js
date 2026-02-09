import { catchError } from 'next/navigation'
import { ErrorFallback } from './catch-error-wrapper'

// catchError can be called from server
const ErrorWrapper = catchError(ErrorFallback)

export default function Layout({ children }) {
  return <ErrorWrapper title="server-catch-error">{children}</ErrorWrapper>
}
