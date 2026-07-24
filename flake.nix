{
  description = "Transono development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in {
        devShells.default = pkgs.mkShell {
          name = "transono-dev";

        shellHook = ''
          project_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

          __prompt() {
            local rel
            rel="$(realpath --relative-to="$project_root" "$PWD")"
            PS1="🦀 transono/$rel \$ "
          }

          PROMPT_COMMAND=__prompt
        '';

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer

            alsa-lib
            pipewire

            openssl

            clang
            llvmPackages.libclang
          ];

          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.alsa-lib
            pkgs.pipewire
            pkgs.openssl
          ];
        };
      });
}
