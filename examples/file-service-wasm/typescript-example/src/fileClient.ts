import { createClient } from './generated/client';
import { uploadUpload, downloadDownloadFileId, uploadUploadProfilePicture } from './generated/sdk.gen';
import type { UploadResponse } from './generated/types.gen';

// Create a client instance
const client = createClient({
  baseUrl: 'http://localhost:8080/api/documents',
});

export class FileServiceClient {
  private bearerToken?: string;

  setBearerToken(token: string) {
    this.bearerToken = token;
  }

  /**
   * Upload a file without authentication
   */
  async uploadFile(file: File): Promise<UploadResponse> {
    const { data, error } = await uploadUpload({
      client,
      body: { file },
    });

    if (error) {
      throw new Error(`Upload failed: ${error}`);
    }

    return data!;
  }

  /**
   * Upload a profile picture (requires authentication)
   */
  async uploadProfilePicture(file: File): Promise<UploadResponse> {
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
      throw new Error(`Profile picture upload failed: ${error}`);
    }

    return data!;
  }

  /**
   * Download a file by ID
   */
  async downloadFile(fileId: string): Promise<Blob> {
    const { data, error } = await downloadDownloadFileId({
      client,
      path: { file_id: fileId },
    });

    if (error) {
      throw new Error(`Download failed: ${error}`);
    }

    return data! as Blob;
  }

  /**
   * Download a file and trigger browser download
   */
  async downloadAndSave(fileId: string, filename?: string): Promise<void> {
    const blob = await this.downloadFile(fileId);
    
    // Create download link
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename || `file-${fileId}`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }
}

// Export a default instance
export const fileService = new FileServiceClient();