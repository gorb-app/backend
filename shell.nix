{
  pkgs ? import <nixpkgs> { },
}:
pkgs.callPackage (
  {
    mkShell,
    cargo,
    rustc,
    mold,
    clang,
    pkg-config,
  }:
  mkShell {
    strictDeps = true;
    nativeBuildInputs = [
      cargo
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
  }
) { }
