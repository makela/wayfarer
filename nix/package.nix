# The blog server with its posts and static files, ready to run:
#   POST_DIR=$out/share/taichiwayfarer/posts
#   STATIC_DIR=$out/share/taichiwayfarer/dist
{
  lib,
  rustPlatform,
  tailwindcss_4,
  vips,
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

  nativeBuildInputs = [
    tailwindcss_4
    vips
  ];

  postInstall = ''
    share=$out/share/taichiwayfarer
    mkdir -p $share
    cp -r posts dist $share/
    tailwindcss -i ./input.css -o $share/dist/styles.css --minify

    # Images: at most 1200 px (twice the content column), converted to sRGB
    # and without metadata such as GPS locations. The names stay the same, so
    # posts keep referring to them as before. The output path is absolute:
    # vipsthumbnail would otherwise write next to the input file.
    # A test in src/assets.rs makes sure dist/img has only these formats.
    shopt -s nullglob nocaseglob
    for image in dist/img/*.jpg dist/img/*.jpeg dist/img/*.png; do
      name=$(basename "$image")
      case "$name" in
        *.[pP][nN][gG])
          options="keep=none,compression=9"
          # Keep icons drawn with few colours small: save them with a palette again.
          if [ "$(vipsheader -f palette "$image" 2>/dev/null)" = 1 ]; then
            options="$options,palette,Q=100"
          fi
          ;;
        *) options="Q=80,keep=none,optimize_coding,interlace" ;;
      esac
      vipsthumbnail "$image" --size '1200x1200>' --export-profile srgb \
        -o "$share/dist/img/$name[$options]"
    done
    shopt -u nullglob nocaseglob
  '';

  meta = {
    description = "Tai Chi Wayfarer blog";
    mainProgram = "taichiwayfarer";
  };
}
