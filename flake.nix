{
  description = "Rust tools workspace (sys-controls, drop-terminal, wifi-portal-watch)";

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

      # 依存関係の定義
      runtimeLibs = with pkgs; [
        dbus
        openssl
        libnotify
      ];

      buildDeps = with pkgs; [
        pkg-config
        makeWrapper # シンボリックリンク作成とパスのラップに使用
      ];

      # ツールが依存する外部コマンド
      externalBinaries = with pkgs; [
        brightnessctl
        pamixer
        alsa-utils
        libnotify
        hyprland
        xdg-utils
        networkmanager # wifi-portal-watch用
      ];
    in {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "benzen-rust-tools";
        version = "0.1.0";
        src = ./.;

        # ワークスペース全体のCargo.lockを使用
        cargoLock = {
          lockFile = ./Cargo.lock;
        };

        nativeBuildInputs = buildDeps;
        buildInputs = runtimeLibs;

        # ビルド後の処理
        postInstall = ''
          # 1. まず、全ての実体バイナリをラップして PATH を通す
          for bin in sys-controls drop-terminal wifi-portal-watch storage-monitor; do
            if [ -e "$out/bin/$bin" ]; then
              wrapProgram "$out/bin/$bin" \
                --prefix PATH : ${pkgs.lib.makeBinPath externalBinaries}
            fi
          done

          # 2. ラップ済みの sys-controls に対してシンボリックリンクを貼る
          # これにより volume / brightness を叩いても正しくラップされた PATH が使われる
          ln -sf sys-controls $out/bin/volume
          ln -sf sys-controls $out/bin/brightness
        '';
      };

      # 開発環境 (nix develop)
      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          cargo
          rustc
          rust-analyzer
          rustfmt
          clippy
          pkg-config
        ];

        buildInputs = runtimeLibs ++ externalBinaries;

        shellHook = ''
          export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibs}:$LD_LIBRARY_PATH"
          echo "🦀 Rust Workspace: sys-controls, drop-terminal, wifi-portal-watch"
        '';
      };
    });
}
