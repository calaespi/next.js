import ErrorWrapper from '../../catch-error-wrapper'

export default function Layout({ children }) {
  return <ErrorWrapper title="server-catch-error">{children}</ErrorWrapper>
}
