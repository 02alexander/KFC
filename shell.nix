let 
  rust-overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> {
    overlays = [
      rust-overlay
    ];
  };
in 
pkgs.mkShell rec {
  packages = with pkgs; [
    elf2uf2-rs
    (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
  ];
  buildInputs = with pkgs; [
    eudev
    pkg-config
  ];
}