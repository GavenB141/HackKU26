use std::process::ExitCode;

use axum::{
    Router,
    routing::{get, post},
};
use tokio::{
    net::TcpListener,
    signal::unix::{SignalKind, signal},
};
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod routes;

#[tokio::main]
async fn main() -> ExitCode {
    // initialize the logging handler
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // initialize the routes
    let router = Router::new()
        .route("/new", post(routes::post_new))
        .route("/generated/{id}", get(routes::get_id))
        .layer(TraceLayer::new_for_http());

    // set up the listener
    let listen_address = "0.0.0.0:9000";
    let listener = match TcpListener::bind(listen_address).await {
        Ok(listener) => listener,
        Err(err) => {
            error!(?err, "failed to bind port");
            return ExitCode::FAILURE;
        }
    };
    if let Ok(addr) = listener.local_addr() {
        info!(%addr, "listener started");
    }

    // do the serving
    if let Err(err) = axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(wait_for_shutdown())
        .await
    {
        error!(?err, "server error");
    }

    // server finished, exit
    ExitCode::SUCCESS
}

/// This function returns a future which resolves once the server needs to shut down.
async fn wait_for_shutdown() {
    // wait for each type of signal
    let sigint = async {
        signal(SignalKind::interrupt()).unwrap().recv().await;
        info!("received SIGINT");
    };
    let sigterm = async {
        signal(SignalKind::terminate()).unwrap().recv().await;
        info!("received SIGTERM");
    };
    tokio::select! {
        _ = sigint => {}
        _ = sigterm => {}
    }
}
