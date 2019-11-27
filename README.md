Photo sharing web application.

Experimental stuff
==================
Using feature [`unsized_locals`] in nightly:

    rustup override set nightly

[`unsized_locals`]: https://doc.rust-lang.org/beta/unstable-book/language-features/unsized-locals.html

Run
===
    ./node_modules/.bin/webpack
    cargo run -- config.toml test.db

Create `config.toml` based on `config.toml-template`.

Working with database migrations
================================
You'll probably have use for `diesel_cli`:

    cargo install diesel_cli --no-default-features --features "sqlite-bundled"

JavaScript
==========
    npm install babel-loader @babel/core @babel/preset-env \
        webpack webpack-cli webpack-dev-server

Building JS bundle up front, for production and backend development:

    ./node_modules/.bin/webpack

Serving JS bundle from webpack-dev-server for rapid iteration on JS code:

    ./node_modules/.bin/webpack-dev-server &
    cargo run --features=dev-server -- config-dev-server.toml test.db

`--features=dev-server` allows the backend to build without the frontend code
ready. `config-dev-server.toml` should be configured so `url` points to the
`webpack-dev-server` URL, likely `http://localhost:8080/`.
