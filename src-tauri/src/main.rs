#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_utils;

use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::exit;

const KEY_SIZE: usize = 2048;

#[derive(Parser)]
#[command(name = "sigillum")]
#[command(version = "0.1.0")]
#[command(about = "PDF Digital Signature Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Keygen,
    Export,
    Sign {
        #[arg(long)]
        name: String,
        
        #[arg(long, default_value = "")]
        extra: String,
        
        #[arg(long)]
        input: PathBuf,
        
        #[arg(long)]
        output: PathBuf,
    },
    Verify {
        #[arg(long)]
        file: PathBuf,
    },
}

fn get_app_data_dir() -> Result<PathBuf, String> {
    let base_dir = if cfg!(target_os = "windows") {
        env::var("APPDATA").map(PathBuf::from).map_err(|_| "APPDATA not set")?
    } else if cfg!(target_os = "macos") {
        let home = env::var("HOME").map(PathBuf::from).map_err(|_| "HOME not set")?;
        home.join("Library/Application Support")
    } else {
        let home = env::var("HOME").map(PathBuf::from).map_err(|_| "HOME not set")?;
        home.join(".local/share")
    };
    
    let app_dir = base_dir.join("com.sigillum.app");
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create app dir: {}", e))?;
    }
    Ok(app_dir)
}

fn get_key_path() -> Result<PathBuf, String> {
    Ok(get_app_data_dir()?.join("keypair.json"))
}

fn run_keygen() -> Result<String, String> {
    use rsa::{pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding}, RsaPrivateKey, RsaPublicKey};
    use rand::rngs::OsRng;
    
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

    let keypair = serde_json::json!({
        "public_key": public_key_pem.clone(),
        "private_key": private_key_pem,
    });

    let key_json = serde_json::to_string_pretty(&keypair).map_err(|e| format!("JSON error: {}", e))?;
    let key_path = get_key_path().map_err(|e| format!("Key path error: {}", e))?;
    fs::write(&key_path, key_json).map_err(|e| format!("Write error: {}", e))?;

    println!("Keypair generated and saved successfully!");
    Ok(public_key_pem)
}

fn run_export() -> Result<String, String> {
    let key_path = get_key_path().map_err(|e| format!("Key path error: {}", e))?;
    
    if !key_path.exists() {
        return Err("No keypair found. Please run --keygen first.".to_string());
    }
    
    let key_json = fs::read_to_string(&key_path).map_err(|e| format!("Read error: {}", e))?;
    let keypair: serde_json::Value = serde_json::from_str(&key_json).map_err(|e| format!("JSON error: {}", e))?;
    
    let private_key = keypair["private_key"].as_str().ok_or("Invalid key file")?;
    println!("{}", private_key);
    Ok(private_key.to_string())
}

fn compute_signature_hash(pdf_data: &[u8], name: &str, timestamp: &str, extra: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
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

fn run_sign(name: String, extra: String, input: PathBuf, output: PathBuf) -> Result<(), String> {
    use rsa::pkcs8::DecodePrivateKey;
    use chrono::Utc;
    
    let key_path = get_key_path().map_err(|e| format!("Key path error: {}", e))?;
    
    if !key_path.exists() {
        return Err("No keypair found. Please run --keygen first.".to_string());
    }
    
    let key_json = fs::read_to_string(&key_path).map_err(|e| format!("Read error: {}", e))?;
    let keypair: serde_json::Value = serde_json::from_str(&key_json).map_err(|e| format!("JSON error: {}", e))?;
    
    let private_key_pem = keypair["private_key"].as_str().ok_or("Invalid key file")?;
    let _private_key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .map_err(|e| format!("Failed to parse private key: {}", e))?;
    
    let pdf_data = fs::read(&input).map_err(|e| format!("Failed to read PDF: {}", e))?;
    
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let signature_display = compute_signature_hash(&pdf_data, &name, &timestamp, &extra);
    let watermark_text = create_watermark_text(&name, &timestamp, &extra, &signature_display);
    
    let mut doc = lopdf::Document::load_mem(&pdf_data)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;
    
    pdf_utils::add_watermark_to_pdf(&mut doc, &watermark_text)?;
    
    doc.save(&output).map_err(|e| format!("Failed to save PDF: {}", e))?;
    
    println!("PDF signed successfully!");
    println!("Output: {}", output.display());
    println!("Signer: {}", name);
    println!("Timestamp: {}", timestamp);
    if !extra.is_empty() {
        println!("Extra: {}", extra);
    }
    println!("Signature: {}", signature_display);
    
    Ok(())
}

fn run_verify(file: PathBuf) -> Result<(), String> {
    let pdf_data = fs::read(&file).map_err(|e| format!("Failed to read PDF: {}", e))?;
    
    if let Some((signer_name, timestamp, extra, signature)) = pdf_utils::extract_signature_info(&pdf_data) {
        println!("✓ PDF has a digital signature");
        println!("");
        println!("Signer: {}", signer_name);
        println!("Timestamp: {}", timestamp);
        println!("Extra: {}", extra);
        println!("Signature: {}", signature);
        Ok(())
    } else {
        println!("✗ PDF does not contain a digital signature");
        exit(1);
    }
}

fn main() {
    let cli = Cli::parse();
    
    let result = match cli.command {
        Some(Commands::Keygen) => run_keygen(),
        Some(Commands::Export) => run_export(),
        Some(Commands::Sign { name, extra, input, output }) => {
            run_sign(name, extra, input, output).map(|_| "".to_string())
        }
        Some(Commands::Verify { file }) => {
            run_verify(file).map(|_| "".to_string())
        }
        None => {
            sigillum_lib::run();
            return;
        }
    };
    
    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(1);
        }
    }
}
