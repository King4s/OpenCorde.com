# Password Reset System - Quick Start Guide

## What Was Built

A complete password reset feature for OpenCorde with:
- Secure token-based password reset flow
- Email service (dev mode logs, production-ready)
- Frontend UI (login page + reset page)
- Database schema (migration)
- API endpoints

## Files Created/Modified

### New Files
1. `/crates/opencorde-db/migrations/029_password_reset.sql` - Database migration
2. `/crates/opencorde-api/src/email.rs` - Email service module
3. `/crates/opencorde-api/src/routes/auth/password_reset.rs` - Password reset handlers
4. `/client/src/routes/reset-password/+page.svelte` - Reset password page
5. `PASSWORD_RESET_SYSTEM.md` - Full documentation

### Modified Files
1. `/crates/opencorde-api/src/config.rs` - Added SMTP configuration
2. `/crates/opencorde-api/src/lib.rs` - Added EmailService to AppState
3. `/crates/opencorde-api/src/main.rs` - Initialize email service
4. `/crates/opencorde-api/src/routes/auth/mod.rs` - Register new routes
5. `/crates/opencorde-api/Cargo.toml` - Added dependencies (hex, password-hash)
6. `/client/src/routes/login/+page.svelte` - Added forgot password form

## How to Deploy

### 1. Run Database Migration
```bash
# On your PostgreSQL instance
psql -U opencorde -d opencorde < migrations/029_password_reset.sql
```

### 2. Configure Environment (optional)

**Development (logs reset links):**
```bash
# No SMTP configuration needed, links appear in logs
```

**Production (production-ready):**
```bash
SMTP_HOST=your-smtp-server.com
SMTP_PORT=587
SMTP_USERNAME=your-email@example.com
SMTP_PASSWORD=your-password
SMTP_FROM=noreply@opencorde.com
BASE_URL=https://your-opencorde-domain.com
```

### 3. Build and Deploy
```bash
# API compiles with full password reset support
cargo build --release --package opencorde-api

# Frontend includes reset password page
npm run build
```

## API Endpoints

### Request Password Reset
```
POST /api/v1/auth/forgot-password
Content-Type: application/json

{
  "email": "user@example.com"
}

Response (always 200):
{
  "success": true,
  "message": "If this email exists, a reset link has been sent."
}
```

### Complete Password Reset
```
POST /api/v1/auth/reset-password
Content-Type: application/json

{
  "token": "hex-encoded-token-from-email",
  "new_password": "NewPassword123"
}

Response (200):
{
  "success": true,
  "message": "Password reset successfully. You can now log in."
}
```

## User Flow

1. User clicks "Forgot password?" on login page
2. User enters email address
3. User sees: "If this email exists, a reset link has been sent."
4. User receives email with reset link (or sees link in dev logs)
5. User clicks link → goes to `/reset-password?token=...`
6. User enters new password twice
7. On success → redirected to login
8. User logs in with new password

## Testing

### Run Tests
```bash
cargo test --package opencorde-api email
cargo test --package opencorde-api password_reset
```

### Manual Testing
1. Start server with `cargo run --bin opencorde-api`
2. Go to `http://localhost:5173/login`
3. Click "Forgot password?"
4. Enter email of existing user
5. Check logs for reset link (dev mode)
6. Use token from logs in URL: `/reset-password?token=...`
7. Enter new password and submit
8. Login with new password

## Security Features

- 256-bit random tokens (32 bytes)
- One-time use only
- 1-hour expiration
- Email enumeration prevention (always return 200)
- Argon2id password hashing
- Comprehensive audit logging

## Environment Variables

```
SMTP_HOST          - SMTP server (optional, dev mode if not set)
SMTP_PORT          - SMTP port (default: 587)
SMTP_USERNAME      - SMTP user (optional)
SMTP_PASSWORD      - SMTP password (optional)
SMTP_FROM          - Sender email (default: noreply@localhost)
BASE_URL           - Base URL for reset links (default: http://localhost:5173)
```

## Troubleshooting

### "Token is missing" message
- Reset link expired (1 hour)
- User accessed reset page directly without token
- Token was already used

### Password reset email not received (dev mode)
- Check server logs for password reset message
- Look for "password reset email (development mode - not sent)"
- Extract token from log and use manually

### "Invalid or expired reset token"
- Token doesn't exist in database
- Token has expired (> 1 hour old)
- Token was already used
- Wrong token in URL

## File Structure

```
crates/
├── opencorde-api/
│   ├── src/
│   │   ├── email.rs                          (NEW - 229 lines)
│   │   ├── config.rs                         (MODIFIED - added SMTP config)
│   │   ├── lib.rs                            (MODIFIED - added EmailService)
│   │   ├── main.rs                           (MODIFIED - initialize email)
│   │   └── routes/
│   │       └── auth/
│   │           ├── password_reset.rs         (NEW - 235 lines)
│   │           └── mod.rs                    (MODIFIED - register routes)
│   └── Cargo.toml                            (MODIFIED - added deps)
└── opencorde-db/
    └── migrations/
        └── 029_password_reset.sql            (NEW - 26 lines)

client/
└── src/
    └── routes/
        ├── login/
        │   └── +page.svelte                  (MODIFIED - added forgot password)
        └── reset-password/
            └── +page.svelte                  (NEW - 165 lines)
```

## What's Next

1. Test the complete flow
2. Verify emails in development mode appear in logs
3. Configure SMTP for production if needed
4. Deploy to dev server for testing
5. Verify reset password works end-to-end

## Documentation

For complete details, see `PASSWORD_RESET_SYSTEM.md`
