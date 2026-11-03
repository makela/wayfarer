# Tai Chi Wayfarer

*Blog about my love to Tai Chi — https://www.taichiwayfarer.com*

A small Rust server (axum + maud) that renders the posts from markdown files,
with an RSS feed (`/feed.xml`), a sitemap (`/sitemap.xml`), `/robots.txt` and a
search (`/search?q=…`, see `src/search.rs`). Search ignores case and accents, so
"man quan" finds "màn quán", and also covers the extra content under posts.

## Development

Enter the dev shell with direnv (`.envrc`) or `nix develop`, then:

```sh
just watch     # run on http://localhost:3000 with Tailwind, restart and reload on changes
cargo test     # tests
nix build      # the deployable package, into ./result
```

`just watch` restarts the server when code, posts, static files or `.env`
change, and the browser reloads by itself.

The dev shell also enables a pre-commit hook (`.githooks/pre-commit`) that runs
`cargo fmt --check`, `cargo clippy` and `cargo test`. Outside the dev shell it
starts one itself. Skip it once with `git commit --no-verify`.

## Configuration

All settings are optional environment variables, read also from a `.env` file.
See `.env.example` for the list and defaults: `HOST`, `PORT`, `SITE_URL`,
`NOINDEX`, `POST_DIR`, `STATIC_DIR`, `DEV` and `RUST_LOG`.

The server refuses to start if a post file, an image a post shows or a static
file the pages link to is missing, and logs which one.

## Writing a post

1. Write the post in `posts/<name>.md`.
2. Add it to the top of `POSTS` in `src/posts.rs`, newest first:
   `("Title", "<name>", "2026-10-24 15:28")`, with the time in Helsinki time.
   A post with a time in the future is published when that time comes, without
   a restart. Until then it is left out of the posts, the feed, the sitemap and
   search, and its page is not found. In dev mode (`just watch`) scheduled
   posts show up right away. A line commented out there is a draft.
3. Put photos in `dist/img` and show them with `![Description](/static/img/photo.jpg)`.
   Full-size photos are fine, they are resized when the package is built.

The first paragraph becomes the post's description for search engines, link
previews and the feed, and the first photo its preview image.

Extra content under some posts, such as the movement names of the slow set, is
in `get_extra_markup` in `src/data.rs`.

The tests check that every post has its file and photos, that dates are valid
and in order, and that names are unique.

## Deployment

`nix/package.nix` builds the server with the posts and static files. On the
way it builds a minified `styles.css`, and resizes the photos to at most
1200 px without metadata such as GPS locations. The originals stay as they are.

`nix/module.nix` is a NixOS module that runs the site as a hardened systemd
service, one per instance, listening on 127.0.0.1:3000 by default. Put a
reverse proxy with TLS, such as nginx, in front. Usage, for example with a
`www` instance and a test copy that search engines don't index, is described
at the top of the module.

To publish, update the clone the server builds from and switch:

```sh
git pull
sudo nixos-rebuild switch
```

`sudo nixos-rebuild switch --rollback` returns to the previous version.

## Styles and static files

- Styles: `input.css`, built by Tailwind into `dist/styles.css`, which is not committed.
- Fonts are self-hosted in `dist/fonts` (SIL Open Font License). The files in
  `PRELOADED_FONTS` (`src/assets.rs`) must match the `@font-face` URLs in `input.css`.
- Files listed in `VERSIONED_FILES` (`src/assets.rs`) get a content hash in
  their URL and are cached for a year, other static files for a day.
- Static files the page layout links to are listed in `LINKED_FILES`
  (`src/assets.rs`), so add new ones there.
- Icons: `dist/favicon.svg` is the yin-yang, the same as on taichichuan.mobi.
  `favicon.ico` (16, 32 and 48 px) and `apple-touch-icon.png` (180 px, on the
  page background) are made from it. Render the SVG with libvips (librsvg):
  ImageMagick's own SVG renderer draws this yin-yang mirrored.

  ```sh
  vipsthumbnail dist/favicon.svg --size 148 -o yy.v
  vips flatten yy.v yy-flat.v --background '15 23 43'
  vips embed yy-flat.v 'dist/apple-touch-icon.png[keep=none,compression=9]' \
      16 16 180 180 --extend background --background '15 23 43'
  ```
- YouTube videos load only after the visitor accepts YouTube's cookies,
  see `dist/embed-video.js`.
