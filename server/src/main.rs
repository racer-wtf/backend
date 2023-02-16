mod ws;

use std::{net::SocketAddr, sync::Arc};

use axum::{routing::get, Router};
use tokio::task::JoinSet;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::ws::publishers::run_publishers;
use crate::ws::{websocket_handler, PubSubState};

#[tokio::main]
async fn main() {
    // enable logging to console
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=trace,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // create global state for web server
    let state = Arc::new(PubSubState::default());

    // define application routes
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state.clone())
        // enable tracing for all tower http requests
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // create joinset for all tasks
    let mut set = JoinSet::new();

    // run the websocket publishers
    set.spawn(async move {
        run_publishers(state).await;
        // if let Err(publisher_error) = run_publishers(state.clone()).await {
        //     tracing::error!("web server shut down: {}", publisher_error);
        // }
    });

    // run the server
    set.spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        tracing::info!("listening on {}", addr);

        if let Err(axum_error) = axum::Server::bind(&addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
        {
            tracing::error!("web server shut down: {}", axum_error);
        }
    });

    // wait for all tasks to complete
    while let Some(res) = set.join_next().await {
        res.unwrap();
    }

    tracing::info!("server shutting down after all tasks finished");
}
