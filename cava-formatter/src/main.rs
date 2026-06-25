use fd_lock::RwLock;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::process::{Command, Stdio};

fn main() -> io::Result<()> {
    // 1. ロックファイルの準備
    let lock_path = "/tmp/cava-formatter.pid.lock";
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(lock_path)?;
    let mut lock = RwLock::new(file);

    // 2. シングルトン（二重起動防止）チェックのインライン化
    // if let を使用することで、Err 時に lock の借用が即座に解除されます
    let _guard = if let Ok(mut guard) = lock.try_write() {
        // ロック成功：最初の起動。自分のPIDを書き込む
        guard.set_len(0)?;
        guard.seek(SeekFrom::Start(0))?;
        writeln!(guard, "{}", std::process::id())?;
        guard
    } else {
        println!("他のインスタンスが動作中。古いプロセスを終了させます...");

        // 古いプロセスが書き込んだPIDを読み取る
        let mut file_to_read = OpenOptions::new().read(true).open(lock_path)?;
        let mut pid_str = String::new();
        let _ = file_to_read.read_to_string(&mut pid_str);

        if let Some(old_pid) = pid_str
            .lines()
            .next()
            .and_then(|line| line.parse::<i32>().ok())
        {
            // 古いプロセスにKILLシグナル（SIGKILL=9）を送る
            let _ = Command::new("kill")
                .arg("-9")
                .arg(old_pid.to_string())
                .status();
            // 完全に死ぬのをわずかに待つ
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // 古い奴が死んでロックが解放されたので、今度はロックできるまで待つ（ブロッキング）
        let mut guard = lock
            .write()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        guard.set_len(0)?;
        guard.seek(SeekFrom::Start(0))?;
        writeln!(guard, "{}", std::process::id())?;
        guard
    };

    // ─── 3. 設定ファイルの埋め込み ───
    let config_content = include_str!("../waybar-cava.conf");
    let tmp_config_path = "/tmp/waybar-cava-runtime.conf";
    {
        let mut f = File::create(tmp_config_path)?;
        f.write_all(config_content.as_bytes())?;
    }

    const BARS_COUNT: usize = 10;
    const STEP: u8 = 255 / 7;
    const BAR_CHARS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

    let mut child = Command::new("cava")
        .arg("-p")
        .arg(tmp_config_path)
        .stdout(Stdio::piped())
        .spawn()?;

    let child_stdout = child.stdout.take().unwrap();
    let mut cava_reader = BufReader::new(child_stdout);
    let stdout = io::stdout();
    let mut stdout_handle = BufWriter::new(stdout.lock());
    let mut buffer = vec![0u8; BARS_COUNT];

    loop {
        if cava_reader.read_exact(&mut buffer).is_err() {
            break;
        }
        let mut line = String::with_capacity(BARS_COUNT * 4);
        for &value in &buffer {
            let index = (value / STEP).min(7) as usize;
            line.push_str(BAR_CHARS[index]);
        }
        writeln!(stdout_handle, "{}", line)?;
        stdout_handle.flush()?;
    }

    let _ = child.kill();
    let _ = std::fs::remove_file(tmp_config_path);

    Ok(())
}
