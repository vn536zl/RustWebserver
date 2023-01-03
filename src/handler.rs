use crate::{ws, Client, Clients, Result};
use serde::Serialize;
use uuid::Uuid;
use warp::{http::StatusCode, reply::json, Reply};

#[derive(Serialize, Debug)]
pub struct RegisterResponse {
    url: String,
}

#[derive(Serialize, Debug)]
pub struct Error {
    error: String
}


pub async fn register_handler(clients: Clients) -> Result<impl Reply> {
    let mut uuid = Uuid::new_v4().simple().to_string();
    let mut used_ids: Vec<String> = Vec::new();
    clients.lock().await.iter_mut().for_each(|(id, _)| {
        used_ids.push(id.to_string());
    });

    while used_ids.contains(&uuid) {
        uuid = Uuid::new_v4().simple().to_string();
    }

    register_client(uuid.clone(), clients).await;
    Ok(warp::reply::with_status(json(&RegisterResponse {
        url: format!("ws://127.0.0.1:4200/ws/{}", uuid),
    }), StatusCode::OK))
}

async fn register_client(id: String, clients: Clients) {
    clients.lock().await.insert(
        id,
        Client {
            topics: vec![String::from("cats")],
            sender: None,
        },
    );
}

pub async fn unregister_handler(id: String, clients: Clients) -> Result<impl Reply> {
    clients.lock().await.remove(&id);
    Ok(StatusCode::OK)
}

pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients) -> Result<impl Reply> {
    let client = clients.lock().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, id, clients, c))),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}