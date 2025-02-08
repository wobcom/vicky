{
  description = "vicky";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/staging-next";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: {
    overlays.default = final: prev: {
      vicky = final.callPackage (
        { lib, stdenv, rustPlatform, pkg-config, openssl, protobuf, postgresql, jless }:

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
          buildInputs = [ openssl postgresql jless ];
        }
      ) { };
      vicky-dashboard = final.callPackage (
        { lib, stdenv, buildNpmPackage}:

        buildNpmPackage {
          pname = "vicky-dashboard";
          version =
            self.shortRev or "dirty-${toString self.lastModifiedDate}";

          src = ./dashboard;

          npmDepsHash = "sha256-hvbUz7Qmy+THMemxRsAcOAZlgzhr4NVZ8FXOzurIumA=";

          installPhase = ''
            runHook preInstall

            mkdir -p $out
            cp -r dist/* $out

            runHook postInstall
          '';
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
      inherit (pkgs) vicky vicky-dashboard;
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