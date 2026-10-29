# The blog server with its posts and static files, ready to run:
#   POST_DIR=$out/share/taichiwayfarer/posts
#   STATIC_DIR=$out/share/taichiwayfarer/dist
{
  lib,
  rustPlatform,
  tailwindcss_4,
}:
let
  fs = lib.fileset;
in
rustPlatform.buildRustPackage {
  pname = "taichiwayfarer";
  version = (lib.importTOML ../Cargo.toml).package.version;

  # Only what the build needs, so e.g. target/ or README edits don't
  # cause rebuilds when this is used straight from a clone.
  src = fs.toSource {
    root = ../.;
    fileset = fs.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../posts
      ../input.css
      (fs.difference ../dist (fs.maybeMissing ../dist/styles.css))
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ tailwindcss_4 ];

  postInstall = ''
    share=$out/share/taichiwayfarer
    mkdir -p $share
    cp -r posts dist $share/
    tailwindcss -i ./input.css -o $share/dist/styles.css --minify
  '';

  meta = {
    description = "Tai Chi Wayfarer blog";
    mainProgram = "taichiwayfarer";
  };
}
