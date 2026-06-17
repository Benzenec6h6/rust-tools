use std::env;
use std::fs;
use std::process::Command;

fn main() {
    // 1. デバイス名の取得
    let args: Vec<String> = env::args().collect();
    let dev_name = args
        .get(1)
        .cloned()
        .or_else(|| env::var("KERNEL").ok())
        .unwrap_or_else(|| "unknown".to_string());

    if dev_name == "unknown" {
        eprintln!("Usage: storage-monitor <device_name>");
        std::process::exit(1);
    }

    // 2. アクティブなユーザーセッションを探す
    // /run/user/<UID> ディレクトリを探すことで、ログイン中のユーザーを特定
    let user_id = fs::read_dir("/run/user").ok().and_then(|entries| {
        entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .find(|name| name.chars().all(|c| c.is_numeric()))
    });

    let uid = match user_id {
        Some(id) => id,
        None => {
            eprintln!("Error: No active user session found in /run/user");
            std::process::exit(1);
        }
    };

    // ユーザー名を取得
    let user_name_output = Command::new("id")
        .arg("-un")
        .arg(&uid)
        .output()
        .expect("Failed to run id command");
    let user_name = String::from_utf8_lossy(&user_name_output.stdout)
        .trim()
        .to_string();

    // 3. 通知の実行
    // sudo -u <user> を使い、かつデスクトップ環境に必要な環境変数を渡す
    let status = Command::new("sudo")
        .arg("-u")
        .arg(&user_name)
        .env("DISPLAY", ":0")
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path=/run/user/{}/bus", uid),
        )
        .env("XDG_RUNTIME_DIR", format!("/run/user/{}", uid))
        .arg("notify-send")
        .arg("--icon=drive-removable-media-symbolic")
        .arg("USB Storage Detected")
        .arg(format!("Device: /dev/{}", dev_name))
        .status();

    match status {
        Ok(s) if s.success() => println!("Successfully notified user {}", user_name),
        _ => eprintln!("Failed to send notification"),
    }
}
