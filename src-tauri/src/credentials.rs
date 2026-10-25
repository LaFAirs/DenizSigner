//! Only public summaries cross IPC. PKCS#12 archives, passwords and tokens live
//! together in the OS credential store, never in app configuration or temp files.
use crate::{error::AppError, secure_storage::credential_entry};
use base64::{Engine, prelude::BASE64_STANDARD};
use openssl::{
    asn1::Asn1Time,
    hash::MessageDigest,
    pkcs12::{ParsedPkcs12_2, Pkcs12},
    x509::X509NameRef,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::State;
use zeroize::{Zeroize, Zeroizing};

const VAULT: &str = "credential-manager-v1";
const IDLE: Duration = Duration::from_secs(300);
#[derive(Default)]
pub struct CredentialAccess(Mutex<Option<Instant>>);
#[derive(Default, Serialize, Deserialize)]
struct Vault {
    profiles: Vec<Profile>,
    tokens: Vec<Token>,
    selected: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Profile {
    id: String,
    name: String,
    archive: String,
    password: String,
}
#[derive(Serialize, Deserialize)]
struct Token {
    id: String,
    name: String,
    value: String,
}
impl Drop for Profile {
    fn drop(&mut self) {
        self.archive.zeroize();
        self.password.zeroize();
    }
}
impl Drop for Token {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateSummary {
    id: String,
    name: String,
    subject: String,
    issuer: String,
    serial: String,
    fingerprint: String,
    not_before: String,
    expires: String,
    valid: bool,
    selected: bool,
}
#[derive(Serialize)]
pub struct TokenSummary {
    id: String,
    name: String,
}
#[derive(Serialize)]
pub struct VaultSummary {
    certificates: Vec<CertificateSummary>,
    tokens: Vec<TokenSummary>,
}
fn failure(s: &str) -> AppError {
    AppError::Keyring(s.into())
}
fn crypto_error(_: openssl::error::ErrorStack) -> AppError {
    failure("Invalid PKCS#12 archive, unsupported encryption, or incorrect certificate password.")
}
fn read_vault() -> Result<Vault, AppError> {
    let data = match credential_entry(VAULT)?.get_password() {
        Ok(v) => Zeroizing::new(v),
        Err(keyring::Error::NoEntry) => return Ok(Vault::default()),
        Err(_) => return Err(failure("Unlock the OS keyring to access credentials.")),
    };
    serde_json::from_str(&data)
        .map_err(|_| failure("Credential store is damaged; no data was overwritten."))
}
fn write_vault(vault: &Vault) -> Result<(), AppError> {
    let data = Zeroizing::new(
        serde_json::to_string(vault).map_err(|_| failure("Cannot encode credentials."))?,
    );
    credential_entry(VAULT)?
        .set_password(&data)
        .map_err(|_| failure("Secure credential storage failed. No file fallback is used."))
}
fn access(
    state: &CredentialAccess,
) -> Result<std::sync::MutexGuard<'_, Option<Instant>>, AppError> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| failure("Credential Manager is locked."))?;
    if !guard.is_some_and(|last| last.elapsed() < IDLE) {
        *guard = None;
        return Err(failure(
            "Credential Manager is locked. Unlock it again (5 minute idle timeout).",
        ));
    }
    *guard = Some(Instant::now());
    Ok(guard)
}
fn parse(der: &[u8], password: &str) -> Result<ParsedPkcs12_2, AppError> {
    let parsed = Pkcs12::from_der(der)
        .and_then(|p| p.parse2(password))
        .map_err(crypto_error)?;
    let cert = parsed
        .cert
        .as_ref()
        .ok_or_else(|| failure("Archive has no certificate."))?;
    let key = parsed
        .pkey
        .as_ref()
        .ok_or_else(|| failure("Archive has no private key."))?;
    if !cert.public_key().map_err(crypto_error)?.public_eq(key) {
        return Err(failure("Certificate and private key do not match."));
    }
    Ok(parsed)
}
fn valid(cert: &openssl::x509::X509Ref) -> Result<bool, AppError> {
    let now = Asn1Time::days_from_now(0).map_err(crypto_error)?;
    Ok(
        cert.not_before().compare(&now).map_err(crypto_error)? != Ordering::Greater
            && cert.not_after().compare(&now).map_err(crypto_error)? == Ordering::Greater,
    )
}
fn name(n: &X509NameRef) -> String {
    n.entries()
        .map(|e| {
            format!(
                "{}={}",
                e.object().nid().short_name().unwrap_or("?"),
                e.data().to_string().unwrap_or_else(|_| "?".into())
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}
fn decode(profile: &Profile) -> Result<ParsedPkcs12_2, AppError> {
    let der = Zeroizing::new(
        BASE64_STANDARD
            .decode(&profile.archive)
            .map_err(|_| failure("Invalid stored certificate."))?,
    );
    parse(&der, &profile.password)
}
fn summary(vault: &Vault) -> Result<VaultSummary, AppError> {
    let mut certificates = Vec::new();
    for p in &vault.profiles {
        let parsed = decode(p)?;
        let cert = parsed.cert.as_ref().unwrap();
        certificates.push(CertificateSummary {
            id: p.id.clone(),
            name: p.name.clone(),
            subject: name(cert.subject_name()),
            issuer: name(cert.issuer_name()),
            serial: cert
                .serial_number()
                .to_bn()
                .and_then(|n| n.to_hex_str())
                .map_err(crypto_error)?
                .to_string(),
            fingerprint: cert
                .digest(MessageDigest::sha256())
                .map_err(crypto_error)?
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(":"),
            not_before: cert.not_before().to_string(),
            expires: cert.not_after().to_string(),
            valid: valid(cert)?,
            selected: vault.selected.as_deref() == Some(&p.id),
        });
    }
    Ok(VaultSummary {
        certificates,
        tokens: vault
            .tokens
            .iter()
            .map(|t| TokenSummary {
                id: t.id.clone(),
                name: t.name.clone(),
            })
            .collect(),
    })
}
fn validate_label(label: &str) -> Result<(), AppError> {
    if label.trim().is_empty() || label.len() > 200 {
        Err(failure("Enter a name between 1 and 200 characters."))
    } else {
        Ok(())
    }
}
#[tauri::command]
pub async fn credential_unlock(
    state: State<'_, CredentialAccess>,
) -> Result<VaultSummary, AppError> {
    // Synchronous Secret Service uses its own DBus connection. Run outside GTK.
    let result = tauri::async_runtime::spawn_blocking(|| {
        let vault = read_vault()?;
        // A fresh vault must also prove secure persistence, not just NoEntry.
        write_vault(&vault)?;
        summary(&vault)
    })
    .await
    .map_err(|_| failure("Keyring request failed."))??;
    *state
        .0
        .lock()
        .map_err(|_| failure("Credential Manager is locked."))? = Some(Instant::now());
    Ok(result)
}
#[tauri::command]
pub fn credential_lock(state: State<'_, CredentialAccess>) {
    if let Ok(mut g) = state.0.lock() {
        *g = None;
    }
}
#[tauri::command]
pub fn credential_list(state: State<'_, CredentialAccess>) -> Result<VaultSummary, AppError> {
    let _guard = access(&state)?;
    summary(&read_vault()?)
}
#[tauri::command]
pub fn credential_import(
    state: State<'_, CredentialAccess>,
    path: String,
    password: String,
    label: String,
) -> Result<VaultSummary, AppError> {
    let password = Zeroizing::new(password);
    let _guard = access(&state)?;
    validate_label(&label)?;
    let path = Path::new(&path);
    if !path
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("p12") || s.eq_ignore_ascii_case("pfx"))
    {
        return Err(failure("Select a .p12 or .pfx certificate."));
    }
    let der = Zeroizing::new(read_limited(path, 16 * 1024 * 1024)?);
    parse(&der, &password)?;
    let mut vault = read_vault()?;
    if vault.profiles.len() >= 64 {
        return Err(failure("Maximum of 64 certificate profiles reached."));
    }
    vault.profiles.push(Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name: label.trim().into(),
        archive: BASE64_STANDARD.encode(&der),
        password: password.to_string(),
    });
    write_vault(&vault)?;
    summary(&vault)
}
#[tauri::command]
pub fn credential_select(
    state: State<'_, CredentialAccess>,
    id: String,
) -> Result<VaultSummary, AppError> {
    let _guard = access(&state)?;
    let mut vault = read_vault()?;
    let p = vault
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| failure("Certificate profile not found."))?;
    if !valid(decode(p)?.cert.as_ref().unwrap())? {
        return Err(failure("Certificate is expired or not yet valid."));
    }
    vault.selected = Some(id);
    write_vault(&vault)?;
    summary(&vault)
}
#[tauri::command]
pub fn credential_remove(
    state: State<'_, CredentialAccess>,
    id: String,
) -> Result<VaultSummary, AppError> {
    let _guard = access(&state)?;
    let mut vault = read_vault()?;
    vault.profiles.retain(|p| p.id != id);
    vault.tokens.retain(|t| t.id != id);
    if vault.selected.as_deref() == Some(&id) {
        vault.selected = None;
    }
    // Delete the OS entry entirely when the final managed item is removed.
    if vault.profiles.is_empty() && vault.tokens.is_empty() {
        match credential_entry(VAULT)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => (),
            Err(_) => return Err(failure("OS keyring refused deletion.")),
        }
    } else {
        write_vault(&vault)?;
    }
    summary(&vault)
}
#[tauri::command]
pub fn credential_save_token(
    state: State<'_, CredentialAccess>,
    label: String,
    value: String,
) -> Result<VaultSummary, AppError> {
    let value = Zeroizing::new(value);
    let _guard = access(&state)?;
    validate_label(&label)?;
    if value.is_empty() || value.len() > 65536 {
        return Err(failure(
            "Credential must contain between 1 and 65536 bytes.",
        ));
    }
    let mut vault = read_vault()?;
    if vault.tokens.len() >= 128 {
        return Err(failure("Maximum of 128 credentials reached."));
    }
    vault.tokens.push(Token {
        id: uuid::Uuid::new_v4().to_string(),
        name: label.trim().into(),
        value: value.to_string(),
    });
    write_vault(&vault)?;
    summary(&vault)
}
#[tauri::command]
pub fn credential_discover() -> Vec<String> {
    let Some(home) = std::env::var_os("HOME") else {
        return vec![];
    };
    let home = PathBuf::from(home);
    let mut found = Vec::new();
    // Bounded, nonrecursive search. Never read or change candidates before import.
    for folder in [
        home.clone(),
        home.join("Downloads"),
        home.join("Documents"),
        home.join("Dokumente"),
        home.join(".local/share/com.denizbudakli.denizsigner/certificates"),
    ] {
        if let Ok(entries) = std::fs::read_dir(folder) {
            for e in entries.take(2000).flatten() {
                let p = e.path();
                if p.is_file()
                    && p.extension().is_some_and(|s| {
                        s.eq_ignore_ascii_case("p12") || s.eq_ignore_ascii_case("pfx")
                    })
                {
                    found.push(p.to_string_lossy().to_string());
                }
            }
        }
    }
    found.sort();
    found.dedup();
    found
}
fn read_limited(path: &Path, max: u64) -> Result<Vec<u8>, AppError> {
    use std::io::Read;
    let f = std::fs::File::open(path).map_err(|_| failure("Cannot read selected file."))?;
    if !f
        .metadata()
        .map_err(|_| failure("Cannot inspect selected file."))?
        .is_file()
    {
        return Err(failure("Select a regular file."));
    }
    let mut data = Vec::new();
    f.take(max + 1)
        .read_to_end(&mut data)
        .map_err(|_| failure("Cannot read selected file."))?;
    if data.len() as u64 > max {
        return Err(failure("Selected file exceeds size limit."));
    }
    Ok(data)
}

#[tauri::command]
pub async fn credential_sign_ipa(
    state: State<'_, CredentialAccess>,
    input: String,
    provisioning: String,
    output: String,
) -> Result<(), AppError> {
    let profile = {
        let _guard = access(&state)?;
        let mut vault = read_vault()?;
        let id = vault
            .selected
            .take()
            .ok_or_else(|| failure("Select a signing profile first."))?;
        let index = vault
            .profiles
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| failure("Selected profile not found."))?;
        vault.profiles.remove(index)
    };
    tauri::async_runtime::spawn_blocking(move || {
        sign_ipa(
            &profile,
            Path::new(&input),
            Path::new(&provisioning),
            Path::new(&output),
        )
    })
    .await
    .map_err(|_| failure("Signing task failed."))?
}
fn sign_ipa(
    profile: &Profile,
    input: &Path,
    provisioning: &Path,
    output: &Path,
) -> Result<(), AppError> {
    use apple_codesign_quick::{
        BundleSigningSettings, ProvisioningProfile, RustCryptoCmsSigner, sign_bundle,
    };
    use rsa::pkcs8::DecodePrivateKey;
    use std::io::Write;
    use x509_cert::der::Decode;
    if output
        .extension()
        .is_none_or(|s| !s.eq_ignore_ascii_case("ipa"))
    {
        return Err(failure("Output must be a new .ipa file."));
    }
    crate::ipa::validate_ipa_path(&input.to_string_lossy())?;
    let parsed = decode(profile)?;
    let cert = parsed.cert.as_ref().unwrap();
    if !valid(cert)? {
        return Err(failure("Certificate is expired or not yet valid."));
    }
    let profile_data = read_limited(provisioning, 16 * 1024 * 1024)?;
    let provision = ProvisioningProfile::parse(&profile_data)
        .map_err(|_| failure("Invalid provisioning profile."))?;
    let expiry = provision
        .plist()
        .get("ExpirationDate")
        .and_then(plist::Value::as_date)
        .ok_or_else(|| failure("Provisioning profile has no expiry date."))?;
    if std::time::SystemTime::from(expiry) <= std::time::SystemTime::now() {
        return Err(failure("Provisioning profile has expired."));
    }
    let cert_der = cert.to_der().map_err(crypto_error)?;
    if !provision
        .plist()
        .get("DeveloperCertificates")
        .and_then(plist::Value::as_array)
        .is_some_and(|a| a.iter().any(|v| v.as_data() == Some(cert_der.as_slice())))
    {
        return Err(failure(
            "Provisioning profile does not authorize the selected certificate.",
        ));
    }
    let key_der = Zeroizing::new(
        parsed
            .pkey
            .as_ref()
            .unwrap()
            .private_key_to_pkcs8()
            .map_err(crypto_error)?,
    );
    let key = rsa::RsaPrivateKey::from_pkcs8_der(&key_der)
        .map_err(|_| failure("iOS signing requires an RSA certificate."))?;
    let xcert = x509_cert::Certificate::from_der(&cert_der)
        .map_err(|_| failure("Cannot decode signing certificate."))?;
    let chain = parsed
        .ca
        .as_ref()
        .map(|ca| {
            ca.iter()
                .map(|c| {
                    c.to_der().map_err(crypto_error).and_then(|der| {
                        x509_cert::Certificate::from_der(&der)
                            .map_err(|_| failure("Invalid certificate chain."))
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let signer = RustCryptoCmsSigner::new(key, xcert, chain);
    let temporary =
        tempfile::tempdir().map_err(|_| failure("Cannot create private signing directory."))?;
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(input).map_err(|_| failure("Cannot open IPA."))?)
            .map_err(|_| failure("Invalid IPA."))?;
    // Limit decompression, reject links/traversal instead of trusting the archive.
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|_| failure("Invalid IPA entry."))?;
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| failure("IPA too large."))?;
        if total > 2 * 1024 * 1024 * 1024 {
            return Err(failure("Expanded IPA exceeds 2 GiB."));
        }
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err(failure("Symlinks in IPA are unsupported."));
        }
        let name = entry
            .enclosed_name()
            .ok_or_else(|| failure("Unsafe IPA path."))?;
        let dest = temporary.path().join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|_| failure("Cannot extract IPA."))?;
        } else {
            std::fs::create_dir_all(dest.parent().unwrap())
                .map_err(|_| failure("Cannot extract IPA."))?;
            let mut file =
                std::fs::File::create(&dest).map_err(|_| failure("Cannot extract IPA."))?;
            std::io::copy(&mut entry, &mut file).map_err(|_| failure("Cannot extract IPA."))?;
        }
    }
    let apps = std::fs::read_dir(temporary.path().join("Payload"))
        .map_err(|_| failure("IPA has no Payload."))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.extension().is_some_and(|e| e == "app"))
        .collect::<Vec<_>>();
    if apps.len() != 1 {
        return Err(failure("IPA must contain one main application."));
    }
    let info = plist::Value::from_file(apps[0].join("Info.plist"))
        .map_err(|_| failure("Invalid Info.plist."))?;
    let bundle = info
        .as_dictionary()
        .and_then(|d| d.get("CFBundleIdentifier"))
        .and_then(plist::Value::as_string)
        .ok_or_else(|| failure("Missing bundle identifier."))?;
    let allowed = provision
        .entitlements()
        .get("application-identifier")
        .and_then(plist::Value::as_string)
        .ok_or_else(|| failure("Profile has no application identifier."))?;
    let target = format!("{}.{}", provision.team_id(), bundle);
    if allowed != target
        && !allowed
            .strip_suffix('*')
            .is_some_and(|prefix| target.starts_with(prefix))
    {
        return Err(failure(
            "Provisioning profile does not match IPA bundle identifier.",
        ));
    }
    let mut settings = BundleSigningSettings::new(
        provision.team_id(),
        provision.entitlements().clone(),
        Some(&signer),
    );
    settings.embedded_mobileprovision = Some(&profile_data);
    // Extensions require separate provisioning profiles. Reject rather than sign incorrectly.
    if apps[0].join("PlugIns").is_dir() {
        return Err(failure(
            "IPAs with app extensions require separate provisioning profiles. Use Apple-ID signing for this IPA.",
        ));
    }
    sign_bundle(&apps[0], &settings).map_err(|_| {
        failure("Code signing failed. Check the certificate chain and provisioning profile.")
    })?;
    // Atomic, no overwrite: keep failed writes and existing IPAs intact.
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut out = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| failure("Cannot create output file."))?;
    {
        let mut writer = zip::ZipWriter::new(out.as_file_mut());
        fn pack(
            writer: &mut zip::ZipWriter<&mut std::fs::File>,
            base: &Path,
            dir: &Path,
        ) -> Result<(), AppError> {
            for entry in std::fs::read_dir(dir).map_err(|_| failure("Cannot package IPA."))? {
                let path = entry.map_err(|_| failure("Cannot package IPA."))?.path();
                let relative = path
                    .strip_prefix(base)
                    .map_err(|_| failure("Invalid package path."))?
                    .to_string_lossy()
                    .replace('\\', "/");
                if path.is_dir() {
                    pack(writer, base, &path)?;
                } else {
                    writer
                        .start_file(
                            relative,
                            zip::write::SimpleFileOptions::default().unix_permissions(0o755),
                        )
                        .map_err(|_| failure("Cannot package IPA."))?;
                    let mut f =
                        std::fs::File::open(&path).map_err(|_| failure("Cannot package IPA."))?;
                    std::io::copy(&mut f, writer).map_err(|_| failure("Cannot package IPA."))?;
                }
            }
            Ok(())
        }
        pack(&mut writer, temporary.path(), temporary.path())?;
        writer.finish().map_err(|_| failure("Cannot finish IPA."))?;
    }
    out.flush().map_err(|_| failure("Cannot flush IPA."))?;
    out.persist_noclobber(output)
        .map_err(|_| failure("Output already exists or cannot be saved. Choose a new filename."))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::{
        bn::BigNum,
        pkey::PKey,
        rsa::Rsa,
        x509::{X509, X509NameBuilder},
    };
    fn fixture(expired: bool) -> Vec<u8> {
        let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", "DenizSigner TEST ONLY")
            .unwrap();
        let name = name.build();
        let mut cert = X509::builder().unwrap();
        cert.set_version(2).unwrap();
        cert.set_serial_number(&BigNum::from_u32(123).unwrap().to_asn1_integer().unwrap())
            .unwrap();
        cert.set_subject_name(&name).unwrap();
        cert.set_issuer_name(&name).unwrap();
        cert.set_pubkey(&key).unwrap();
        cert.set_not_before(&Asn1Time::from_unix(0).unwrap())
            .unwrap();
        let end = if expired {Asn1Time::from_unix(1).unwrap()} else {Asn1Time::days_from_now(30).unwrap()};
        cert.set_not_after(&end).unwrap();
        cert.sign(&key, MessageDigest::sha256()).unwrap();
        Pkcs12::builder()
            .name("test")
            .pkey(&key)
            .cert(&cert.build())
            .build2("test-only-password")
            .unwrap()
            .to_der()
            .unwrap()
    }
    #[test]
    fn password_and_validity() {
        let der = fixture(false);
        assert!(parse(&der, "wrong").is_err());
        assert!(parse(b"invalid", "x").is_err());
        let p = parse(&der, "test-only-password").unwrap();
        assert!(valid(p.cert.as_ref().unwrap()).unwrap());
        let expired = parse(&fixture(true), "test-only-password").unwrap();
        assert!(!valid(expired.cert.as_ref().unwrap()).unwrap());
    }
    #[test]
    fn lock_and_timeout() {
        let state = CredentialAccess::default();
        assert!(access(&state).is_err());
        *state.0.lock().unwrap() = Some(Instant::now());
        assert!(access(&state).is_ok());
        *state.0.lock().unwrap() = Some(Instant::now() - IDLE);
        assert!(access(&state).is_err());
    }
    #[test]
    fn summaries_do_not_expose_secrets() {
        let profile = Profile {
            id: "test".into(),
            name: "Test profile".into(),
            archive: BASE64_STANDARD.encode(fixture(false)),
            password: "test-only-password".into(),
        };
        let vault = Vault {
            profiles: vec![profile],
            tokens: vec![Token {
                id: "token".into(),
                name: "Example".into(),
                value: "secret-marker".into(),
            }],
            selected: Some("test".into()),
        };
        let json = serde_json::to_string(&summary(&vault).unwrap()).unwrap();
        assert!(!json.contains("test-only-password"));
        assert!(!json.contains("secret-marker"));
        assert!(!json.contains("archive"));
        assert!(json.contains("TEST ONLY"));
    }
    #[test]
    fn offline_ipa_signing_and_cms_verification() {
        use openssl::{
            cms::{CMSOptions, CmsContentInfo},
            stack::Stack,
        };
        use std::io::{Read, Write};
        let archive = fixture(false);
        let parsed = parse(&archive, "test-only-password").unwrap();
        let cert = parsed.cert.as_ref().unwrap();
        let key = parsed.pkey.as_ref().unwrap();
        let mut entitlements = plist::Dictionary::new();
        entitlements.insert(
            "application-identifier".into(),
            plist::Value::String("TESTTEAM.com.example.test".into()),
        );
        let mut data = plist::Dictionary::new();
        data.insert(
            "TeamIdentifier".into(),
            plist::Value::Array(vec![plist::Value::String("TESTTEAM".into())]),
        );
        data.insert(
            "Entitlements".into(),
            plist::Value::Dictionary(entitlements),
        );
        data.insert(
            "ExpirationDate".into(),
            plist::Value::Date((std::time::SystemTime::now() + Duration::from_secs(86400)).into()),
        );
        data.insert(
            "DeveloperCertificates".into(),
            plist::Value::Array(vec![plist::Value::Data(cert.to_der().unwrap())]),
        );
        let mut xml = Vec::new();
        plist::Value::Dictionary(data)
            .to_writer_xml(&mut xml)
            .unwrap();
        let provision =
            CmsContentInfo::sign(Some(cert), Some(key), None, Some(&xml), CMSOptions::BINARY)
                .unwrap()
                .to_der()
                .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let pp = dir.path().join("test.mobileprovision");
        std::fs::write(&pp, provision).unwrap();
        let input = dir.path().join("input.ipa");
        let output = dir.path().join("signed.ipa");
        // Synthetic ARM64 Mach-O fixture: signing exercises real CMS and CodeDirectory generation.
        let mut macho = vec![0u8; 512];
        fn u32le(b: &mut [u8], o: usize, v: u32) {
            b[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        fn u64le(b: &mut [u8], o: usize, v: u64) {
            b[o..o + 8].copy_from_slice(&v.to_le_bytes());
        }
        for (o, v) in [
            (0, 0xfeedfacf),
            (4, 0x0100000c),
            (12, 2),
            (16, 3),
            (20, 160),
        ] {
            u32le(&mut macho, o, v);
        }
        for (offset, name, fileoff, size) in [(32, "__TEXT", 0, 512), (104, "__LINKEDIT", 512, 0)] {
            u32le(&mut macho, offset, 0x19);
            u32le(&mut macho, offset + 4, 72);
            macho[offset + 8..offset + 8 + name.len()].copy_from_slice(name.as_bytes());
            u64le(&mut macho, offset + 32, 16384);
            u64le(&mut macho, offset + 40, fileoff);
            u64le(&mut macho, offset + 48, size);
        }
        u32le(&mut macho, 176, 0x1d);
        u32le(&mut macho, 180, 16);
        u32le(&mut macho, 184, 512);
        let mut info = plist::Dictionary::new();
        info.insert(
            "CFBundleIdentifier".into(),
            plist::Value::String("com.example.test".into()),
        );
        info.insert(
            "CFBundleExecutable".into(),
            plist::Value::String("Test".into()),
        );
        let mut info_xml = Vec::new();
        plist::Value::Dictionary(info)
            .to_writer_xml(&mut info_xml)
            .unwrap();
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&input).unwrap());
        for (name, bytes) in [
            ("Payload/Test.app/Info.plist", info_xml),
            ("Payload/Test.app/Test", macho),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&bytes).unwrap();
        }
        zip.finish().unwrap();
        let profile = Profile {
            id: "test".into(),
            name: "Test".into(),
            archive: BASE64_STANDARD.encode(archive),
            password: "test-only-password".into(),
        };
        sign_ipa(&profile, &input, &pp, &output).unwrap();
        assert!(
            sign_ipa(&profile, &input, &pp, &output).is_err(),
            "must not overwrite existing output"
        );
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
        assert!(
            zip.by_name("Payload/Test.app/_CodeSignature/CodeResources")
                .is_ok()
        );
        let mut signed = Vec::new();
        zip.by_name("Payload/Test.app/Test")
            .unwrap()
            .read_to_end(&mut signed)
            .unwrap();
        fn be(b: &[u8], o: usize) -> usize {
            u32::from_be_bytes(b[o..o + 4].try_into().unwrap()) as usize
        }
        let off = u32::from_le_bytes(signed[184..188].try_into().unwrap()) as usize;
        let blob = &signed[off..];
        assert_eq!(be(blob, 0), 0xfade0cc0);
        let mut code = None;
        let mut cms = None;
        for i in 0..be(blob, 8) {
            let slot = be(blob, 12 + i * 8);
            let offset = be(blob, 16 + i * 8);
            let b = &blob[offset..offset + be(blob, offset + 4)];
            if slot == 0 {
                code = Some(b)
            } else if slot == 0x10000 {
                cms = Some(&b[8..])
            }
        }
        let mut cms = CmsContentInfo::from_der(cms.unwrap()).unwrap();
        let mut certs = Stack::new().unwrap();
        certs.push(cert.to_owned()).unwrap();
        cms.verify(
            Some(&certs),
            None,
            Some(code.unwrap()),
            None,
            CMSOptions::NOVERIFY | CMSOptions::BINARY,
        )
        .unwrap();
    }
    #[test]
    #[ignore = "requires an unlocked real OS credential store; no mock backend"]
    fn real_keyring_roundtrip() {
        let account = format!("test-only-{}", uuid::Uuid::new_v4());
        let entry = credential_entry(&account).unwrap();
        let secret = Zeroizing::new(uuid::Uuid::new_v4().to_string());
        entry.set_password(&secret).unwrap();
        assert_eq!(entry.get_password().unwrap(), *secret);
        entry.delete_credential().unwrap();
        assert!(matches!(entry.get_password(), Err(keyring::Error::NoEntry)));
    }
}
