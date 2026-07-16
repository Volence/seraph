//! Batch library extraction CLI. Run from src-tauri/:
//!   cargo run --bin extract_library -- smps   --in <dir>  --game "Sonic 2" --out <dir>
//!   cargo run --bin extract_library -- gyb    --in <file> --game "<pack>"  --out <dir>
//!   cargo run --bin extract_library -- zyrinx --rom <file> --game "Batman & Robin" --out <dir>
//!   cargo run --bin extract_library -- psg-table --out <dir>

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::exit;

use seraph_lib::library::extract;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else { usage() };
    let opts: HashMap<String, String> = args[1..]
        .chunks(2)
        .filter_map(|c| match c {
            [k, v] if k.starts_with("--") => Some((k[2..].to_string(), v.clone())),
            _ => None,
        })
        .collect();
    let get = |k: &str| -> String {
        opts.get(k).cloned().unwrap_or_else(|| { eprintln!("missing --{k}"); exit(2) })
    };
    let out = PathBuf::from(get("out"));
    let res = match cmd.as_str() {
        "smps" => extract::extract_smps_dir(&PathBuf::from(get("in")), &get("game"), &out),
        "gyb" => extract::extract_gyb(&PathBuf::from(get("in")), &get("game"), &out),
        "zyrinx" => extract::extract_zyrinx(&PathBuf::from(get("rom")), &get("game"), &out),
        "psg-table" => extract::extract_psg_table(&out),
        _ => usage(),
    };
    match res {
        Ok(s) => println!(
            "songs={} voices_seen={} unique_written={} failed={}",
            s.songs, s.voices_seen, s.unique_written, s.failed
        ),
        Err(e) => { eprintln!("error: {e}"); exit(1) }
    }
}

fn usage() -> ! {
    eprintln!("usage: extract_library <smps|gyb|zyrinx|psg-table> [--in PATH] [--rom PATH] [--game NAME] --out DIR");
    exit(2)
}
