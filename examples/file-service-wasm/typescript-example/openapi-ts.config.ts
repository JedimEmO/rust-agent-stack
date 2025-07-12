import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  client: 'fetch',
  input: '../file-service-api/target/openapi/documentservice.json',
  output: './src/generated',
  schemas: {
    export: true,
  },
});