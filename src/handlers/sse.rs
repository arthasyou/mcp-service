use axum::{
    Extension,
    extract::Query,
    http::StatusCode,
    response::{Sse, sse::Event},
};
use futures::{Stream, TryStreamExt};
use mcp_server_rs::{
    core::{protocol::message::JsonRpcMessage, utils::CleanupStream},
    router::impls::{chart::ChartRouter, counter::CounterRouter},
    server::Server,
    transport::server::sse::SseTransport,
};
use tokio::{io, sync::mpsc};
use tokio_stream::{StreamExt, once, wrappers::UnboundedReceiverStream};

use crate::models::{App, PostQuery, SseQuery, session_id};

pub async fn sse_handler(
    Extension(app): Extension<App>,
    Query(SseQuery { service }): Query<SseQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, io::Error>>>, StatusCode> {
    let router = get_router(service.as_str()).ok_or(StatusCode::BAD_REQUEST)?;

    let session = session_id();
    tracing::info!(%session, "new SSE connection");

    let (to_client_tx, to_client_rx) = mpsc::unbounded_channel::<JsonRpcMessage>();
    let (to_server_tx, to_server_rx) = mpsc::unbounded_channel::<JsonRpcMessage>();
    app.channels
        .write()
        .await
        .insert(session.clone(), to_server_tx.clone());

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let session_for_task = session.clone();
    let channels = app.channels.clone();

    tokio::spawn(async move {
        let transport = SseTransport::new(to_client_tx, to_server_rx);
        let server = Server::new(router);
        let result = tokio::select! {
            res = server.run(transport) => {
                tracing::info!(%session_for_task, "server.run completed");
                res
            },
            _ = shutdown_rx => {
                tracing::info!(%session_for_task, "client disconnected, cleaning up");
                Ok(())
            }
        };

        tracing::info!(%session_for_task, "Cleaning up session");
        channels.write().await.remove(&session_for_task);

        if let Err(e) = result {
            tracing::error!(?e, "server run error");
        }
    });

    let init_event = Event::default()
        .event("endpoint")
        .data(format!("?sessionId={}", session));

    let init_stream = once(Ok::<Event, io::Error>(init_event));

    let message_stream = UnboundedReceiverStream::new(to_client_rx)
        .map(|msg| {
            serde_json::to_string(&msg).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        })
        .and_then(|json| futures::future::ok(Event::default().event("message").data(json)));

    let full_stream = init_stream.chain(message_stream);

    let full_stream = CleanupStream {
        inner: full_stream,
        shutdown_tx: Some(shutdown_tx),
    };

    Ok(Sse::new(full_stream))
}

pub async fn post_handler(
    Extension(app): Extension<App>,
    Query(PostQuery { session_id }): Query<PostQuery>,
    body: axum::body::Bytes,
) -> Result<String, String> {
    let channels = app.channels.read().await;
    if let Some(sender) = channels.get(session_id.as_str()) {
        let data = String::from_utf8(body.to_vec()).map_err(|e| e.to_string())?;
        let msg: JsonRpcMessage = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        println!("Received message: {:?}", &msg);
        sender
            .send(msg)
            .map_err(|_| "Failed to send message".to_string())?;

        // In a real case, this message would be pushed to a proper transport reader.
        // But here we just log it.
        // tracing::info!(?msg, "Received POST message");

        Ok("Accepted".to_string())
    } else {
        Err("Session not found".to_string())
    }
}

fn get_router(service: &str) -> Option<Box<dyn mcp_server_rs::router::traits::Router>> {
    match service {
        "chart" => Some(Box::new(ChartRouter::new())),
        "counter" => Some(Box::new(CounterRouter::new())),
        _ => None,
    }
}
