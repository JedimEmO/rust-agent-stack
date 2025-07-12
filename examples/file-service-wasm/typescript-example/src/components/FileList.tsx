import { createSignal, For, Show } from 'solid-js';
import { getClient } from '../lib/client';

interface FileItem {
  file_id: string;
  file_name: string;
  size: number;
  uploaded_at?: string;
}

interface FileListProps {
  files: FileItem[];
  requireAuth?: boolean;
}

export default function FileList(props: FileListProps) {
  const [downloading, setDownloading] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);

  const handleDownload = async (fileId: string, fileName: string) => {
    setDownloading(fileId);
    setError(null);

    try {
      const client = await getClient();
      // For now, all downloads use the public endpoint
      // In a real app, you'd need different endpoints for secure downloads
      const blob = await client.download(fileId);

      // Create a download link
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = fileName;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Download failed');
    } finally {
      setDownloading(null);
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / 1024 / 1024).toFixed(1) + ' MB';
  };

  return (
    <div class="w-full">
      <Show when={error()}>
        <div class="mb-4 p-3 bg-red-50 border border-red-200 rounded text-sm text-red-600">
          {error()}
        </div>
      </Show>

      <Show when={props.files.length === 0}>
        <div class="text-center py-12 text-gray-500">
          <svg
            class="w-16 h-16 mx-auto mb-4 text-gray-300"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
            />
          </svg>
          <p>No files uploaded yet</p>
        </div>
      </Show>

      <Show when={props.files.length > 0}>
        <div class="bg-white rounded-lg shadow overflow-hidden">
          <table class="min-w-full divide-y divide-gray-200">
            <thead class="bg-gray-50">
              <tr>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  File Name
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Size
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  File ID
                </th>
                <th class="relative px-6 py-3">
                  <span class="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody class="bg-white divide-y divide-gray-200">
              <For each={props.files}>
                {(file) => (
                  <tr class="hover:bg-gray-50">
                    <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                      {file.file_name}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {formatSize(file.size)}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 font-mono">
                      {file.file_id}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                      <button
                        onClick={() => handleDownload(file.file_id, file.file_name)}
                        disabled={downloading() === file.file_id}
                        class="text-blue-600 hover:text-blue-900 disabled:text-gray-400"
                      >
                        {downloading() === file.file_id ? 'Downloading...' : 'Download'}
                      </button>
                    </td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </Show>
    </div>
  );
}