use std::process::exit;

use which::which;

const CODE_NIX_NOT_INSTALLED: i32 = 1;

fn ensure_nix() {
    if which("nix").is_err() {
        log::error!("\"nix\" binary not found. Please install nix or run on a nix-os host.");
        exit(CODE_NIX_NOT_INSTALLED);
    }
}
