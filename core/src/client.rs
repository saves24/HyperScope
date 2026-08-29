use crate::{set_private_file_mode, SharedState};
// Minimal HTTP/TLS client with cert fingerprint verification
use serde_json::Value;
use std::time::Duration;

// Panel client certificate (used when connecting to nodes with mTLS)
#[cfg(target_os = "linux")]
pub const CLIENT_CERT_FILE: &str = "/etc/hyper-panel/client-cert.pem";
#[cfg(target_os = "windows")]
pub const CLIENT_CERT_FILE: &str = "C:\\ProgramData\\hyper-panel\\client-cert.pem";
#[cfg(target_os = "linux")]
pub const CLIENT_KEY_FILE: &str = "/etc/hyper-panel/client-key.pem";
#[cfg(target_os = "windows")]
pub const CLIENT_KEY_FILE: &str = "C:\\ProgramData\\hyper-panel\\client-key.pem";

// Minimal stream abstraction (TLS or plaintext)
pub trait AsyncReadWrite: tokio::io::AsyncRead + tokio::io::AsyncWrite {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite> AsyncReadWrite for T {}

pub async fn tls_connect(
    host: &str,
    port: u16,
    cert_fp: &str,
    timeout: Duration,
) -> Result<
    (
        tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
        String,
    ),
    String,
> {
    let addr = format!("{host}:{port}");
    let connect = tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&addr)).await;
    let stream = connect
        .map_err(|_| "connection timeout".to_string())?
        .map_err(|e| format!("connect failed: {e}"))?;

    // Use panel client certificate config (server can verify panel identity)
    let mut config = client_auth_config().map_err(|e| format!("client cert config failed: {e}"))?;
    // Adjust verifier per node cert fingerprint (TOFU: accept any when fingerprint empty)
    let verifier = std::sync::Arc::new(FingerprintVerifier {
        expected_fp: cert_fp.to_string(),
    });
    config.dangerous().set_certificate_verifier(verifier);

    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|_| "invalid server name".to_string())?;
    let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
    let tls = tokio::time::timeout(timeout, connector.connect(server_name, stream))
        .await
        .map_err(|_| "TLS handshake timeout".to_string())?
        .map_err(|e| format!("TLS handshake failed: {e}"))?;

    // Get peer certificate fingerprint
    let fp = tls
        .get_ref()
        .1
        .peer_certificates()
        .and_then(|certs| certs.first())
        .map(|cert| {
            use sha2::Digest;
            let digest = sha2::Sha256::digest(cert.as_ref());
            let hex: String = digest.iter().map(|b| format!("{b:02X}")).collect();
            format!("SHA256:{hex}")
        })
        .unwrap_or_default();

    // Trust on first use when cert_fp is empty; otherwise must match (FingerprintVerifier already checked)
    if cert_fp.is_empty() {
        return Ok((tls, fp));
    }
    if !fp.is_empty() && fp != cert_fp {
        return Err(format!(
            "certificate fingerprint mismatch (expected {cert_fp}, got {fp})"
        ));
    }
    Ok((tls, fp))
}
// Connection + request options bundle (avoids long arg lists)
pub struct HttpOptions<'a> {
    pub host: &'a str,
    pub port: u16,
    pub method: &'a str,
    pub path: &'a str,
    pub key: &'a str,
    pub timeout: Duration,
    pub tls: bool,
    pub cert_fp: &'a str,
    pub use_pool: bool,
}

