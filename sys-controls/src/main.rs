use fs2::FileExt;
use notify_rust::{Hint, Notification};
use std::env;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

// 誤爆や連続キー入力を弾くウェイト
const COOL_DOWN_MS: u64 = 100;

fn try_lock_process(name: &str) -> Option<File> {
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let lock_path = format!("{}/{}.lock", runtime_dir, name);

    let file = File::create(&lock_path).ok()?;
    if file.try_lock_exclusive().is_err() {
        return None;
    }
    Some(file)
}

fn run_cmd(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

/// D-Bus経由で直接通知を送信（外部プロセス不要）
fn send_osd_notification(id: u32, icon: &str, title: &str, val: u32, tag: &str) {
    let _ = Notification::new()
        .appname("System")
        .id(id)
        .icon(icon)
        .summary(title)
        .hint(Hint::Custom(
            "x-canonical-private-synchronous".into(),
            tag.into(),
        ))
        .hint(Hint::CustomInt("value".into(), val as i32))
        .hint(Hint::Urgency(notify_rust::Urgency::Low))
        .timeout(Duration::from_millis(1000))
        .show();
}

// ====================================================================
// ☀️ 輝度（Brightness）
// ====================================================================
fn get_brightness() -> u32 {
    let out = run_cmd("brightnessctl", &["-m"]);
    out.lines()
        .next()
        .and_then(|line| line.split(',').nth(3))
        .and_then(|s| s.replace('%', "").parse::<u32>().ok())
        .unwrap_or(0)
}

fn handle_brightness(arg: &str) {
    let Some(_lock) = try_lock_process("brightness") else {
        return;
    };

    match arg {
        "--inc" => {
            let _ = Command::new("brightnessctl").args(["set", "5%+"]).output();
        }
        "--dec" => {
            let _ = Command::new("brightnessctl")
                .args(["set", "5%-", "--min-value=1"])
                .output();
        }
        "--inc-fine" => {
            let _ = Command::new("brightnessctl").args(["set", "1%+"]).output();
        }
        "--dec-fine" => {
            let _ = Command::new("brightnessctl")
                .args(["set", "1%-", "--min-value=1"])
                .output();
        }
        "--get" => {
            println!("{}", get_brightness());
            return;
        }
        _ => return,
    }

    let b = get_brightness();
    send_osd_notification(
        999,
        "display-brightness-high-symbolic",
        &format!("Brightness: {}%", b),
        b,
        "brightness_notif",
    );
    thread::sleep(Duration::from_millis(COOL_DOWN_MS));
}

// ====================================================================
// 🔊 音量・マイク（Volume）
// ====================================================================
fn is_headphones(target: &str) -> bool {
    let out = run_cmd("wpctl", &["inspect", target]).to_lowercase();
    out.contains("headphone") || out.contains("headset")
}

fn handle_volume(arg: &str) {
    let Some(_lock) = try_lock_process("volume") else {
        return;
    };

    let is_mic = arg.contains("mic");
    let target = if is_mic {
        "@DEFAULT_AUDIO_SOURCE@"
    } else {
        "@DEFAULT_AUDIO_SINK@"
    };

    // 1. コマンド処理
    match arg {
        "--toggle" | "--toggle-mic" => {
            let _ = Command::new("wpctl")
                .args(["set-mute", target, "toggle"])
                .status();
        }
        "--inc" | "--mic-inc" => {
            // ミュート中なら解除
            let raw = run_cmd("wpctl", &["get-volume", target]);
            if raw.contains("[MUTED]") {
                let _ = Command::new("wpctl")
                    .args(["set-mute", target, "0"])
                    .status();
            }
            let _ = Command::new("wpctl")
                .args(["set-volume", "-l", "1.0", target, "0.05+"])
                .status();
        }
        "--dec" | "--mic-dec" => {
            let _ = Command::new("wpctl")
                .args(["set-volume", target, "0.05-"])
                .status();
        }
        "--inc-fine" | "--mic-inc-fine" => {
            let raw = run_cmd("wpctl", &["get-volume", target]);
            if raw.contains("[MUTED]") {
                let _ = Command::new("wpctl")
                    .args(["set-mute", target, "0"])
                    .status();
            }
            let _ = Command::new("wpctl")
                .args(["set-volume", "-l", "1.0", target, "0.01+"])
                .status();
        }
        "--dec-fine" | "--mic-dec-fine" => {
            let _ = Command::new("wpctl")
                .args(["set-volume", target, "0.01-"])
                .status();
        }
        _ => {}
    }

    // 2. 現在値の取得
    let raw_status = run_cmd("wpctl", &["get-volume", target]);
    let mut parts = raw_status.split_whitespace();
    let vol_f: f32 = parts.nth(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let vol_val = (vol_f * 100.0).round() as u32;
    let muted = raw_status.contains("[MUTED]");

    if arg == "--get" || arg == "--get-mic" {
        println!("{}", vol_val);
        return;
    }

    // 3. アイコン・ラベル判定
    let (icon, label, id, tag) = if !is_mic {
        let is_hp = is_headphones(target);
        let icon = match (muted, is_hp) {
            (true, true) => "audio-volume-muted-headphones-symbolic",
            (true, false) => "audio-volume-muted-symbolic",
            (false, true) => "audio-volume-headphones-symbolic",
            (false, false) => "audio-volume-high-symbolic",
        };
        let label = if muted || vol_val == 0 {
            "Volume: Muted".into()
        } else {
            format!("Volume: {}%", vol_val)
        };
        (icon, label, 998, "volume_notif")
    } else {
        let icon = if muted {
            "audio-input-microphone-muted-symbolic"
        } else {
            "audio-input-microphone-high-symbolic"
        };
        let label = if muted || vol_val == 0 {
            "Microphone: Muted".into()
        } else {
            format!("Microphone: {}%", vol_val)
        };
        (icon, label, 997, "mic_notif")
    };

    let disp_val = if muted { 0 } else { vol_val };

    // 4. D-Bus 経由で即座に通知
    send_osd_notification(id, icon, &label, disp_val, tag);

    thread::sleep(Duration::from_millis(COOL_DOWN_MS));
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: <command> [--inc|--dec|...]");
        std::process::exit(1);
    }

    let exe_path = Path::new(&args[0]);
    let exe_name = exe_path.file_name().unwrap().to_string_lossy();

    if exe_name.contains("volume") {
        handle_volume(&args[1]);
    } else if exe_name.contains("brightness") {
        handle_brightness(&args[1]);
    } else {
        eprintln!("Error: Please run this via a symlink named 'volume' or 'brightness'.");
        std::process::exit(1);
    }

    Ok(())
}
