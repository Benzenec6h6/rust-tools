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
    let out = run_cmd("amixer", &["contents"]);
    let mut lines = out.lines();
    while let Some(line) = lines.next() {
        if line.to_lowercase().contains("headphone") {
            for _ in 0..2 {
                if let Some(next_line) = lines.next() {
                    if next_line.contains("values=on") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn handle_volume(arg: &str) {
    if let Some(_lock) = try_lock_process("volume") {
        let is_mic = arg.contains("mic");
        let source_flag = if is_mic {
            vec!["--default-source"]
        } else {
            vec![]
        };

        // ★ 修正点: セミコロンを外し、run_cmd の評価結果（String）を返すように変更
        let pamixer = |extra_args: &[&str]| -> String {
            let mut final_args = source_flag.clone();
            final_args.extend_from_slice(extra_args);
            run_cmd("pamixer", &final_args)
        };

        match arg {
            "--get" | "--get-mic" => {
                println!("{}", pamixer(&["--get-volume"]));
                return;
            }
            "--toggle" | "--toggle-mic" => {
                pamixer(&["-t"]);
            }
            "--inc" | "--mic-inc" => {
                pamixer(&["-u"]);
                pamixer(&["-i", "5", "--allow-boost"]);
            }
            "--dec" | "--mic-dec" => {
                pamixer(&["-u"]);
                pamixer(&["-d", "5"]);
            }
            "--inc-fine" | "--mic-inc-fine" => {
                pamixer(&["-u"]);
                pamixer(&["-i", "1", "--allow-boost"]);
            }
            "--dec-fine" | "--mic-mic-dec-fine" => {
                pamixer(&["-u"]);
                pamixer(&["-d", "1"]);
            }
            _ => return,
        }

        let vol = pamixer(&["--get-volume"]);
        let muted = pamixer(&["--get-mute"]) == "true";

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
