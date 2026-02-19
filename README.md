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

## Example Timeline

```
Day 0:  You check in
        ├─ Timer starts: 7 days
        │
Day 6:  Warning stage (1 day before deadline)
        ├─ Email sent: "Please check in soon"
        │
Day 6:  You check in again
        ├─ Timer resets: 7 days
        │
Day 13: You forget to check in
        ├─ Final deadline reached
        ├─ Email sent to emergency contact
        ├─ Webhook triggered
        └─ Custom script executed
```

## Creating a Switch

1. Click "Create New Switch" in dashboard
2. Fill in:
   - Name: "My Weekly Check"
   - Timeout: 604800 seconds (7 days)
   - Warning stages: 86400 (1 day before)
   - Final actions: Add email/webhook/script

3. Save and copy the API token for automated check-ins

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

**Email:** Send email via SMTP
**Webhook:** POST/GET to any URL
**Script:** Run bash scripts from `scripts/` folder (dropdown selection in UI)

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
