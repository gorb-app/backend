{
  pkgs ? import <nixpkgs> { },
}:
pkgs.callPackage (
  {
    mkShell,
    cargo,
    clippy,
    rustc,
    mold,
    clang,
    pkg-config,
  }:
  mkShell {
    strictDeps = true;
    nativeBuildInputs = [
      cargo
      clippy
      rustc
      mold
      clang
      pkg-config
    ];
    buildInputs = [
      # Add openssl required by the backend
      pkgs.openssl
    ];

    RUSTC_LINKER = "${pkgs.llvmPackages.clangUseLLVM}/bin/clang";
    RUSTFLAGS = "-Clink-arg=-fuse-ld=${pkgs.mold}/bin/mold";
    RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  }
) { }
