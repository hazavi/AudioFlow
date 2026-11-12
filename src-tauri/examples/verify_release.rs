use base64::Engine;
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Pass the downloaded release assets directory")?,
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("tauri.conf.json"))?)?;
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(dir.join("latest.json"))?)?;
    let platform = manifest["platforms"]
        .get("windows-x86_64-nsis")
        .or_else(|| manifest["platforms"].get("windows-x86_64"))
        .ok_or("Missing Windows platform")?;
    let decode = |text: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::from_utf8(
            base64::engine::general_purpose::STANDARD.decode(text)?,
        )?)
    };
    let key = minisign_verify::PublicKey::decode(&decode(
        config["plugins"]["updater"]["pubkey"]
            .as_str()
            .ok_or("Missing public key")?,
    )?)?;
    let signature = minisign_verify::Signature::decode(&decode(
        platform["signature"].as_str().ok_or("Missing signature")?,
    )?)?;
    let url = platform["url"].as_str().ok_or("Missing URL")?;
    let name = url.rsplit('/').next().ok_or("Invalid artifact URL")?;
    if name.contains(['%', '\\']) || name == ".." {
        return Err("Unexpected artifact file name".into());
    }
    let bytes = fs::read(dir.join(name))?;
    key.verify(&bytes, &signature, true)?;
    let trusted = signature.trusted_comment();
    let signed = trusted
        .split('\t')
        .find_map(|v| v.strip_prefix("version:"))
        .ok_or("Missing signed version")?;
    let announced = manifest["version"].as_str().ok_or("Missing version")?;
    if signed.trim_start_matches('v') != announced.trim_start_matches('v') {
        return Err("Signed version does not match the manifest".into());
    }
    println!("Verified the artifact signature and signed version {announced}.");
    Ok(())
}
