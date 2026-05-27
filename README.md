I need a name for this thing please 🙏

# Development

First, copy `.env.example` to `.env` and fill in the values accordingly. Also copy `config.example.json5` to `config.json5` and fill in the values accordingly.

First, do `docker compose up -d` to start a container with PostgreSQL, then do `cargo sqlx migrate run` (in the root directory, not `bot/`).
To start the bot after you have applied the initial sqlx migrations, you can do `pnpm dev` (in the root directory), which will start both the bot and the dashboard.

# Building for production

You can't rn

# Resources

CSS Loaders: https://cssloaders.github.io

Generating the cookie secret: `openssl rand -base64 64`