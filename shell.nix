let
  pkgs = import <nixpkgs> { };
  unstablePkgs = import (fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/0bb2eedc20e4739f29e8045034f2828ad7054f8f.tar.gz";
    sha256 = "sha256-fiPJ88izTVucjeo2EQ3jk4aWmLg03I1D4vxnX2QjEgE=";
  }) { };
in
pkgs.mkShell {
  packages = with unstablePkgs; [
    rustc
    cargo
    rust-analyzer
    clippy
    rustfmt
  ];

  RUST_SRC_PATH = "${unstablePkgs.rustPlatform.rustLibSrc}";
}
