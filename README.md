Photo sharing web application.

Dependencies and setup
======================
For working with database migrations, you'll have use for `diesel_cli`:

    cargo install diesel_cli --no-default-features --features "sqlite-bundled"

For JavaScript compilation:

    npm install babel-loader @babel/core @babel/preset-env \
        webpack webpack-cli webpack-dev-server

Create `config.toml` based on `config.toml-template`. For rapid iteration on
JS code, also create `config-dev-server.toml`. It should be configured so
`url` points to the `webpack-dev-server` URL, likely `http://localhost:8080/`.

You need to set yourself up as an authorized uploader in the database file. To
do so, first start the program to get the correct schemas set up, then execute

    sqlite3 test.db 'INSERT INTO uploaders VALUES ("your-email-address");'

Run
===
    ./node_modules/.bin/webpack
    cargo run -- config.toml test.db

JavaScript
==========
Building JS bundle up front, for production and backend development:

    ./node_modules/.bin/webpack

Serving JS bundle from webpack-dev-server for rapid iteration on JS code:

    ./node_modules/.bin/webpack-dev-server &
    cargo run --features=dev-server -- config-dev-server.toml test.db

`--features=dev-server` allows the backend to build without the frontend code
ready.
