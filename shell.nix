with import <nixpkgs> {};
  mkShell {
    nativeBuildInputs = [
      rustup
      rust-analyzer
      rustPlatform.bindgenHook
      openssl
      pkg-config
    ];
    shellHook = ''
      rustup default stable
      rustup component add rust-analyzer
    '';
  }
