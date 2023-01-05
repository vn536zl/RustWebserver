use crate::{Client, Clients, Board};
use futures::{FutureExt, StreamExt};
use serde::Deserialize;
use serde_json::from_str;
use std::cell::RefCell;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

#[derive(Deserialize, Copy, Clone)]
pub struct SentBoard {
    board: Board,
}


thread_local! {static GAME_BOARD: RefCell<Board> = RefCell::new([[0; 20]; 10])}

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
        let board = client_msg(&id, msg, &clients).await;
        GAME_BOARD.with(|val| {
            *val.borrow_mut() = board;
        })
    }

    clients.lock().await.remove(&id);
    println!("{} disconnected", id);
}

async fn client_msg(id: &str, msg: Message, clients: &Clients) -> Board {
    println!("Received message from {}; {:?}", id, msg);
    let mut board: Board = [[0; 20]; 10];
    GAME_BOARD.with(|static_board| {
        board = *static_board.borrow();
    });

    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return board,
    };

    if message == "ping" || message == "ping\n" {
        clients.lock().await.iter_mut()
            .filter(|(user_id, _)| user_id.as_str() == id)
            .for_each(|(_, client)| {
                if let Some(sender) = &client.sender {
                    let _ = sender.send(Ok(Message::text("Pong")));
                }
            });
        return board;
    }

    if message == "get_board" || message == "get_board\n" {
        send_board(id, clients).await;
    }

    let sent_board: SentBoard = match from_str(&message) {
        Ok(msg) => msg,
        Err(e) => {
            eprintln!("Error processing request: {}", e);
            return board;
        }
    };
    sent_board.board

}

async fn send_board(id: &str, clients: &Clients) {
    clients.lock().await
        .iter_mut()
        .filter(|(client_id, _)| client_id.to_string() == id.to_string())
        .for_each(|(_, client)| {
            if let Some(sender) = &client.sender {

                GAME_BOARD.with(|board| {
                    let board_text = format!("{:?}", *board.borrow());
                    sender.send(Ok(Message::text(board_text))).expect("Error Sending Board");
                    println!("Sent Board: {}", board_text);
                }).expect("Error with Constant");
            };
        });
}