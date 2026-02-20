# Sentinel - Dead Man Switch

<p align="center">
  <img src="./nginx/landing/dead_man_switch.jpg" width="300" alt="Dead Man Switch">
</p>

A dead man switch that sends alerts if you don't check in regularly.

## Setup

1. Create `.env` file:
```bash
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
SMTP_FROM=sentinel@yourdomain.com
ADMIN_USERNAME=admin
ADMIN_PASSWORD=your-secure-password
```

2. Start:
```bash
docker-compose up -d
```

3. Open http://localhost:9999 and login.

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  You check in regularly (web UI or API call)                   │
│                                                                 │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
         ┌───────────────────────┐
         │   Last Check-in Time  │
         │   Timer resets        │
         └───────────┬───────────┘
                     │
                     ▼
         ┌───────────────────────┐
         │   Watchdog checks     │
         │   every 10 seconds    │
         └───────────┬───────────┘
                     │
        ┌────────────┴────────────┐
        │                         │
        ▼                         ▼
   ┌─────────┐             ┌──────────┐
   │ Still   │             │ Timeout  │
   │ OK      │             │ reached  │
   └─────────┘             └────┬─────┘
                                │
                    ┌───────────┴───────────┐
                    │                       │
                    ▼                       ▼
          ┌──────────────────┐    ┌─────────────────┐
          │ Warning Stage    │    │ Final Deadline  │
          │ (optional)       │    │ (required)      │
          │                  │    │                 │
          │ Send warnings    │    │ Execute final   │
          │ at 1h, 2h, etc   │    │ actions         │
          │ before deadline  │    │                 │
          └──────────────────┘    └────────┬────────┘
                                           │
                             ┌─────────────┼─────────────┐
                             │             │             │
                             ▼             ▼             ▼
                        ┌────────┐   ┌──────────┐  ┌────────┐
                        │ Email  │   │ Webhook  │  │ Script │
                        └────────┘   └──────────┘  └────────┘
```

## Creating a Switch

1. Click "Create New Switch" in dashboard
2. Fill in:
   - **Name:** "My Weekly Check"
   - **Timeout:** 604800 seconds (7 days)
   - **Trigger Count:** 1 (how many times to trigger, 0 = infinite)
   - **Trigger Interval:** 300 seconds (time between trigger executions)
   - **Warning stages:** 86400 (1 day before - optional, comma-separated)
   - **Warning Actions:** Optional actions to run at warning stages
   - **Final Actions:** Required actions to run when deadline is reached

3. Save and copy the API token for automated check-ins

### Adding Email Actions

When configuring email actions:
- **BCC Recipients:** Enter email addresses (comma-separated for multiple)
- Recipients are sent as BCC - they won't see each other's addresses
- The email will have a "To:" header set to your SMTP From address
- This ensures privacy while maintaining email delivery standards

Example: `alice@example.com, bob@example.com, charlie@example.com`

**⚠️ Important:** Switches cannot be deleted once created. 

## Check-in Methods

**Web UI:** Click the "Check In" button

**API Call:**
```bash
curl -X POST http://localhost:9999/api/checkin/SWITCH_ID \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Cron job (daily at noon):**
```bash
0 12 * * * curl -X POST http://localhost:9999/api/checkin/SWITCH_ID -H "Authorization: Bearer YOUR_TOKEN"
```

## Action Types

**Email:**
- Send email via SMTP to multiple recipients (BCC)
- All recipients are blind-copied for privacy
- Email "To:" header is set to your SMTP From address
- Recipients cannot see each other's addresses

**Webhook:**
- POST/GET to any URL
- Optional custom headers and body

**Script:**
- Run bash scripts from `scripts/` folder
- Scripts appear in dropdown selection in UI
- Can pass custom arguments

## Files

```
.
├── docker-compose.yml    # Start with docker-compose up -d
├── .env                  # Your config (create from .env.example)
├── scripts/              # Put your custom scripts here
│   └── example.sh        # Example script
└── nginx/
    └── landing/          # Landing page files
```

## Logs

```bash
docker-compose logs -f sentinel
```

## Reset

```bash
docker-compose down
docker volume rm dead-man-switch_sentinel-data
docker-compose up -d
```

## Disclaimer 

This project was mostly vibecoded. Use at your own risk, I tested it and it does what I want.

## Contributions

PRs, issues, suggestions are welcomed. 