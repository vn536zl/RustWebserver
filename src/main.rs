use std::{
    time::Duration,
};
use async_std::{
    task,
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
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await.unwrap();

    let get = b"GET / HTTP/1.1";
    let sleep = b"GET /sleep HTTP/1.1";

    let (status_line, content) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK", include_bytes!("html/content/index.html").to_vec())
    } else if buffer.starts_with(sleep) {
        task::sleep(Duration::from_secs(5)).await;
        ("HTTP/1.1 200 OK", include_bytes!("html/content/index.html").to_vec())
    } else {
        ("HTTP/1.1 404 NOT FOUND", include_bytes!("html/errors/404.html").to_vec())
    };

    let contents = String::from_utf8(content).expect("Error to string");
    let length = contents.len();

    let response =
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    println!("Connected!");
    stream.write_all(response.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
}