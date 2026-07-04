use futures_util::stream::StreamExt;
use notify_rust::Notification;
use std::error::Error;
use std::process::Command;
use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    // プロパティだけ定義します。
    // zbusはこれに対して自動的に `receive_connectivity_changed()` を生成します。
    #[zbus(property)]
    fn connectivity(&self) -> zbus::Result<u32>;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let connection = zbus::Connection::system().await?;
    let proxy = NetworkManagerProxy::new(&connection).await?;

    // 1. 現在の状態を確認
    let current_status = proxy.connectivity().await?;
    println!("Current connectivity: {}", current_status);
    check_status(current_status).await;

    // 2. プロパティの変化を監視するストリームを取得
    // プロパティ名が connectivity なのでメソッド名は receive_connectivity_changed になります
    let mut stream = proxy.receive_connectivity_changed().await;
    println!("Monitoring connectivity changes...");

    while let Some(change) = stream.next().await {
        // プロパティの変更通知から新しい値を取得
        if let Ok(value) = change.get().await {
            check_status(value).await;
        }
    }

    Ok(())
}

async fn check_status(status: u32) {
    match status {
        2 => {
            println!("Portal detected!");
            let _ = Notification::new()
                .summary("Wi-Fi Login Required")
                .body("Captive portal detected.")
                .show();
            let _ = open::that("http://neverssl.com");
        }
        4 => {
            println!("Connectivity is FULL. Signaling Waybar...");
            // Waybarに対して signal 8 (SIGRTMIN+8) を送る
            // これにより Waybar は interval を待たずに exec を実行します
            let _ = Command::new("pkill").args(["-RTMIN+8", "waybar"]).spawn();
        }
        _ => println!("Connectivity status: {}", status),
    }
}
