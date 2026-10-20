// This is free and unencumbered software released into the public domain.

use super::Sandbox;
use alloc::{format, string::String, vec, vec::Vec};
use asimov_http::{
    Component, SecretKey, SignatureInput, SignatureParams, Signer, VerificationPolicy, Verifier,
    VerifyingKey,
    body::{sign_buffered_response, verify_request},
    http::{Request, Response, StatusCode},
    key_id,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use clientele::SysexitsError::*;
use core::time::Duration;
use std::{
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    thread::{self, JoinHandle},
    time::Instant,
};

const KEY_ID: &str = "remote-deployment-key";
const TARGET: &str = "/service?check=1";

fn signed_response(status: StatusCode) -> Response<Vec<u8>> {
    let key = SecretKey::from_bytes(&[42; 32]);
    let mut params = SignatureParams::fresh(&key.public(), "remote-check").unwrap();
    params.key_id = KEY_ID.into();
    let signer = Signer::new(
        key,
        "asimov",
        SignatureInput::new(
            ["@status", "content-digest"]
                .into_iter()
                .map(|name| Component::new(name).unwrap())
                .collect(),
            params,
        )
        .unwrap(),
    )
    .unwrap();
    let mut response = Response::builder().status(status).body(Vec::new()).unwrap();
    sign_buffered_response(&mut response, None, &signer).unwrap();
    response
}

fn read_request(socket: &mut TcpStream) -> Request<Vec<u8>> {
    let mut reader = BufReader::new(socket);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let mut parts = line.split_whitespace();
    let mut request = Request::builder()
        .method(parts.next().unwrap())
        .uri(parts.next().unwrap());
    let mut length = 0;
    loop {
        let mut header = String::new();
        assert_ne!(reader.read_line(&mut header).unwrap(), 0);
        if header == "\r\n" {
            break;
        }
        let (name, value) = header.split_once(':').unwrap();
        request = request.header(name, value.trim());
        if name.eq_ignore_ascii_case("content-length") {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    request.body(body).unwrap()
}

fn server(response: Response<Vec<u8>>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let target = format!("http://{}{TARGET}", listener.local_addr().unwrap());
    let url = target.clone();
    let task = thread::spawn(move || {
        let key = SecretKey::from_bytes(&[42; 32]).public();
        let jwks = Response::builder()
            .header("content-type", "application/json")
            .body(
                serde_json::to_vec(&serde_json::json!({ "keys": [{
                    "kty": "OKP", "crv": "Ed25519", "kid": KEY_ID,
                    "x": URL_SAFE_NO_PAD.encode(key.as_bytes()),
                }] }))
                .unwrap(),
            )
            .unwrap();
        for (index, response) in [jwks, response].into_iter().enumerate() {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "no request received");
                        thread::sleep(Duration::from_millis(5));
                    },
                    Err(error) => panic!("accepting request: {error}"),
                }
            };
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let request = read_request(&mut socket);
            if index == 0 {
                assert_eq!(request.method(), "GET");
                assert_eq!(request.uri(), "/.well-known/jwks.json");
                assert!(!request.headers().contains_key("authorization"));
            } else {
                assert_eq!(request.method(), "OPTIONS");
                assert_eq!(request.uri(), TARGET);
                assert!(request.body().is_empty());
                let bearer = request.headers()["authorization"]
                    .to_str()
                    .unwrap()
                    .strip_prefix("Bearer ")
                    .unwrap();
                let public: asimov_id::PublicKey = format!("ⒶY{bearer}").parse().unwrap();
                let public = VerifyingKey::from_bytes(&public.into_bytes()).unwrap();
                assert_ne!(public, key);
                let verifier = Verifier::new(
                    public,
                    &key_id(&public),
                    "asimov",
                    VerificationPolicy::request(),
                )
                .unwrap();
                verify_request(
                    &request,
                    &target,
                    &verifier,
                    SignatureParams::now().unwrap(),
                )
                .unwrap();
            }
            write!(
                socket,
                "HTTP/1.1 {}\r\nConnection: close\r\n",
                response.status()
            )
            .unwrap();
            if response.status() != StatusCode::NO_CONTENT {
                write!(socket, "Content-Length: {}\r\n", response.body().len()).unwrap();
            }
            for (name, value) in response.headers() {
                write!(socket, "{name}: {}\r\n", value.to_str().unwrap()).unwrap();
            }
            socket.write_all(b"\r\n").unwrap();
            socket.write_all(response.body()).unwrap();
        }
    });
    (url, task)
}

fn add_remote(sandbox: &Sandbox, url: &str) {
    sandbox.run(&["remote", "add", "origin", url], EX_OK);
}

fn run(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    sandbox
        .command()
        .args(args)
        .env("NO_PROXY", "*")
        .env("no_proxy", "*")
        .output()
        .unwrap()
}

#[test]
fn successful_checks_are_silent_or_report_verified_rtt_to_stderr() {
    for args in [
        vec!["remote", "check", "origin"],
        vec!["-v", "remote", "ping", "origin"],
    ] {
        let sandbox = Sandbox::new();
        let (url, server) = server(signed_response(StatusCode::NO_CONTENT));
        add_remote(&sandbox, &url);
        let output = run(&sandbox, &args);
        server.join().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        if args[0] != "-v" {
            assert!(
                output.stderr.is_empty(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        } else {
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(stderr.contains("remote \"origin\" OK"));
            assert!(stderr.contains("verified signed response"));
            let rtt = stderr
                .split_once("RTT: ")
                .unwrap()
                .1
                .split_once(" ms")
                .unwrap()
                .0
                .parse::<f64>()
                .unwrap();
            assert!(rtt.is_finite() && rtt > 0.0);
        }
    }
}

#[test]
fn failed_checks_report_errors_without_a_success_report() {
    for failure in ["unsigned", "tampered", "http"] {
        let sandbox = Sandbox::new();
        let mut response = signed_response(if failure == "http" {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::OK
        });
        if failure == "unsigned" {
            response.headers_mut().remove("signature");
        }
        if failure == "tampered" {
            *response.body_mut() = b"changed".to_vec();
        }
        let (url, server) = server(response);
        add_remote(&sandbox, &url);
        let args: &[&str] = if failure == "unsigned" {
            &["remote", "check", "origin"]
        } else {
            &["-v", "remote", "check", "origin"]
        };
        let output = run(&sandbox, args);
        server.join().unwrap();
        let expected = if failure == "http" {
            EX_UNAVAILABLE
        } else {
            EX_DATAERR
        };
        assert_eq!(
            output.status.code(),
            Some(expected as i32),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("failed to check remote \"origin\""));
        assert!(!stderr.contains("RTT:") && !stderr.contains(" OK"));
    }
}

#[test]
fn signing_key_failures_prevent_network_requests() {
    let sandbox = Sandbox::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    add_remote(
        &sandbox,
        &format!("http://{}", listener.local_addr().unwrap()),
    );
    let output = sandbox
        .command()
        .args(["remote", "check", "origin"])
        .env("ASIMOV_KEYRING_BACKEND", "invalid")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(EX_UNAVAILABLE as i32));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to load local signing key"));
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
}
