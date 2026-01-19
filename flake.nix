{
  description = "vicky";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: {
    overlays.default = final: prev: {
      vicky = final.callPackage (
        { lib,
          stdenv,
          rustPlatform,
          pkg-config,
          openssl,
          protobuf,
          postgresql,
          jless,
          crates ? ["vicky"],
        }:

        rustPlatform.buildRustPackage {
          pname = "vicky";
          version =
            self.shortRev or "dirty-${toString self.lastModifiedDate}";
          src = self;

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          cargoBuildFlags = lib.concatMapStrings (c: "-p ${c} ") crates;
          cargoTestFlags = lib.concatMapStrings (c: "-p ${c} ") crates;
          nativeBuildInputs = [ pkg-config protobuf ];
          buildInputs = [ openssl postgresql jless ];
        }
      ) { };
      vickyctl = final.vicky.override { crates = [ "vickyctl" ]; };
      vicky-dashboard = final.callPackage (
        { lib, stdenv, buildNpmPackage, importNpmLock }:

        buildNpmPackage {
          pname = "vicky-dashboard";
          version =
            self.shortRev or "dirty-${toString self.lastModifiedDate}";

          src = ./dashboard;

          npmDeps = importNpmLock {
            npmRoot = ./dashboard;
          };

          installPhase = ''
            runHook preInstall

            mkdir -p $out
            cp -r dist/* $out

            runHook postInstall
          '';

          npmConfigHook = importNpmLock.npmConfigHook;
        }
      ) { };
    };
  } // flake-utils.lib.eachDefaultSystem (system: let
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ self.overlays.default ];
    };
  in {
    packages = {
      inherit (pkgs) vicky vickyctl vicky-dashboard;
      default = pkgs.vicky;
    };
    legacyPackages = pkgs;

    devShells.default = pkgs.mkShell {
      name = "vicky-shell";
      RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      DATABASE_URL = "postgres://vicky:vicky@localhost/vicky";
      nativeBuildInputs = with pkgs; [ rustc clippy cargo rustfmt pkg-config protobuf devenv diesel-cli ];
      buildInputs = with pkgs; [ openssl postgresql ];
    };

  });
}