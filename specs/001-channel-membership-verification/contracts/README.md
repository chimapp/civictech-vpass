# API Contracts

This directory contains API contract specifications for the VPass system.

## Files

- **openapi.yaml**: OpenAPI 3.0 specification for all REST endpoints

## Viewing the API Documentation

### Option 1: Swagger UI (Recommended)

```bash
# Install swagger-ui-watcher
npm install -g swagger-ui-watcher

# View documentation
swagger-ui-watcher openapi.yaml
```

Then open http://localhost:8000

### Option 2: Online Editor

Visit https://editor.swagger.io/ and paste the contents of `openapi.yaml`

### Option 3: VS Code Extension

Install "OpenAPI (Swagger) Editor" extension and open `openapi.yaml`

## API Endpoints Summary

### Authentication Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/auth/{platform}/login` | Initiate OAuth2 login |
| GET | `/auth/{platform}/callback` | OAuth2 callback handler |
| POST | `/auth/logout` | Log out current user |
| GET | `/auth/session` | Get current session info |

### Card Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/cards/claim` | Claim new membership card |
| GET | `/cards/{card_id}` | Get card details |
| GET | `/cards/my-cards` | List user's cards |
| GET | `/cards/{card_id}/qr` | Get QR code image |

### Verification Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/verify/scan` | Verify scanned QR code |
| GET | `/verify/history` | Get verification history |

### Issuer Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/issuers` | List available card issuers |

## Authentication

All endpoints except `/auth/{platform}/login`, `/auth/{platform}/callback`, and `/issuers` require authentication via session cookie.

Session cookie name: `vpass_session`

## Error Responses

All error responses follow this format:

```json
{
  "error": "machine_readable_code",
  "message": "Human-readable error message",
  "details": {
    "additional": "context"
  }
}
```

Common HTTP status codes:

- **400**: Bad Request (invalid input)
- **401**: Unauthorized (not authenticated)
- **403**: Forbidden (authenticated but not authorized)
- **404**: Not Found
- **500**: Internal Server Error
- **503**: Service Unavailable (platform API down)

## Rate Limiting

Not implemented in v1. Future versions may add rate limiting headers:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1609459200
```

## Versioning

API version is included in response headers:

```
X-API-Version: 1.0.0
```

Breaking changes will increment major version.

## Testing

Use tools like:

- **curl**: Command-line HTTP testing
- **httpie**: Human-friendly HTTP client
- **Postman**: GUI-based API testing
- **Insomnia**: Alternative to Postman

Example curl command:

```bash
# Initiate OAuth login
curl -i "http://localhost:3000/auth/youtube/login?role=member"

# Get session info (with cookie)
curl -i -b "vpass_session=..." "http://localhost:3000/auth/session"

# Claim a card
curl -X POST \
  -H "Content-Type: application/json" \
  -b "vpass_session=..." \
  -d '{
        "issuer_id": "123e4567-e89b-12d3-a456-426614174000",
        "verification_comment_url": "https://www.youtube.com/watch?v=VIDEO_ID&lc=COMMENT_ID"
      }' \
  "http://localhost:3000/cards/claim"
```

## Implementation Notes

- Use Axum's `Router` for route definition
- Session management via `tower-sessions`
- Request/response serialization with `serde_json`
- OAuth flows implemented in `src/api/auth.rs`
- Card operations in `src/api/cards.rs`
- Verification logic in `src/api/verification.rs`
- Members-only comment verification implemented in `src/services/comment_verifier.rs`

## Future Enhancements

- WebSocket endpoint for real-time verification updates
- Batch card issuance endpoint
- Admin endpoints for issuer management
- Metrics and health check endpoints
