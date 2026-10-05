// This is free and unencumbered software released into the public domain.

use tokio::io::{AsyncRead, AsyncReadExt};

/// Waits for a complete HTTP/1 header block in a local test request.
pub(crate) async fn read_request_headers(reader: &mut (impl AsyncRead + Unpin)) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let mut request = [0; 4096];
        let mut length = 0;
        loop {
            assert!(
                length < request.len(),
                "test request headers exceed 4096 bytes"
            );
            let count = reader.read(&mut request[length..]).await.unwrap();
            assert_ne!(count, 0, "EOF before test request headers completed");
            length += count;
            if request[..length]
                .windows(4)
                .any(|bytes| bytes == b"\r\n\r\n")
            {
                break;
            }
        }
    })
    .await
    .expect("timed out reading test request headers");
}
