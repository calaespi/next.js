import { nextTestSetup } from 'e2e-utils'

describe('app-dir - catchError', () => {
  const { next, isNextDev } = nextTestSetup({
    files: __dirname,
  })

  describe('client component error', () => {
    it('should catch error and render fallback with props', async () => {
      const browser = await next.browser('/client-component/catch-error')
      await browser
        .elementByCss('#error-trigger-button')
        .click()
        .waitForElementByCss('#error-boundary-message')

      if (isNextDev) {
        // In dev mode, the error overlay may appear
      } else {
        expect(
          await browser.elementByCss('#error-boundary-message').text()
        ).toBe('this is a test')
        expect(await browser.elementByCss('#error-boundary-title').text()).toBe(
          'client-catch-error'
        )
      }
    })

    it('should reset error boundary', async () => {
      const browser = await next.browser('/client-component/catch-error')

      await browser
        .elementByCss('#error-trigger-button')
        .click()
        .waitForElementByCss('#error-boundary-message')

      await browser
        .elementByCss('#reset')
        .click()
        .waitForElementByCss('#error-trigger-button')

      expect(await browser.elementByCss('#error-trigger-button').text()).toBe(
        'Trigger Error!'
      )
    })

    it('should retry and recover from client error', async () => {
      const browser = await next.browser('/client-component/catch-error')

      for (let i = 0; i < 3; i++) {
        await browser
          .elementByCss('#error-trigger-button')
          .click()
          .waitForElementByCss('#error-boundary-message')

        expect(
          await browser.elementByCss('#error-boundary-message').text()
        ).toBe('this is a test')

        await browser
          .elementByCss('#retry')
          .click()
          .waitForElementByCss('#error-trigger-button')

        expect(await browser.elementByCss('#error-trigger-button').text()).toBe(
          'Trigger Error!'
        )
      }
    })
  })

  describe('server component error', () => {
    it('should catch server component error and render fallback with props', async () => {
      const browser = await next.browser('/server-component/catch-error')

      expect(
        await browser.waitForElementByCss('#error-boundary-message').text()
      ).toBe(
        isNextDev
          ? 'this is a test'
          : 'An error occurred in the Server Components render. The specific message is omitted in production builds to avoid leaking sensitive details. A digest property is included on this error instance which may provide additional details about the nature of the error.'
      )
      expect(await browser.elementByCss('#error-boundary-title').text()).toBe(
        'server-catch-error'
      )
    })

    it('should recover server component error after retry', async () => {
      const browser = await next.browser(
        '/server-component/catch-error/recover'
      )

      expect(
        await browser.waitForElementByCss('#error-boundary-message').text()
      ).toBe(
        isNextDev
          ? 'this is a test'
          : 'An error occurred in the Server Components render. The specific message is omitted in production builds to avoid leaking sensitive details. A digest property is included on this error instance which may provide additional details about the nature of the error.'
      )

      await browser
        .elementByCss('#retry')
        .click()
        .waitForElementByCss('#recover')

      expect(await browser.elementByCss('#recover').text()).toBe('Recovered')
    })
  })
})
