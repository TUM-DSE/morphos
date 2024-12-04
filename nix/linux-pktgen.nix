{ pkgs, kernel, ... }: pkgs.stdenv.mkDerivation {
  name = "linux-pktgen-scripts";
  src = kernel.src;
  nativeBuildInputs = [ pkgs.makeWrapper ];
  buildPhase = ''
    patchShebangs samples/pktgen/*
  '';
  installPhase = ''
    mkdir -p $out/share
    cp samples/pktgen/* $out/share

    mkdir -p $out/bin
    for file in $out/share/pktgen_*.sh; do
      makeWrapper "$file" "$out/bin/$(basename ''${file%.sh})"
    done

  '';
}
