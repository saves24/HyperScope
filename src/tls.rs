use crate::{CERT_FILE, KEY_DIR, KEY_PRIV_FILE, TRUST_FILE};
// TLS certificate management and client trust verification
use std::fs;

pub(crate) fn gen_self_signed_cert() -> Result<(String, String), String> {
    let mut params = rcgen::CertificateParams::new(vec!["hyper-node".to_string()])
        .map_err(|e| format!("certificate params error: {e}"))?;
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now - time::Duration::days(1);
    params.not_after = now + time::Duration::days(3650); // ~10 years
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let key_pair = rcgen::KeyPair::generate().map_err(|e| format!("key generation failed: {e}"))?;
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("certificate generation failed: {e}"))?;
    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();
    Ok((cert_pem, key_pem))
}
pub(crate) fn ensure_cert() -> Result<(), String> {
    let cert_exists = std::path::Path::new(CERT_FILE).exists();
    let key_exists = std::path::Path::new(KEY_PRIV_FILE).exists();
    if cert_exists && key_exists {
        return Ok(());
    }
    let (cert_pem, key_pem) = gen_self_signed_cert()?;
    std::fs::write(CERT_FILE, cert_pem).map_err(|e| format!("failed to write certificate: {e}"))?;
    std::fs::write(KEY_PRIV_FILE, key_pem)
        .map_err(|e| format!("failed to write private key: {e}"))?;
    crate::chmod(KEY_PRIV_FILE, 0o600);
    println!("self-signed TLS certificate generated: {CERT_FILE}");
    println!("certificate fingerprint: {}", cert_fingerprint()?);
    Ok(())
}
pub(crate) fn cert_fingerprint() -> Result<String, String> {
    let pem = std::fs::read_to_string(CERT_FILE)
        .map_err(|e| format!("failed to read certificate: {e}"))?;
    let cert = rustls_pemfile::certs(&mut pem.as_bytes())
        .next()
        .ok_or("certificate parse failed")?
        .map_err(|e| format!("certificate parse failed: {e}"))?;
    use sha2::Digest;
    let digest = sha2::Sha256::digest(&cert);
    let hex: String = digest.iter().map(|b| format!("{b:02X}")).collect();
    Ok(format!("SHA256:{hex}"))
}
pub(crate) fn load_trust() -> Vec<String> {
    match std::fs::read_to_string(TRUST_FILE) {
        Ok(content) => serde_json::from_str::<Vec<String>>(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}
pub(crate) fn save_trust(list: &[String]) -> Result<(), String> {
    let _ = fs::create_dir_all(KEY_DIR);
    fs::write(TRUST_FILE, serde_json::to_string(list).unwrap_or_default())
        .map_err(|e| format!("failed to write trust list: {e}"))?;
    crate::chmod(TRUST_FILE, 0o600);
    Ok(())
}
pub(crate) fn tls_config() -> Result<rustls::ServerConfig, String> {
    let certs_pem =
        std::fs::read(CERT_FILE).map_err(|e| format!("failed to read certificate: {e}"))?;
    let key_pem =
        std::fs::read(KEY_PRIV_FILE).map_err(|e| format!("failed to read private key: {e}"))?;
    let certs = rustls_pemfile::certs(&mut certs_pem.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("certificate parse failed: {e}"))?;
    let key = rustls_pemfile::private_key(&mut key_pem.as_slice())
        .map_err(|e| format!("private key parse failed: {e}"))?
        .ok_or_else(|| "empty private key".to_string())?;
    let trust = load_trust();
    let cfg = if trust.is_empty() {
        // No trust list: client cert not required
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| format!("TLS config failed: {e}"))?
    } else {
        // Trust list configured: require trusted client cert (mutual verification)
        let verifier = std::sync::Arc::new(TrustVerifier {
            fingerprints: trust,
        });
        rustls::ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(certs, key)
            .map_err(|e| format!("TLS config failed: {e}"))?
    };
    let mut cfg = cfg;
    cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(cfg)
}

// Client cert verifier: check client cert fingerprint against trust list
#[derive(Debug)]
struct TrustVerifier {
    fingerprints: Vec<String>,
}

impl rustls::server::danger::ClientCertVerifier for TrustVerifier {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::server::danger::ClientCertVerified, rustls::Error> {
        use sha2::Digest;
        let digest = sha2::Sha256::digest(end_entity.as_ref());
        let hex: String = digest.iter().map(|b| format!("{b:02X}")).collect();
        let fp = format!("SHA256:{hex}");
        if self.fingerprints.iter().any(|t| t == &fp) {
            Ok(rustls::server::danger::ClientCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "untrusted client certificate".to_string(),
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

// Load TLS config (cert + key) for axum HTTPS
// Enable client cert verification (mutual TLS) when trust list is configured
