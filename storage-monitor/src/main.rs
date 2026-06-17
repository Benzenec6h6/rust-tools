use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dev_name = args.get(1).map(|s| s.as_str()).unwrap_or("unknown");

    // 1. ユーザーIDの特定 (UID 1000付近を探す)
    let uid = fs::read_dir("/run/user")
        .ok()
        .and_then(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .find(|name| name.chars().all(|c| c.is_numeric()))
        })
        .unwrap_or_else(|| "1000".to_string());

    // 2. 通知の実行
    // sudo の後に env を挟むことで、確実に環境変数を notify-send に渡す
    let status = Command::new("/run/wrappers/bin/sudo")
        .args([
            "-u",
            "teto",
            "/run/current-system/sw/bin/env", // envコマンドをフルパスで
            &format!("DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{}/bus", uid),
            &format!("XDG_RUNTIME_DIR=/run/user/{}", uid),
            "DISPLAY=:0",
            "/run/current-system/sw/bin/notify-send",
            "--icon=drive-removable-media-symbolic",
            "USB Storage Detected",
            &format!("Device: /dev/{}", dev_name),
        ])
        .status();

    if let Err(e) = status {
        eprintln!("Failed to execute command: {}", e);
    }
}
