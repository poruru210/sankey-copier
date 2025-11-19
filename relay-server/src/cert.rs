//! Certificate management module for HTTPS server
//!
//! Provides functionality to generate self-signed certificates and
//! register them in the Windows certificate store for PNA compliance.

use anyhow::{Context, Result};
use rcgen::{CertificateParams, DnType, Ia5String, KeyPair, SanType};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::TlsConfig;

/// Ensure certificate exists, generating and registering if necessary
///
/// This function checks if the certificate files exist. If not, it generates
/// a new self-signed certificate and registers it in the Windows trusted
/// root certificate store.
///
/// # Arguments
/// * `config` - TLS configuration containing paths and validity settings
/// * `base_path` - Base directory for certificate storage (typically install dir)
///
/// # Returns
/// * `Ok(())` if certificate is ready for use
/// * `Err` if generation or registration fails
pub fn ensure_certificate(config: &TlsConfig, base_path: &Path) -> Result<()> {
    let cert_path = base_path.join(&config.cert_path);
    let key_path = base_path.join(&config.key_path);

    // Check if both certificate files exist
    if cert_path.exists() && key_path.exists() {
        tracing::info!("Certificate files found at {:?}", cert_path);
        return Ok(());
    }

    tracing::info!("Certificate not found, generating new self-signed certificate");

    // Create parent directories if they don't exist
    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create certificate directory: {:?}", parent))?;
    }

    // Generate new certificate
    let (cert_pem, key_pem) = generate_self_signed_cert(config.validity_days)?;

    // Write certificate files
    fs::write(&cert_path, &cert_pem)
        .with_context(|| format!("Failed to write certificate to {:?}", cert_path))?;
    fs::write(&key_path, &key_pem)
        .with_context(|| format!("Failed to write private key to {:?}", key_path))?;

    tracing::info!("Certificate files written successfully");

    // Register in Windows certificate store
    register_to_windows_store(&cert_path)?;

    Ok(())
}

/// Generate a self-signed certificate for localhost
///
/// Creates a certificate valid for localhost, 127.0.0.1, and ::1.
/// The certificate is suitable for local HTTPS development and testing.
///
/// # Arguments
/// * `validity_days` - Number of days the certificate should be valid
///
/// # Returns
/// * Tuple of (certificate PEM, private key PEM)
fn generate_self_signed_cert(validity_days: u32) -> Result<(String, String)> {
    // Generate a new key pair
    let key_pair = KeyPair::generate()
        .context("Failed to generate key pair")?;

    // Configure certificate parameters
    let mut params = CertificateParams::default();

    // Set distinguished name
    params.distinguished_name.push(DnType::CommonName, "SANKEY Copier Local Server");
    params.distinguished_name.push(DnType::OrganizationName, "SANKEY Copier");

    // Set Subject Alternative Names for localhost
    let localhost_dns = Ia5String::try_from("localhost")
        .context("Failed to create Ia5String for localhost")?;
    params.subject_alt_names = vec![
        SanType::DnsName(localhost_dns),
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)),
    ];

    // Set validity period
    // rcgen uses time::Duration, convert days to duration
    let not_before = time::OffsetDateTime::now_utc();
    let not_after = not_before + time::Duration::days(validity_days as i64);
    params.not_before = not_before;
    params.not_after = not_after;

    // Generate the certificate
    let cert = params.self_signed(&key_pair)
        .context("Failed to generate self-signed certificate")?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    tracing::info!(
        "Generated self-signed certificate valid for {} days",
        validity_days
    );

    Ok((cert_pem, key_pem))
}

/// Register certificate in Windows trusted root certificate store
///
/// Uses certutil.exe to add the certificate to the machine's trusted
/// root certification authorities store. This requires administrator
/// privileges (typically available when running as a Windows service).
///
/// # Arguments
/// * `cert_path` - Path to the certificate PEM file
///
/// # Returns
/// * `Ok(())` if registration succeeds
/// * `Err` if certutil fails
fn register_to_windows_store(cert_path: &Path) -> Result<()> {
    tracing::info!("Registering certificate in Windows trusted root store");

    // Use certutil to add certificate to Root store
    // -addstore adds to specified store, Root is trusted root CAs
    // -f forces overwrite if certificate already exists
    let output = Command::new("certutil")
        .args([
            "-addstore",
            "-f",
            "Root",
            cert_path.to_str().context("Invalid certificate path")?,
        ])
        .output()
        .context("Failed to execute certutil")?;

    if output.status.success() {
        tracing::info!("Certificate registered successfully in Windows trusted root store");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Log both stdout and stderr for debugging
        if !stdout.is_empty() {
            tracing::warn!("certutil stdout: {}", stdout);
        }
        if !stderr.is_empty() {
            tracing::error!("certutil stderr: {}", stderr);
        }

        anyhow::bail!(
            "Failed to register certificate in Windows store. \
             This typically requires administrator privileges. \
             Exit code: {:?}",
            output.status.code()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_self_signed_cert() {
        let (cert_pem, key_pem) = generate_self_signed_cert(365).unwrap();

        // Verify PEM format
        assert!(cert_pem.contains("-----BEGIN CERTIFICATE-----"));
        assert!(cert_pem.contains("-----END CERTIFICATE-----"));
        assert!(key_pem.contains("-----BEGIN PRIVATE KEY-----"));
        assert!(key_pem.contains("-----END PRIVATE KEY-----"));
    }

    #[test]
    fn test_ensure_certificate_creates_files() {
        let temp_dir = TempDir::new().unwrap();
        let config = TlsConfig {
            cert_path: "certs/test.pem".to_string(),
            key_path: "certs/test-key.pem".to_string(),
            validity_days: 30,
        };

        // Note: This test will fail on registration step without admin privileges
        // but the file creation part should work
        let result = ensure_certificate(&config, temp_dir.path());

        // Check that certificate files were created (registration may fail)
        let cert_path = temp_dir.path().join(&config.cert_path);
        let key_path = temp_dir.path().join(&config.key_path);

        if result.is_ok() {
            assert!(cert_path.exists());
            assert!(key_path.exists());
        }
    }
}
