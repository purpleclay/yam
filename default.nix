{
  darwin,
  fetchFromGitHub,
  lib,
  pkg-config,
  rustPlatform,
  stdenv,
  zlib,
}:
rustPlatform.buildRustPackage {
  pname = "yam";
  version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    zlib
  ];

  meta = with lib; {
    homepage = "https://github.com/purpleclay/yam";
    changelog = "https://github.com/purpleclay/yam/releases/tag/${version}";
    description = "Context aware YAML to markdown document generator";
    license = licenses.mit;
    maintainers = with maintainers; [purpleclay];
  };

  doCheck = false;
}
