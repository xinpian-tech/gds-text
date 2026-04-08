{
  description = "gds-text -- render text snippets to GDSII + PDF with dummy fill";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        runtimeLibs = with pkgs; [
          libGL
          libxkbcommon
          wayland
          libx11
          libxcursor
          libxi
          libxrandr
          vulkan-loader
          sarasa-gothic
          noto-fonts-cjk-serif
          noto-fonts
          fontconfig
        ];

        buildInputs = with pkgs; [
          fontconfig
          freetype
          expat
        ] ++ runtimeLibs;

        nativeBuildInputs = with pkgs; [
          rustc
          cargo
          rustfmt
          clippy
          pkg-config
          makeWrapper
        ];

        fontsConf = pkgs.makeFontsConf {
          fontDirectories = [
            pkgs.sarasa-gothic
            pkgs.noto-fonts-cjk-serif
            pkgs.noto-fonts
          ];
        };

        gds-text = pkgs.rustPlatform.buildRustPackage {
          pname = "gds-text";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          inherit nativeBuildInputs buildInputs;
          postFixup = ''
            wrapProgram $out/bin/gds-text \
              --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath runtimeLibs}" \
              --set FONTCONFIG_FILE "${fontsConf}"
          '';
          meta = with pkgs.lib; {
            description = "Render text snippets to GDSII and PDF with Calibre-style dummy fill";
            license = licenses.mit;
            platforms = platforms.linux;
            mainProgram = "gds-text";
          };
        };
      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
          FONTCONFIG_FILE = fontsConf;
          shellHook = ''
            echo "gds-text dev shell -- rust $(rustc --version)"
          '';
        };

        packages.default = gds-text;
        packages.gds-text = gds-text;

        apps.default = flake-utils.lib.mkApp {
          drv = gds-text;
          name = "gds-text";
        };
      }
    );
}
