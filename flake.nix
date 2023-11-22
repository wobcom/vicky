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
      vicky-dashboard = final.callPackage (
        { lib, stdenv, buildNpmPackage}:

        buildNpmPackage {
          pname = "vicky-dashboard";
          version =
            self.shortRev or "dirty-${toString self.lastModifiedDate}";

          src = ./dashboard;

          npmDepsHash = "sha256-z1Uv629N8qUEPmC/ec7cCj6regp/dp//Gwiq5Wa25ZI=";

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
  in rec {
    packages = {
      inherit (pkgs) vicky vicky-dashboard;
      default = packages.vicky;


      generate-certs = pkgs.writeShellScriptBin "generate-certs" ''
        rm -rf certs || true
        ${pkgs.certstrap}/bin/certstrap --depot-path certs init --common-name "Vicky CA" --passphrase="" 
        ${pkgs.certstrap}/bin/certstrap --depot-path certs request-cert --common-name "Vicky" --passphrase="" --domain "localhost" --ip "127.0.0.1"
        ${pkgs.certstrap}/bin/certstrap --depot-path certs request-cert --common-name "etcd" --passphrase="" --domain "localhost" --ip "127.0.0.1"
        ${pkgs.certstrap}/bin/certstrap --depot-path=certs sign "Vicky" --CA="Vicky CA" --passphrase=""
        ${pkgs.certstrap}/bin/certstrap --depot-path=certs sign "etcd" --CA="Vicky CA" --passphrase=""
      '';
    };
    legacyPackages = pkgs;

    devShells.default = pkgs.mkShell {
      name = "vicky-shell";
      RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      nativeBuildInputs = with pkgs; [ rustc clippy cargo rustfmt pkg-config protobuf ];
      buildInputs = with pkgs; [ openssl ];
    };

  });
}