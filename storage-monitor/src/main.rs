use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dev_name = args.get(1).map(|s| s.as_str()).unwrap_or("unknown");

    // 1. ユーザーIDの特定
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
    // wrapProgram によって PATH は通っているので、notify-send を直接呼ぶ。
    // ただし、sudo は PATH をリセットするため、env コマンドで現在の PATH を明示的に渡す。
    let current_path = env::var("PATH").unwrap_or_default();

    let status = Command::new("/run/wrappers/bin/sudo")
        .args([
            "-u",
            "teto",
            "env",
            &format!("PATH={}", current_path), // ここが重要！Nixが用意したPATHをsudo先に持ち込む
            &format!("DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{}/bus", uid),
            &format!("XDG_RUNTIME_DIR=/run/user/{}", uid),
            "DISPLAY=:0",
            "notify-send", // フルパスではなくコマンド名だけでOK
            "--icon=drive-removable-media-symbolic",
            "USB Storage Detected",
            &format!("Device: /dev/{}", dev_name),
        ])
        .status();

    if let Err(e) = status {
        eprintln!("Failed to execute sudo: {}", e);
    }
}
