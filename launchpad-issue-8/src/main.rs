use std::sync::atomic::AtomicUsize;
use std::{collections::HashMap, sync::Arc};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::{routing::get, Router};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::{mpsc::UnboundedSender, RwLock};

static NEXT_USERID: AtomicUsize = AtomicUsize::new(1);

type Users = Arc<RwLock<HashMap<usize, UnboundedSender<Message>>>>;

async fn hello_world() -> &'static str {
    "Hello, world!"
}

#[derive(Serialize, Deserialize)]
struct Msg {
    name: String,
    uid: Option<usize>,
    message: String,
}

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let users = Users::default();

    let router = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(users);

    Ok(router.into())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Users>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(ws: WebSocket, state: Users) {
    println!("Hello {:?}", state);

    let my_id = NEXT_USERID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (mut sender, mut receiver) = ws.split();

    let (tx, mut rx): (UnboundedSender<Message>, UnboundedReceiver<Message>) =
        mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            sender.send(msg).await.expect("Error!");
        }
        sender.close().await.unwrap();
    });

    state.write().await.insert(my_id, tx);

    while let Some(Ok(result)) = receiver.next().await {
        println!("{:?}", result);
        if let Ok(result) = enrich_result(result, my_id) {
            broadcast_msg(result, &state).await;
        }
    }

    disconnect(my_id, &state).await;
}

fn enrich_result(result: Message, id: usize) -> Result<Message, serde_json::Error> {
    match result {
        Message::Text(msg) => {
            let mut msg: Msg = serde_json::from_str(&msg)?;
            msg.uid = Some(id);
            let msg = serde_json::to_string(&msg)?;
            Ok(Message::Text(msg))
        }
        _ => Ok(result),
    }
}

async fn broadcast_msg(msg: Message, users: &Users) {
    if let Message::Text(msg) = msg {
        for (&_uid, tx) in users.read().await.iter() {
            tx.send(Message::Text(msg.clone()))
                .expect("Failed to send Message")
        }
    }
}

async fn disconnect(my_id: usize, users: &Users) {
    println!("Good bye user {}", my_id);
    users.write().await.remove(&my_id);
    println!("Disconnected {my_id}");
}
