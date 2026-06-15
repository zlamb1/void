let
    pkgs = import <nixpkgs> {};
    cross = pkgs.pkgsCross.x86_64-embedded;
in
pkgs.mkShell {
    nativeBuildInputs = with pkgs.buildPackages; [
        cross.buildPackages.binutils
        cross.buildPackages.gcc
        gdb
        parted
        wget
    ];
}