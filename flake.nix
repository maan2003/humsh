{
  outputs = { nixpkgs, self } @ inputs:
  let
    system = "x86_64-linux";
    overlay = self: super: {
      humsh = self.rustPlatform.buildRustPackage {
        pname = "humsh";
        version = "0.1.0";
        src = inputs.self;
        cargoHash = "sha256-yKISYvlKH+Gzrp6jjAK9nX6xQWXmT9m510LSsn1KkfY=";
      };
    };
    pkgs = import nixpkgs { inherit system; overlays = [ overlay ]; };
  in
  {
    inherit overlay;
    packages.${system}.default = pkgs.humsh;
    devShells.${system}.default = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [ cargo rustc clippy rustfmt rust-analyzer zoxide fzf fd ];
    };
  };
}
