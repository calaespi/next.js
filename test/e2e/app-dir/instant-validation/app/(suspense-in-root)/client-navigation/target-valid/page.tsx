import { connection } from 'next/server'
import { Suspense } from 'react'

export const unstable_instant = { prefetch: 'static' }

export default async function Page() {
  return (
    <main id="target-valid">
      <p>This page properly wraps dynamic content in Suspense.</p>
      <Suspense fallback={<div>Loading...</div>}>
        <Dynamic />
      </Suspense>
    </main>
  )
}

async function Dynamic() {
  await connection()
  return <div id="dynamic-content">Dynamic content</div>
}
