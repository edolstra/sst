{
  name = "sst";

  edition = 201906;

  description = "A simple, extensible, unambiguous markup language";

  inputs =
    [ "nixpkgs"
      github:edolstra/import-cargo
    ];

  nonFlakeInputs.grcov = github:mozilla/grcov;

  outputs = inputs:
    with import inputs.nixpkgs { system = "x86_64-linux"; };
    with pkgs;

    rec {

      builders.buildPackage = { isShell ? false, doCoverage ? false }: stdenv.mkDerivation rec {
        name = "sst-${lib.substring 0 8 inputs.self.lastModified}-${inputs.self.shortRev or "0000000"}";

        buildInputs =
          [ rustc
            cargo
          ] ++ (if isShell then [
            rustfmt
          ] else [
            (inputs.import-cargo.builders.importCargo {
              lockFile = rust/Cargo.lock;
              inherit pkgs;
            }).cargoHome
          ]) ++ lib.optionals doCoverage [ packages.grcov lcov ];

        src = if isShell then null else inputs.self;

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

      packages.sst = builders.buildPackage { };

      defaultPackage = packages.sst;

      checks.build = defaultPackage;
      checks.coverage = builders.buildPackage { doCoverage = true; };

      devShell = builders.buildPackage { isShell = true; };

      # FIXME: should move this into a separate flake.
      packages.grcov = stdenv.mkDerivation rec {
        name = "grcov";

        buildInputs =
          [ rustc
            cargo
            (inputs.import-cargo.builders.importCargo {
              lockFile = inputs.grcov + "/Cargo.lock";
              inherit pkgs;
            }).cargoHome
          ];

        src = inputs.grcov;

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
