//! Batch library extraction CLI. Run from src-tauri/:
//!   cargo run --bin extract_library -- smps   --in <dir>  --game "Sonic 2" --out <dir>
//!   cargo run --bin extract_library -- gyb    --in <file> --game "<pack>"  --out <dir>
//!   cargo run --bin extract_library -- zyrinx --rom <file> --game "Batman & Robin" --out <dir>
//!   cargo run --bin extract_library -- psg-table --out <dir>
//!   cargo run --bin extract_library -- uvb --out <dir>
//!
//! ORDER for the Sonic 3 & Knuckles pack: run `uvb` FIRST, then the `smps`
//! pass into the SAME --out dir. Shared voices produce identical hashes and
//! names in both passes, so the later smps pass overwrites the uvb pass's
//! empty-provenance files with the song-provenance versions; the uvb pass
//! contributes only bank voices no parsed song uses. The reverse order would
//! clobber song provenance with empty lists.

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
    // `--out` is resolved INSIDE each arm, not before the match. Hoisting it
    // meant `get("out")` ran first on every invocation, so `--help` (and any
    // unknown or misspelled subcommand) died on "missing --out" and the
    // `usage()` arm below was unreachable -- the one path a confused user
    // takes was the one path that could not print help (README pass, item 1).
    let res = match cmd.as_str() {
        "smps" => extract::extract_smps_dir(
            &PathBuf::from(get("in")), &get("game"), &PathBuf::from(get("out"))),
        "gyb" => extract::extract_gyb(
            &PathBuf::from(get("in")), &get("game"), &PathBuf::from(get("out"))),
        "zyrinx" => extract::extract_zyrinx(
            &PathBuf::from(get("rom")), &get("game"), &PathBuf::from(get("out"))),
        "psg-table" => extract::extract_psg_table(&PathBuf::from(get("out"))),
        "uvb" => extract::extract_uvb(&PathBuf::from(get("out"))),
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
    eprintln!("usage: extract_library <smps|gyb|zyrinx|psg-table|uvb> [--in PATH] [--rom PATH] [--game NAME] --out DIR");
    eprintln!("note: for Sonic 3 & Knuckles, run `uvb` BEFORE `smps` into the same --out dir");
    exit(2)
}
