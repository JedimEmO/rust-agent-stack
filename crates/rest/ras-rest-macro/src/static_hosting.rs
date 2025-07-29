//! Static file hosting module for REST services
//!
//! This module provides functionality to serve static files, particularly for API documentation
//! and OpenAPI spec hosting.

use crate::ServiceDefinition;
use proc_macro2::TokenStream;
use quote::quote;

/// Configuration for static file hosting
#[derive(Debug, Clone)]
pub struct StaticHostingConfig {
    /// Whether to enable static hosting
    pub serve_docs: bool,
    /// URL path for documentation (default "/docs")
    pub docs_path: String,
    /// UI theme selection (default "default")
    pub ui_theme: String,
}

impl Default for StaticHostingConfig {
    fn default() -> Self {
        Self {
            serve_docs: false,
            docs_path: "/docs".to_string(),
            ui_theme: "default".to_string(),
        }
    }
}

/// Generates static file serving routes code
pub fn generate_static_hosting_code(
    service_def: &ServiceDefinition,
    static_config: &StaticHostingConfig,
) -> TokenStream {
    if !static_config.serve_docs {
        return quote! {};
    }

    let service_name = &service_def.service_name;
    let base_path = &service_def.base_path;
    let docs_path = &static_config.docs_path;
    let ui_theme = &static_config.ui_theme;

    let openapi_fn_name = quote::format_ident!(
        "generate_{}_openapi",
        service_name.to_string().to_lowercase()
    );

    let docs_handler_name =
        quote::format_ident!("{}_docs_handler", service_name.to_string().to_lowercase());

    quote! {
        #[cfg(feature = "server")]
        // Handler for serving the documentation index
        async fn #docs_handler_name() -> ::axum::response::Html<String> {
            let openapi_spec = #openapi_fn_name();
            let spec_json = ::serde_json::to_string_pretty(&openapi_spec)
                .unwrap_or_else(|_| "{}".to_string());

            let html_content = generate_docs_html(&spec_json, #ui_theme, #base_path, #docs_path);
            ::axum::response::Html(html_content)
        }

        #[cfg(feature = "server")]
        // Generate HTML content for the API explorer page
        fn generate_docs_html(openapi_spec: &str, theme: &str, base_path: &str, docs_path: &str) -> String {
            format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - REST API Explorer</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <style>
        /* Reset and base styles */
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        :root {{
            --primary-color: #2563eb;
            --primary-hover: #1d4ed8;
            --secondary-color: #64748b;
            --success-color: #10b981;
            --warning-color: #f59e0b;
            --error-color: #ef4444;
            --bg-primary: #ffffff;
            --bg-secondary: #f8fafc;
            --bg-tertiary: #f1f5f9;
            --text-primary: #0f172a;
            --text-secondary: #64748b;
            --text-muted: #94a3b8;
            --border-color: #e2e8f0;
            --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
            --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1);
            --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1);
            --radius-sm: 0.375rem;
            --radius-md: 0.5rem;
            --radius-lg: 0.75rem;
        }}

        [data-theme="dark"] {{
            --bg-primary: #0f172a;
            --bg-secondary: #1e293b;
            --bg-tertiary: #334155;
            --text-primary: #f8fafc;
            --text-secondary: #cbd5e1;
            --text-muted: #64748b;
            --border-color: #334155;
        }}

        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            font-size: 14px;
            line-height: 1.5;
            color: var(--text-primary);
            background-color: var(--bg-secondary);
            min-height: 100vh;
        }}

        /* Layout */
        .app-container {{
            display: flex;
            min-height: 100vh;
        }}

        .sidebar {{
            width: 320px;
            background: var(--bg-primary);
            border-right: 1px solid var(--border-color);
            overflow-y: auto;
            flex-shrink: 0;
        }}

        .main-content {{
            flex: 1;
            overflow: hidden;
            display: flex;
            flex-direction: column;
        }}

        /* Header */
        .header {{
            background: var(--bg-primary);
            border-bottom: 1px solid var(--border-color);
            padding: 1rem 1.5rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
            flex-shrink: 0;
        }}

        .header h1 {{
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--text-primary);
        }}

        .header-actions {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}

        /* Auth section */
        .auth-section {{
            padding: 1rem;
            border-bottom: 1px solid var(--border-color);
        }}

        .auth-title {{
            font-size: 0.875rem;
            font-weight: 600;
            color: var(--text-primary);
            margin-bottom: 0.75rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}

        .auth-status {{
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: var(--error-color);
        }}

        .auth-status.authenticated {{
            background: var(--success-color);
        }}

        .input-group {{
            margin-bottom: 0.75rem;
        }}

        .input-label {{
            font-size: 0.75rem;
            font-weight: 500;
            color: var(--text-secondary);
            margin-bottom: 0.25rem;
            display: block;
        }}

        .input-field {{
            width: 100%;
            padding: 0.5rem 0.75rem;
            border: 1px solid var(--border-color);
            border-radius: var(--radius-md);
            background: var(--bg-primary);
            color: var(--text-primary);
            font-size: 0.875rem;
            transition: border-color 0.2s;
        }}

        .input-field:focus {{
            outline: none;
            border-color: var(--primary-color);
            box-shadow: 0 0 0 3px rgb(37 99 235 / 0.1);
        }}

        .btn {{
            padding: 0.5rem 1rem;
            border: none;
            border-radius: var(--radius-md);
            font-size: 0.875rem;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.2s;
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
        }}

        .btn-primary {{
            background: var(--primary-color);
            color: white;
        }}

        .btn-primary:hover {{
            background: var(--primary-hover);
        }}

        .btn-secondary {{
            background: var(--bg-tertiary);
            color: var(--text-primary);
            border: 1px solid var(--border-color);
        }}

        .btn-secondary:hover {{
            background: var(--border-color);
        }}

        .btn-sm {{
            padding: 0.375rem 0.75rem;
            font-size: 0.75rem;
        }}

        /* Endpoints list */
        .endpoints-section {{
            flex: 1;
            overflow-y: auto;
        }}

        .search-container {{
            padding: 1rem;
            border-bottom: 1px solid var(--border-color);
        }}

        .search-input {{
            width: 100%;
            padding: 0.5rem 0.75rem;
            border: 1px solid var(--border-color);
            border-radius: var(--radius-md);
            background: var(--bg-primary);
            color: var(--text-primary);
            font-size: 0.875rem;
        }}

        .search-input::placeholder {{
            color: var(--text-muted);
        }}

        .endpoints-list {{
            padding: 0.5rem;
        }}

        .endpoint-item {{
            padding: 0.75rem;
            margin-bottom: 0.5rem;
            border: 1px solid var(--border-color);
            border-radius: var(--radius-md);
            cursor: pointer;
            transition: all 0.2s;
            background: var(--bg-primary);
        }}

        .endpoint-item:hover {{
            border-color: var(--primary-color);
            box-shadow: var(--shadow-sm);
        }}

        .endpoint-item.active {{
            border-color: var(--primary-color);
            background: rgb(37 99 235 / 0.05);
        }}

        .endpoint-header {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
            margin-bottom: 0.25rem;
        }}

        .method-badge {{
            padding: 0.25rem 0.5rem;
            border-radius: var(--radius-sm);
            font-size: 0.75rem;
            font-weight: 600;
            text-transform: uppercase;
            min-width: 60px;
            text-align: center;
        }}

        .method-get {{ background: #dbeafe; color: #1e40af; }}
        .method-post {{ background: #dcfce7; color: #166534; }}
        .method-put {{ background: #fef3c7; color: #92400e; }}
        .method-patch {{ background: #ede9fe; color: #6b21a8; }}
        .method-delete {{ background: #fee2e2; color: #991b1b; }}

        .endpoint-path {{
            font-family: 'JetBrains Mono', 'Consolas', monospace;
            font-size: 0.875rem;
            color: var(--text-primary);
            font-weight: 500;
        }}

        .endpoint-description {{
            font-size: 0.75rem;
            color: var(--text-secondary);
            margin-top: 0.25rem;
        }}

        .auth-indicator {{
            font-size: 0.6875rem;
            padding: 0.125rem 0.375rem;
            border-radius: var(--radius-sm);
            background: var(--bg-tertiary);
            color: var(--text-muted);
            margin-left: auto;
        }}

        .auth-indicator.protected {{
            background: #fef3c7;
            color: #92400e;
        }}

        /* Request panel */
        .request-panel {{
            flex: 1;
            background: var(--bg-primary);
            overflow-y: auto;
            display: flex;
            flex-direction: column;
        }}

        .request-header {{
            padding: 1.5rem;
            border-bottom: 1px solid var(--border-color);
        }}

        .request-title {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
            margin-bottom: 0.5rem;
        }}

        .request-title h2 {{
            font-size: 1.125rem;
            font-weight: 600;
            color: var(--text-primary);
        }}

        .request-description {{
            color: var(--text-secondary);
            font-size: 0.875rem;
        }}

        .request-form {{
            padding: 1.5rem;
            flex: 1;
        }}

        .form-section {{
            margin-bottom: 2rem;
        }}

        .form-section h3 {{
            font-size: 0.875rem;
            font-weight: 600;
            color: var(--text-primary);
            margin-bottom: 1rem;
        }}

        .param-row {{
            display: grid;
            grid-template-columns: 120px 1fr 100px;
            gap: 0.75rem;
            align-items: start;
            margin-bottom: 0.75rem;
        }}

        .param-label {{
            font-size: 0.75rem;
            font-weight: 500;
            color: var(--text-secondary);
            padding-top: 0.5rem;
        }}

        .code-editor {{
            width: 100%;
            min-height: 120px;
            padding: 0.75rem;
            border: 1px solid var(--border-color);
            border-radius: var(--radius-md);
            background: var(--bg-secondary);
            color: var(--text-primary);
            font-family: 'JetBrains Mono', 'Consolas', monospace;
            font-size: 0.875rem;
            resize: vertical;
        }}

        .send-button {{
            background: var(--success-color);
            color: white;
            padding: 0.75rem 1.5rem;
            font-weight: 600;
        }}

        .send-button:hover {{
            background: #059669;
        }}

        .send-button:disabled {{
            background: var(--text-muted);
            cursor: not-allowed;
        }}

        /* Response panel */
        .response-panel {{
            background: var(--bg-primary);
            border-top: 1px solid var(--border-color);
            max-height: 50vh;
            overflow-y: auto;
        }}

        .response-header {{
            padding: 1rem 1.5rem;
            border-bottom: 1px solid var(--border-color);
            display: flex;
            align-items: center;
            justify-content: space-between;
        }}

        .response-title {{
            font-size: 0.875rem;
            font-weight: 600;
            color: var(--text-primary);
        }}

        .response-status {{
            padding: 0.25rem 0.75rem;
            border-radius: var(--radius-sm);
            font-size: 0.75rem;
            font-weight: 600;
        }}

        .status-2xx {{ background: #dcfce7; color: #166534; }}
        .status-4xx {{ background: #fef3c7; color: #92400e; }}
        .status-5xx {{ background: #fee2e2; color: #991b1b; }}

        .response-content {{
            padding: 1.5rem;
        }}

        .response-tabs {{
            display: flex;
            gap: 0.5rem;
            margin-bottom: 1rem;
        }}

        .response-tab {{
            padding: 0.5rem 0.75rem;
            border: 1px solid var(--border-color);
            border-radius: var(--radius-sm);
            background: var(--bg-secondary);
            color: var(--text-secondary);
            font-size: 0.75rem;
            cursor: pointer;
            transition: all 0.2s;
        }}

        .response-tab.active {{
            background: var(--primary-color);
            color: white;
            border-color: var(--primary-color);
        }}

        .response-body {{
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: var(--radius-md);
            padding: 1rem;
            font-family: 'JetBrains Mono', 'Consolas', monospace;
            font-size: 0.875rem;
            white-space: pre-wrap;
            overflow-x: auto;
        }}

        /* Dark mode toggle */
        .theme-toggle {{
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            color: var(--text-primary);
            width: 2.5rem;
            height: 2.5rem;
            border-radius: var(--radius-md);
            display: flex;
            align-items: center;
            justify-content: center;
            cursor: pointer;
        }}

        /* Responsive design */
        @media (max-width: 768px) {{
            .app-container {{
                flex-direction: column;
            }}

            .sidebar {{
                width: 100%;
                height: auto;
                border-right: none;
                border-bottom: 1px solid var(--border-color);
            }}

            .param-row {{
                grid-template-columns: 1fr;
                gap: 0.5rem;
            }}

            .param-label {{
                padding-top: 0;
            }}
        }}

        /* Loading states */
        .loading {{
            opacity: 0.6;
            pointer-events: none;
        }}

        .spinner {{
            width: 1rem;
            height: 1rem;
            border: 2px solid var(--border-color);
            border-top: 2px solid var(--primary-color);
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }}

        @keyframes spin {{
            0% {{ transform: rotate(0deg); }}
            100% {{ transform: rotate(360deg); }}
        }}

        /* Error states */
        .error-message {{
            background: #fee2e2;
            color: #991b1b;
            padding: 0.75rem;
            border-radius: var(--radius-md);
            font-size: 0.875rem;
            margin-bottom: 1rem;
        }}

        /* Copy button */
        .copy-btn {{
            position: absolute;
            top: 0.5rem;
            right: 0.5rem;
            background: var(--bg-primary);
            border: 1px solid var(--border-color);
            color: var(--text-secondary);
            padding: 0.25rem 0.5rem;
            border-radius: var(--radius-sm);
            font-size: 0.75rem;
            cursor: pointer;
        }}

        .response-wrapper {{
            position: relative;
        }}
    </style>
</head>
<body>
    <div class="app-container">
        <!-- Sidebar -->
        <div class="sidebar">
            <!-- Authentication Section -->
            <div class="auth-section">
                <div class="auth-title">
                    <span class="auth-status" id="auth-status"></span>
                    Authentication
                </div>
                <div class="input-group">
                    <label class="input-label" for="jwt-token">JWT Token</label>
                    <input type="password" id="jwt-token" class="input-field" placeholder="Enter your JWT token...">
                </div>
                <div style="display: flex; gap: 0.5rem;">
                    <button id="save-token" class="btn btn-primary btn-sm">Save Token</button>
                    <button id="clear-token" class="btn btn-secondary btn-sm">Clear</button>
                </div>
            </div>

            <!-- Search -->
            <div class="search-container">
                <input type="text" id="endpoint-search" class="search-input" placeholder="Search endpoints...">
            </div>

            <!-- Endpoints List -->
            <div class="endpoints-section">
                <div class="endpoints-list" id="endpoints-list">
                    <!-- Endpoints will be populated by JavaScript -->
                </div>
            </div>
        </div>

        <!-- Main Content -->
        <div class="main-content">
            <!-- Header -->
            <div class="header">
                <h1>{} API Explorer</h1>
                <div class="header-actions">
                    <button class="theme-toggle" id="theme-toggle" title="Toggle dark mode">
                        🌙
                    </button>
                </div>
            </div>

            <!-- Request Panel -->
            <div class="request-panel" id="request-panel">
                <div class="request-header">
                    <div class="request-title">
                        <span class="method-badge" id="current-method">GET</span>
                        <h2 id="current-endpoint">Select an endpoint</h2>
                    </div>
                    <div class="request-description" id="current-description">
                        Choose an endpoint from the sidebar to get started
                    </div>
                </div>

                <div class="request-form" id="request-form">
                    <p class="text-muted">Select an endpoint to see the request form</p>
                </div>
            </div>

            <!-- Response Panel -->
            <div class="response-panel" id="response-panel" style="display: none;">
                <div class="response-header">
                    <div class="response-title">Response</div>
                    <div class="response-status" id="response-status"></div>
                </div>
                <div class="response-content">
                    <div class="response-tabs">
                        <div class="response-tab active" data-tab="body">Body</div>
                        <div class="response-tab" data-tab="headers">Headers</div>
                    </div>
                    <div class="response-wrapper">
                        <div class="response-body" id="response-body"></div>
                        <button class="copy-btn" id="copy-response" title="Copy response">Copy</button>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <script>
        // HTML escape function to prevent XSS
        function escapeHtml(unsafe) {{
            return unsafe
                .replace(/&/g, "&amp;")
                .replace(/</g, "&lt;")
                .replace(/>/g, "&gt;")
                .replace(/"/g, "&quot;")
                .replace(/'/g, "&#039;");
        }}

        // Global state
        let apiSpec = null;
        let currentEndpoint = null;
        let jwtToken = localStorage.getItem('jwt-token') || '';

        // Initialize the application
        document.addEventListener('DOMContentLoaded', async () => {{
            await loadApiSpec();
            initializeAuth();
            initializeTheme();
            initializeEventListeners();
            renderEndpoints();
        }});

        // Load OpenAPI specification
        async function loadApiSpec() {{
            try {{
                const response = await fetch('{}');
                apiSpec = await response.json();
                console.log('API specification loaded:', apiSpec);
            }} catch (error) {{
                console.error('Failed to load API specification:', error);
                showError('Failed to load API specification');
            }}
        }}

        // Initialize authentication
        function initializeAuth() {{
            const tokenInput = document.getElementById('jwt-token');
            const authStatus = document.getElementById('auth-status');

            if (jwtToken) {{
                tokenInput.value = jwtToken;
                authStatus.classList.add('authenticated');
            }}
        }}

        // Initialize theme
        function initializeTheme() {{
            const savedTheme = localStorage.getItem('theme') || 'light';
            if (savedTheme === 'dark') {{
                document.documentElement.setAttribute('data-theme', 'dark');
                document.getElementById('theme-toggle').textContent = '☀️';
            }}
        }}

        // Initialize event listeners
        function initializeEventListeners() {{
            // Authentication
            document.getElementById('save-token').addEventListener('click', saveToken);
            document.getElementById('clear-token').addEventListener('click', clearToken);

            // Theme toggle
            document.getElementById('theme-toggle').addEventListener('click', toggleTheme);

            // Search
            document.getElementById('endpoint-search').addEventListener('input', filterEndpoints);

            // Response tabs
            document.querySelectorAll('.response-tab').forEach(tab => {{
                tab.addEventListener('click', (e) => switchResponseTab(e.target.dataset.tab));
            }});

            // Copy response
            document.getElementById('copy-response').addEventListener('click', copyResponse);
        }}

        // Save JWT token
        function saveToken() {{
            const tokenInput = document.getElementById('jwt-token');
            const authStatus = document.getElementById('auth-status');

            jwtToken = tokenInput.value.trim();
            localStorage.setItem('jwt-token', jwtToken);

            if (jwtToken) {{
                authStatus.classList.add('authenticated');
                showSuccess('Token saved successfully');
            }} else {{
                authStatus.classList.remove('authenticated');
            }}
        }}

        // Clear JWT token
        function clearToken() {{
            jwtToken = '';
            localStorage.removeItem('jwt-token');
            document.getElementById('jwt-token').value = '';
            document.getElementById('auth-status').classList.remove('authenticated');
            showSuccess('Token cleared');
        }}

        // Toggle theme
        function toggleTheme() {{
            const currentTheme = document.documentElement.getAttribute('data-theme');
            const newTheme = currentTheme === 'dark' ? 'light' : 'dark';

            document.documentElement.setAttribute('data-theme', newTheme);
            localStorage.setItem('theme', newTheme);

            const toggle = document.getElementById('theme-toggle');
            toggle.textContent = newTheme === 'dark' ? '☀️' : '🌙';
        }}

        // Render endpoints list
        function renderEndpoints() {{
            if (!apiSpec || !apiSpec.paths) return;

            const container = document.getElementById('endpoints-list');
            container.innerHTML = '';

            Object.entries(apiSpec.paths).forEach(([path, pathItem]) => {{
                Object.entries(pathItem).forEach(([method, operationItem]) => {{
                    if (['get', 'post', 'put', 'patch', 'delete'].includes(method)) {{
                        const endpointEl = createEndpointElement(path, method, operationItem);
                        container.appendChild(endpointEl);
                    }}
                }});
            }});
        }}

        // Create endpoint element
        function createEndpointElement(path, method, operation) {{
            const div = document.createElement('div');
            div.className = 'endpoint-item';
            div.dataset.path = path;
            div.dataset.method = method;

            const isProtected = operation['x-authentication'] || (operation.security && operation.security.length > 0);

            div.innerHTML = `
                <div class="endpoint-header">
                    <span class="method-badge method-${{escapeHtml(method)}}">${{escapeHtml(method.toUpperCase())}}</span>
                    <span class="endpoint-path">${{escapeHtml(path)}}</span>
                    <span class="auth-indicator ${{isProtected ? 'protected' : ''}}">
                        ${{isProtected ? '🔒' : '🌐'}}
                    </span>
                </div>
                ${{operation.summary ? `<div class="endpoint-description">${{escapeHtml(operation.summary)}}</div>` : ''}}
            `;

            div.addEventListener('click', () => selectEndpoint(path, method, operation));

            return div;
        }}

        // Select endpoint
        function selectEndpoint(path, method, operation) {{
            // Update UI state
            document.querySelectorAll('.endpoint-item').forEach(el => el.classList.remove('active'));
            // Use CSS.escape to safely escape selector values
            const safePathSelector = CSS.escape ? CSS.escape(path) : path.replace(/[\\"]/g, '\\$&');
            const safeMethodSelector = CSS.escape ? CSS.escape(method) : method.replace(/[\\"]/g, '\\$&');
            document.querySelector(`[data-path="${{safePathSelector}}"][data-method="${{safeMethodSelector}}"]`).classList.add('active');

            currentEndpoint = {{ path, method, operation }};

            // Update request panel
            updateRequestPanel(path, method, operation);
        }}

        // Update request panel
        function updateRequestPanel(path, method, operation) {{
            document.getElementById('current-method').textContent = method.toUpperCase();
            document.getElementById('current-method').className = `method-badge method-${{escapeHtml(method)}}`;
            document.getElementById('current-endpoint').textContent = path;
            document.getElementById('current-description').textContent = operation.summary || operation.description || '';

            // Generate request form
            const formContainer = document.getElementById('request-form');
            formContainer.innerHTML = generateRequestForm(path, method, operation);

            // Add form event listeners
            const sendButton = document.getElementById('send-request');
            if (sendButton) {{
                sendButton.addEventListener('click', () => sendRequest(path, method, operation));
            }}
        }}

        // Generate request form
        function generateRequestForm(path, method, operation) {{
            let html = '';

            // Path parameters
            const pathParams = operation.parameters?.filter(p => p.in === 'path') || [];
            if (pathParams.length > 0) {{
                html += `
                    <div class="form-section">
                        <h3>Path Parameters</h3>
                        ${{pathParams.map(param => `
                            <div class="param-row">
                                <label class="param-label">${{escapeHtml(param.name)}}</label>
                                <input type="text" class="input-field" data-param="path" data-name="${{escapeHtml(param.name)}}"
                                       placeholder="${{escapeHtml(param.description || param.name)}}" ${{param.required ? 'required' : ''}}>
                                <span class="param-type">${{escapeHtml(param.schema?.type || 'string')}}</span>
                            </div>
                        `).join('')}}
                    </div>
                `;
            }}

            // Query parameters
            const queryParams = operation.parameters?.filter(p => p.in === 'query') || [];
            if (queryParams.length > 0) {{
                html += `
                    <div class="form-section">
                        <h3>Query Parameters</h3>
                        ${{queryParams.map(param => `
                            <div class="param-row">
                                <label class="param-label">${{escapeHtml(param.name)}}</label>
                                <input type="text" class="input-field" data-param="query" data-name="${{escapeHtml(param.name)}}"
                                       placeholder="${{escapeHtml(param.description || param.name)}}" ${{param.required ? 'required' : ''}}>
                                <span class="param-type">${{escapeHtml(param.schema?.type || 'string')}}</span>
                            </div>
                        `).join('')}}
                    </div>
                `;
            }}

            // Request body
            if (operation.requestBody) {{
                const content = operation.requestBody.content;
                const jsonSchema = content['application/json']?.schema;

                html += `
                    <div class="form-section">
                        <h3>Request Body</h3>
                        <textarea class="code-editor" id="request-body" placeholder="Enter JSON request body...">${{
                            jsonSchema ? JSON.stringify(generateExampleFromSchema(jsonSchema), null, 2) : ''
                        }}</textarea>
                    </div>
                `;
            }}

            // Send button
            html += `
                <div class="form-section">
                    <button id="send-request" class="btn send-button">
                        <span>Send Request</span>
                    </button>
                </div>
            `;

            return html;
        }}

        // Resolve schema reference
        function resolveSchemaRef(schemaOrRef) {{
            if (!schemaOrRef) return null;

            // If it's a reference, resolve it
            if (schemaOrRef['$ref']) {{
                const refPath = schemaOrRef['$ref'];
                if (refPath.startsWith('#/components/schemas/')) {{
                    const schemaName = refPath.replace('#/components/schemas/', '');
                    return apiSpec?.components?.schemas?.[schemaName] || null;
                }}
            }}

            // Otherwise return as-is
            return schemaOrRef;
        }}

        // Generate example from schema
        function generateExampleFromSchema(schemaOrRef) {{
            const schema = resolveSchemaRef(schemaOrRef);
            if (!schema) return {{}};

            if (schema.example) return schema.example;
            if (schema.properties) {{
                const example = {{}};
                Object.entries(schema.properties).forEach(([key, prop]) => {{
                    // Handle type arrays (e.g., ["string", "null"] for Option<String>)
                    let propType = prop.type;
                    if (Array.isArray(propType)) {{
                        // For nullable types, use the non-null type for the example
                        propType = propType.find(t => t !== 'null') || propType[0];
                    }}

                    if (propType === 'string') {{
                        example[key] = prop.example || `example_${{key}}`;
                    }} else if (propType === 'number' || propType === 'integer') {{
                        example[key] = prop.example || 0;
                    }} else if (propType === 'boolean') {{
                        example[key] = prop.example || false;
                    }} else if (propType === 'array') {{
                        example[key] = [];
                    }} else if (prop.type && Array.isArray(prop.type) && prop.type.includes('null')) {{
                        // For nullable types, provide a meaningful example or null
                        if (prop.type.includes('string')) {{
                            example[key] = `example_${{key}}`;
                        }} else if (prop.type.includes('number') || prop.type.includes('integer')) {{
                            example[key] = 0;
                        }} else if (prop.type.includes('boolean')) {{
                            example[key] = false;
                        }} else {{
                            example[key] = null;
                        }}
                    }} else {{
                        example[key] = generateExampleFromSchema(prop);
                    }}
                }});
                return example;
            }}

            return {{}};
        }}

        // Send request
        async function sendRequest(path, method, operation) {{
            const sendButton = document.getElementById('send-request');
            const originalText = sendButton.innerHTML;
            sendButton.innerHTML = '<span class="spinner"></span> Sending...';
            sendButton.disabled = true;

            try {{
                // Build URL
                let url = window.location.origin + '{}' + path;

                // Replace path parameters
                const pathParams = document.querySelectorAll('[data-param="path"]');
                pathParams.forEach(input => {{
                    if (input.value) {{
                        url = url.replace(`{{${{input.dataset.name}}}}`, encodeURIComponent(input.value));
                    }}
                }});

                // Add query parameters
                const queryParams = new URLSearchParams();
                document.querySelectorAll('[data-param="query"]').forEach(input => {{
                    if (input.value) {{
                        queryParams.append(input.dataset.name, input.value);
                    }}
                }});

                if (queryParams.toString()) {{
                    url += '?' + queryParams.toString();
                }}

                // Build request options
                const options = {{
                    method: method.toUpperCase(),
                    headers: {{
                        'Content-Type': 'application/json',
                    }}
                }};

                // Add authorization header if token exists
                if (jwtToken) {{
                    options.headers['Authorization'] = `Bearer ${{jwtToken}}`;
                }}

                // Add request body
                const bodyEditor = document.getElementById('request-body');
                if (bodyEditor && bodyEditor.value.trim()) {{
                    try {{
                        options.body = JSON.stringify(JSON.parse(bodyEditor.value));
                    }} catch (e) {{
                        throw new Error('Invalid JSON in request body');
                    }}
                }}

                // Make request
                const response = await fetch(url, options);
                const responseData = await response.text();

                // Parse response
                let parsedData;
                try {{
                    parsedData = JSON.parse(responseData);
                }} catch (e) {{
                    parsedData = responseData;
                }}

                // Show response
                showResponse(response, parsedData, response.headers);

            }} catch (error) {{
                console.error('Request failed:', error);
                showError(error.message);
            }} finally {{
                sendButton.innerHTML = originalText;
                sendButton.disabled = false;
            }}
        }}

        // Show response
        function showResponse(response, data, headers) {{
            const panel = document.getElementById('response-panel');
            const statusEl = document.getElementById('response-status');
            const bodyEl = document.getElementById('response-body');

            panel.style.display = 'block';

            // Update status
            const statusClass = response.status < 300 ? 'status-2xx' :
                               response.status < 500 ? 'status-4xx' : 'status-5xx';
            statusEl.className = `response-status ${{statusClass}}`;
            statusEl.textContent = `${{response.status}} ${{response.statusText}}`;

            // Update body
            bodyEl.textContent = typeof data === 'string' ? data : JSON.stringify(data, null, 2);

            // Store response data for copying
            window.lastResponse = {{ body: data, headers: Object.fromEntries(headers.entries()) }};
        }}

        // Switch response tab
        function switchResponseTab(tab) {{
            document.querySelectorAll('.response-tab').forEach(t => t.classList.remove('active'));
            // Use CSS.escape to safely escape selector values
            const safeTabSelector = CSS.escape ? CSS.escape(tab) : tab.replace(/[\\"]/g, '\\$&');
            document.querySelector(`[data-tab="${{safeTabSelector}}"]`).classList.add('active');

            const bodyEl = document.getElementById('response-body');

            if (tab === 'headers' && window.lastResponse) {{
                bodyEl.textContent = JSON.stringify(window.lastResponse.headers, null, 2);
            }} else if (tab === 'body' && window.lastResponse) {{
                const data = window.lastResponse.body;
                bodyEl.textContent = typeof data === 'string' ? data : JSON.stringify(data, null, 2);
            }}
        }}

        // Copy response
        async function copyResponse() {{
            const bodyEl = document.getElementById('response-body');
            try {{
                await navigator.clipboard.writeText(bodyEl.textContent);
                showSuccess('Response copied to clipboard');
            }} catch (error) {{
                console.error('Failed to copy:', error);
            }}
        }}

        // Filter endpoints
        function filterEndpoints() {{
            const query = document.getElementById('endpoint-search').value.toLowerCase();
            const endpoints = document.querySelectorAll('.endpoint-item');

            endpoints.forEach(endpoint => {{
                const path = endpoint.dataset.path.toLowerCase();
                const method = endpoint.dataset.method.toLowerCase();
                const text = endpoint.textContent.toLowerCase();

                if (path.includes(query) || method.includes(query) || text.includes(query)) {{
                    endpoint.style.display = 'block';
                }} else {{
                    endpoint.style.display = 'none';
                }}
            }});
        }}

        // Show success message
        function showSuccess(message) {{
            // You could implement a toast notification here
            console.log('Success:', message);
        }}

        // Show error message
        function showError(message) {{
            // You could implement a toast notification here
            console.error('Error:', message);
        }}
    </script>
</body>
</html>"#,
                stringify!(#service_name),
                stringify!(#service_name),
                "./docs/openapi.json",
                #base_path
            )
        }

        #[cfg(feature = "server")]
        // Generate OpenAPI JSON endpoint handler
        async fn openapi_json_handler() -> ::axum::Json<::serde_json::Value> {
            ::axum::Json(#openapi_fn_name())
        }
    }
}

/// Generates route registrations for static hosting
pub fn generate_static_routes(
    service_def: &ServiceDefinition,
    static_config: &StaticHostingConfig,
) -> TokenStream {
    if !static_config.serve_docs {
        return quote! {};
    }

    let docs_path = &static_config.docs_path;
    let openapi_path = format!("{}/openapi.json", docs_path);
    let docs_handler_name = quote::format_ident!(
        "{}_docs_handler",
        service_def.service_name.to_string().to_lowercase()
    );

    quote! {
        #[cfg(feature = "server")]
        {
            // Register static hosting routes
            router = router
                .route(#docs_path, ::axum::routing::get(#docs_handler_name))
                .route(#openapi_path, ::axum::routing::get(openapi_json_handler));
        }
    }
}
