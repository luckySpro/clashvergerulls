use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub type Result<T> = std::result::Result<T, String>;
pub const LIMIT: u64 = 2 * 1024 * 1024;
pub const APP_ID: &str = "io.github.clash-verge-rev.clash-verge-rev";
const OWNED: &str = "x-verge-route-studio";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub source: String,
    pub domains: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema: u32,
    pub version: String,
    pub services: Vec<Service>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainRule {
    pub domain: String,
    pub policy: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Draft {
    pub rules: Vec<DomainRule>,
    pub china_direct: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub root: String,
    pub profile: String,
    pub target: String,
    pub fingerprint: String,
    pub groups: Vec<String>,
    pub prepend: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preview {
    pub snapshot: Snapshot,
    pub before: String,
    pub after: String,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub target: String,
    pub backup: String,
    pub applied_hash: String,
}

pub fn hash(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
pub fn read(path: &Path) -> Result<String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    if !f.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Not a regular file".into());
    }
    let mut bytes = Vec::new();
    f.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("File exceeds 2 MiB limit".into());
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
pub fn atomic(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("Missing parent")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(data).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn domain_valid(s: &str) -> bool {
    s.len() <= 253
        && s.contains('.')
        && s.parse::<std::net::IpAddr>().is_err()
        && s.split('.').all(|p| {
            !p.is_empty()
                && p.len() <= 63
                && !p.starts_with('-')
                && !p.ends_with('-')
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}
pub fn validate_catalog(c: &Catalog) -> Result<()> {
    if c.schema != 1 || c.version.is_empty() || c.version.len() > 80 || c.services.len() > 100 {
        return Err("Unsupported catalog schema/size".into());
    }
    let mut ids = BTreeSet::new();
    for s in &c.services {
        if s.id.is_empty()
            || s.id.len() > 60
            || !s.id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !ids.insert(&s.id)
        {
            return Err("Invalid/duplicate service ID".into());
        }
        if s.name.is_empty()
            || s.name.len() > 160
            || !s.source.starts_with("https://")
            || s.source.len() > 500
            || s.domains.is_empty()
            || s.domains.len() > 1000
        {
            return Err("Invalid service metadata".into());
        }
        let mut seen = BTreeSet::new();
        for d in &s.domains {
            if !domain_valid(d) || !seen.insert(d) {
                return Err(format!("Invalid/duplicate domain: {d}"));
            }
        }
    }
    Ok(())
}
fn yaml(s: &str) -> Result<Value> {
    serde_yaml::from_str(s).map_err(|e| e.to_string())
}
fn strings(v: &Value) -> Vec<String> {
    v.as_sequence()
        .map(|s| {
            s.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
fn filename(root: &Path, name: &str) -> Result<PathBuf> {
    if name.is_empty() || name.contains(['/', '\\']) || !name.ends_with(".yaml") || name == ".." {
        return Err("Unsafe profile filename".into());
    }
    let profiles = root
        .join("profiles")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !profiles.starts_with(root) {
        return Err("Profiles directory escapes root".into());
    }
    let p = profiles.join(name);
    if fs::symlink_metadata(&p)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Symlink targets are not writable".into());
    }
    let p = p.canonicalize().map_err(|e| e.to_string())?;
    if !p.starts_with(&profiles) {
        return Err("Target escapes profiles".into());
    }
    Ok(p)
}
pub fn inspect(root: &str) -> Result<Snapshot> {
    let root = Path::new(root).canonicalize().map_err(|e| e.to_string())?;
    let index_text = read(&root.join("profiles.yaml"))?;
    let index = yaml(&index_text)?;
    let current = index["current"].as_str().ok_or("No active profile")?;
    let items = index["items"]
        .as_sequence()
        .ok_or("Missing profile items")?;
    let profile = items
        .iter()
        .find(|i| i["uid"].as_str() == Some(current))
        .ok_or("Active profile missing")?;
    let uid = profile["option"]["rules"]
        .as_str()
        .ok_or("Create a Rules enhancement for this profile in Clash Verge first")?;
    let item = items
        .iter()
        .find(|i| i["uid"].as_str() == Some(uid) && i["type"].as_str() == Some("rules"))
        .ok_or("Rules enhancement missing")?;
    let target = filename(&root, item["file"].as_str().ok_or("Missing rule file")?)?;
    let text = read(&target)?;
    let rules = yaml(&text)?;
    if !rules.is_mapping() {
        return Err("Rules file must be a mapping".into());
    }
    for key in ["prepend", "append", "delete"] {
        if !rules[key].is_null()
            && (!rules[key].is_sequence()
                || rules[key]
                    .as_sequence()
                    .is_some_and(|v| v.iter().any(|x| !x.is_string())))
        {
            return Err(format!("Invalid {key} rules"));
        }
    }
    let runtime_text = read(&root.join("clash-verge.yaml"))?;
    let runtime = yaml(&runtime_text)?;
    let mut groups = vec!["DIRECT".into(), "REJECT".into()];
    if let Some(gs) = runtime["proxy-groups"].as_sequence() {
        for g in gs {
            if let Some(n) = g["name"].as_str() {
                groups.push(n.into());
            }
        }
    }
    Ok(Snapshot {
        root: root.to_string_lossy().into(),
        profile: profile["name"].as_str().unwrap_or(current).into(),
        target: target.to_string_lossy().into(),
        fingerprint: hash(format!("{index_text}\0{text}\0{runtime_text}").as_bytes()),
        groups,
        prepend: strings(&rules["prepend"]),
    })
}
pub fn compile(before: &str, draft: &Draft, groups: &[String]) -> Result<(String, Vec<String>)> {
    if draft.rules.len() > 10000 {
        return Err("Too many rules".into());
    }
    let mut rules = yaml(before)?;
    let map = rules.as_mapping_mut().ok_or("Invalid rules mapping")?;
    let mut generated = vec![
        "GEOSITE,private,DIRECT".to_owned(),
        "GEOIP,private,DIRECT,no-resolve".to_owned(),
    ];
    let mut seen = BTreeSet::new();
    for r in &draft.rules {
        if !domain_valid(&r.domain) || !seen.insert(&r.domain) {
            return Err(format!("Invalid/duplicate domain: {}", r.domain));
        }
        if !groups.contains(&r.policy) || r.policy.contains([',', '\n', '\r']) {
            return Err(format!("Unknown/invalid policy: {}", r.policy));
        }
        generated.push(format!("DOMAIN-SUFFIX,{},{}", r.domain, r.policy));
    }
    // Most specific suffix first: a personal api.example.com override precedes example.com.
    generated[2..]
        .sort_by_key(|s| std::cmp::Reverse(s.split(',').nth(1).unwrap_or("").split('.').count()));
    if draft.china_direct {
        generated.extend([
            "GEOSITE,CN,DIRECT".into(),
            "GEOIP,CN,DIRECT,no-resolve".into(),
        ]);
    }
    let previous = strings(map.get(Value::String(OWNED.into())).unwrap_or(&Value::Null));
    let existing = strings(
        map.get(Value::String("prepend".into()))
            .unwrap_or(&Value::Null),
    );
    let mut warnings = vec!["YAML formatting/comments are normalized. Backup preserves original bytes. Reload the profile in Clash Verge after writing; scripts/global enhancements may change effective order.".into()];
    let retained: Vec<_> = existing
        .into_iter()
        .filter(|r| !previous.contains(r))
        .collect();
    for r in &retained {
        if let Some(d) = r.split(',').nth(1) {
            if seen.contains(&d.to_string()) {
                warnings.push(format!("Existing rule retained below managed rules: {r}"));
            }
        }
    }
    let owned: Vec<Value> = generated.iter().map(|s| Value::String(s.clone())).collect();
    generated.extend(retained);
    map.insert(
        Value::String("prepend".into()),
        Value::Sequence(generated.into_iter().map(Value::String).collect()),
    );
    map.insert(Value::String(OWNED.into()), Value::Sequence(owned));
    serde_yaml::to_string(&rules)
        .map(|s| (s, warnings))
        .map_err(|e| e.to_string())
}
pub fn preview(root: &str, draft: &Draft) -> Result<Preview> {
    let snapshot = inspect(root)?;
    let before = read(Path::new(&snapshot.target))?;
    let (after, warnings) = compile(&before, draft, &snapshot.groups)?;
    Ok(Preview {
        snapshot,
        before,
        after,
        warnings,
    })
}
pub fn apply(root: &str, draft: &Draft, fingerprint: &str) -> Result<Receipt> {
    let p = preview(root, draft)?;
    if p.snapshot.fingerprint != fingerprint {
        return Err("Configuration changed since preview. Read and preview again.".into());
    }
    let target = Path::new(&p.snapshot.target);
    let mut backup = tempfile::Builder::new()
        .prefix("route-studio-")
        .suffix(".bak")
        .tempfile_in(target.parent().ok_or("No parent")?)
        .map_err(|e| e.to_string())?;
    backup
        .write_all(p.before.as_bytes())
        .map_err(|e| e.to_string())?;
    backup.as_file().sync_all().map_err(|e| e.to_string())?;
    let (_, backup) = backup.keep().map_err(|e| e.to_string())?;
    // Recheck immediately before replacement; never knowingly overwrite an external edit.
    if inspect(root)?.fingerprint != fingerprint {
        return Err("Concurrent configuration edit; write cancelled".into());
    }
    atomic(target, p.after.as_bytes())?;
    Ok(Receipt {
        target: p.snapshot.target,
        backup: backup.to_string_lossy().into(),
        applied_hash: hash(p.after.as_bytes()),
    })
}
pub fn rollback(root: &str, receipt: &Receipt) -> Result<()> {
    let current = inspect(root)?;
    if current.target != receipt.target
        || hash(read(Path::new(&receipt.target))?.as_bytes()) != receipt.applied_hash
    {
        return Err("Target changed after apply; refusing destructive rollback".into());
    }
    let backup = Path::new(&receipt.backup);
    if backup.parent() != Path::new(&receipt.target).parent()
        || !backup
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with("route-studio-") && s.ends_with(".bak"))
        || fs::symlink_metadata(backup)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("Invalid backup path".into());
    }
    let text = read(backup)?;
    let value = yaml(&text)?;
    if !value.is_mapping() {
        return Err("Invalid backup".into());
    }
    atomic(Path::new(&receipt.target), text.as_bytes())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> Draft {
        Draft {
            china_direct: true,
            rules: vec![DomainRule {
                domain: "claude.ai".into(),
                policy: "PROXY".into(),
            }],
        }
    }
    #[test]
    fn domains_reject_injection() {
        for s in [
            "x.com,DIRECT",
            "*.com",
            "https://a.com",
            "a..com",
            "127.0.0.1",
            "x.com\nMATCH",
            "-a.com",
        ] {
            assert!(!domain_valid(s));
        }
        assert!(domain_valid("api.openai.com"));
    }
    #[test]
    fn stable_preserves_unowned() {
        let src = "prepend: [DOMAIN-SUFFIX,mocked]\nappend: []";
        assert!(yaml(src).is_ok());
        let src = "prepend:\n- DOMAIN-SUFFIX,example.com,DIRECT\nappend: []\ncustom: yes\n";
        let (once, _) = compile(src, &draft(), &["PROXY".into()]).unwrap();
        let (twice, _) = compile(&once, &draft(), &["PROXY".into()]).unwrap();
        assert_eq!(once, twice);
        assert!(twice.contains("example.com,DIRECT"));
        assert!(twice.contains("custom:"));
    }
    #[test]
    fn unknown_policy_refused() {
        assert!(compile("{}", &draft(), &["DIRECT".into()]).is_err());
    }
    #[test]
    fn catalog_validation() {
        let c: Catalog = serde_json::from_str(include_str!("../../rules/catalog.json")).unwrap();
        validate_catalog(&c).unwrap();
    }
    fn fixture() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir(t.path().join("profiles")).unwrap();
        fs::write(t.path().join("profiles.yaml"), "current: sub\nitems:\n- uid: sub\n  name: Test\n  option: {rules: rule}\n- uid: rule\n  type: rules\n  file: rule.yaml\n").unwrap();
        fs::write(
            t.path().join("profiles/rule.yaml"),
            "# original\nprepend: []\nappend: []\ndelete: []\n",
        )
        .unwrap();
        fs::write(
            t.path().join("clash-verge.yaml"),
            "proxy-groups:\n- name: PROXY\n",
        )
        .unwrap();
        t
    }
    #[test]
    fn transaction_roundtrip() {
        let t = fixture();
        let root = t.path().to_str().unwrap();
        let p = preview(root, &draft()).unwrap();
        let r = apply(root, &draft(), &p.snapshot.fingerprint).unwrap();
        assert_eq!(read(Path::new(&r.backup)).unwrap(), p.before);
        rollback(root, &r).unwrap();
        assert_eq!(read(Path::new(&r.target)).unwrap(), p.before);
    }
    #[test]
    fn stale_preview_refused() {
        let t = fixture();
        let root = t.path().to_str().unwrap();
        let p = preview(root, &draft()).unwrap();
        fs::write(
            t.path().join("profiles/rule.yaml"),
            "prepend: []\n# changed",
        )
        .unwrap();
        assert!(apply(root, &draft(), &p.snapshot.fingerprint).is_err());
    }
    #[test]
    fn changed_target_not_rolled_back() {
        let t = fixture();
        let root = t.path().to_str().unwrap();
        let p = preview(root, &draft()).unwrap();
        let r = apply(root, &draft(), &p.snapshot.fingerprint).unwrap();
        fs::write(&r.target, "prepend: []").unwrap();
        assert!(rollback(root, &r).is_err());
    }
    #[test]
    fn traversal_refused() {
        let t = fixture();
        assert!(filename(t.path(), "../secret.yaml").is_err());
        assert!(filename(t.path(), "..\\secret.yaml").is_err());
    }
}
