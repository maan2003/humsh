{
  outputs = { nixpkgs, self } @ inputs:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in
  {
    overlay = self: super: {
      humsh = self.rustPlatform.buildRustPackage {
        pname = "humsh";
        version = "0.1.0";
        src = inputs.self;
        cargoHash = "sha256-uHwpeXFFYQ1tcMHKpI7v+Vaden7Uux36tH89t/HXChU=";
      };
    };
    devShells.${system}.default = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [ cargo rustc clippy rustfmt rust-analyzer zoxide fzf fd ];
    };
  };
}
