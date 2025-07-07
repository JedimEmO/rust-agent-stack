import { createSignal, Show } from 'solid-js';
import { getClient, type UploadResponse } from '../lib/client';

interface FileUploadProps {
  onUploadSuccess?: (response: UploadResponse) => void;
  requireAuth?: boolean;
}

export default function FileUpload(props: FileUploadProps) {
  const [file, setFile] = createSignal<File | null>(null);
  const [uploading, setUploading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [dragActive, setDragActive] = createSignal(false);

  const handleDrag = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === "dragenter" || e.type === "dragover") {
      setDragActive(true);
    } else if (e.type === "dragleave") {
      setDragActive(false);
    }
  };

  const handleDrop = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
    
    if (e.dataTransfer?.files && e.dataTransfer.files[0]) {
      setFile(e.dataTransfer.files[0]);
      setError(null);
    }
  };

  const handleFileSelect = (e: Event) => {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files[0]) {
      setFile(target.files[0]);
      setError(null);
    }
  };

  const handleUpload = async () => {
    const selectedFile = file();
    if (!selectedFile) return;

    setUploading(true);
    setError(null);

    try {
      const client = await getClient();
      const response = props.requireAuth 
        ? await client.upload_profile_picture(selectedFile)
        : await client.upload(selectedFile);
      
      props.onUploadSuccess?.(response as UploadResponse);
      setFile(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    } finally {
      setUploading(false);
    }
  };

  return (
    <div class="w-full max-w-md mx-auto">
      <div
        class={`relative border-2 border-dashed rounded-lg p-6 transition-colors ${
          dragActive() ? 'border-blue-500 bg-blue-50' : 'border-gray-300'
        }`}
        onDragEnter={handleDrag}
        onDragLeave={handleDrag}
        onDragOver={handleDrag}
        onDrop={handleDrop}
      >
        <input
          type="file"
          id="file-input"
          class="hidden"
          onChange={handleFileSelect}
          accept="*/*"
        />
        
        <label
          for="file-input"
          class="flex flex-col items-center cursor-pointer"
        >
          <svg
            class="w-12 h-12 text-gray-400 mb-3"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
            />
          </svg>
          
          <p class="text-sm text-gray-600 text-center">
            <span class="font-semibold">Click to upload</span> or drag and drop
          </p>
          <p class="text-xs text-gray-500 mt-1">Any file type</p>
        </label>

        <Show when={file()}>
          <div class="mt-4 p-3 bg-gray-50 rounded">
            <p class="text-sm font-medium text-gray-700 truncate">
              {file()!.name}
            </p>
            <p class="text-xs text-gray-500">
              {(file()!.size / 1024 / 1024).toFixed(2)} MB
            </p>
          </div>
        </Show>
      </div>

      <Show when={error()}>
        <div class="mt-3 p-3 bg-red-50 border border-red-200 rounded text-sm text-red-600">
          {error()}
        </div>
      </Show>

      <button
        onClick={handleUpload}
        disabled={!file() || uploading()}
        class={`mt-4 w-full py-2 px-4 rounded font-medium transition-colors ${
          file() && !uploading()
            ? 'bg-blue-600 hover:bg-blue-700 text-white'
            : 'bg-gray-200 text-gray-400 cursor-not-allowed'
        }`}
      >
        {uploading() ? 'Uploading...' : 'Upload File'}
      </button>
    </div>
  );
}