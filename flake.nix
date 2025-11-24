{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
    naersk,
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};

    # TODO use minimal toolchain, add other deps in devshell.
    # TEMP for now, use complete
    toolchain = fenix.packages.${system}.complete.toolchain;

    naerskLib = pkgs.callPackage naersk {
      cargo = toolchain;
      rustc = toolchain;
    };
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = [
        toolchain
        pkgs.cargo-flamegraph
      ];
    };

    packages.${system}.default = naerskLib.buildPackage {
      src = ./.;
    };
  };
}
