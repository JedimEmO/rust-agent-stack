import { createSignal } from 'solid-js';
import { getClient } from '../lib/client';

const [token, setToken] = createSignal<string | null>(
  localStorage.getItem('auth_token')
);

export function useAuth() {
  const setAuthToken = async (newToken: string | null) => {
    setToken(newToken);
    
    if (newToken) {
      localStorage.setItem('auth_token', newToken);
    } else {
      localStorage.removeItem('auth_token');
    }
    
    const client = await getClient();
    client.set_bearer_token(newToken || undefined);
  };

  const isAuthenticated = () => token() !== null;

  return {
    token,
    setToken: setAuthToken,
    isAuthenticated,
  };
}