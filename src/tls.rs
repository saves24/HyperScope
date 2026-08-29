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
    crate::atomic_write(CERT_FILE, &cert_pem, 0o644)?;
    crate::atomic_write(KEY_PRIV_FILE, &key_pem, 0o600)?;
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
pub(crate) fn load_trust() -> Result<Vec<String>, String> {
    match std::fs::read_to_string(TRUST_FILE) {
        Ok(content) => serde_json::from_str::<Vec<String>>(&content)
            .map_err(|e| format!("failed to parse trust list: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("failed to read trust list: {e}")),
    }
}
pub(crate) fn save_trust(list: &[String]) -> Result<(), String> {
    let _ = fs::create_dir_all(KEY_DIR);
    let content =
        serde_json::to_string(list).map_err(|e| format!("failed to serialize trust list: {e}"))?;
    crate::atomic_write(TRUST_FILE, &content, 0o600)
}
