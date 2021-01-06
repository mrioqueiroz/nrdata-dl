with import <nixpkgs> {};

stdenv.mkDerivation {
    name = "nrdata-dl-environment";
    buildInputs = [
        rustup
        openssl
    ];
}
