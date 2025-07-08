import { render } from 'solid-js/web';
import { createSignal, onMount, Show } from 'solid-js';
// @ts-ignore
import init, { WasmUserServiceClient } from '@wasm/rest_api.js';

interface User {
  id: string;
  name: string;
  email: string;
  role: string;
}

function App() {
  const [client, setClient] = createSignal<WasmUserServiceClient | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [response, setResponse] = createSignal<any>(null);
  const [token, setToken] = createSignal<string>('');

  onMount(async () => {
    try {
      // Initialize WASM module
      await init();
      
      // Create client instance
      const wasmClient = new WasmUserServiceClient(window.location.origin);
      setClient(wasmClient);
      setLoading(false);
    } catch (err) {
      setError(`Failed to initialize WASM client: ${err}`);
      setLoading(false);
    }
  });

  const handleGetUsers = async () => {
    const c = client();
    if (!c) return;

    try {
      setError(null);
      const result = await c.get_users();
      setResponse(result);
    } catch (err) {
      setError(`Error: ${err}`);
      setResponse(null);
    }
  };

  const handleGetUser = async () => {
    const c = client();
    if (!c) return;

    try {
      setError(null);
      const result = await c.get_users_by_id('1');
      setResponse(result);
    } catch (err) {
      setError(`Error: ${err}`);
      setResponse(null);
    }
  };

  const handleCreateUser = async () => {
    const c = client();
    if (!c) return;

    try {
      setError(null);
      const newUser = {
        name: 'New User',
        email: 'newuser@example.com'
      };
      const result = await c.post_users(newUser);
      setResponse(result);
    } catch (err) {
      setError(`Error: ${err}`);
      setResponse(null);
    }
  };

  const handleSetToken = () => {
    const c = client();
    if (!c) return;

    c.set_bearer_token(token() || null);
    setResponse({ message: 'Token set successfully' });
  };

  const setUserToken = () => {
    setToken('validtoken');
    const c = client();
    if (c) {
      c.set_bearer_token('validtoken');
      setResponse({ message: 'User token set successfully' });
    }
  };

  const setAdminToken = () => {
    setToken('admintoken');
    const c = client();
    if (c) {
      c.set_bearer_token('admintoken');
      setResponse({ message: 'Admin token set successfully' });
    }
  };

  return (
    <div class="container">
      <h1>REST WASM Client Example</h1>

      <Show when={loading()}>
        <p class="loading">Loading WASM client...</p>
      </Show>

      <Show when={error()}>
        <div class="response error">{error()}</div>
      </Show>

      <Show when={!loading() && client()}>
        <div class="section">
          <h2>Authentication</h2>
          <input
            type="text"
            placeholder="Bearer token"
            value={token()}
            onInput={(e) => setToken(e.currentTarget.value)}
            style="padding: 8px; margin-right: 10px; border: 1px solid #ddd; border-radius: 4px;"
          />
          <button onClick={handleSetToken}>Set Token</button>
          <div style="margin-top: 10px;">
            <button onClick={setUserToken} style="background: #28a745;">Set User Token</button>
            <button onClick={setAdminToken} style="background: #dc3545;">Set Admin Token</button>
          </div>
          <p style="color: #666; font-size: 14px; margin-top: 10px;">
            Use "validtoken" for user access or "admintoken" for admin access
          </p>
        </div>

        <div class="section">
          <h2>Public Endpoints</h2>
          <button onClick={handleGetUsers}>Get All Users</button>
          <button onClick={handleGetUser}>Get User by ID</button>
        </div>

        <div class="section">
          <h2>Protected Endpoints (Requires Auth)</h2>
          <button onClick={handleCreateUser}>Create User (Admin Only)</button>
        </div>

        <Show when={response()}>
          <div class="section">
            <h3>Response:</h3>
            <div class="response">
              {JSON.stringify(response(), null, 2)}
            </div>
          </div>
        </Show>
      </Show>
    </div>
  );
}

const root = document.getElementById('app');
if (root) {
  render(() => <App />, root);
}