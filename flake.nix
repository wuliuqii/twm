{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      eachSystem = f:
        nixpkgs.lib.genAttrs supportedSystems (system: f {
          pkgs = import nixpkgs { inherit system; };
        });
    in
    {
      devShells = eachSystem ({ pkgs }: {
        default = pkgs.mkShell (with pkgs; rec {
          packages = [
            weston
          ];

          nativeBuildInputs = [
            pkg-config
            mold
            clang
          ];

          buildInputs = [
            udev
            alsa-lib
            vulkan-loader
            libxkbcommon
            pixman
            libglvnd
            seatd
            libinput
            mesa
            wayland # To use the wayland feature
            # xorg.libX11
            # xorg.libXcursor
            # xorg.libXi
            # xorg.libXrandr
          ];

          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        });
      });
    };
}
