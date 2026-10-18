// This is free and unencumbered software released into the public domain.

#![cfg(all(unix, feature = "source"))]

extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use asimov_http::{
    Component, SecretKey, SignatureInput, SignatureParams, Signer, VerificationPolicy, Verifier,
    body::{sign_buffered_response, verify_request},
    http::{Request, Response},
    key_id,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use clientele::SysexitsError::*;
use core::time::Duration;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt, symlink},
    process::{Command, Stdio},
    thread::{self, JoinHandle},
    time::Instant,
};
use temp_dir::TempDir;

struct Sandbox(TempDir);

impl Sandbox {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let user = whoami::username().unwrap_or_else(|_| "default".into());
        let secrets = root.child(format!(".asimov/.keyring/{user}"));
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&secrets)
            .unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(secrets.join("secret"))
            .unwrap()
            .write_all(&[7; 32])
            .unwrap();
        std::fs::create_dir_all(root.child(".asimov/keyring")).unwrap();
        // A valid but stale public key must be repaired before bearer selection.
        std::fs::write(
            root.child(format!(".asimov/keyring/{user}")),
            asimov_id::PublicKey::from(SecretKey::from_bytes(&[8; 32]).public()).to_string(),
        )
        .unwrap();
        std::fs::create_dir_all(root.child("bin")).unwrap();
        Self(root)
    }

    fn install(&self, enabled: bool, programs: &[&str], script: &str) {
        let installed = self.0.child(".asimov/modules/installed/local-test");
        std::fs::create_dir_all(&installed).unwrap();
        let programs: Vec<_> = programs
            .iter()
            .map(|program| format!("asimov-local-test-{program}"))
            .collect();
        std::fs::write(
            installed.join("manifest.json"),
            json!({
                "name": "local-test",
                "provides": { "programs": programs },
                "handles": { "url_prefixes": ["https://local.example/"] },
            })
            .to_string(),
        )
        .unwrap();
        if enabled {
            std::fs::create_dir_all(self.0.child(".asimov/modules/enabled")).unwrap();
            symlink(
                &installed,
                self.0.child(".asimov/modules/enabled/local-test"),
            )
            .unwrap();
        }
        for program in programs {
            let path = self.0.child(format!("bin/{program}"));
            std::fs::write(&path, script).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
        command
            .current_dir(self.0.path())
            .env("HOME", self.0.path())
            .env("ASIMOV_ROOT", self.0.child(".asimov"))
            .env("ASIMOV_KEYRING_BACKEND", "file")
            .env("DO_NOT_TRACK", "1")
            .env("PATH", self.0.child("bin"))
            .env("NO_PROXY", "*")
            .env("no_proxy", "*")
            .env_remove("ASIMOV_RELAY_URL")
            .env_remove("ASIMOV_RELAY_SECRET")
            .env_remove("TYPESAFE_API_TOKEN")
            .stdin(Stdio::null());
        command
    }
}

fn read_request(stream: &mut TcpStream) -> Request<Vec<u8>> {
    let mut reader = BufReader::new(stream);
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

fn signed_response(body: &[u8]) -> Response<Vec<u8>> {
    let key = SecretKey::from_bytes(&[42; 32]);
    let signer = Signer::new(
        key.clone(),
        "asimov",
        SignatureInput::new(
            ["@status", "content-type", "content-digest"]
                .into_iter()
                .map(|name| Component::new(name).unwrap())
                .collect(),
            SignatureParams::fresh(&key.public(), "relay-response").unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let mut response = Response::builder()
        .header("content-type", "application/jsonl")
        .body(body.to_vec())
        .unwrap();
    sign_buffered_response(&mut response, None, &signer).unwrap();
    response
}

fn mock_relay(responses: Vec<Response<Vec<u8>>>) -> (String, JoinHandle<Vec<Request<Vec<u8>>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        let key = SecretKey::from_bytes(&[42; 32]).public();
        let jwks = Response::builder()
            .header("content-type", "application/json")
            .body(
                serde_json::to_vec(&json!({ "keys": [{
                    "kty": "OKP", "crv": "Ed25519", "kid": key_id(&key),
                    "x": URL_SAFE_NO_PAD.encode(key.as_bytes()),
                }] }))
                .unwrap(),
            )
            .unwrap();
        let mut requests = Vec::new();
        for (index, mut response) in core::iter::once(jwks).chain(responses).enumerate() {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "no relay request received");
                        thread::sleep(Duration::from_millis(5));
                    },
                    Err(error) => panic!("accepting request: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let request = read_request(&mut stream);
            if index == 0 {
                assert_eq!(request.method(), "GET");
                assert_eq!(request.uri(), "/.well-known/jwks.json");
                assert!(!request.headers().contains_key("authorization"));
                assert!(!request.headers().contains_key("signature"));
            } else {
                requests.push(request);
            }
            if !response.headers().contains_key("transfer-encoding") {
                let length = response.body().len().into();
                response.headers_mut().insert("content-length", length);
            }
            write!(
                stream,
                "HTTP/1.1 {}\r\nConnection: close\r\n",
                response.status()
            )
            .unwrap();
            for (name, value) in response.headers() {
                write!(stream, "{name}: {}\r\n", value.to_str().unwrap()).unwrap();
            }
            stream.write_all(b"\r\n").unwrap();
            stream.write_all(response.body()).unwrap();
        }
        requests
    });
    (url, server)
}

