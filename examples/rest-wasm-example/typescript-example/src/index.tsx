import { render } from 'solid-js/web';
import { createSignal, Show, For } from 'solid-js';
import * as api from './generated';
import type { User, UsersResponse, CreateUserRequest, Task, TasksResponse, CreateTaskRequest } from './generated';

// API configuration
const API_BASE_URL = window.location.origin + '/api/v1';

function App() {
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [users, setUsers] = createSignal<User[]>([]);
  const [selectedUser, setSelectedUser] = createSignal<User | null>(null);
  const [tasks, setTasks] = createSignal<Task[]>([]);
  const [token, setToken] = createSignal<string>('');

  // Get auth headers
  const getAuthHeaders = () => {
    const tokenValue = token();
    if (tokenValue) {
      return { Authorization: `Bearer ${tokenValue}` };
    }
    return {};
  };

  // Get all users (public endpoint)
  const handleGetUsers = async () => {
    try {
      setLoading(true);
      setError(null);
      
      const response = await api.getUsers({
        baseUrl: API_BASE_URL,
      });
      
      if (response.data) {
        setUsers(response.data.users);
      } else if (response.error) {
        setError(`Error: ${response.error.error || 'Unknown error'}`);
      }
    } catch (err) {
      setError(`Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  // Get specific user (public endpoint)
  const handleGetUser = async (userId: string) => {
    try {
      setLoading(true);
      setError(null);
      
      const response = await api.getUsersId({
        baseUrl: API_BASE_URL,
        path: { id: userId },
      });
      
      if (response.data) {
        setSelectedUser(response.data);
      } else if (response.error) {
        setError(`Error: ${response.error.error || 'Unknown error'}`);
      }
    } catch (err) {
      setError(`Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  // Create user (admin endpoint)
  const handleCreateUser = async () => {
    try {
      setLoading(true);
      setError(null);
      
      const newUser: CreateUserRequest = {
        name: 'New User',
        email: 'newuser@example.com',
      };
      
      const response = await api.postUsers({
        baseUrl: API_BASE_URL,
        headers: getAuthHeaders(),
        body: newUser,
      });
      
      if (response.data) {
        // Refresh users list
        await handleGetUsers();
      } else if (response.error) {
        setError(`Error: ${response.error.error || 'Unauthorized'}`);
      }
    } catch (err) {
      setError(`Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  // Get user tasks (user endpoint)
  const handleGetUserTasks = async (userId: string) => {
    try {
      setLoading(true);
      setError(null);
      
      const response = await api.getUsersUserIdTasks({
        baseUrl: API_BASE_URL,
        headers: getAuthHeaders(),
        path: { user_id: userId },
      });
      
      if (response.data) {
        setTasks(response.data.tasks);
      } else if (response.error) {
        setError(`Error: ${response.error.error || 'Unauthorized'}`);
      }
    } catch (err) {
      setError(`Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: '20px', "font-family": 'Arial, sans-serif' }}>
      <h1>REST API TypeScript Client Demo</h1>
      <p>This demo uses a TypeScript client generated from the OpenAPI specification.</p>
      
      <div style={{ "margin-bottom": '20px' }}>
        <h3>Authentication</h3>
        <div style={{ "margin-bottom": '10px' }}>
          <button onClick={() => setToken('validtoken')} style={{ "margin-right": '10px' }}>
            Use User Token
          </button>
          <button onClick={() => setToken('admintoken')} style={{ "margin-right": '10px' }}>
            Use Admin Token
          </button>
          <button onClick={() => setToken('')}>
            Clear Token
          </button>
        </div>
        <div style={{ "font-size": '14px', color: '#666' }}>
          Current token: {token() ? `"${token()}"` : 'None'}
        </div>
      </div>

      <div style={{ "margin-bottom": '20px' }}>
        <h3>Public Endpoints</h3>
        <button onClick={handleGetUsers} disabled={loading()}>
          Get All Users
        </button>
      </div>

      <div style={{ "margin-bottom": '20px' }}>
        <h3>Admin Endpoints</h3>
        <button onClick={handleCreateUser} disabled={loading()}>
          Create User (requires admin token)
        </button>
      </div>

      <Show when={error()}>
        <div style={{ color: 'red', "margin-bottom": '20px' }}>
          {error()}
        </div>
      </Show>

      <Show when={loading()}>
        <div>Loading...</div>
      </Show>

      <Show when={users().length > 0}>
        <div style={{ "margin-bottom": '20px' }}>
          <h3>Users</h3>
          <table style={{ "border-collapse": 'collapse', width: '100%' }}>
            <thead>
              <tr>
                <th style={{ border: '1px solid #ddd', padding: '8px' }}>ID</th>
                <th style={{ border: '1px solid #ddd', padding: '8px' }}>Name</th>
                <th style={{ border: '1px solid #ddd', padding: '8px' }}>Email</th>
                <th style={{ border: '1px solid #ddd', padding: '8px' }}>Role</th>
                <th style={{ border: '1px solid #ddd', padding: '8px' }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              <For each={users()}>
                {(user) => (
                  <tr>
                    <td style={{ border: '1px solid #ddd', padding: '8px' }}>{user.id}</td>
                    <td style={{ border: '1px solid #ddd', padding: '8px' }}>{user.name}</td>
                    <td style={{ border: '1px solid #ddd', padding: '8px' }}>{user.email}</td>
                    <td style={{ border: '1px solid #ddd', padding: '8px' }}>{user.role}</td>
                    <td style={{ border: '1px solid #ddd', padding: '8px' }}>
                      <button onClick={() => handleGetUser(user.id)} style={{ "margin-right": '5px' }}>
                        View
                      </button>
                      <button onClick={() => handleGetUserTasks(user.id)}>
                        View Tasks
                      </button>
                    </td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </Show>

      <Show when={selectedUser()}>
        <div style={{ "margin-bottom": '20px' }}>
          <h3>Selected User</h3>
          <pre>{JSON.stringify(selectedUser(), null, 2)}</pre>
        </div>
      </Show>

      <Show when={tasks().length > 0}>
        <div>
          <h3>Tasks</h3>
          <table style={{ "border-collapse": 'collapse', width: '100%' }}>
            <thead>
              <tr>
                <th style={{ border: '1px solid #ddd', padding: '8px' }}>ID</th>
                <th style={{ border: '1px solid #ddd', padding: '8px' }}>Title</th>
                <th style={{ border: '1px solid #ddd', padding: '8px' }}>Description</th>
                <th style={{ border: '1px solid #ddd', padding: '8px' }}>Completed</th>
              </tr>
            </thead>
            <tbody>
              <For each={tasks()}>
                {(task) => (
                  <tr>
                    <td style={{ border: '1px solid #ddd', padding: '8px' }}>{task.id}</td>
                    <td style={{ border: '1px solid #ddd', padding: '8px' }}>{task.title}</td>
                    <td style={{ border: '1px solid #ddd', padding: '8px' }}>{task.description}</td>
                    <td style={{ border: '1px solid #ddd', padding: '8px' }}>
                      {task.completed ? '✓' : '✗'}
                    </td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </Show>

      <div style={{ "margin-top": '40px', "font-size": '14px', color: '#666' }}>
        <h4>Features:</h4>
        <ul>
          <li>✅ Fully type-safe client generated from OpenAPI spec</li>
          <li>✅ Automatic type inference for requests and responses</li>
          <li>✅ Built-in error handling with typed error responses</li>
          <li>✅ Auto-completion and IntelliSense support</li>
          <li>✅ No manual type definitions needed</li>
          <li>✅ Automatic client regeneration via Vite plugin</li>
        </ul>
      </div>
    </div>
  );
}

render(() => <App />, document.getElementById('app')!);