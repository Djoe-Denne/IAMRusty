//! `wiremock` remote `Sign` / `GetPublicKey` mocks (ADR-0309).

use rustycog::testing::wiremock::MockServerFixture;
use std::sync::Arc;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::resources::*;

/// Only for wire-level cases WireMock normalizes (chunking, EOF and slow bodies).
/// One connection, ephemeral loopback bind; no container or process-global state.
#[derive(Clone, Copy)]
pub enum Framing {
    Length,
    Chunked,
    Close,
}

pub struct RawResponse {
    pub status: u16,
    pub body: Vec<u8>,
    pub framing: Framing,
    pub headers: Vec<(String, String)>,
    pub header_delay: std::time::Duration,
    pub chunk_delay: std::time::Duration,
    pub truncate_after: Option<usize>,
}

impl RawResponse {
    pub fn ok(body: Vec<u8>, framing: Framing) -> Self {
        Self {
            status: 200,
            body,
            framing,
            headers: vec![],
            header_delay: std::time::Duration::ZERO,
            chunk_delay: std::time::Duration::ZERO,
            truncate_after: None,
        }
    }
}

pub struct RawSigner {
    pub url: String,
    pub requests: Arc<std::sync::atomic::AtomicUsize>,
    task: Option<tokio::task::JoinHandle<std::io::Result<()>>>,
}

impl RawSigner {
    pub async fn start(response: RawResponse) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = requests.clone();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await?;
            serve_raw(stream, response, seen).await
        });
        Self {
            url,
            requests,
            task: Some(task),
        }
    }

    pub async fn shutdown(mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Drop for RawSigner {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

pub async fn serve_raw<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    mut stream: S,
    response: RawResponse,
    requests: Arc<std::sync::atomic::AtomicUsize>,
) -> std::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut request = Vec::new();
    let header_end = loop {
        let byte = stream.read_u8().await?;
        request.push(byte);
        if request.ends_with(b"\r\n\r\n") {
            break request.len();
        }
        if request.len() > 16384 {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
    };
    let headers = std::str::from_utf8(&request).map_err(|_| std::io::ErrorKind::InvalidData)?;
    let length = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map_or(0, |(_, value)| value.trim().parse::<usize>().unwrap());
    if length > 16384 {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    request.resize(header_end + length, 0);
    stream.read_exact(&mut request[header_end..]).await?;
    requests.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    tokio::time::sleep(response.header_delay).await;
    let mut head = format!(
        "HTTP/1.1 {} Fixture\r\nConnection: close\r\n",
        response.status
    );
    match response.framing {
        Framing::Length => head.push_str(&format!("Content-Length: {}\r\n", response.body.len())),
        Framing::Chunked => head.push_str("Transfer-Encoding: chunked\r\n"),
        Framing::Close => {}
    }
    for (name, value) in response.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).await?;
    stream.flush().await?;
    let delivered = &response.body[..response.truncate_after.unwrap_or(response.body.len())];
    for chunk in delivered.chunks(128) {
        tokio::time::sleep(response.chunk_delay).await;
        if matches!(response.framing, Framing::Chunked) {
            stream
                .write_all(format!("{:X}\r\n", chunk.len()).as_bytes())
                .await?;
        }
        stream.write_all(chunk).await?;
        if matches!(response.framing, Framing::Chunked) {
            stream.write_all(b"\r\n").await?;
        }
        stream.flush().await?;
    }
    if matches!(response.framing, Framing::Chunked) && response.truncate_after.is_none() {
        stream.write_all(b"0\r\n\r\n").await?;
    }
    stream.shutdown().await
}

pub struct RemoteSignerMockService {
    server: Arc<MockServer>,
    fixture: MockServerFixture,
}

impl RemoteSignerMockService {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self { server, fixture }
    }

    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    pub async fn reset(&self) {
        self.fixture.reset().await;
    }

    pub async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.server.received_requests().await.unwrap_or_default()
    }

    pub async fn mock_response(
        &self,
        method_name: &str,
        request_path: &str,
        response: ResponseTemplate,
    ) {
        Mock::given(method(method_name))
            .and(path(request_path))
            .respond_with(response)
            .mount(&self.server)
            .await;
    }

    pub async fn mock_sign_ok(&self, signature_b64: &str) -> &Self {
        Mock::given(method("POST"))
            .and(path("/sign"))
            .respond_with(ResponseTemplate::new(200).set_body_json(SignResponseBody {
                signature: signature_b64.to_string(),
            }))
            .mount(&self.server)
            .await;
        self
    }

    pub async fn mock_get_public_key_ok(&self, public_key_pem: &str) -> &Self {
        self.mock_get_public_key_ok_for("kid-1", public_key_pem)
            .await
    }

    pub async fn mock_get_public_key_ok_for(&self, key_id: &str, public_key_pem: &str) -> &Self {
        Mock::given(method("GET"))
            .and(path(format!("/keys/{key_id}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(PublicKeyResponseBody {
                    public_key: public_key_pem.to_string(),
                }),
            )
            .mount(&self.server)
            .await;
        self
    }
}
