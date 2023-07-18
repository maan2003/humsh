{
  outputs = { nixpkgs, self }:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in
  {
    devShells.${system}.default = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [ cargo rustc rustfmt rust-analyzer starship zoxide fish fzf ];
    };
  };
}