#[test]
fn signs_and_verifies_relay_requests_and_forwards_remote_options() {
    let key = SecretKey::from_bytes(&[7; 32]);
    let id = key_id(&key.public());
    let public_key = id.strip_prefix("ed25519:").unwrap();
    let verifier =
        Verifier::new(key.public(), &id, "asimov", VerificationPolicy::request()).unwrap();
    for operation in ["fetch", "list"] {
        for token in [None, Some("explicit-token")] {
            let sandbox = Sandbox::new();
            let (url, server) = mock_relay(vec![signed_response(b"123"); 2]);
            let mut command = sandbox.command();
            command
                .env("ASIMOV_RELAY_URL", format!("{url}/api/relay"))
                .args([
                    "source",
                    operation,
                    "--output=jsonl",
                    "--max-age=1m 30s",
                    "--deadline=0.5s",
                    "--jev=Keep matching entries",
                    "--json=name",
                    "--jq=.name",
                ]);
            if let Some(token) = token {
                command.env("ASIMOV_RELAY_SECRET", token);
            }
            let mut options = json!({
                "output": "jsonl", "max-age": "1m 30s", "deadline": "500ms",
                "jev": "Keep matching entries", "jq": ".name",
                "json": ["name"],
            });
            if operation == "list" {
                command.args([
                    "--sort=-name",
                    "--limit=2",
                    "--before=HTTPS://Example.COM/a%2fb?x=a=b#end",
                    "--after=urn:entry:1",
                ]);
                options["sort"] = json!("-name");
                options["limit"] = json!(2);
                options["before"] = json!("HTTPS://Example.COM/a%2fb?x=a=b#end");
                options["after"] = json!("urn:entry:1");
            }
            let inputs = [
                "https://remote.example/first",
                "https://remote.example/second",
            ];
            let output = command.args(inputs).output().unwrap();
            assert!(output.status.success(), "{output:?}");
            // No local credentials or second selection/jq pass; append final LFs.
            assert_eq!(output.stdout, b"123\n123\n");
            let requests = server.join().unwrap();
            let mut nonces = Vec::new();
            let mut seen = Vec::new();
            for request in requests {
                assert_eq!(request.method(), "POST");
                assert_eq!(request.uri().path(), format!("/api/relay/{operation}"));
                assert_eq!(
                    request.headers()["authorization"],
                    format!("Bearer {}", token.unwrap_or(public_key))
                );
                assert_eq!(request.headers()["te"], "trailers");
                let signature = verify_request(
                    &request,
                    &format!("{url}{}", request.uri()),
                    &verifier,
                    SignatureParams::now().unwrap(),
                )
                .unwrap();
                nonces.push(String::from(signature.input.parameter("nonce").unwrap()));
                let body: Value = serde_json::from_slice(request.body()).unwrap();
                assert_eq!(body["options"], options);
                seen.push(if operation == "fetch" {
                    body["urls"][0].clone()
                } else {
                    body["url"].clone()
                });
            }
            seen.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            assert_eq!(seen, inputs.map(|input| json!(input)));
            assert_eq!(nonces.len(), 2);
            assert_ne!(nonces[0], nonces[1]);
        }
    }
}

