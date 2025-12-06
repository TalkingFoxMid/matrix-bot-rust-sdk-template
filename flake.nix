{
  description = "Matrix bot example";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
          sqlite
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

          shellHook = ''
            echo "🦀 Rust development environment loaded!"
            echo ""
            echo "Available commands:"
            echo "  cargo build   - Build the project"
            echo "  cargo run     - Run the bot (requires arguments)"
            echo "  cargo check   - Check the code for errors"
            echo ""
            echo "To run the bot:"
            echo "  cargo run -- <homeserver_url> <username> <password>"
            echo ""
            echo "Example:"
            echo "  cargo run -- https://matrix.org @mybot:matrix.org mypassword"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "example-getting-started";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;
        };
      }
    );
}
