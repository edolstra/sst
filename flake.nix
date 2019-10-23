{
  edition = 201909;

  description = "A simple, extensible, unambiguous markup language";

  inputs.import-cargo.uri = "github:edolstra/import-cargo";
  inputs.grcov = { uri = github:mozilla/grcov; flake = false; };

  outputs = { self, nixpkgs, import-cargo, grcov }:
    with import nixpkgs { system = "x86_64-linux"; };
    with pkgs;

    let

      buildPackage = { isShell ? false, doCoverage ? false }: stdenv.mkDerivation rec {
        name = "sst-${lib.substring 0 8 self.lastModified}-${self.shortRev or "0000000"}";

        buildInputs =
          [ rustc
            cargo
          ] ++ (if isShell then [
            rustfmt
          ] else [
            (import-cargo.builders.importCargo {
              lockFile = rust/Cargo.lock;
              inherit pkgs;
            }).cargoHome
          ]) ++ lib.optionals doCoverage [ self.packages.x86_64-linux.grcov lcov ];

        src = if isShell then null else self;

        RUSTC_BOOTSTRAP = if doCoverage then "1" else null;
        RUSTFLAGS = lib.optionals doCoverage [ "-Zprofile" "-Ccodegen-units=1" "-Cinline-threshold=0" "-Clink-dead-code" "-Coverflow-checks=off" "-Zno-landing-pads" ];
        CARGO_INCREMENTAL = if doCoverage then "0" else null;

        target = if doCoverage then "" else "--release";

        buildPhase = "cd rust; cargo build ${target} --frozen --offline";

        doCheck = true;

        checkPhase = "cargo test ${target} --frozen --offline";

        installPhase =
          if doCoverage then ''
            grcov ./target/ -s . -t lcov --llvm --branch --ignore-not-existing --ignore-dir "/*" -o app.info
            # FIXME: unify with this with makeGCOVReport in Nixpkgs.
            mkdir -p $out/coverage $out/nix-support
            genhtml -o $out/coverage --show-details --highlight --ignore-errors source --legend app.info
            echo "report coverage $out/coverage" >> $out/nix-support/hydra-build-products
          '' else ''
            mkdir -p $out
            cargo install --frozen --offline --path . --root $out
            rm $out/.crates.toml
          '';
      };

    in {

      packages.x86_64-linux.sst = buildPackage { };

      defaultPackage.x86_64-linux = self.packages.x86_64-linux.sst;

      checks.x86_64-linux.build = self.defaultPackage.x86_64-linux;
      checks.x86_64-linux.coverage = buildPackage { doCoverage = true; };

      devShell.x86_64-linux = buildPackage { isShell = true; };

      # FIXME: should move this into a separate flake.
      packages.x86_64-linux.grcov = stdenv.mkDerivation rec {
        name = "grcov";

        buildInputs =
          [ rustc
            cargo
            (import-cargo.builders.importCargo {
              lockFile = grcov + "/Cargo.lock";
              inherit pkgs;
            }).cargoHome
          ];

        src = grcov;

        buildPhase = "cargo build --release --frozen --offline";

        installPhase =
          ''
            mkdir -p $out
            cargo install --frozen --offline --path . --root $out
            rm $out/.crates.toml
          '';
      };
    };
}
