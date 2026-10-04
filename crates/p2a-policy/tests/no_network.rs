//! Håndhever at analysekoden ikke har nettverkstilgang.
//!
//! Alle `p2a-*`-pakker i workspacet sjekkes automatisk, unntatt de som står i
//! [`NETWORK_ALLOWED`]. Nye analysepakker blir dermed dekket uten at noen må
//! huske å legge dem til her.

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Pakker som får lov til å bruke nettverket. Tom til bestilling (M7), da
/// kommer `p2a-order`, som bare sender den ferdige trykk-PDF-en.
const NETWORK_ALLOWED: &[&str] = &["p2a-policy"];

/// Pakker som gir nettverkstilgang (HTTP-klienter, sockets, TLS).
const BANNED_CRATES: &[&str] = &[
    "attohttpc",
    "curl",
    "curl-sys",
    "h2",
    "h3",
    "hyper",
    "hyper-util",
    "isahc",
    "minreq",
    "native-tls",
    "openssl",
    "quinn",
    "reqwest",
    "rustls",
    "socket2",
    "surf",
    "tokio-tungstenite",
    "tungstenite",
    "ureq",
    "websocket",
];

/// Pakker som er lov, men ikke med disse funksjonene slått på.
const BANNED_FEATURES: &[(&str, &str)] = &[("tokio", "net"), ("async-std", "default")];

/// Kildetekst som tyder på direkte nettverksbruk.
const BANNED_SOURCE: &[&str] = &[
    "std::net",
    "TcpStream",
    "TcpListener",
    "UdpSocket",
    "ToSocketAddrs",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace-roten")
        .to_path_buf()
}

fn cargo_metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked"])
        .current_dir(workspace_root())
        .output()
        .expect("kunne ikke kjøre cargo metadata");
    assert!(
        out.status.success(),
        "cargo metadata feilet: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("ugyldig JSON fra cargo metadata")
}

/// Navnene på workspace-pakkene som skal sjekkes.
fn checked_packages(meta: &Value) -> Vec<(String, String)> {
    let members: HashSet<&str> = meta["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| members.contains(p["id"].as_str().unwrap()))
        .map(|p| {
            (
                p["name"].as_str().unwrap().to_string(),
                p["id"].as_str().unwrap().to_string(),
            )
        })
        .filter(|(name, _)| name.starts_with("p2a-") && !NETWORK_ALLOWED.contains(&name.as_str()))
        .collect()
}

/// Finner forbudte avhengigheter i det transitive treet til `root`.
/// Følger vanlige avhengigheter og build-avhengigheter, ikke dev-avhengigheter.
fn violations(meta: &Value, root: &str) -> Vec<String> {
    let nodes: HashMap<&str, &Value> = meta["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["id"].as_str().unwrap(), n))
        .collect();
    let names: HashMap<&str, &str> = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();

    let mut found = Vec::new();
    let mut seen = HashSet::new();
    let mut stack = vec![(root, names[root].to_string())];
    while let Some((id, path)) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let name = names[id];
        if BANNED_CRATES.contains(&name) {
            found.push(format!("{path} (forbudt pakke)"));
        }
        let node = nodes[id];
        for f in node["features"].as_array().into_iter().flatten() {
            let f = f.as_str().unwrap();
            if BANNED_FEATURES.contains(&(name, f)) {
                found.push(format!("{path} (forbudt funksjon «{f}»)"));
            }
        }
        for dep in node["deps"].as_array().into_iter().flatten() {
            let not_dev_only = dep["dep_kinds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|k| k["kind"].as_str() != Some("dev"));
            if not_dev_only {
                let dep_id = dep["pkg"].as_str().unwrap();
                stack.push((dep_id, format!("{path} → {}", names[dep_id])));
            }
        }
    }
    found.sort();
    found
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn analysis_crates_have_no_network_dependencies() {
    let meta = cargo_metadata();
    let packages = checked_packages(&meta);
    assert!(
        packages.iter().any(|(n, _)| n == "p2a-core"),
        "fant ikke p2a-core; sjekken ser ikke de riktige pakkene"
    );
    let problems: Vec<String> = packages
        .iter()
        .flat_map(|(_, id)| violations(&meta, id))
        .collect();
    assert!(
        problems.is_empty(),
        "Analysekoden har fått nettverkstilgang (CLAUDE.md, «Lokalt først»):\n  {}",
        problems.join("\n  ")
    );
}

#[test]
fn analysis_crates_do_not_use_std_net() {
    let meta = cargo_metadata();
    let mut problems = Vec::new();
    for (name, _) in checked_packages(&meta) {
        let mut files = Vec::new();
        rust_files(&workspace_root().join("crates").join(&name), &mut files);
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            for (i, line) in text.lines().enumerate() {
                for pat in BANNED_SOURCE {
                    if line.contains(pat) {
                        problems.push(format!("{}:{}: {pat}", file.display(), i + 1));
                    }
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "Analysekoden bruker nettverks-API-er direkte:\n  {}",
        problems.join("\n  ")
    );
}

/// Sjekker at selve sjekken virker: et kunstig avhengighetstre med `reqwest`
/// og `tokio/net` skal gi treff, mens dev-avhengigheter skal ignoreres.
#[test]
fn guard_detects_banned_dependencies() {
    let meta = serde_json::json!({
        "packages": [
            {"id": "a", "name": "p2a-core"},
            {"id": "b", "name": "reqwest"},
            {"id": "c", "name": "tokio"},
            {"id": "d", "name": "ureq"}
        ],
        "resolve": {"nodes": [
            {"id": "a", "features": [], "deps": [
                {"pkg": "b", "dep_kinds": [{"kind": null}]},
                {"pkg": "c", "dep_kinds": [{"kind": "build"}]},
                {"pkg": "d", "dep_kinds": [{"kind": "dev"}]}
            ]},
            {"id": "b", "features": [], "deps": []},
            {"id": "c", "features": ["net"], "deps": []},
            {"id": "d", "features": [], "deps": []}
        ]}
    });
    let found = violations(&meta, "a");
    assert_eq!(
        found,
        vec![
            "p2a-core → reqwest (forbudt pakke)".to_string(),
            "p2a-core → tokio (forbudt funksjon «net»)".to_string(),
        ]
    );
}
