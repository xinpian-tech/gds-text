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
          gtk3
          glib
          gdk-pixbuf
          pango
          atk
          cairo
          gsettings-desktop-schemas
          sarasa-gothic
          noto-fonts-cjk-serif
          noto-fonts
          fontconfig
        ];

        gtkSchemaDirs = with pkgs; [
          "${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}"
          "${gtk3}/share/gsettings-schemas/${gtk3.name}"
          "${glib}/share/gsettings-schemas/${glib.name}"
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

        devTools = with pkgs; [
          xvfb-run
          xorg-server
          xdotool
          imagemagick
          scrot
          klayout
          jq
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
              --prefix XDG_DATA_DIRS : "${builtins.concatStringsSep ":" gtkSchemaDirs}" \
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
          buildInputs = buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ devTools;
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
          XDG_DATA_DIRS = builtins.concatStringsSep ":" gtkSchemaDirs;
          FONTCONFIG_FILE = fontsConf;
          # Mesa is only needed by the headless screenshot script. The dev
          # shell exposes its path so the script never has to scan /nix/store.
          GDS_TEXT_MESA = "${pkgs.mesa}";
          GDS_TEXT_MESA_EGL_VENDOR = "${pkgs.mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
          GDS_TEXT_MESA_DRI_PATH = "${pkgs.mesa}/lib/dri";
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
