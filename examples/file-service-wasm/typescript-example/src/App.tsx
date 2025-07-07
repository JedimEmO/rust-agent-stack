import { createSignal, onMount, Show } from 'solid-js';
import FileUpload from './components/FileUpload';
import FileList from './components/FileList';
import AuthForm from './components/AuthForm';
import { useAuth } from './stores/auth';
import { getClient, type UploadResponse } from './lib/client';

function App() {
  const { isAuthenticated } = useAuth();
  const [files, setFiles] = createSignal<UploadResponse[]>([]);
  const [activeTab, setActiveTab] = createSignal<'public' | 'secure'>('public');
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      // Initialize WASM client
      await getClient();
      setLoading(false);
    } catch (err) {
      setError('Failed to initialize WASM client');
      setLoading(false);
    }
  });

  const handleUploadSuccess = (response: UploadResponse) => {
    setFiles((prev) => [...prev, response]);
  };

  const tabClass = (tab: 'public' | 'secure') =>
    `px-4 py-2 font-medium rounded-t-lg transition-colors ${
      activeTab() === tab
        ? 'bg-white text-blue-600 border-b-2 border-blue-600'
        : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
    }`;

  return (
    <div class="min-h-screen bg-gray-100">
      {/* Header */}
      <header class="bg-white shadow-sm">
        <div class="max-w-6xl mx-auto px-4 py-4">
          <div class="flex items-center justify-between">
            <h1 class="text-2xl font-bold text-gray-900">
              File Service Demo
            </h1>
            <div class="text-sm text-gray-500">
              SolidJS + WASM + Rust
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main class="max-w-6xl mx-auto px-4 py-8">
        <Show when={loading()}>
          <div class="flex items-center justify-center py-12">
            <div class="text-center">
              <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
              <p class="mt-4 text-gray-600">Initializing WASM client...</p>
            </div>
          </div>
        </Show>

        <Show when={error()}>
          <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
            <p class="text-red-600">{error()}</p>
          </div>
        </Show>

        <Show when={!loading() && !error()}>
          <div class="space-y-8">
            {/* Authentication Section */}
            <section>
              <h2 class="text-lg font-semibold text-gray-900 mb-4">
                Authentication
              </h2>
              <AuthForm />
            </section>

            {/* Upload Section */}
            <section>
              <h2 class="text-lg font-semibold text-gray-900 mb-4">
                File Upload
              </h2>
              
              <div class="mb-4">
                <div class="flex gap-2">
                  <button
                    onClick={() => setActiveTab('public')}
                    class={tabClass('public')}
                  >
                    Public Upload
                  </button>
                  <button
                    onClick={() => setActiveTab('secure')}
                    class={tabClass('secure')}
                    disabled={!isAuthenticated()}
                  >
                    Secure Upload
                    {!isAuthenticated() && (
                      <span class="ml-1 text-xs">(Auth Required)</span>
                    )}
                  </button>
                </div>
              </div>

              <div class="bg-white rounded-lg shadow p-6">
                <FileUpload
                  onUploadSuccess={handleUploadSuccess}
                  requireAuth={activeTab() === 'secure'}
                />
              </div>
            </section>

            {/* Files Section */}
            <section>
              <h2 class="text-lg font-semibold text-gray-900 mb-4">
                Uploaded Files
              </h2>
              <FileList
                files={files()}
                requireAuth={activeTab() === 'secure'}
              />
            </section>
          </div>
        </Show>
      </main>

      {/* Footer */}
      <footer class="mt-16 py-6 bg-white border-t">
        <div class="max-w-6xl mx-auto px-4 text-center text-sm text-gray-500">
          <p>
            Built with{' '}
            <a
              href="https://www.solidjs.com/"
              target="_blank"
              class="text-blue-600 hover:underline"
            >
              SolidJS
            </a>
            ,{' '}
            <a
              href="https://rustwasm.github.io/"
              target="_blank"
              class="text-blue-600 hover:underline"
            >
              WebAssembly
            </a>
            , and{' '}
            <a
              href="https://www.rust-lang.org/"
              target="_blank"
              class="text-blue-600 hover:underline"
            >
              Rust
            </a>
          </p>
        </div>
      </footer>
    </div>
  );
}

export default App;