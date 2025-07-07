import { createSignal } from 'solid-js';
import { useAuth } from '../stores/auth';

export default function AuthForm() {
  const { token, setToken, isAuthenticated } = useAuth();
  const [inputToken, setInputToken] = createSignal('');

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    const tokenValue = inputToken().trim();
    setToken(tokenValue || null);
    if (!tokenValue) {
      setInputToken('');
    }
  };

  const handleLogout = () => {
    setToken(null);
    setInputToken('');
  };

  if (isAuthenticated()) {
    return (
      <div class="bg-green-50 border border-green-200 rounded-lg p-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center">
            <svg
              class="w-5 h-5 text-green-600 mr-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
              />
            </svg>
            <span class="text-green-700 font-medium">Authenticated</span>
          </div>
          <button
            onClick={handleLogout}
            class="text-sm text-red-600 hover:text-red-800 font-medium"
          >
            Logout
          </button>
        </div>
        <p class="mt-2 text-xs text-gray-600 truncate">
          Token: {token()?.substring(0, 20)}...
        </p>
      </div>
    );
  }

  return (
    <form onSubmit={handleSubmit} class="bg-gray-50 rounded-lg p-4">
      <label class="block text-sm font-medium text-gray-700 mb-2">
        Authentication Token (Optional)
      </label>
      <div class="flex gap-2">
        <input
          type="text"
          value={inputToken()}
          onInput={(e) => setInputToken(e.currentTarget.value)}
          placeholder="Bearer token"
          class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <button
          type="submit"
          class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
        >
          Set Token
        </button>
      </div>
      <p class="mt-2 text-xs text-gray-500">
        Leave empty for public access or use "validtoken" for authenticated access
      </p>
    </form>
  );
}