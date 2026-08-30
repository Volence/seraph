//! Helpers shared by tests that live in different modules.
//!
//! `#[cfg(test)]`-only, so nothing here reaches a shipped binary. It exists
//! because two unrelated test suites -- the PSG-table drift check in
//! `audio::frequency` and the Zyrinx ROM import tests in `import` -- both need
//! to reach material that sits BESIDE this repo rather than inside it, and a
//! second copy of that logic is precisely the duplicated convention audit F38
//! had just finished removing.

/// The directory this checkout SITS IN -- the parent that the repo's sibling
/// reference material lives beside (`skdisasm/`, the commercial ROMs). `None`
/// rather than a guess when git cannot answer or the layout is unexpected.
///
/// Reaching a sibling means leaving the checkout, and the number of `..` hops
/// that takes is a property of WHERE the checkout happens to sit, so it cannot
/// be a constant. Counting hops (`../../` from `src-tauri/`) is right from the
/// main checkout at `<parent>/seraph/src-tauri` and WRONG from an agent
/// worktree at `<parent>/seraph/.claude/worktrees/<agent>/src-tauri`, where the
/// same two hops land inside `.claude/worktrees/` (audit F39).
///
/// `git rev-parse --show-toplevel` does NOT fix this: inside a linked worktree
/// it reports the *worktree's* root, which is the wrong directory again.
/// `--git-common-dir` is the one path every worktree shares -- it resolves to
/// `<main checkout>/.git` from the main checkout and from every linked worktree
/// alike -- so its parent is the repo and its grandparent is the directory the
/// siblings live in.
///
/// Callers fall back to a literal and, failing that, panic with instructions.
/// An unreachable source must never be a pass (audit F32), which is why this
/// returns `None` instead of inventing an answer.
pub fn sibling_root() -> Option<std::path::PathBuf> {
    // Anchored at the crate, not at the process's cwd, so the answer does not
    // depend on where the test binary was launched from.
    const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(CRATE_DIR)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8(out.stdout).ok()?;
    let raw = std::path::Path::new(raw.trim());
    // Older gits print a path relative to the cwd we handed them.
    let common = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        std::path::Path::new(CRATE_DIR).join(raw)
    };
    let common = std::fs::canonicalize(common).ok()?;
    Some(common.parent()?.parent()?.to_path_buf())
}
