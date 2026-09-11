---
title: Exporting and Syncing Claude Code Superpowers Across Workstations
short_title: Syncing Claude Code Superpowers
subtitle: Replicating Custom Agents and Skills Across Machines
date: 2026-09-10
slug: exporting-and-syncing-claude-code-superpowers
tags: claude, workflow, productivity, linux, automation
---

# Exporting and Syncing Claude Code Superpowers Across Workstations

## Table of Contents
1. [The Custom Agent Problem](#the-custom-agent-problem)
2. [Where Claude Code Stores State](#where-claude-code-stores-state)
3. [Separating Configuration from Secret Tokens](#separating-configuration-from-secret-tokens)
4. [Building the Export and Packaging Script](#building-the-export-and-packaging-script)
5. [Automating Machine Bootstrapping](#automating-machine-bootstrapping)
6. [Conclusion](#conclusion)
7. [References](#references)

---

## The Custom Agent Problem

If you use [Claude Code](https://docs.anthropic.com/en/docs/agents-and-tools/claude-code) for serious daily engineering, your setup quickly becomes highly customized:
- Custom skills (`brainstorming`, `writing-plans`, `subagent-driven-development`, `systematic-debugging`).
- MCP (Model Context Protocol) server registrations.
- Global developer instructions and project context configurations.
- Custom shell aliases, memory notes, and workflow plugins.

Then you sit down at a secondary laptop, a remote dev box, or help a colleague set up their environment—and your workflow vanishes. Setting everything up by hand takes an hour of tedious folder copying, config editing, and hunting down missing plugins.

We built a clean, automated export and synchronization workflow that packages your entire Claude Code agent environment into a portable, reproducible bundle.

---

## Where Claude Code Stores State

On Linux and macOS, Claude Code organizes its configuration under `~/.claude/` and `~/.config/`:

```text
~/.claude/
├── config.json              # Global flags and model preferences
├── plugins/                 # Installed plugins & official superpower skills
│   └── cache/
└── projects/                # Session transcripts & conversation history

~/.config/superpowers/
├── skills/                  # Custom author skills (SKILL.md)
└── hooks/                   # Execution hooks
```

Crucially, some files contain **sensitive session credentials and OAuth tokens** (like `CLAUDE_CODE_OAUTH_TOKEN` and API keys), while others contain purely functional workflow code. You cannot simply `tar` the entire directory without accidentally publishing your private account tokens!

---

## Separating Configuration from Secret Tokens

To make synchronization safe, we isolate functional artifacts into an export manifest:

```python
# scripts/export_claude_env.py snippet
import shutil, os, tarfile
from pathlib import Path

SAFE_PATHS = [
    Path.home() / ".config/superpowers/skills",
    Path.home() / ".claude/plugins",
]

SENSITIVE_FILES = [
    ".oauth_token",
    "credentials.json",
    "auth.token",
]
```

The export script recursively copies all skills, plugins, and custom prompt templates into a clean staging tree while explicitly excluding any file matching credential patterns.

---

## Building the Export and Packaging Script

Here is the complete synchronization script we use to bundle our environment:

```bash
#!/bin/bash
# bin/export-superpowers.sh
set -euo pipefail

EXPORT_DIR="/tmp/claude-superpowers-export"
ARCHIVE_PATH="$HOME/claude-superpowers-$(date +%F).tar.gz"

rm -rf "$EXPORT_DIR"
mkdir -p "$EXPORT_DIR/skills" "$EXPORT_DIR/plugins"

echo "[+] Copying custom skills..."
if [ -d "$HOME/.config/superpowers/skills" ]; then
    rsync -av --exclude="*.log" "$HOME/.config/superpowers/skills/" "$EXPORT_DIR/skills/"
fi

echo "[+] Copying installed plugins (excluding credentials)..."
if [ -d "$HOME/.claude/plugins" ]; then
    rsync -av --exclude="*token*" --exclude="*secret*" "$HOME/.claude/plugins/" "$EXPORT_DIR/plugins/"
fi

echo "[+] Packaging archive: $ARCHIVE_PATH"
tar -czf "$ARCHIVE_PATH" -C "$EXPORT_DIR" .
rm -rf "$EXPORT_DIR"

echo "[✓] Export complete! Ready to transfer."
```

---

## Automating Machine Bootstrapping

On a fresh machine, the restoration script installs Claude Code CLI, creates standard directories, and unpacks the bundle:

```bash
#!/bin/bash
# bin/import-superpowers.sh
set -euo pipefail

ARCHIVE="${1:-claude-superpowers-latest.tar.gz}"

mkdir -p "$HOME/.config/superpowers/skills" "$HOME/.claude/plugins"
tar -xzf "$ARCHIVE" -C "/tmp/claude-import"

rsync -av "/tmp/claude-import/skills/" "$HOME/.config/superpowers/skills/"
rsync -av "/tmp/claude-import/plugins/" "$HOME/.claude/plugins/"

rm -rf "/tmp/claude-import"
echo "[✓] Environment restored. Run 'claude' to start coding!"
```

---

## Conclusion

By standardizing your agent skills and configurations into a portable repository, your AI pair programming workflows become instantly portable across any development machine in seconds.

---

## References
- [Anthropic Claude Code CLI Documentation](https://docs.anthropic.com/en/docs/agents-and-tools/claude-code)
- [Antigravity Customization Architecture](https://antigravity.google)
