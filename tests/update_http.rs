#![cfg(windows)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use poemercpricer::update::fetch_bounded;

fn response(
    bytes: Vec<u8>,
    stall: bool,
) -> (String, mpsc::Sender<()>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/asset", listener.local_addr().unwrap());
    let (done, wait) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buffer).unwrap();
            if n == 0 {
                return;
            }
            request.extend_from_slice(&buffer[..n]);
        }
        let _ = stream.write_all(&bytes);
        if stall {
            let _ = wait.recv_timeout(Duration::from_secs(5));
        }
    });
    (url, done, server)
}

#[test]
fn bounded_download_accepts_exact_length_with_or_without_content_length() {
    for headers in ["Content-Length: 4\r\n", ""] {
        let (url, _, server) = response(
            format!("HTTP/1.1 200 OK\r\n{headers}Connection: close\r\n\r\ndata").into_bytes(),
            false,
        );
        assert_eq!(
            fetch_bounded(&url, Duration::from_secs(3), 4).unwrap(),
            b"data"
        );
        server.join().unwrap();
    }
}

#[test]
fn bounded_download_rejects_oversized_headers_and_chunked_bodies() {
    for wire in [
        "HTTP/1.1 200 OK\r\nContent-Length: 1000000000\r\nConnection: close\r\n\r\n",
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n8\r\ntoo much\r\n0\r\n\r\n",
        "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\ntoo much",
    ] {
        let (url, _, server) = response(wire.as_bytes().to_vec(), false);
        let error = fetch_bounded(&url, Duration::from_secs(3), 4).unwrap_err();
        assert!(error.to_string().contains("byte limit"), "{error:#}");
        server.join().unwrap();
    }
}

#[test]
fn download_deadline_covers_a_body_that_stalls_after_headers() {
    let (url, done, server) = response(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nab".to_vec(),
        true,
    );
    let started = Instant::now();
    let error = fetch_bounded(&url, Duration::from_millis(200), 4).unwrap_err();
    let elapsed = started.elapsed();
    let _ = done.send(());
    server.join().unwrap();
    assert!(error.to_string().contains("timed out"), "{error:#}");
    assert!(
        elapsed < Duration::from_secs(2),
        "body timeout took {elapsed:?}"
    );
}
