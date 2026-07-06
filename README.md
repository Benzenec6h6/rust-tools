# Rust OS Tools

A collection of lightweight, high‑performance system utilities for **NixOS / Hyprland**, unified in a single Rust workspace for maximum efficiency and maintainability.

---

## 📦 Tools in this Repository

| Tool                | Purpose                     | Why Rust?                                                                                                    |
|---------------------|----------------------------|--------------------------------------------------------------------------------------------------------------|
| **sys-controls**   | Brightness & Volume control | Prevents *“Notification Storms”* and UI freezes via atomic file locking.                                      |
| **drop-terminal**  | Scratchpad terminal        | Guarantees UI stability. Fixes a bug where spawning a terminal over heavy apps (e.g., browsers) caused workspace flickering. |
| **wifi-portal-watch** | Network monitoring          | Replaces brittle shell logic with robust D‑Bus integration for reliable captive‑portal detection.            |

---

## 🛠 Tool Details

### ☀️ 🔊 `sys-controls` (Brightness & Volume)

> **Problem** – On an HP OMEN 16 laptop the brightness keys generated dozens of events per second.  
> Traditional shell scripts spawned hundreds of `notify-send` processes, saturating D‑Bus and hanging the whole graphical session.

**Solution** – `sys-controls` uses **atomic file locking** (`fs2`). If an instance is already running, new invocations exit within milliseconds, throttling notifications to a safe rate 
for the desktop environment.

---

### ⌨️ `drop-terminal` (Dropdown Terminal)

A Hyprland‑specific dropdown terminal manager with smooth animations.

*Why Rust?*  
The original Bash version was logically correct but caused UI misbehaviour: toggling the terminal over resource‑heavy applications (e.g., browsers) occasionally made the workspace 
glitch or the browser lose focus. Rust’s faster execution and precise timing for Hyprland IPC calls removed these race conditions, delivering a rock‑solid UI experience.

---

### 🌐 `wifi-portal-watch` (Wi‑Fi Portal Monitor)

Monitors network changes via D‑Bus and automatically handles captive‑portal detection.

*Why Rust?*  
By using `zbus` to listen directly to NetworkManager events, the tool avoids the overhead and unreliability of polling‑based shell scripts, providing a clean, event‑driven solution.

---

## 🚀 Installation (Nix Flake)

Since the utilities share a single workspace, a single flake input ships all binaries.

1. **Add to your inputs**

   ```nix
   {
     inputs.rust-tools.url = "github:Benzenec6h6/rust-tools";
   }
   ```

2. **Add to your environment**

   ```nix
   environment.systemPackages = [
     inputs.rust-tools.packages.${pkgs.system}.default
   ];
   ```

This installs `sys-controls`, `drop-terminal`, and `wifi-portal-watch` simultaneously.

---

## 🛠 Development & Maintenance

### Workflow

- **Update Locks** – GitHub Actions automatically refresh `Cargo.lock` and `flake.lock` every Monday.
- **Build Everything**

  ```bash
  nix build
  ```

- **Enter Development Shell**

  ```bash
  nix develop
  # Provides: rustc, cargo, clippy, rust-analyzer, etc.
  ```

### Binary Locations (after `nix build`)

```
result/bin/
├── sys-controls
├── drop-terminal
└── wifi-portal-watch
```

---

## 🖥 Tested Environment

| Component | Details |
|----------|---------|
| **Model** | HP OMEN Laptop 16‑k0xxx |
| **OS**    | NixOS (Unstable / 25.05) |
| **Kernel**| Linux 6.18.x‑cachyos (needed for specific ACPI/input event handling) |
| **WM**    | Hyprland 0.55.2 (Wayland) |
| **Rust Edition** | 2024 |

---

Feel free to open issues or submit pull requests if you encounter bugs or have ideas for new utilities!
