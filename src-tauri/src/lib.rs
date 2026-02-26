mod pdf_utils;

use chrono::Utc;
use digest::Digest;
use lopdf::Document;
use rand::rngs::OsRng;
use rsa::{
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding},
    RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const KEY_SIZE: usize = 2048;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyPair {
    pub public_key: String,
    pub private_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureInfo {
    pub signer_name: String,
    pub timestamp: String,
    pub extra: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignPdfRequest {
    pub pdf_data: Vec<u8>,
    pub name: String,
    pub extra: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignPdfResponse {
    pub signed_pdf: Vec<u8>,
    pub signature_info: SignatureInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyPdfResponse {
    pub is_signed: bool,
    pub signature_info: Option<SignatureInfo>,
    pub message: String,
}

fn get_key_path(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| format!("Failed to create dir: {}", e))?;
    }
    Ok(path.join("keypair.json"))
}

#[tauri::command]
fn has_key(app: AppHandle) -> bool {
    match get_key_path(&app) {
        Ok(path) => path.exists(),
        Err(_) => false,
    }
}

#[tauri::command]
fn generate_keypair(app: AppHandle) -> Result<String, String> {
    let mut rng = OsRng;
    let private_key = RsaPrivateKey::new(&mut rng, KEY_SIZE).map_err(|e| format!("Failed to generate key: {}", e))?;
    let public_key = RsaPublicKey::from(&private_key);

    let private_key_pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| format!("Failed to encode private key: {}", e))?
        .to_string();
    let public_key_pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| format!("Failed to encode public key: {}", e))?;

    let keypair = KeyPair {
        public_key: public_key_pem.clone(),
        private_key: private_key_pem,
    };

    let key_json = serde_json::to_string_pretty(&keypair).map_err(|e| format!("JSON error: {}", e))?;
    let key_path = get_key_path(&app).map_err(|e| format!("Key path error: {}", e))?;
    fs::write(&key_path, key_json).map_err(|e| format!("Write error: {}", e))?;

    log::info!("Keypair generated and saved");
    Ok(public_key_pem)
}

#[tauri::command]
fn import_key(app: AppHandle, private_key_pem: String, public_key_pem: String) -> Result<String, String> {
    let _private_key = RsaPrivateKey::from_pkcs8_pem(&private_key_pem)
        .map_err(|e| format!("Invalid private key: {}", e))?;
    let _public_key = RsaPublicKey::from_public_key_pem(&public_key_pem)
        .map_err(|e| format!("Invalid public key: {}", e))?;

    let keypair = KeyPair {
        public_key: public_key_pem.clone(),
        private_key: private_key_pem,
    };

    let key_json = serde_json::to_string_pretty(&keypair).map_err(|e| format!("JSON error: {}", e))?;
    let key_path = get_key_path(&app).map_err(|e| format!("Key path error: {}", e))?;
    fs::write(&key_path, key_json).map_err(|e| format!("Write error: {}", e))?;

    log::info!("Keypair imported and saved");
    Ok(public_key_pem)
}

#[tauri::command]
fn export_key(app: AppHandle) -> Result<String, String> {
    let key_path = get_key_path(&app).map_err(|e| format!("Key path error: {}", e))?;
    let key_json = fs::read_to_string(&key_path).map_err(|e| format!("Read error: {}", e))?;
    let keypair: KeyPair = serde_json::from_str(&key_json).map_err(|e| format!("JSON error: {}", e))?;
    Ok(keypair.private_key)
}

#[tauri::command]
fn get_public_key(app: AppHandle) -> Result<String, String> {
    let key_path = get_key_path(&app).map_err(|e| format!("Key path error: {}", e))?;
    let key_json = fs::read_to_string(&key_path).map_err(|e| format!("Read error: {}", e))?;
    let keypair: KeyPair = serde_json::from_str(&key_json).map_err(|e| format!("JSON error: {}", e))?;
    Ok(keypair.public_key)
}

fn compute_signature_hash(pdf_data: &[u8], name: &str, timestamp: &str, extra: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pdf_data);
    hasher.update(name.as_bytes());
    hasher.update(timestamp.as_bytes());
    hasher.update(extra.as_bytes());
    let hash = hasher.finalize();
    format!("SHA256: {}", hex::encode(hash))
}

fn create_watermark_text(name: &str, timestamp: &str, extra: &str, signature: &str) -> String {
    if extra.is_empty() {
        format!("Digitally signed by {}\n{}\nHash:{}", name, timestamp, signature)
    } else {
        format!("Digitally signed by {}\n{}\n{}\nHash:{}", name, timestamp, extra, signature)
    }
}

#[tauri::command]
fn sign_pdf(app: AppHandle, request: SignPdfRequest) -> Result<SignPdfResponse, String> {
    let key_path = get_key_path(&app).map_err(|e| format!("Key path error: {}", e))?;
    let key_json = fs::read_to_string(&key_path).map_err(|e| format!("Read error: {}", e))?;
    let keypair: KeyPair = serde_json::from_str(&key_json).map_err(|e| format!("JSON error: {}", e))?;
    
    let _private_key = RsaPrivateKey::from_pkcs8_pem(&keypair.private_key)
        .map_err(|e| format!("Failed to parse private key: {}", e))?;
    
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let signature_display = compute_signature_hash(&request.pdf_data, &request.name, &timestamp, &request.extra);
    let watermark_text = create_watermark_text(&request.name, &timestamp, &request.extra, &signature_display);
    
    let mut doc = Document::load_mem(&request.pdf_data)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;
    
    pdf_utils::add_watermark_to_pdf(&mut doc, &watermark_text)?;
    
    let mut signed_pdf_bytes = Vec::new();
    doc.save_to(&mut signed_pdf_bytes).map_err(|e| format!("Save error: {}", e))?;
    
    Ok(SignPdfResponse {
        signed_pdf: signed_pdf_bytes,
        signature_info: SignatureInfo {
            signer_name: request.name,
            timestamp,
            extra: request.extra,
            signature: signature_display,
        },
    })
}

#[tauri::command]
fn verify_pdf(pdf_data: Vec<u8>) -> Result<VerifyPdfResponse, String> {
    log::info!("Verifying PDF, size: {} bytes", pdf_data.len());
    
    if let Some((signer_name, timestamp, extra, signature)) = pdf_utils::extract_signature_info(&pdf_data) {
        return Ok(VerifyPdfResponse {
            is_signed: true,
            signature_info: Some(SignatureInfo {
                signer_name,
                timestamp,
                extra,
                signature,
            }),
            message: "PDF has a digital signature".to_string(),
        });
    }
    
    Ok(VerifyPdfResponse {
        is_signed: false,
        signature_info: None,
        message: "PDF does not contain a digital signature".to_string(),
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            has_key,
            generate_keypair,
            import_key,
            export_key,
            get_public_key,
            sign_pdf,
            verify_pdf,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
