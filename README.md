# GitHub Secret Scanner

Scans GitHub repos for accidentally exposed secrets. Works standalone or through Telex chat interface.

Will find your deepest darkest secrets. lol
## What it does

Detects leaked credentials in your git history:
- AWS access keys and secret keys
- API tokens (OpenAI, Stripe, SendGrid, etc.)
- Database connection strings
- Private SSH/RSA keys
- OAuth tokens
- Generic secrets in config files

Uses Gemini AI to analyze findings and cut down false positives. Gives you actual remediation advice instead of just panic.

## Three scan modes

**Quick** - Last 100 commits, fast  
**Running** - Incremental, tracks what you've scanned  
**Deep** - Full repository history (slow, thorough) (Do not use though. Unstable, in progress, currently building...)

## Setup

```bash
git clone https://github.com/Pheonix0x01/secret-detector
cd secret-detector

cp .env.example .env

cargo build --release

cargo run --release
```

### Environment variables

```bash
HOST=0.0.0.0
PORT=8080
GEMINI_API_KEY=your_key_here
GEMINI_MODEL=gemini-2.0-flash-exp
GITHUB_TOKEN=optional
RUST_LOG=info
MAX_SCAN_COMMITS=100
SCAN_STATE_FILE=scan_states.json
```

## Usage

### Via API

```bash
curl -X POST http://localhost:8080/a2a/agent/githubScanner \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": "test-123",
    "method": "message/send",
    "params": {
      "message": {
        "kind": "message",
        "role": "user",
        "messageId": "msg-456",
        "parts": [
          {
            "kind": "text",
            "text": "scan https://github.com/octocat/Hello-World"
          }
        ]
      }
    }
  }'
```

### Via Telex

1. Add the agent to Telex using the workflow JSON below
2. Chat with it:
   - "scan https://github.com/user/repo"
   - "start running scan https://github.com/user/repo"
   - "continue scan"
   - "status"
   - "help"

### Telex Workflow JSON

```json
{
  "active": true,
  "category": "security",
  "description": "AI-powered GitHub secret scanner that detects accidentally exposed credentials",
  "id": "github_secret_scanner_v1",
  "name": "secret-detector",
  "long_description": "\nYou are a security-focused GitHub assistant that helps developers identify accidentally exposed secrets in their repositories.\n\nYour capabilities:\n- Scan public GitHub repositories for exposed secrets (API keys, passwords, tokens, etc.)\n- Quick scan mode: analyzes recent commits\n- Running scan mode: incremental scanning\n- Deep scan mode: full repository history\n- Intelligent analysis using AI to reduce false positives\n- Provide actionable remediation advice\n\nWhen responding:\n- Always ask for a repository URL if none is provided\n- Present findings clearly with severity levels\n- Give specific remediation steps for each finding\n- Be security-conscious but friendly and helpful\n\nExamples:\n- \"scan https://github.com/user/repo\" - Quick scan\n- \"deep scan https://github.com/user/repo\" - Full scan\n- \"start running scan https://github.com/user/repo\" - Begin incremental\n- \"help\" - Show available commands",
  "short_description": "Scan GitHub repos for exposed secrets and credentials",
  "nodes": [
    {
      "id": "github_scanner_agent",
      "name": "GitHub Secret Scanner",
      "parameters": {},
      "position": [500, 300],
      "type": "a2a/generic-a2a-node",
      "typeVersion": 1,
      "url": "http://YOUR_SERVER_IP/a2a/agent/githubScanner"
    }
  ],
  "pinData": {},
  "settings": {
    "executionOrder": "v1"
  }
}
```

Replace `YOUR_SERVER_IP` with your actual server IP or api deployed link


## How it works

1. Receives scan request (via API or Telex)
2. Uses Gemini to parse user intent
3. Fetches commits from GitHub API
4. Runs regex patterns against each commit's diff
5. Collects potential secrets
6. Uses Gemini again to analyze findings and generate response
7. Returns results in A2A format

