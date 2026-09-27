use notify_rust::Notification;
use std::env;
use std::fs;
use std::time::Duration;

// C言語の標準システムコールを宣言（rootからの権限降格用）
unsafe extern "C" {
    fn setgid(gid: u32) -> i32;
    fn setuid(uid: u32) -> i32;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let dev_name = args.get(1).map(|s| s.as_str()).unwrap_or("unknown");

    // 1. ログイン中の一般ユーザー (UID >= 1000) を自動特定
    let uid: u32 = fs::read_dir("/run/user")
        .ok()
        .and_then(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter_map(|name| name.parse::<u32>().ok())
                .find(|&id| id >= 1000) // root(0)やシステムUIDを弾き、一般ユーザーを探す
        })
        .unwrap_or(1000);

    // 2. rootで実行されている場合、対象ユーザーへ権限降格 (Drop Privileges)
    // D-Bus は UID が一致しないプロセスからの接続を拒絶するため
    unsafe {
        let _ = setgid(uid);
        let _ = setuid(uid);
    }

    // 3. 対象ユーザーの D-Bus セッションバスを指定
    let bus_path = format!("unix:path=/run/user/{}/bus", uid);
    unsafe {
        env::set_var("DBUS_SESSION_BUS_ADDRESS", &bus_path);
        env::set_var("XDG_RUNTIME_DIR", format!("/run/user/{}", uid));
    }

    // 4. notify-rust で D-Bus 経由で直接通知を送信 (sudo も notify-send も不要)
    let res = Notification::new()
        .appname("Storage Monitor")
        .summary("USB Storage Detected")
        .body(&format!("Device: /dev/{}", dev_name))
        .icon("drive-removable-media-symbolic")
        .timeout(Duration::from_secs(5))
        .show();

    if let Err(e) = res {
        eprintln!("Failed to send notification via D-Bus: {}", e);
    }
}
