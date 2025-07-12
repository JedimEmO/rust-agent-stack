import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  client: '@hey-api/client-fetch',
  input: '../rest-backend/target/openapi/userservice.json',
  output: './src/generated',
  schemas: {
    export: true,
  },
});