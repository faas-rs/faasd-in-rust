{
  description = "FaaS Rust Project";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url  = "github:numtide/flake-utils";
    nix-github-actions.url = "github:nix-community/nix-github-actions";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, nix-github-actions, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
      # reference: https://crane.dev/examples/quick-start-workspace.html
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        inherit (pkgs) lib;

        rustToolchainFor = p: p.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" ];
        });
        
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchainFor;
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;
          # Add additional build inputs here
          buildInputs = with pkgs; [
            cni
            cni-plugins
            openssl
            protobuf
            pkg-config
          ];

          nativeBuildInputs = with pkgs; [
            openssl
            protobuf
            pkg-config
          ];
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
          doCheck = false;
        };

        fileSetForCrate = crate: lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            (craneLib.fileset.commonCargoSources ./crates/faas-containerd)
            (craneLib.fileset.commonCargoSources ./crates/gateway)
            (craneLib.fileset.commonCargoSources ./crates/my-workspace-hack)
            (craneLib.fileset.commonCargoSources crate)
          ];
        };

        faas-rs-crate = craneLib.buildPackage ( individualCrateArgs // {
          pname = "faas-rs";
          cargoExtraArgs = "-p faas-containerd";
          src = fileSetForCrate ./crates/faas-containerd;
        });
      in
      with pkgs;
      {
        checks = {
          inherit faas-rs-crate;

          # Run clippy (and deny all warnings) on the workspace source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings -Z unstable-options";
          });

          doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on other crate derivations
          # if you do not want the tests to run twice
          nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            cargoNextestPartitionsExtraArgs = "--no-tests=pass";
          });

          # Ensure that cargo-hakari is up to date
          hakari = craneLib.mkCargoDerivation {
            inherit src;
            pname = "my-workspace-hack";
            cargoArtifacts = null;
            doInstallCargoArtifacts = false;

            buildPhaseCargoCommand = ''
              cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
              cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
              cargo hakari verify
            '';

            nativeBuildInputs = [
              pkgs.cargo-hakari
            ];
          };
        };

        packages.default = faas-rs-crate;

        apps = {
          faas-rs = flake-utils.lib.mkApp {
            drv = faas-rs-crate;
            meta = {
              description = "A containerd base lightweight FaaS platform written in Rust.";
            };
          };
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          inputsFrom = [ faas-rs-crate ];

          packages = [
            pkgs.cargo-hakari
            pkgs.containerd
            pkgs.runc
          ];
        };
      }
    )
    // {
      githubActions = nix-github-actions.lib.mkGithubMatrix {
        checks.x86_64-linux = self.checks.x86_64-linux;
        checks.aarch64-linux.faas-rs-crate = self.checks.aarch64-linux.faas-rs-crate;
      };
    };
}
