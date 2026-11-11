# axum-notes-api

[![CI](https://github.com/zairithm/axum-notes-api/actions/workflows/ci.yml/badge.svg)](https://github.com/zairithm/axum-notes-api/actions/workflows/ci.yml)

A REST API for notes, built with Axum. It starts with an in-memory store and moves to PostgreSQL with authentication.

## What this project covers

- HTTP routing, extractors and JSON with `axum`
- Shared application state
- PostgreSQL with `sqlx` and migrations
- Authentication with hashed passwords and JWT
- Tests that call the router directly

## Getting started

```bash
cargo run
curl http://127.0.0.1:3000/health
```

## Roadmap

Each open item is tracked as a [GitHub issue](https://github.com/zairithm/axum-notes-api/issues).

- [x] Server with a `/health` endpoint and a test
- [ ] CRUD for notes with an in-memory store
- [ ] Input validation and proper error responses
- [ ] PostgreSQL with `sqlx`
- [ ] Sign-up, login and JWT-protected routes
- [ ] Docker Compose and a deployment

## License

MIT
