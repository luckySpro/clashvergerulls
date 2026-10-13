#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod core;
use core::{Catalog, Draft, Preview, Receipt, Result, Snapshot};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::Manager;

struct AppState {
    data: PathBuf,
    operation: Arc<Mutex<()>>,
}
#[derive(Serialize)]
struct Bootstrap {
    candidates: Vec<String>,
    catalog: Catalog,
    saved: Option<Workspace>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Workspace {
    schema: u32,
    language: u8,
    catalog: Catalog,
    policies: BTreeMap<String, String>,
    overrides: BTreeMap<String, Vec<String>>,
    custom: Vec<core::DomainRule>,
    china: bool,
    root: String,
    repo: String,
    branch: String,
    #[serde(default)]
    proxy: String,
    receipt: Option<Receipt>,
}
fn validate_workspace(w: &Workspace) -> Result<()> {
    core::validate_catalog(&w.catalog)?;
    if w.schema != 1
        || w.language > 3
        || w.custom.len() > 10000
        || w.overrides.len() > 100
        || w.policies.len() > 100
        || w.root.len() > 4096
        || w.proxy.len() > 200
    {
        return Err("Invalid workspace schema or size".into());
    }
    source_url(&w.repo, &w.branch)?;
    for ds in w.overrides.values() {
        if ds.is_empty() || ds.len() > 1000 || ds.iter().any(|d| !core::domain_valid(d)) {
            return Err("Invalid local domain override".into());
        }
    }
    for r in &w.custom {
        if !core::domain_valid(&r.domain)
            || r.policy.len() > 200
            || r.policy.contains([',', '\n', '\r'])
        {
            return Err("Invalid personal rule".into());
        }
    }
    Ok(())
}
#[derive(Serialize, Deserialize)]
struct SyncResult {
    catalog: Catalog,
    sha256: String,
    url: String,
}

#[tauri::command]
fn bootstrap(state: tauri::State<AppState>) -> Result<Bootstrap> {
    let mut candidates = Vec::new();
    if let Some(base) = dirs::data_dir() {
        candidates.push(base.join(core::APP_ID).to_string_lossy().into_owned());
    }
    // Older Linux installs used XDG_CONFIG_HOME; manual selection supports portable layouts.
    if let Some(base) = dirs::config_dir() {
        let p = base.join(core::APP_ID).to_string_lossy().into_owned();
        if !candidates.contains(&p) {
            candidates.push(p);
        }
    }
    let saved: Option<Workspace> = if state.data.join("workspace.json").exists() {
        Some(
            serde_json::from_str(&core::read(&state.data.join("workspace.json"))?)
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    if let Some(w) = &saved {
        validate_workspace(w)?;
    }
    let catalog: Catalog = serde_json::from_str(include_str!("../../rules/catalog.json"))
        .map_err(|e| e.to_string())?;
    core::validate_catalog(&catalog)?;
    Ok(Bootstrap {
        candidates,
        catalog,
        saved,
    })
}
#[tauri::command]
async fn inspect(root: String) -> Result<Snapshot> {
    tauri::async_runtime::spawn_blocking(move || core::inspect(&root))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn preview(root: String, draft: Draft) -> Result<Preview> {
    tauri::async_runtime::spawn_blocking(move || core::preview(&root, &draft))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn apply(
    root: String,
    draft: Draft,
    fingerprint: String,
    state: tauri::State<'_, AppState>,
) -> Result<Receipt> {
    let operation = state.operation.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = operation
            .try_lock()
            .map_err(|_| "Another write is in progress")?;
        core::apply(&root, &draft, &fingerprint)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn rollback(root: String, receipt: Receipt, state: tauri::State<'_, AppState>) -> Result<()> {
    let operation = state.operation.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = operation
            .try_lock()
            .map_err(|_| "Another write is in progress")?;
        core::rollback(&root, &receipt)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn save_workspace(value: Workspace, state: tauri::State<AppState>) -> Result<()> {
    validate_workspace(&value)?;
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| "Another write is in progress")?;
    let bytes = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > core::LIMIT {
        return Err("Workspace exceeds size limit".into());
    }
    std::fs::create_dir_all(&state.data).map_err(|e| e.to_string())?;
    core::atomic(&state.data.join("workspace.json"), &bytes)
}
fn source_url(repo: &str, branch: &str) -> Result<String> {
    let parts: Vec<_> = repo.split('/').collect();
    let safe = |s: &str| {
        !s.is_empty()
            && s.len() <= 100
            && s != "."
            && s != ".."
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    };
    if parts.len() != 2 || !parts.iter().all(|s| safe(s)) || !safe(branch) {
        return Err("Use owner/repository and a simple branch name or commit SHA".into());
    }
    Ok(format!(
        "https://raw.githubusercontent.com/{repo}/{branch}/rules/catalog.json"
    ))
}
#[tauri::command]
async fn sync_catalog(repo: String, branch: String, proxy: Option<String>) -> Result<SyncResult> {
    let url = source_url(&repo, &branch)?;
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("VergeRouteStudio/0.1.0");
    if let Some(proxy) = proxy.filter(|s| !s.is_empty()) {
        let parsed = reqwest::Url::parse(&proxy).map_err(|e| e.to_string())?;
        if parsed.scheme() != "http"
            || !matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
        {
            return Err("Only local HTTP proxy addresses are allowed".into());
        }
        builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|e| e.to_string())?);
    }
    let client = builder.build().map_err(|e| e.to_string())?;
    let mut response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|n| n > core::LIMIT) {
        return Err("Catalog too large".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > core::LIMIT as usize {
            return Err("Catalog too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let catalog: Catalog = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    core::validate_catalog(&catalog)?;
    Ok(SyncResult {
        catalog,
        sha256: core::hash(&bytes),
        url,
    })
}
fn main() {
    // Read-only diagnostics; never includes subscription URLs or node credentials.
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|x| x == "--inspect") {
        let result = args
            .get(2)
            .ok_or_else(|| "Missing Clash data directory".to_string())
            .and_then(|p| core::inspect(p));
        match result {
            Ok(s) => {
                println!("{}", serde_json::to_string_pretty(&s).unwrap_or_default());
                return;
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
    tauri::Builder::default()
        .setup(|app| {
            app.manage(AppState {
                data: app.path().app_data_dir()?,
                operation: Arc::new(Mutex::new(())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            inspect,
            preview,
            apply,
            rollback,
            save_workspace,
            sync_catalog
        ])
        .run(tauri::generate_context!())
        .expect("Desktop runtime failed");
}
#[cfg(test)]
mod tests {
    #[test]
    fn github_scope() {
        assert!(super::source_url("luckySpro/clashvergerulls", "main").is_ok());
        for r in ["https://evil.com", "../repo", "x/y/z", "a/b?token=x"] {
            assert!(super::source_url(r, "main").is_err());
        }
        assert!(super::source_url("a/b", "../main").is_err());
    }
}
