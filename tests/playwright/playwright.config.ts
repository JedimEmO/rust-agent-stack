import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? [['github'], ['html', { open: 'never' }]] : [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: 'http://127.0.0.1',
    launchOptions: {
      slowMo: Number(process.env.PLAYWRIGHT_SLOW_MO ?? 0)
    },
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure'
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] }
    }
  ],
  webServer: [
    {
      command: 'cargo run -p playwright-rest-fixture',
      url: 'http://127.0.0.1:3101/api/v1/docs/openapi.json',
      reuseExistingServer: !process.env.CI,
      timeout: 120_000
    },
    {
      command: 'cargo run -p playwright-jsonrpc-fixture',
      url: 'http://127.0.0.1:3102/rpc/explorer/openrpc.json',
      reuseExistingServer: !process.env.CI,
      timeout: 120_000
    }
  ]
});
