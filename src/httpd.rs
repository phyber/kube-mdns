// HTTPd for dealing with health and readiness checks.
use axum::{
    routing,
    Router,
};
use axum::http::StatusCode;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::signal::unix::{
    signal,
    SignalKind,
};

const OK_RESPONSE: &str = "ok";

#[derive(Debug)]
pub struct Server;

// Return HTTP 200 with a body of "ok" on healthcheck.
async fn healthz() -> (StatusCode, &'static str) {
    (StatusCode::OK, OK_RESPONSE)
}

// Handle graceful shutdown via ctrl+c or SIGTERM.
#[allow(clippy::ignored_unit_patterns)]
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("ctrl+c handler");
    };

    let mut sigterm = signal(SignalKind::terminate())
        .expect("sigterm handler");

    tokio::select! {
        _ = ctrl_c => {},
        _ = sigterm.recv() => {},
    }
}

// Run a simple HTTP server for healthchecks.
impl Server {
    pub async fn run() {
        let app = Router::new()
            .route("/healthz", routing::get(healthz));

        let listener = TcpListener::bind("0.0.0.0:8080")
            .await
            .expect("httpd bind");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .expect("httpd serve");
    }
}
