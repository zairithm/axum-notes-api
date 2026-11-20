# axum-notes-api

[![CI](https://github.com/zairithm/axum-notes-api/actions/workflows/ci.yml/badge.svg)](https://github.com/zairithm/axum-notes-api/actions/workflows/ci.yml)

A REST API for notes with user accounts, written in Rust with Axum. Data is stored in PostgreSQL through `sqlx`, or in memory when no database is configured.

## API

| Method | Path | Login | Result |
|---|---|---|---|
| GET | `/health` | | `{"status":"ok"}` |
| POST | `/signup` | | 201 with `{id, username}` |
| POST | `/login` | | `{token, token_type, expires_in}` |
| POST | `/notes` | yes | 201 with the created note |
| GET | `/notes` | yes | Your notes, oldest first |
| GET | `/notes/{id}` | yes | The note, or 404 |
| PUT | `/notes/{id}` | yes | The updated note, or 404 |
| DELETE | `/notes/{id}` | yes | 204, or 404 |

Routes marked "yes" need `Authorization: Bearer <token>`. Each user sees only their own notes; someone else's note answers 404, the same as one that doesn't exist.

Usernames are 3 to 32 letters, digits or `_` (stored lowercase). Passwords are 8 to 128 characters, hashed with Argon2id. Tokens are HS256 JWTs valid for 24 hours.

A note body looks like `{"title": "Groceries", "body": "milk, eggs"}`. `body` is optional, and the title is trimmed.

Errors are always JSON, `{"error": "..."}`:

| Status | When |
|---|---|
| 400 | The body isn't valid JSON for the endpoint, or the path is wrong (`/notes/abc`) |
| 401 | Missing or invalid token, or wrong username/password (same message for both) |
| 404 | No such note, or no such route |
| 409 | Username already taken |
| 422 | Empty title, title over 200 characters, body over 10,000 |
| 500 | A database error (details go to the server log, not the client) |

## Run it

In memory, nothing to set up:

```bash
cargo run
curl -X POST localhost:3000/signup -H 'content-type: application/json' -d '{"username":"zaid","password":"password123"}'
TOKEN=$(curl -s -X POST localhost:3000/login -H 'content-type: application/json' -d '{"username":"zaid","password":"password123"}' | jq -r .token)
curl -X POST localhost:3000/notes -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' -d '{"title":"First note"}'
curl localhost:3000/notes -H "authorization: Bearer $TOKEN"
```

With PostgreSQL (migrations in `migrations/` run on start):

```bash
docker run -d --name notes-db -e POSTGRES_USER=notes -e POSTGRES_PASSWORD=notes -p 5432:5432 postgres:16
DATABASE_URL=postgres://notes:notes@localhost:5432/notes cargo run
```

| Variable | Meaning |
|---|---|
| `DATABASE_URL` | Use PostgreSQL; otherwise data is kept in memory |
| `JWT_SECRET` | At least 32 characters. If unset, a random one is used and tokens stop working on restart |
| `ADDR` | Listen address, default `127.0.0.1:3000` |

## How it is built

```
src/
  lib.rs       Router, AppState and handlers
  auth.rs      Argon2 hashing, JWT issue/verify, AuthUser extractor
  store.rs     Store enum: Memory or Postgres, same async methods
  memory.rs    In-memory store (Arc<RwLock<BTreeMap>>)
  postgres.rs  PostgreSQL store with sqlx, runtime-checked queries
  models.rs    Note, NewNote and validation
  error.rs     AppError -> JSON response, plus AppJson/AppPath extractors
migrations/    SQL schema
tests/
  notes.rs     The whole API against the in-memory store
  auth.rs      Sign-up, login, tokens and note ownership
  postgres.rs  The same flow against a real database
```

- Handlers only talk to `Store`, so they don't know which backend is in use
- `AuthUser` is an extractor: a handler that takes it only runs for a valid token
- Password hashing runs in `spawn_blocking` so it doesn't stall other requests
- Failed logins check a dummy hash when the user doesn't exist, so timing doesn't reveal which usernames are registered
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
- [x] Sign-up, login and JWT-protected routes, notes private to each user
- [ ] Docker Compose and a deployment

## License

MIT
