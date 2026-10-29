[parallel]
watch: dev tailwind

# Restart on code, post, static file and .env changes. Gitignored files are
# watched too, so a rebuilt dist/styles.css gets a new asset hash.
dev:
    DEV= watchexec --restart --no-vcs-ignore --no-project-ignore \
        --watch src --watch posts --watch dist --watch Cargo.toml --watch .env \
        -- cargo run

tailwind:
    tailwindcss -i ./input.css -o ./dist/styles.css --watch

release:
    tailwindcss -i ./input.css -o ./dist/styles.css --minify
    cargo build --release
