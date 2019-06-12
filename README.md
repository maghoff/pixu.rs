Photo sharing web application.

Experimental stuff
==================
Developed with nightly:

    rustup override set nightly

Run
===
    cargo run --bin pixurs -- test.db

Database migrations
===================
You'll need `diesel_cli`:

    cargo install diesel_cli --no-default-features --features "sqlite-bundled"

TODO
====
 * Login:
    - Redirect flow so you end up where you started when unauthorized
    - Email-verification login
    - Proper secret for signing JWT
    - Persistent cookies
