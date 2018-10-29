{ nixpkgs ? fetchTarball channel:nixos-18.09
, pkgs ? import nixpkgs {}
}:

with pkgs;

stdenv.mkDerivation {
  name = "sst-rust";

  buildInputs = [ rustc cargo ];
}
