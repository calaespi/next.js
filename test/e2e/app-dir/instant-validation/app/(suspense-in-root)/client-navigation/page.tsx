import Link from 'next/link'

// No instant config on this page - it's just a starting point for navigation
export default function ClientNavigationPage() {
  return (
    <main>
      <h1>Client Navigation Test</h1>
      <p>
        This page tests that validation runs during client navigation (via Link)
      </p>
      <nav>
        <ul>
          <li>
            <Link
              href="/client-navigation/target-missing-suspense"
              id="link-missing-suspense"
            >
              Go to page with missing suspense (should error)
            </Link>
          </li>
          <li>
            <Link href="/client-navigation/target-valid" id="link-valid">
              Go to valid page (should not error)
            </Link>
          </li>
        </ul>
      </nav>
    </main>
  )
}
