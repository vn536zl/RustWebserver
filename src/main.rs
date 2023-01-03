use async_std::{
    task::spawn,
    prelude::*,
    net::{TcpListener, TcpStream}
};
use futures::StreamExt;

#[async_std::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();

    listener
        .incoming()
        .for_each_concurrent(/* limit */ None, |stream| async move {
            let stream = stream.unwrap();
            spawn(handle_connection(stream));
        })
        .await
}


async fn handle_connection(mut stream: TcpStream) {
    let mut buffer = vec![0; 1024];
    stream.read(&mut buffer).await.unwrap();

    let get = b"GET / HTTP/1.1";

    let status_line = if buffer.starts_with(get) {
        "HTTP/1.1 200 OK"
    } else {
        "HTTP/1.1 404 NOT FOUND"
    };

    let response = status_line.to_owned() + "\r\n\r\n";

    buffer.retain(|&i| i != 0);

    let http_request = String::from_utf8(buffer).unwrap();

    println!("Request: {}", http_request);
    stream.write_all(response.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
}