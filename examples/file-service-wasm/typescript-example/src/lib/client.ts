// @ts-ignore
import init, { WasmDocumentServiceClient } from '@wasm/file_service_api.js';

let wasmInitialized = false;
let clientInstance: WasmDocumentServiceClient | null = null;

export async function getClient(): Promise<WasmDocumentServiceClient> {
  if (!wasmInitialized) {
    await init();
    wasmInitialized = true;
  }
  
  if (!clientInstance) {
    // Use relative path for API calls (will be proxied by Vite in dev)
    clientInstance = new WasmDocumentServiceClient(window.location.origin);
  }
  
  return clientInstance;
}

export interface UploadResponse {
  file_id: string;
  file_name: string;
  size: number;
}