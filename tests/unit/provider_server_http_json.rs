// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn request(address: std::net::SocketAddr) -> String {
    let mut stream = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect HTTP JSON server");
    stream
        .write_all(b"GET /work HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
        .await
        .expect("write HTTP JSON request");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read HTTP JSON response");
    String::from_utf8(response).expect("HTTP JSON response is UTF-8")
}

#[tokio::test]
async fn independent_connections_execute_concurrently_with_a_bounded_lane() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind HTTP JSON server");
    let address = listener.local_addr().expect("server address");
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let observed_active = Arc::clone(&active);
    let observed_maximum = Arc::clone(&maximum);
    let server = tokio::spawn(async move {
        super::serve_http_json(listener, shutdown_rx, move |_request| {
            let active = Arc::clone(&observed_active);
            let maximum = Arc::clone(&observed_maximum);
            async move {
                let in_flight = active.fetch_add(1, Ordering::AcqRel) + 1;
                maximum.fetch_max(in_flight, Ordering::AcqRel);
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                active.fetch_sub(1, Ordering::AcqRel);
                super::HttpJsonResponse::json(200, &serde_json::json!({"state": "ready"}))
            }
        })
        .await
    });

    let (first, second) = tokio::join!(request(address), request(address));
    assert!(first.contains("200 OK"));
    assert!(second.contains("200 OK"));
    assert_eq!(maximum.load(Ordering::Acquire), 2);
    shutdown_tx.send(true).expect("signal server shutdown");
    server
        .await
        .expect("join HTTP JSON server")
        .expect("HTTP JSON server result");
}
