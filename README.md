Photo sharing web application.

Experimental stuff
==================
Developed with nightly:

    rustup override set nightly-2018-12-20

Run
===
    cargo run -- test.db

Database migrations
===================
You'll need `diesel_cli`:

    cargo install diesel_cli --no-default-features --features "sqlite-bundled"
