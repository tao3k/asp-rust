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
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        stream.read_to_end(&mut response),
    )
    .await
    .expect("HTTP response deadline")
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
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let observed_active = Arc::clone(&active);
    let observed_maximum = Arc::clone(&maximum);
    let server = tokio::spawn(async move {
        super::serve_http_json(listener, shutdown_rx, move |_request| {
            let barrier = Arc::clone(&barrier);
            let active = Arc::clone(&observed_active);
            let maximum = Arc::clone(&observed_maximum);
            async move {
                let in_flight = active.fetch_add(1, Ordering::AcqRel) + 1;
                maximum.fetch_max(in_flight, Ordering::AcqRel);
                tokio::time::timeout(std::time::Duration::from_secs(5), barrier.wait())
                    .await
                    .expect("independent handlers must rendezvous concurrently");
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

async fn finish(server: tokio::task::JoinHandle<Result<(), String>>) {
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .expect("server shutdown deadline")
        .expect("join server")
        .expect("server result");
}

#[tokio::test]
async fn idle_listener_observes_shutdown_without_another_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let server = tokio::spawn(super::serve_http_json(listener, shutdown_rx, |_| async {
        Ok(super::HttpJsonResponse::encoded_json(200, b"{}".to_vec()))
    }));
    tokio::task::yield_now().await;
    shutdown_tx.send(true).unwrap();
    finish(server).await;
}

#[tokio::test]
async fn saturated_lane_stays_bounded_and_shutdown_drains_admitted_requests() {
    let limit = super::connection_parallelism();
    assert!((4..=16).contains(&limit));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let (entered_tx, mut entered_rx) = tokio::sync::mpsc::unbounded_channel();
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    let handler_release = Arc::clone(&release);
    let server = tokio::spawn(super::serve_http_json(listener, shutdown_rx, move |_| {
        let release = Arc::clone(&handler_release);
        let entered = entered_tx.clone();
        async move {
            entered.send(()).unwrap();
            release.acquire().await.unwrap().forget();
            Ok(super::HttpJsonResponse::encoded_json(200, b"{}".to_vec()))
        }
    }));
    let mut clients = tokio::task::JoinSet::new();
    for _ in 0..limit {
        clients.spawn(request(address));
    }
    for _ in 0..limit {
        tokio::time::timeout(std::time::Duration::from_secs(5), entered_rx.recv())
            .await
            .expect("all admitted requests entered")
            .unwrap();
    }
    // Extra connection is queued while every admitted handler is held at a barrier.
    let extra = tokio::spawn(request(address));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), entered_rx.recv())
            .await
            .is_err()
    );
    shutdown_tx.send(true).unwrap();
    tokio::task::yield_now().await;
    assert!(
        !server.is_finished(),
        "shutdown must drain admitted requests"
    );
    release.add_permits(limit);
    while let Some(result) = clients.join_next().await {
        assert!(result.unwrap().contains("200 OK"));
    }
    finish(server).await;
    extra.abort();
    let _ = extra.await;
}

#[tokio::test]
async fn handler_error_returns_json_500_and_next_request_succeeds() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let calls = AtomicUsize::new(0);
    let server = tokio::spawn(super::serve_http_json(listener, shutdown_rx, move |_| {
        let first = calls.fetch_add(1, Ordering::Relaxed) == 0;
        async move {
            if first {
                Err("deliberate failure".to_owned())
            } else {
                Ok(super::HttpJsonResponse::encoded_json(200, b"{}".to_vec()))
            }
        }
    }));
    let response = request(address).await;
    assert!(response.contains("500 Internal Server Error"));
    let body: serde_json::Value =
        serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["error"], "deliberate failure");
    assert!(request(address).await.contains("200 OK"));
    shutdown_tx.send(true).unwrap();
    finish(server).await;
}

async fn parse_frame(frame: Vec<u8>) -> Result<super::HttpJsonRequest, String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let writer = tokio::spawn(async move {
        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.write_all(&frame).await.unwrap();
        stream.shutdown().await.unwrap();
    });
    let (mut stream, _) = listener.accept().await.unwrap();
    let parsed = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        super::read_request(&mut stream),
    )
    .await
    .expect("parse deadline");
    writer.await.unwrap();
    parsed
}

#[tokio::test]
async fn request_parser_checks_lengths_truncation_and_exact_body_boundary() {
    for length in [usize::MAX, super::MAX_REQUEST_BYTES] {
        let error = parse_frame(
            format!("POST / HTTP/1.1\r\nContent-Length: {length}\r\n\r\n").into_bytes(),
        )
        .await
        .unwrap_err();
        assert!(error.contains("bounded frame limit"));
    }
    for (frame, expected) in [
        (
            b"GET / HTTP/1.0\r\n\r\n".as_slice(),
            "request line is invalid",
        ),
        (
            b"POST / HTTP/1.1\r\nContent-Length: nope\r\n\r\n".as_slice(),
            "content-length is invalid",
        ),
        (
            b"POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\n{}".as_slice(),
            "body ended early",
        ),
        (b"GET / HTTP/1.1\r\n".as_slice(), "ended before its headers"),
    ] {
        assert!(
            parse_frame(frame.to_vec())
                .await
                .unwrap_err()
                .contains(expected)
        );
    }
    let parsed =
        parse_frame(b"POST /query HTTP/1.1\r\ncOnTeNt-LeNgTh: 2\r\n\r\n{}trailing".to_vec())
            .await
            .unwrap();
    assert_eq!(parsed.method, "POST");
    assert_eq!(parsed.path, "/query");
    assert_eq!(parsed.body, b"{}");
}

#[tokio::test]
#[ignore = "loopback performance receipt; run explicitly with --ignored --nocapture"]
async fn http_loopback_latency_profile() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let server = tokio::spawn(super::serve_http_json(listener, shutdown_rx, |_| async {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok(super::HttpJsonResponse::encoded_json(200, b"{}".to_vec()))
    }));
    let mut serial = Vec::new();
    let mut parallel = Vec::new();
    for iteration in 0..25 {
        let started = std::time::Instant::now();
        for _ in 0..4 {
            assert!(request(address).await.contains("200 OK"));
        }
        let serial_ms = started.elapsed().as_secs_f64() * 1000.0;
        let started = std::time::Instant::now();
        let responses = tokio::join!(
            request(address),
            request(address),
            request(address),
            request(address)
        );
        for response in [responses.0, responses.1, responses.2, responses.3] {
            assert!(response.contains("200 OK"));
        }
        if iteration >= 5 {
            serial.push(serial_ms);
            parallel.push(started.elapsed().as_secs_f64() * 1000.0);
        }
    }
    serial.sort_by(f64::total_cmp);
    parallel.sort_by(f64::total_cmp);
    println!(
        "{}",
        serde_json::json!({
            "case": "http.loopback.four-requests", "warmup": 5, "samples": 20,
            "handlerDelayMs": 10, "connectionLimit": super::connection_parallelism(),
            "serial": {"p50Ms": serial[9], "p95Ms": serial[18], "p99Ms": serial[19]},
            "parallel": {"p50Ms": parallel[9], "p95Ms": parallel[18], "p99Ms": parallel[19]},
            "p95Ratio": parallel[18] / serial[18]
        })
    );
    shutdown_tx.send(true).unwrap();
    finish(server).await;
}
