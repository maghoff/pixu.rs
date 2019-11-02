Photo sharing web application.

Experimental stuff
==================
Using feature [`unsized_locals`] in nightly:

    rustup override set nightly

[`unsized_locals`]: https://doc.rust-lang.org/beta/unstable-book/language-features/unsized-locals.html

Run
===
    cargo run -- config.toml test.db

Create `config.toml` based on `config.toml-template`.

Database migrations
===================
You'll need `diesel_cli`:

    cargo install diesel_cli --no-default-features --features "sqlite-bundled"
