import { fileService } from './fileClient';

/**
 * Example usage of the generated TypeScript file service client
 */
export async function exampleUsage() {
  try {
    // Example: Upload a file
    const fileInput = document.createElement('input');
    fileInput.type = 'file';
    
    fileInput.onchange = async (event) => {
      const target = event.target as HTMLInputElement;
      const file = target.files?.[0];
      
      if (!file) return;

      console.log('Uploading file:', file.name);
      
      // Upload file (no auth required)
      const uploadResult = await fileService.uploadFile(file);
      console.log('Upload successful:', uploadResult);
      
      // Download the same file
      console.log('Downloading file:', uploadResult.file_id);
      await fileService.downloadAndSave(uploadResult.file_id, uploadResult.file_name);
      
      // Example with authentication
      fileService.setBearerToken('your-jwt-token-here');
      
      // Upload profile picture (requires auth)
      try {
        const profileResult = await fileService.uploadProfilePicture(file);
        console.log('Profile picture upload successful:', profileResult);
      } catch (error) {
        console.error('Profile upload failed (expected without valid token):', error);
      }
    };
    
    // Trigger file picker
    fileInput.click();
    
  } catch (error) {
    console.error('File operation failed:', error);
  }
}

// Auto-run example if in browser
if (typeof window !== 'undefined') {
  window.addEventListener('load', () => {
    console.log('File service client ready! Call exampleUsage() to test.');
    // Uncomment to auto-run:
    // exampleUsage();
  });
}