#[test]
fn enabled_handlers_and_explicit_modules_prevent_relay_fallback() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    for operation in ["fetch", "list"] {
        for (enabled, programs, explicit, exit, expected) in [
            (true, vec!["fetcher", "lister"], false, 0, EX_OK),
            (true, vec!["fetcher", "lister"], false, 66, EX_NOINPUT),
            (true, vec!["reader"], false, 0, EX_UNAVAILABLE),
            (false, vec!["fetcher", "lister"], true, 0, EX_UNAVAILABLE),
            (true, vec!["fetcher", "lister"], true, 0, EX_SOFTWARE),
        ] {
            let sandbox = Sandbox::new();
            sandbox.install(enabled, &programs, &format!("#!/bin/sh\nexit {exit}\n"));
            let mut command = sandbox.command();
            command
                // Cached public identity permits startup, but signing must not run.
                .env("ASIMOV_KEYRING_BACKEND", "invalid")
                .env("ASIMOV_RELAY_URL", format!("http://{}", listener.local_addr().unwrap()))
                .args(["source", operation]);
            if explicit {
                command.arg(if enabled {
                    "--module=missing"
                } else {
                    "--module=local-test"
                });
            }
            let output = command.arg("https://local.example/data").output().unwrap();
            assert_eq!(output.status.code(), Some(expected as i32), "{output:?}");
            assert!(
                matches!(listener.accept(), Err(error) if error.kind() == ErrorKind::WouldBlock)
            );
        }
    }
}

#[test]
fn relay_is_opt_in_and_disabled_modules_do_not_block_it() {
    for operation in ["fetch", "list"] {
        let sandbox = Sandbox::new();
        sandbox.install(false, &["fetcher", "lister"], "#!/bin/sh\nexit 99\n");
        let output = sandbox
            .command()
            .env("ASIMOV_RELAY_SECRET", "token")
            .args(["source", operation, "https://local.example/data"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(EX_UNAVAILABLE as i32),
            "{output:?}"
        );
        let (url, server) = mock_relay(vec![signed_response(b"{}\n")]);
        let output = sandbox
            .command()
            .env("ASIMOV_RELAY_URL", url)
            .args(["source", operation, "https://local.example/data"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"{}\n");
        assert_eq!(server.join().unwrap().len(), 1);
    }
}

#[test]
fn mixed_sources_preserve_input_order_and_filter_only_local_output() {
    for operation in ["fetch", "list"] {
        let sandbox = Sandbox::new();
        sandbox.install(
            true,
            &["fetcher", "lister"],
            "#!/bin/sh\nprintf '%s\\n' '{\"name\":\"local\"}'\n",
        );
        let (url, server) = mock_relay(vec![signed_response(b"\"remote\"")]);
        let output = sandbox
            .command()
            .env("ASIMOV_RELAY_URL", url)
            .args([
                operation,
                "--json=name",
                "--jq=.name",
                "https://LOCAL.example/first",
                "https://remote.example/second",
                "https://local.example/third",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"\"local\"\n\"remote\"\n\"local\"\n");
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 1);
        let body: Value = serde_json::from_slice(requests[0].body()).unwrap();
        assert_eq!(body["options"]["json"], json!(["name"]));
    }
}

#[test]
fn remote_status_and_terminal_stream_failures_are_reported() {
    for operation in ["fetch", "list"] {
        for failure in ["status", "unsigned", "tampered", "truncated"] {
            let sandbox = Sandbox::new();
            let mut response = signed_response(b"{\"id\":1}\n");
            match failure {
                "status" => *response.status_mut() = 503.try_into().unwrap(),
                "unsigned" => {
                    response.headers_mut().remove("signature");
                },
                "tampered" => *response.body_mut() = b"{\"id\":2}\n".to_vec(),
                "truncated" => {
                    response
                        .headers_mut()
                        .insert("transfer-encoding", "chunked".parse().unwrap());
                    *response.body_mut() = b"9\r\n{\"id\":1}\n\r\n".to_vec();
                },
                _ => unreachable!(),
            }
            let (url, server) = mock_relay(vec![response]);
            let output = sandbox
                .command()
                .env("ASIMOV_RELAY_URL", url)
                .args(["source", operation])
                // Reaching the requested count must not skip EOF verification.
                .args((operation == "list").then_some("--limit=1"))
                .arg("https://remote.example/data")
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(EX_UNAVAILABLE as i32),
                "{failure}: {output:?}"
            );
            assert!(!output.stderr.is_empty());
            assert_eq!(server.join().unwrap().len(), 1);
        }
    }
}

#[test]
fn signing_failures_prevent_requests_even_with_an_explicit_bearer() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    for operation in ["fetch", "list"] {
        let sandbox = Sandbox::new();
        let output = sandbox
            .command()
            .env("ASIMOV_KEYRING_BACKEND", "invalid")
            .env(
                "ASIMOV_RELAY_URL",
                format!("http://{}", listener.local_addr().unwrap()),
            )
            .env("ASIMOV_RELAY_SECRET", "explicit-token")
            .args(["source", operation, "https://remote.example/data"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(EX_UNAVAILABLE as i32),
            "{output:?}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("failed to load local signing key")
        );
        assert!(output.stdout.is_empty());
        assert!(matches!(listener.accept(), Err(error) if error.kind() == ErrorKind::WouldBlock));
    }
}
