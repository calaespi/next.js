import ErrorWrapper from '../../catch-error-wrapper'

export default function Layout({ children }) {
  return <ErrorWrapper title="client-catch-error">{children}</ErrorWrapper>
}
