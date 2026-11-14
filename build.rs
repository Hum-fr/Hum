use std::path::PathBuf;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(has_embedded_yt_dlp)");
    println!("cargo:rerun-if-env-changed=HUM_EMBED_YT_DLP");

    let bin_name = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };

    let candidate = std::env::var_os("HUM_EMBED_YT_DLP")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| {
            let local = PathBuf::from("vendor").join(bin_name);
            local.is_file().then_some(local)
        })
        .or_else(|| {
            let target_bin = PathBuf::from("target").join(bin_name);
            target_bin.is_file().then_some(target_bin)
        });

    if let Some(path) = candidate {
        let abs = std::fs::canonicalize(&path).unwrap_or(path);
        println!("cargo:rerun-if-changed={}", abs.display());
        println!("cargo:rustc-cfg=has_embedded_yt_dlp");
        println!("cargo:rustc-env=HUM_EMBED_YT_DLP_PATH={}", abs.display());
    }
}
