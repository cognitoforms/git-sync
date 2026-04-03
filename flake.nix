{
  description = "git-sync desktop development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      };
      isAarch64Darwin = system == "aarch64-darwin";
      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = ["rustfmt" "rust-src"];
        targets = pkgs.lib.optionals isAarch64Darwin ["x86_64-apple-darwin"];
      };
    in {
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          rustToolchain
          nodejs
          pnpm
          just
        ];
        # When cross-compiling for x86_64 on Apple Silicon, the Nix shell injects
        # its arm64 libiconv into LIBRARY_PATH.  The linker ignores it (wrong arch)
        # and then can't find an x86_64 libiconv for libgit2-sys to link against.
        # Point the x86_64 Cargo target at the x86_64-darwin libiconv from nixpkgs.
        CARGO_TARGET_X86_64_APPLE_DARWIN_RUSTFLAGS = pkgs.lib.optionalString isAarch64Darwin (
          let x86Pkgs = import nixpkgs {system = "x86_64-darwin";};
          in "-L${x86Pkgs.libiconv}/lib"
        );
      };
    });
}
