import { connection } from 'next/server'

export const unstable_instant = { prefetch: 'static' }

export default async function Page() {
  await connection()
  return (
    <main id="target-missing-suspense">
      <p>
        This page has dynamic content without a Suspense boundary. Validation
        should catch this during client navigation.
      </p>
    </main>
  )
}
