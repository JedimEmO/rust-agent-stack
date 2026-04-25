use std::net::SocketAddr;

use axum::Router;
use axum_test::TestServer;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Spawn the given router behind an `axum-test::TestServer` configured with a
/// real TCP listener on a random port. The returned [`TestServer`] exposes a
/// real `http://127.0.0.1:PORT` URL via [`TestServer::server_address`], which
/// lets generated reqwest-based clients talk to it.
///
/// Use this for HTTP / JSON-RPC over HTTP / file service tests.
pub fn spawn_http(router: Router) -> TestServer {
    TestServer::builder()
        .http_transport()
        .build(router)
        .expect("failed to start axum-test TestServer with http transport")
}

/// Spawn the given router on a freshly-bound `127.0.0.1` port using a real
/// `axum::serve` task. Returns the bound address and the join handle for the
/// server task. Drop the handle to abort the server.
///
/// Use this for WebSocket tests where the generated client uses
/// `tokio-tungstenite` and needs a genuine TCP socket.
pub async fn spawn_tcp(router: Router) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind ephemeral test port");
    let addr = listener
        .local_addr()
        .expect("failed to read local addr from test listener");

    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    (addr, handle)
}
