use fs2::FileExt;
use std::env;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

const COOL_DOWN_MS: u64 = 60; // チャタリングを弾くウェイト時間（ミリ秒）

// ====================================================================
// 💡 共通ヘルパー: ファイルロック (超高速ブレーキ)
// ====================================================================
fn try_lock_process(name: &str) -> Option<File> {
    // XDG_RUNTIME_DIR (/run/user/1000等) を取得、なければ /tmp
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());

    let lock_path = format!("{}/{}.lock", runtime_dir, name);

    // デバッグ用に失敗した時に理由がわかるようにすると学習が捗ります
    let file = File::create(&lock_path)
        .map_err(|e| {
            eprintln!("Failed to create lock file {}: {}", lock_path, e);
            e
        })
        .ok()?;

    if file.try_lock_exclusive().is_err() {
        return None;
    }
    Some(file)
}

fn run_cmd(program: &str, args: &[&str]) -> String {
    let output = Command::new(program).args(args).output();
    match output {
        Ok(out) => {
            if !out.status.success() {
                // コマンドがエラー（権限不足など）を返した場合
                let err_msg = String::from_utf8_lossy(&out.stderr);
                eprintln!("Command '{}' failed: {}", program, err_msg);
            }
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        Err(e) => {
            // そもそもコマンドが見つからない場合
            eprintln!("Failed to execute '{}': {}", program, e);
            String::new()
        }
    }
}

// ====================================================================
// ☀️ 輝度（Brightness）セクション
// ====================================================================
fn get_brightness() -> String {
    let out = run_cmd("brightnessctl", &["-m"]);
    if out.is_empty() {
        return "N/A".to_string(); // エラーがわかりやすいようにする
    }

    // 最初の1行を取り出し、カンマで分割
    out.lines()
        .next()
        .and_then(|line| line.split(',').nth(3)) // 4番目の要素(50%)を取得
        .map(|s| s.replace('%', "")) // %を消す
        .unwrap_or_else(|| "0".to_string()) // ダメなら"0"
}

fn send_brightness_notif() {
    let b = get_brightness();
    let _ = Command::new("notify-send")
        .args([
            "-a",
            "System",
            "-t",
            "1000",
            "-r",
            "999",
            "--icon",
            "display-brightness-high-symbolic",
            "-h",
            &format!("int:value:{}", b),
            "-h",
            "string:x-canonical-private-synchronous:brightness_notif",
            "-u",
            "low",
            "Brightness",
            &format!("{}%", b),
        ])
        .output();
}

fn handle_brightness(arg: &str) {
    if let Some(_lock) = try_lock_process("brightness") {
        match arg {
            "--inc" => {
                let _ = Command::new("brightnessctl").args(["set", "5%+"]).output();
                send_brightness_notif();
            }
            "--dec" => {
                let _ = Command::new("brightnessctl")
                    .args(["set", "5%-", "--min-value=1"])
                    .output();
                send_brightness_notif();
            }
            "--inc-fine" => {
                let _ = Command::new("brightnessctl").args(["set", "1%+"]).output();
                send_brightness_notif();
            }
            "--dec-fine" => {
                let _ = Command::new("brightnessctl")
                    .args(["set", "1%-", "--min-value=1"])
                    .output();
                send_brightness_notif();
            }
            "--get" => println!("{}", get_brightness()),
            _ => {}
        }
        thread::sleep(Duration::from_millis(COOL_DOWN_MS));
    }
}

// ====================================================================
// 🔊 音量・マイク（Volume）セクション
// ====================================================================
fn is_headphones_connected() -> bool {
    let out = run_cmd("wpctl", &["status"]);
    for line in out.lines() {
        if line.contains("[*]") && line.to_lowercase().contains("headphone") {
            return true;
        }
    }
    false
}

fn handle_volume(arg: &str) {
    if let Some(_lock) = try_lock_process("volume") {
        let is_mic = arg.contains("mic");

        // 操作対象の決定
        let target = if is_mic {
            "@DEFAULT_AUDIO_SOURCE@"
        } else {
            "@DEFAULT_AUDIO_SINK@"
        };

        // 共通のコマンド実行ヘルパー
        let wpctl_set = |args: &[&str]| {
            let mut final_args = vec![];
            final_args.extend_from_slice(args);
            run_cmd("wpctl", &final_args);
        };

        // 1. 音量・ミュートの変更アクションの実行
        match arg {
            "--toggle" | "--toggle-mic" => {
                wpctl_set(&["set-mute", target, "toggle"]);
            }
            "--inc" | "--mic-inc" => {
                wpctl_set(&["set-mute", target, "0"]); // ミュート解除
                wpctl_set(&["set-volume", "-l", "1.5", target, "0.05+"]); // --allow-boost の代わりに -l 1.5 (150%上限)
            }
            "--dec" | "--mic-dec" => {
                wpctl_set(&["set-mute", target, "0"]);
                wpctl_set(&["set-volume", target, "0.05-"]);
            }
            "--inc-fine" | "--mic-inc-fine" => {
                wpctl_set(&["set-mute", target, "0"]);
                wpctl_set(&["set-volume", "-l", "1.5", target, "0.01+"]);
            }
            "--dec-fine" | "--mic-dec-fine" => {
                wpctl_set(&["set-mute", target, "0"]);
                wpctl_set(&["set-volume", target, "0.01-"]);
            }
            _ => {}
        }

        // 2. 現在の状態を取得 (パース処理)
        // wpctl get-volume の出力例: "Volume: 0.40" または "Volume: 0.20 [MUTED]"
        let raw_status = run_cmd("wpctl", &["get-volume", target]);

        if arg == "--get" || arg == "--get-mic" {
            // --get 要求時は純粋な数値（%）だけを標準出力して終了
            let vol_str = raw_status.split_whitespace().nth(1).unwrap_or("0.00");
            if let Ok(vol_f) = vol_str.parse::<f32>() {
                println!("{:.0}", vol_f * 100.0);
            } else {
                println!("0");
            }
            return;
        }

        let muted = raw_status.contains("[MUTED]");

        // 文字列から音量（%）を計算
        let vol = {
            let vol_str = raw_status.split_whitespace().nth(1).unwrap_or("0.00");
            if let Ok(vol_f) = vol_str.parse::<f32>() {
                format!("{:.0}", vol_f * 100.0)
            } else {
                "0".to_string()
            }
        };

        // 3. 通知用アイコン・ラベルの生成 (既存のロジックを流用)
        let (icon, label, id, sync_key) = if !is_mic {
            let icon = if muted {
                if is_headphones_connected() {
                    "audio-volume-muted-headphones-symbolic"
                } else {
                    "audio-volume-muted-symbolic"
                }
            } else {
                if is_headphones_connected() {
                    "audio-volume-headphones-symbolic"
                } else {
                    "audio-volume-high-symbolic"
                }
            };
            let label = if muted || vol == "0" {
                "Volume: Muted".to_string()
            } else {
                format!("Volume: {}%", vol)
            };
            (icon, label, "998", "volume_notif")
        } else {
            let icon = if muted {
                "audio-input-microphone-muted-symbolic"
            } else {
                "audio-input-microphone-high-symbolic"
            };
            let label = if muted || vol == "0" {
                "Microphone: Muted".to_string()
            } else {
                format!("Microphone: {}%", vol)
            };
            (icon, label, "997", "mic_notif")
        };

        let disp_vol = if muted { "0" } else { &vol };
        let _ = Command::new("notify-send")
            .args([
                "-e",
                "-a",
                "System",
                "-r",
                id,
                "-h",
                &format!("string:x-canonical-private-synchronous:{}", sync_key),
                "-h",
                &format!("int:value:{}", disp_vol),
                "-u",
                "low",
                "--icon",
                icon,
                &label,
            ])
            .status();

        thread::sleep(Duration::from_millis(COOL_DOWN_MS));
    }
}

// ====================================================================
// 🚀 エントリポイント
// ====================================================================
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: <command> [--inc|--dec|...]");
        std::process::exit(1);
    }

    // 💡 修正：current_exe() を使わず、args[0]（実行されたコマンド名）からファイル名を切り出す
    let exe_path = Path::new(&args[0]);
    let exe_name = exe_path.file_name().unwrap().to_string_lossy();

    // デバッグ時に判定しやすいよう、条件を明確にします
    if exe_name.contains("volume") {
        handle_volume(&args[1]);
    } else if exe_name.contains("brightness") {
        handle_brightness(&args[1]);
    } else {
        // 万が一 sys-controls のまま叩かれた場合のヘルプ
        eprintln!("Error: Please run this via a symlink named 'volume' or 'brightness'.");
        std::process::exit(1);
    }

    Ok(())
}
