{
  description = "vicky";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nix-github-actions.url = "github:nix-community/nix-github-actions";
  inputs.nix-github-actions.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, flake-utils, nix-github-actions }: {
    overlays.default = final: prev: {
      vicky = final.callPackage (
        { lib, stdenv, rustPlatform, pkg-config, openssl, protobuf, postgresql }:

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
          buildInputs = [ openssl postgresql ];
        }
      ) { };
      vicky-dashboard = final.callPackage (
        { lib, stdenv, buildNpmPackage}:

        buildNpmPackage {
          pname = "vicky-dashboard";
          version =
            self.shortRev or "dirty-${toString self.lastModifiedDate}";

          src = ./dashboard;

          npmDepsHash = "sha256-n+dgMhdPZ8LgpDhOdnybDDYHWKZoFopVUb7acK15cd0=";

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
    githubActions = nix-github-actions.lib.mkGithubMatrix { 
      checks = nixpkgs.lib.getAttrs [ "x86_64-linux" ] self.packages;
    };
    packages = {
      inherit (pkgs) vicky vicky-dashboard;
      default = self.vicky;
    };
    legacyPackages = pkgs;

    devShells.default = pkgs.mkShell {
      name = "vicky-shell";
      RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      nativeBuildInputs = with pkgs; [ rustc clippy cargo rustfmt pkg-config protobuf ];
      buildInputs = with pkgs; [ openssl postgresql ];
    };

  });
}