# axum-notes-api

[![CI](https://github.com/zairithm/axum-notes-api/actions/workflows/ci.yml/badge.svg)](https://github.com/zairithm/axum-notes-api/actions/workflows/ci.yml)

A REST API for notes, written in Rust with Axum. Notes are stored in PostgreSQL through `sqlx`, or in memory when no database is configured.

## API

| Method | Path | Result |
|---|---|---|
| GET | `/health` | `{"status":"ok"}` |
| POST | `/notes` | 201 with the created note |
| GET | `/notes` | All notes, oldest first |
| GET | `/notes/{id}` | The note, or 404 |
| PUT | `/notes/{id}` | The updated note, or 404 |
| DELETE | `/notes/{id}` | 204, or 404 |

A note body looks like `{"title": "Groceries", "body": "milk, eggs"}`. `body` is optional, and the title is trimmed.

Errors are always JSON, `{"error": "..."}`:

| Status | When |
|---|---|
| 400 | The body isn't valid JSON for the endpoint, or the path is wrong (`/notes/abc`) |
| 404 | No such note, or no such route |
| 422 | Empty title, title over 200 characters, body over 10,000 |
| 500 | A database error (details go to the server log, not the client) |

## Run it

In memory, nothing to set up:

```bash
cargo run
curl -X POST localhost:3000/notes -H 'content-type: application/json' -d '{"title":"First note"}'
curl localhost:3000/notes
```

With PostgreSQL (migrations in `migrations/` run on start):

```bash
docker run -d --name notes-db -e POSTGRES_USER=notes -e POSTGRES_PASSWORD=notes -p 5432:5432 postgres:16
DATABASE_URL=postgres://notes:notes@localhost:5432/notes cargo run
```

`ADDR` changes the listen address (default `127.0.0.1:3000`).

## How it is built

```
src/
  lib.rs       Router and handlers
  store.rs     Store enum: Memory or Postgres, same async methods
  memory.rs    In-memory store (Arc<RwLock<BTreeMap>>)
  postgres.rs  PostgreSQL store with sqlx, runtime-checked queries
  models.rs    Note, NewNote and validation
  error.rs     AppError -> JSON response, plus AppJson/AppPath extractors
migrations/    SQL schema
tests/
  notes.rs     The whole API against the in-memory store
  postgres.rs  The same flow against a real database
```

- Handlers only talk to `Store`, so they don't know which backend is in use
- Queries use `sqlx::query_as` (checked at run time), so building never needs a database
- `AppJson` and `AppPath` wrap axum's extractors so bad input becomes a JSON error instead of plain text

## Tests

```bash
cargo test                                   # in-memory tests; the Postgres test skips
DATABASE_URL=postgres://... cargo test       # also runs the Postgres test
```

CI starts a `postgres:16` service and runs everything, and fails if the database test would be skipped.

## Roadmap

- [x] Server with a `/health` endpoint
- [x] CRUD for notes with an in-memory store
- [x] Input validation and JSON error responses
- [x] PostgreSQL with `sqlx` and migrations
- [ ] Sign-up, login and JWT-protected routes
- [ ] Docker Compose and a deployment

## License

MIT
