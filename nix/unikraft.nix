{ inputs, pkgs, unstable, unikraftDeps, ... }: let
    runMake = (pkgs.buildFHSEnv {
            name = "runMake";
            targetPkgs = pkgs: (
                    unikraftDeps ++ [
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
    src = ../.;
    updateAutotoolsGnuConfigScriptsPhase = ''
        echo "wft is this. Skip it."
        '';
    postUnpack = ''
        # srcsUnpack src_absolute destination_relative
        function srcsUnpack () {
            if [[ -d $1 ]]; then
                mkdir -p $sourceRoot/$2
                cp -r $1/* $sourceRoot/$2
            else
                mkdir -p $(dirname $sourceRoot/$2)
                cp -r $1 $sourceRoot/$2
            fi
            chmod -R u+w $sourceRoot/$2
        }

        srcsUnpack ${inputs.unikraft} libs/unikraft
        pushd $sourceRoot/libs/unikraft
        echo Patching $(pwd)
        patch -p1 < ../../nix/unikraft.disable-assert.patch
        popd

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

}
