# pulse
Martial Arts Tournament app.

## Run locally
1. Install Rust and Cargo.
2. Start the server:
   - `cargo run`
3. Open `http://localhost:8000`.

Set `DATABASE_URL` to a MariaDB/MySQL connection string, e.g.
`mysql://root@127.0.0.1:3306/pulse-db`.
You can use a `.env` file (see `.env.example`).

## Deployment (RunCloud + GHCR)
This repo includes a Dockerfile and a GitHub Actions workflow that builds and pushes
`ghcr.io/<owner>/pulse:latest` on each push to `master`, then SSHes into your server to
pull and restart the container.

### Server setup
Create `/opt/pulse/docker-compose.yml` on the VPS:

```
services:
  app:
    image: ghcr.io/<owner>/pulse:latest
    restart: unless-stopped
    ports:
      - "127.0.0.1:8001:8001"
    environment:
      ROCKET_ADDRESS: 0.0.0.0
      ROCKET_PORT: 8001
      DATABASE_URL: mysql://root:YOUR_PASSWORD@127.0.0.1:3306/pulse-db
```

Then point RunCloud's Nginx vhost to `http://127.0.0.1:8001`.

Example Nginx location block:

```
location / {
  proxy_pass http://127.0.0.1:8001;
  proxy_set_header Host $host;
  proxy_set_header X-Real-IP $remote_addr;
  proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
  proxy_set_header X-Forwarded-Proto $scheme;
}
```

### GitHub Secrets
Add these repository secrets:

- `SERVER_HOST`
- `SERVER_USER`
- `SERVER_SSH_KEY`
- optional: `SERVER_PORT`

### First deploy checklist
1. Create the `/opt/pulse` folder and `docker-compose.yml`.
2. Make sure Docker and Compose are installed on the server.
3. Add the GitHub secrets above.
4. Push to `master` and confirm the workflow run succeeds.
5. Visit your domain and complete the initial setup.
