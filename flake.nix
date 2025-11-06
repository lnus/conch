# TODO: could add fenix. for now, whatever
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    naersk,
  }: let
    # software is probably cross-platform
    pkgs = nixpkgs.legacyPackages."x86_64-linux";
    naerskLib = pkgs.callPackage naersk {};
  in {
    devShells."x86_64-linux".default = pkgs.mkShell {
      buildInputs = with pkgs; [
        cargo
        rustc
        rustfmt
        clippy
        rust-analyzer
      ];

      nativeBuildInputs = [pkgs.pkg-config];
      env.RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
    };

    packages."x86_64-linux".default = naerskLib.buildPackage {
      src = ./.;
      buildInputs = [];
      nativeBuildInputs = [pkgs.pkg-config];
    };
  };
}
