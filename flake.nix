{
  description = "gds-text — render text snippets to GDSII + PDF with dummy fill";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
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
      }
    );
}
