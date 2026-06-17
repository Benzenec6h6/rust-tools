use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    debug: bool,
    terminal_cmd: String,
}

#[derive(Deserialize, Debug)]
struct HyprClient {
    address: String,
    workspace: Workspace,
    at: [i32; 2],
    size: [i32; 2],
    #[serde(rename = "focusHistoryID")]
    focus_history_id: i32,
}

#[derive(Deserialize, Debug)]
struct Workspace {
    id: i32,
    name: String,
}

#[derive(Deserialize, Debug)]
struct HyprMonitor {
    name: String,
    focused: bool,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale: f32,
}

const ADDR_FILE: &str = "/tmp/dropdown_terminal_addr";

fn hyprctl(args: &[&str]) -> String {
    let output = Command::new("hyprctl")
        .args(args)
        .output()
        .expect("failed to execute hyprctl");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn get_focused_monitor() -> Option<HyprMonitor> {
    let json = hyprctl(&["monitors", "-j"]);
    let monitors: Vec<HyprMonitor> = serde_json::from_str(&json).ok()?;
    monitors.into_iter().find(|m| m.focused)
}

fn get_client_by_address(addr: &str) -> Option<HyprClient> {
    let json = hyprctl(&["clients", "-j"]);
    let clients: Vec<HyprClient> = serde_json::from_str(&json).ok()?;
    clients.into_iter().find(|c| c.address == addr)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // 現在のモニター情報を取得
    let monitor = get_focused_monitor().expect("Could not get focused monitor");

    // 保存されたアドレスの読み込み
    let stored_data = fs::read_to_string(ADDR_FILE).ok();
    let addr = stored_data
        .as_ref()
        .and_then(|s| s.split_whitespace().next());

    if let Some(address) = addr {
        if let Some(client) = get_client_by_address(address) {
            // 既存ターミナルがある場合の処理
            if client.workspace.name == "special:scratchpad" {
                show_terminal(&client, &monitor, &args.terminal_cmd)?;
            } else {
                hide_terminal(&client)?;
            }
            return Ok(());
        }
    }

    // 新規作成
    spawn_terminal(&monitor, &args.terminal_cmd)?;

    Ok(())
}

fn show_terminal(client: &HyprClient, monitor: &HyprMonitor, _cmd: &str) -> anyhow::Result<()> {
    // 1. 位置計算 (Bashのロジックを移植)
    let width = (monitor.width as f32 / monitor.scale * 0.5) as i32;
    let height = (monitor.height as f32 / monitor.scale * 0.5) as i32;
    let x = monitor.x + ((monitor.width as f32 / monitor.scale) as i32 - width) / 2;
    let y = monitor.y + ((monitor.height as f32 / monitor.scale) * 0.05) as i32;

    // 2. 移動と表示
    let active_ws = hyprctl(&["activeworkspace", "-j"]);
    let ws_id: Workspace = serde_json::from_str(&active_ws)?;

    hyprctl(&[
        "dispatch",
        "movetoworkspacesilent",
        &format!("{},address:{}", ws_id.id, client.address),
    ]);
    hyprctl(&["dispatch", "pin", &format!("address:{}", client.address)]);
    hyprctl(&[
        "dispatch",
        "resizewindowpixel",
        &format!("exact {} {},address:{}", width, height, client.address),
    ]);

    // アニメーション (スライドダウンを簡易実装)
    animate_slide(client, x, y - height, x, y, 5);

    hyprctl(&[
        "dispatch",
        "focuswindow",
        &format!("address:{}", client.address),
    ]);
    Ok(())
}

fn hide_terminal(client: &HyprClient) -> anyhow::Result<()> {
    // 1. アニメーション（スライドアップ）して隠す
    animate_slide(
        client,
        client.at[0],
        client.at[1],
        client.at[0],
        client.at[1] - client.size[1] - 50,
        5,
    );

    hyprctl(&["dispatch", "pin", &format!("address:{}", client.address)]); // Unpin
    hyprctl(&[
        "dispatch",
        "movetoworkspacesilent",
        &format!("special:scratchpad,address:{}", client.address),
    ]);
    Ok(())
}

fn spawn_terminal(monitor: &HyprMonitor, cmd: &str) -> anyhow::Result<()> {
    let width = (monitor.width as f32 / monitor.scale * 0.5) as i32;
    let height = (monitor.height as f32 / monitor.scale * 0.5) as i32;

    // ターミナル起動コマンドの実行
    let exec_cmd = format!(
        "[float; size {} {}; workspace special:scratchpad silent] {}",
        width, height, cmd
    );
    hyprctl(&["dispatch", "exec", &exec_cmd]);

    // 起動後のアドレス取得などは少しディレイが必要
    sleep(Duration::from_millis(300));
    let json = hyprctl(&["clients", "-j"]);
    let clients: Vec<HyprClient> = serde_json::from_str(&json)?;
    if let Some(new_client) = clients.iter().max_by_key(|c| c.focus_history_id) {
        fs::write(
            ADDR_FILE,
            format!("{} {}", new_client.address, monitor.name),
        )?;
        // 初回表示
        show_terminal(new_client, monitor, cmd)?;
    }

    Ok(())
}

fn animate_slide(
    client: &HyprClient,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    steps: i32,
) {
    for i in 0..=steps {
        let curr_x = start_x + (end_x - start_x) * i / steps;
        let curr_y = start_y + (end_y - start_y) * i / steps;
        hyprctl(&[
            "dispatch",
            "movewindowpixel",
            &format!("exact {} {},address:{}", curr_x, curr_y, client.address),
        ]);
        sleep(Duration::from_millis(20));
    }
}
