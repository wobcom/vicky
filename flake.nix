{
  description = "vicky";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/master";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: {
    overlays.default = final: prev: {
      vicky = final.callPackage (
        { lib, stdenv, rustPlatform, pkg-config, openssl, protobuf }:

        rustPlatform.buildRustPackage {
          pname = "vicky";
          version =
            self.shortRev or "dirty-${toString self.lastModifiedDate}";
          src = self;

          cargoBuildFlags = lib.optionals (stdenv.hostPlatform.isMusl && stdenv.hostPlatform.isStatic) [ "--features" "mimalloc" ];
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          nativeBuildInputs = [ pkg-config protobuf ];
          buildInputs = [ openssl ];
        }
      ) { };
    };
  } // flake-utils.lib.eachDefaultSystem (system: let
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ self.overlays.default ];
    };
  in rec {
    packages = {
      inherit (pkgs) vicky;
      default = packages.vicky;
    };
    legacyPackages = pkgs;

    devShells.default = pkgs.mkShell {
      name = "rucli-shell";
      RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      nativeBuildInputs = with pkgs; [ rustc clippy cargo rustfmt pkg-config protobuf ];
      buildInputs = with pkgs; [ openssl ];
    };
  });
}