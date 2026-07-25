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
              source ${./nix/prompt.sh}

              export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.glibc.dev}/include"

            mkdir -p .cargo
            cat <<EOF > .cargo/config.toml
[env]
PATH = { value = "${pkgs.lib.makeBinPath [ pkgs.pkg-config pkgs.clang pkgs.rustc pkgs.cargo ]}:\$PATH", relative = false }

PKG_CONFIG_PATH = { value = "${pkgs.lib.makeSearchPath "lib/pkgconfig" [ pkgs.pipewire.dev pkgs.alsa-lib.dev ]}", relative = false }

BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.glibc.dev}/include"

LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib"
EOF
'';

          nativeBuildInputs = with pkgs; [
            pkg-config
            clang
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
          ];

          buildInputs = with pkgs; [
            glibc.dev
            alsa-lib
            pipewire
            openssl
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