pub async fn http_request(app: &SharedState, o: HttpOptions<'_>) -> Result<Value, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    const MAX_RESPONSE_BODY: usize = 4 * 1024 * 1024;

    let conn_key = format!(
        "{}:{}:{}",
        o.host,
        o.port,
        if o.tls { "tls" } else { "plain" }
    );
    let conn_hdr = if o.use_pool { "keep-alive" } else { "close" };

    // Take connection from pool (keep-alive reuse), create if absent; POST always creates fresh
    let mut stream: Box<dyn AsyncReadWrite + Send + Unpin> = if o.use_pool {
        match app.conns.lock().await.remove(&conn_key) {
            Some(s) => s,
            None => connect_stream(o.host, o.port, o.timeout, o.tls, o.cert_fp).await?,
        }
    } else {
        connect_stream(o.host, o.port, o.timeout, o.tls, o.cert_fp).await?
    };

    let req = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Length: 0\r\nConnection: {}\r\n\r\n",
        o.method, o.path, o.host, o.key, conn_hdr
    );
    if stream.write_all(req.as_bytes()).await.is_err() {
        // Reused connection went stale; drop and reconnect
        if o.use_pool {
            let _ = stream;
            stream = connect_stream(o.host, o.port, o.timeout, o.tls, o.cert_fp).await?;
            stream
                .write_all(req.as_bytes())
                .await
                .map_err(|e| format!("send failed: {e}"))?;
        } else {
            return Err("send failed".to_string());
        }
    }

    // Read response headers (until \r\n\r\n), then read body by Content-Length
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = tokio::time::timeout(o.timeout, stream.read(&mut byte))
            .await
            .map_err(|_| "read timeout".to_string())?
            .map_err(|e| format!("read failed: {e}"))?;
        if read == 0 {
            return Err("connection closed".to_string());
        }
        head.push(byte[0]);
        if head.len() >= 4 && &head[head.len() - 4..] == b"\r\n\r\n" {
            break;
        }
        if head.len() > 65536 {
            return Err("response header too large".to_string());
        }
    }

    let head_text = String::from_utf8_lossy(&head);
    let status_line = head_text.lines().next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if status == 401 {
        return Err("unauthorized".to_string());
    }
    if status != 200 {
        return Err(format!("HTTP {status}"));
    }

    let content_len: Option<usize> = head_text
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok());

    let mut body = Vec::new();
    match content_len {
        Some(len) => {
            if len > MAX_RESPONSE_BODY {
                return Err("response body too large".to_string());
            }
            body.reserve(len);
            while body.len() < len {
                let remaining = len - body.len();
                let mut chunk = vec![0u8; remaining];
                let read = tokio::time::timeout(o.timeout, stream.read(&mut chunk))
                    .await
                    .map_err(|_| "read timeout".to_string())?
                    .map_err(|e| format!("read failed: {e}"))?;
                if read == 0 {
                    break;
                }
                body.extend_from_slice(&chunk[..read]);
            }
            // Body fully read -> GET connection reusable, return to pool
            if body.len() == len && o.use_pool {
                app.conns.lock().await.insert(conn_key, stream);
            }
        }
        None => {
            let mut rest = Vec::new();
            tokio::time::timeout(o.timeout, stream.read_to_end(&mut rest))
                .await
                .map_err(|_| "read timeout".to_string())?
                .map_err(|e| format!("read failed: {e}"))?;
            if rest.len() > MAX_RESPONSE_BODY {
                return Err("response body too large".to_string());
            }
            body = rest;
        }
    }

    serde_json::from_slice(&body).map_err(|e| format!("parse failed: {e}"))
}
pub async fn connect_stream(
    host: &str,
    port: u16,
    timeout: Duration,
    tls: bool,
    cert_fp: &str,
) -> Result<Box<dyn AsyncReadWrite + Send + Unpin>, String> {
    if tls {
        let (tls_stream, _fp) = tls_connect(host, port, cert_fp, timeout).await?;
        Ok(Box::new(tls_stream))
    } else {
        let addr = format!("{host}:{port}");
        let connect = tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&addr)).await;
        let s = connect
            .map_err(|_| "connection timeout".to_string())?
            .map_err(|e| format!("connect failed: {e}"))?;
        Ok(Box::new(s))
    }
}
pub async fn http_request_json(app: &SharedState, o: HttpOptions<'_>) -> Result<Value, String> {
    let use_pool = o.method == "GET";
    let opts = HttpOptions { use_pool, ..o };
    http_request(app, opts).await
}
pub fn ensure_client_cert() -> Result<String, String> {
    if std::path::Path::new(CLIENT_CERT_FILE).exists()
        && std::path::Path::new(CLIENT_KEY_FILE).exists()
    {
        return cert_fingerprint_of(CLIENT_CERT_FILE);
    }
    let mut params = rcgen::CertificateParams::new(vec!["hyper-panel".to_string()])
        .map_err(|e| format!("certificate params error: {e}"))?;
    params.not_before = rcgen::date_time_ymd(2024, 1, 1);
    params.not_after = rcgen::date_time_ymd(2034, 1, 1);
    let key_pair = rcgen::KeyPair::generate().map_err(|e| format!("key generation failed: {e}"))?;
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("certificate generation failed: {e}"))?;
    crate::atomic_write(CLIENT_CERT_FILE, &cert.pem(), 0o644)?;
    crate::atomic_write(CLIENT_KEY_FILE, &key_pair.serialize_pem(), 0o600)?;
    set_private_file_mode(CLIENT_KEY_FILE)?;
    cert_fingerprint_of(CLIENT_CERT_FILE)
}
pub fn cert_fingerprint_of(path: &str) -> Result<String, String> {
    let pem =
        std::fs::read_to_string(path).map_err(|e| format!("failed to read certificate: {e}"))?;
    let cert = rustls_pemfile::certs(&mut pem.as_bytes())
        .next()
        .ok_or("certificate parse failed")?
        .map_err(|e| format!("certificate parse failed: {e}"))?;
    use sha2::Digest;
    let digest = sha2::Sha256::digest(&cert);
    let hex: String = digest.iter().map(|b| format!("{b:02X}")).collect();
    Ok(format!("SHA256:{hex}"))
}
pub fn client_auth_config() -> Result<rustls::ClientConfig, String> {
    ensure_client_cert()?;
    let certs_pem =
        std::fs::read(CLIENT_CERT_FILE).map_err(|e| format!("failed to read certificate: {e}"))?;
    let key_pem =
        std::fs::read(CLIENT_KEY_FILE).map_err(|e| format!("failed to read private key: {e}"))?;
    let certs = rustls_pemfile::certs(&mut certs_pem.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("certificate parse failed: {e}"))?;
    let key = rustls_pemfile::private_key(&mut key_pem.as_slice())
        .map_err(|e| format!("private key parse failed: {e}"))?
        .ok_or_else(|| "empty private key".to_string())?;
    let builder = rustls::ClientConfig::builder();
    let verifier = std::sync::Arc::new(FingerprintVerifier {
        expected_fp: String::new(),
    });
    let mut cfg = builder
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(certs, key)
        .map_err(|e| format!("client cert config failed: {e}"))?;
    cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(cfg)
}
pub async fn fetch_json(
    app: &SharedState,
    url: &str,
    key: &str,
    timeout: Duration,
    cert_fp: &str,
) -> Result<Value, String> {
    // url shaped like http(s)://host:port/path?query
    let (tls, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return Err("invalid URL format".to_string());
    };
    let (hostport, path) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (rest, "/".to_string()),
    };
    // IPv6 literal: [2001:db8::1]:5000 or [2001:db8::1]
    let (host, port) = if hostport.starts_with('[') {
        match hostport.find(']') {
            Some(end) => {
                let h = &hostport[1..end];
                let rest = &hostport[end + 1..];
                let p = rest
                    .strip_prefix(':')
                    .and_then(|r| r.parse::<u16>().ok())
                    .unwrap_or(80);
                (h.to_string(), p)
            }
            None => (
                hostport.strip_prefix('[').unwrap_or(hostport).to_string(),
                80,
            ),
        }
    } else {
        match hostport.split_once(':') {
            Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(80)),
            None => (hostport.to_string(), 80),
        }
    };
    http_request_json(
        app,
        HttpOptions {
            host: &host,
            port,
            method: "GET",
            path: &path,
            key,
            timeout,
            tls,
            cert_fp,
            use_pool: true,
        },
    )
    .await
}

#[derive(Debug)]
pub(crate) struct FingerprintVerifier {
    expected_fp: String,
}

impl rustls::client::danger::ServerCertVerifier for FingerprintVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        use sha2::Digest;
        let digest = sha2::Sha256::digest(end_entity.as_ref());
        let hex: String = digest.iter().map(|b| format!("{b:02X}")).collect();
        let fp = format!("SHA256:{hex}");
        if self.expected_fp.is_empty() || fp == self.expected_fp {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "certificate fingerprint mismatch".to_string(),
            ))
        }
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
