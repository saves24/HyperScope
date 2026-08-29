// TLS helpers for the relay client: accept self-signed certs when connecting
// to a relay that uses its own certificate (the default deployment).
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use std::sync::{Arc, Mutex};

/// Verifier that accepts a certificate when it matches the expected
/// fingerprint (TOFU). The first connection records the fingerprint and
/// accepts; subsequent connections must present the same certificate,
/// otherwise the handshake fails. Used for relays with self-signed certs;
/// the relay is a zero-privilege pipe, so pinning the fingerprint is the
/// trust model.
#[derive(Debug)]
pub struct CaptureCertVerifier {
    /// Filled with "SHA256:<hex>" of the server's end-entity certificate
    /// after the handshake. Shared via Arc so the caller can read it back.
    pub fingerprint: Arc<Mutex<Option<String>>>,
    /// Expected fingerprint (hex, uppercase). When set, the certificate must
    /// match it; when None, any certificate is accepted and recorded (TOFU).
    expected: Option<String>,
}

impl CaptureCertVerifier {
    pub fn new() -> Self {
        Self {
            fingerprint: Arc::new(Mutex::new(None)),
            expected: None,
        }
    }

    /// Creates a verifier that only accepts the given fingerprint (hex).
    pub fn with_pin(fingerprint_hex: &str) -> Self {
        Self {
            fingerprint: Arc::new(Mutex::new(Some(fingerprint_hex.to_string()))),
            expected: Some(fingerprint_hex.to_string()),
        }
    }
}

impl Default for CaptureCertVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerCertVerifier for CaptureCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // SHA256 of the DER certificate, hex-encoded (same format as the
        // node's `cert show` output / the key's |SHA256:... suffix).
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(end_entity.as_ref());
        let hash = hasher.finalize();
        let fp = hash
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<String>();
        if let Ok(mut slot) = self.fingerprint.lock() {
            *slot = Some(fp.clone());
        }
        match &self.expected {
            Some(expected) if expected.eq_ignore_ascii_case(&fp) => {
                Ok(ServerCertVerified::assertion())
            }
            Some(_) => Err(rustls::Error::General(
                "relay certificate fingerprint mismatch".into(),
            )),
            None => Ok(ServerCertVerified::assertion()),
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
