# Blog!

My blog's backend, built with the Axum web framework and MongoDB.

## How to build yourself

Create an environment variable with name "BLOG_DB", which contains a MongoDB cluster database URL.

```bash
$ export BLOG_DB="very secure and sensitive database url"
```

Install Rust, then run

```bash
$ cargo run
```

The server will run on port 4000. Have fun!
