{
  description = "Rust tools workspace";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      # 共通の依存関係を定義
      runtimeDeps = [
        pkgs.dbus
        pkgs.openssl
        pkgs.libnotify
      ];

      buildDeps = [
        pkgs.pkg-config
      ];

      externalTools = [
        pkgs.brightnessctl
        pkgs.pamixer
        pkgs.alsa-utils
        pkgs.hyprland
      ];
    in {
      # 1. nix build でビルドできるようにする設定
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "my-rust-tools";
        version = "0.1.0";
        src = ./.; # ワークスペースのルートを指定

        # workspace内のCargo.lockを参照
        cargoLock = {
          lockFile = ./Cargo.lock;
        };

        nativeBuildInputs = buildDeps;
        buildInputs = runtimeDeps;

        # テスト時にDBusなどのサービスが必要な場合はスキップ設定が必要なこともありますが、
        # 基本的にはこれで全バイナリがコンパイルされます。
      };

      # 2. 開発環境 (nix develop)
      devShells.default = pkgs.mkShell {
        nativeBuildInputs =
          [
            pkgs.cargo
            pkgs.rustc
            pkgs.rust-analyzer
            pkgs.rustfmt
            pkgs.clippy
          ]
          ++ buildDeps;

        buildInputs = runtimeDeps ++ externalTools;

        # Rustのライブラリが見つけやすくするための環境変数
        shellHook = ''
          export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeDeps}:$LD_LIBRARY_PATH"
          export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPath "lib/pkgconfig" buildDeps}"

          echo "🦀 Rust Workspace Environment Loaded"
          echo "Available tools: sys-controls, wifi-portal-watch"
        '';
      };
    });
}
