# sys-controls

A robust, lightweight system control utility for Wayland/Hyprland, written in Rust.

## The Story: Why this exists
This tool was born out of a necessity to solve a specific hardware-software conflict on an HP OMEN 16 laptop running NixOS. A hardware or driver-level anomaly caused the brightness keys to trigger rapid-fire events (dozens of "Unknown key" signals per second, scan code 0xab).

When using traditional shell scripts linked to these keys:

- Every single key event spawned a new shell process or systemd service.
- Each process sent a D-Bus message via notify-send.
- SwayNC (the notification center) attempted to process and render hundreds of icons simultaneously, causing a massive spike in D-Bus traffic and GPU load.
- This led to a "Session Hang": the screen would go pitch black and the graphical interface became completely unresponsive. While the system remained alive in the background (the kernel was still responsive), the UI could only be recovered by manually switching to another TTY (e.g., Ctrl+Alt+F3) and back to the graphical session (Ctrl+Alt+F2) to force a display reset.

## The Solution: Rust to the Rescue

`sys-controls` prevents this "Notification Storm" using an atomic file-locking mechanism:

- **Atomic Locking:** Upon execution, the tool attempts to acquire an exclusive lock on a temporary file.
- **Rapid-fire Prevention:** If another instance is already running (during a key spam event), the new process exits immediately (within milliseconds) without calling heavy 
sub-processes like `brightnessctl` or `notify-send`.
- **Throttling:** A mandatory cooldown period ensures that system notifications are sent at a maximum frequency that the desktop environment can safely handle.

## Features

- **☀️ Brightness Control**: Smooth increment/decrement with minimum value protection (prevents total blackout).
- **🔊 Volume & Mic Control**: Handles speakers and microphones using `pamixer`.
- **🎧 Smart Icons**: Automatically detects headphone connection status to display the correct symbolic icon.
- **🔄 Notification Sync**: Uses sync-IDs to ensure notifications replace each other instead of stacking up.
- **📦 BusyBox Style**: A single binary that changes behavior based on its symlink name (brightness or volume).

## Prerequisites

The following tools must be available in your `$PATH`:

- `brightnessctl`
- `pamixer`
- `alsa-utils` (for amixer headphone detection)
- `libnotify` (for `notify-send`)

## Usage

This tool is designed to be called via symlinks.

### Commands
```markdown
| Symlink       | Arguments          | Description                                                                 |
|---------------|--------------------|-----------------------------------------------------------------------------|
| brightness    | --inc / --dec      | Adjust brightness by 5%                                                     |
| brightness    | --get              | Get current brightness value                                                 |
| volume        | --inc / --dec      | Adjust volume by 5% (unmutes automatically)                                 |
| volume        | --toggle           | Toggle mute/unmute                                                           |
| volume        | --mic-inc          | Adjust microphone volume                                                    |

### Example Hyprland Binding
```bash
# Use 'binde' for repeat, sys-controls will handle the throttling
bind = , XF86MonBrightnessUp, exec, brightness --inc
bind = , XF86AudioRaiseVolume, exec, volume --inc
```

## Installation (Nix Flake)

Add this flake to your inputs:

```nix
{
  inputs.sys-controls.url = "github:youruser/sys-controls";
}
```

Then add it to your `home.packages`:

```nix
home.packages = [ inputs.sys-controls.packages.${pkgs.system}.default ];
```

## Development

Built with Rust for performance and safety.

### Building the Tool

1. Clone this repository.
2. Build the tool:
   ```bash
   cargo build --release
   ```
3. Set up local testing symlinks:
   ```bash
   ln -s target/release/sys-controls brightness
   ln -s target/release/sys-controls volume
   ```

## Tested Environment

- **Model:** HP OMEN Laptop 16-k0xxx
- **OS:** NixOS (Unstable/26.05)
- **Kernel:** Linux 6.18.x-cachyos
- **WM:** Hyprland 0.55.2
