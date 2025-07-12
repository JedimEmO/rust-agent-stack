import { createClient } from '../generated/client';
import { uploadUpload, downloadDownloadFileId, uploadUploadProfilePicture } from '../generated/sdk.gen';
import type { UploadResponse as GeneratedUploadResponse } from '../generated/types.gen';

// Re-export the generated type
export type UploadResponse = GeneratedUploadResponse;

// Create the client instance
const client = createClient({
  baseUrl: `${window.location.origin}/api/documents`,
});

// Simple client wrapper that matches the WASM client interface
export class DocumentServiceClient {
  private bearerToken?: string;

  setBearerToken(token: string) {
    this.bearerToken = token;
  }

  async upload(file: File): Promise<UploadResponse> {
    const { data, error } = await uploadUpload({
      client,
      body: { file },
    });

    if (error) {
      throw new Error(`Upload failed: ${JSON.stringify(error)}`);
    }

    return data!;
  }

  async upload_profile_picture(file: File): Promise<UploadResponse> {
    if (!this.bearerToken) {
      throw new Error('Authentication required for profile picture upload');
    }

    const { data, error } = await uploadUploadProfilePicture({
      client,
      body: { file },
      headers: {
        Authorization: `Bearer ${this.bearerToken}`,
      },
    });

    if (error) {
      throw new Error(`Profile picture upload failed: ${JSON.stringify(error)}`);
    }

    return data!;
  }

  async download(fileId: string): Promise<Blob> {
    const { data, error } = await downloadDownloadFileId({
      client,
      path: { file_id: fileId },
    });

    if (error) {
      throw new Error(`Download failed: ${JSON.stringify(error)}`);
    }

    return data! as Blob;
  }
}

// Global client instance
let clientInstance: DocumentServiceClient | null = null;

export async function getClient(): Promise<DocumentServiceClient> {
  if (!clientInstance) {
    clientInstance = new DocumentServiceClient();
  }
  
  return clientInstance;
}