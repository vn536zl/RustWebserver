use crate::{Client, Clients};
use futures::{FutureExt, StreamExt};
use serde::Deserialize;
use serde_json::from_str;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

#[derive(Deserialize, Debug)]
pub struct TopicsRequest {
    topics: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct SentMessage {
    topic: String,
    message: String,
}

pub async fn client_connection(ws: WebSocket, id: String, clients: Clients, mut client: Client) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("Error sending websocket msg: {}", e);
        }
    }));

    client.sender = Some(client_sender);
    clients.lock().await.insert(id.clone(), client);

    println!("{} connected", id);

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Error receiving WS message for id: {}): {}", id.clone(), e);
                break;
            }
        };
        client_msg(&id, msg, &clients).await;
    }

    clients.lock().await.remove(&id);
    println!("{} disconnected", id);
}

async fn client_msg(id: &str, msg: Message, clients: &Clients) {
    println!("Received message from {}; {:?}", id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    if message == "ping" || message == "ping\n" {
        clients.lock().await.iter_mut()
            .filter(|(user_id, _)| user_id.as_str() == id)
            .for_each(|(_, client)| {
                if let Some(sender) = &client.sender {
                    let _ = sender.send(Ok(Message::text("Pong")));
                }
            });
        return;
    }


    let topics_req: TopicsRequest = match from_str(&message) {
        Ok(v) => v,
        Err(_) => {
            let json_msg: SentMessage = from_str(message).unwrap();
            clients.lock().await.iter_mut()
                .filter(|(_, client)| client.topics.contains(&json_msg.topic))
                .for_each(|(_, client)| {
                    if let Some(sender) = &client.sender {
                        let _ = sender.send(Ok(Message::text(&json_msg.message)));
                    }
                });
            return;
        }
    };

    let mut locked = clients.lock().await;
    match locked.get_mut(id) {
        Some(v) => {
            v.topics = topics_req.topics;
        },
        None => return,
    };
}