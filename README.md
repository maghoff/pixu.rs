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
       1. Initiate with email address over POST request
       2. Get back JWT Cookie containing random integer and email address
       3. Get email with link with embedded JWT with same email and random
          integer
       4. When opening URL, server verifies against cookie (prevent session
          hijacking)
       5. If OK, issue real login JWT
    - Proper secret for signing JWT
    - Persistent cookies
