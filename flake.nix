{
  nixConfig.extra-substituters = [
    "https://tum-dse.cachix.org"
  ];

  nixConfig.extra-trusted-public-keys = [
    "tum-dse.cachix.org-1:v67rK18oLwgO0Z4b69l30SrV1yRtqxKpiHodG4YxhNM="
  ];

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
    unstablepkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

        unikraft = {
            flake = false;
            url = "github:unikraft/unikraft/RELEASE-0.16.3";
        };

        lib-musl = {
            flake = false;
            url = "github:unikraft/lib-musl/stable";
        };
        musl = {
            flake = false;
            url = "file+https://www.musl-libc.org/releases/musl-1.2.3.tar.gz";
        };

        lib-libunwind = {
            flake = false;
            url = "github:unikraft/lib-libunwind/stable";
        };
        libunwind = {
            flake = false;
            url = "file+https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.6/libunwind-14.0.6.src.tar.xz";
        };

        lib-libcxxabi = {
            flake = false;
            url = "github:unikraft/lib-libcxxabi/stable";
        };
        libcxxabi = {
            flake = false;
            url = "file+https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.6/libcxxabi-14.0.6.src.tar.xz";
        };

        lib-libcxx= {
            flake = false;
            url = "github:unikraft/lib-libcxx/stable";
        };
        libcxx= {
            flake = false;
            url = "file+https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.6/libcxx-14.0.6.src.tar.xz";
        };

        lib-openssl = {
            flake = false;
            url = "github:unikraft/lib-openssl/stable";
        };
        openssl = {
            flake = false;
            url = "file+https://www.openssl.org/source/old/1.1.1/openssl-1.1.1c.tar.gz";
        };

        lib-compiler-rt = {
            flake = false;
            url = "github:unikraft/lib-compiler-rt/stable";
        };
        compiler-rt = {
            flake = false;
            url = "file+https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.6/compiler-rt-14.0.6.src.tar.xz";
        };

        unikraft_click = {
            flake = false;
            url = "file+https://codeload.github.com/kohler/click/zip/a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2";
        };
        
        og-click = {
            url = "git+https://github.com/kohler/click.git";
            flake = false;
        };

    };

    outputs = { self, nixpkgs, unstablepkgs, flake-utils, ... } @ inputs:
        (flake-utils.lib.eachDefaultSystem (system: let
            pkgs = nixpkgs.legacyPackages.${system};
            unstable = unstablepkgs.legacyPackages.${system};
            flakepkgs = self.packages.${system};
            buildDeps = pkgs: (with pkgs; [
                pkg-config
                gnumake
                flex
                bison
                git
                wget
                libuuid
                gcc
                # qemu
                (qemu_kvm.overrideAttrs (new: old: {
                                         patches = old.patches ++ [
                                         ];
                                         }))
                cmake
                unzip
                clang
                openssl
                ncurses
                bridge-utils
                python3
                python3Packages.numpy
                python3Packages.matplotlib
                python3Packages.scipy
                gnuplot
                llvmPackages_15.bintools
                perl
                doxygen
                gzip
                ncurses
                ncurses.dev
                (pkgs.runCommand "gcc-nm" {} ''
                 # only bring in gcc-nm from libgcc.out, because it otherwise prevents crt1.so from musl to be found
                 mkdir -p $out/bin
                 cp ${pkgs.libgcc.out}/bin/gcc-nm $out/bin
                 cp -r ${pkgs.libgcc.out}/libexec/ $out/
                 '')
                gdb
              ]);
              prevailDeps = pkgs: (with pkgs; [
                gcc
                git
                cmake
                boost
                yaml-cpp
              ]);
              make-disk-image = import (./nix/make-disk-image.nix);
            in
            {
              packages.unikraft = let
                runMake = (pkgs.buildFHSEnv {
                  name = "runMake";
                  targetPkgs = pkgs: (
                    (buildDeps pkgs) ++ (prevailDeps pkgs) ++ [
                      unstable.kraft
                      unstable.rustup
                      unstable.bmon
                      unstable.gh
                    ]
                  );
                  runScript = "bash -c \"KRAFTKIT_NO_CHECK_UPDATES=true make\"";
                });
              in pkgs.stdenv.mkDerivation {
                name = "unikraft";
                src = ./.;
                updateAutotoolsGnuConfigScriptsPhase = ''
                  echo "wft is this. Skip it."
                '';
                postUnpack = ''
                  # srcsUnpack src_absolute destination_relative
                  function srcsUnpack () {
                    mkdir -p $(dirname $sourceRoot/$2)
                    cp -r $1 $sourceRoot/$2
                    chmod -R o+w $sourceRoot/$2
                  }
                  srcsUnpack ${inputs.unikraft} libs/unikraft

                            srcsUnpack ${inputs.lib-musl} libs/musl
                            srcsUnpack ${inputs.musl} .unikraft/build/libmusl/musl-1.2.3.tar.gz

                            srcsUnpack ${inputs.lib-libunwind} libs/libunwind
                            srcsUnpack ${inputs.libunwind} .unikraft/build/libunwind/libunwind-14.0.6.src.tar.xz

                            srcsUnpack ${inputs.lib-libcxxabi} libs/libcxxabi
                            srcsUnpack ${inputs.libcxxabi} .unikraft/build/libcxxabi/libcxxabi-14.0.6.src.tar.xz

                            srcsUnpack ${inputs.lib-libcxx} libs/libcxx
                            srcsUnpack ${inputs.libcxx} .unikraft/build/libcxx/libcxx-14.0.6.src.tar.xz

                            srcsUnpack ${inputs.lib-openssl} libs/openssl
                            srcsUnpack ${inputs.openssl} .unikraft/build/libssl/openssl-1.1.1c.tar.gz

                            srcsUnpack ${inputs.lib-compiler-rt} libs/compiler-rt
                            srcsUnpack ${inputs.compiler-rt} .unikraft/build/libcompiler_rt/compiler-rt-14.0.6.src.tar.xz

                            srcsUnpack ${inputs.unikraft_click} .unikraft/build/libclick/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2.zip
                            '';
                        buildPhase = ''
                            touch .unikraft/build/libclick/.origin
                            ${runMake}/bin/runMake
                            '';

                        installPhase = ''
                            mkdir -p $out
                            cp .unikraft/build/click_* $out/
                            cp .unikraft/build/config $out/
                            touch $out/foobar
                            '';

                    };

                    guest-image = make-disk-image {
                        config = self.nixosConfigurations.guest.config;
                        inherit (pkgs) lib;
                        inherit pkgs;
                        format = "qcow2";
                    };

                    click = pkgs.callPackage ./nix/click.nix {
                        linux = pkgs.linuxPackages_6_6.kernel;
                        selfpkgs = flakepkgs;
                        inherit self;
                    };


                devShells = {
                    default = pkgs.mkShell {
                        name = "devShell";
                        buildInputs = (buildDeps pkgs) ++ (prevailDeps pkgs) ++ [
                            unstable.kraft
                                unstable.rustup
                                unstable.bmon
                                unstable.gh
                                unstable.just
                                unstable.bridge-utils
                        ];
                        KRAFTKIT_NO_WARN_SUDO = "1";
                        KRAFTKIT_NO_CHECK_UPDATES = "true";
                    };
                    devShells.fhs = (pkgs.buildFHSEnv {
                            name = "devShell";
                            targetPkgs = pkgs: (
                                    (buildDeps pkgs) ++ (prevailDeps pkgs) ++ [
                                    unstable.kraft
                                    unstable.rustup
                                    unstable.bmon
                                    unstable.gh
                                    unstable.just
                                    ]
                                    );
                            runScript = "bash";
# KRAFTKIT_NO_WARN_SUDO = "1";
# KRAFTKIT_NO_CHECK_UPDATES = "true";
                            }).env;
                };
                
            }
            )) // {
                nixosConfigurations = let
                pkgs = nixpkgs.legacyPackages.x86_64-linux;
                flakepkgs = self.packages.x86_64-linux;
                in {
                    guest = nixpkgs.lib.nixosSystem {
                        system = "x86_64-linux";
                        modules = [ 
                            (import ./nix/guest-config.nix 
                            { 
                                inherit pkgs;
                                inherit (pkgs) lib;
                                inherit flakepkgs;
                            })
                            ./nix/nixos-generators-qcow.nix
                        ];
                    };
                };
            };
}
