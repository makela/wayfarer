[parallel]
watch: dev tailwind

dev:
    DEV= cargo watch --no-gitignore -x "run"

tailwind:
    tailwindcss -i ./input.css -o ./dist/styles.css --watch

release:
    tailwindcss -i ./input.css -o ./dist/styles.css --minify
    cargo build --release
