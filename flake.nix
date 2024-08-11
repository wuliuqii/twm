{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      eachSystem =
        f: nixpkgs.lib.genAttrs supportedSystems (system: f { pkgs = import nixpkgs { inherit system; }; });
    in
    {
      devShells = eachSystem (
        { pkgs }:
        {
          default = pkgs.mkShell (
            with pkgs;
            rec {
              packages = [ ];

              nativeBuildInputs = [
                pkg-config
                mold
                clang
              ];

              buildInputs = [
                udev
                alsa-lib
                vulkan-loader
                pixman
                seatd
                libinput
                stdenv.cc.cc.lib
                mesa
                wayland
                libglvnd
                libxkbcommon
              ];

              LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
            }
          );
        }
      );
    };
}
