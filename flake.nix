{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    (flake-utils.lib.eachDefaultSystem
        (system:
            let
              pkgs = import nixpkgs {
                  inherit system;
              };
              buildInputs = with pkgs; [
                pkg-config
                gnumake
                flex
                bison
                git
                wget
                libuuid
                gcc
                qemu
                qemu_kvm
              ];
            in
            with pkgs;
            {
              devShells.default = mkShell {
                name = "devShell";
                inherit buildInputs;
              };
            }
        )
    );
}
