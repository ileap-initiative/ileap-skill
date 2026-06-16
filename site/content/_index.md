+++
title = "iLEAP Skill for Claude"
template = "index.html"
+++

## What it does

The iLEAP skill lets you explore **transport emissions data** — shipments,
Transport Operation Categories (TOCs), Hub Operation Categories (HOCs), footprints
and more — just by asking Claude in plain language. No spreadsheets, no API calls.
Ask for a dashboard, a what-if scenario, or an export, and Claude fetches the data
and builds it for you.

(TOC = Transport Operation Category. HOC = Hub Operation Category.)

## Install in Claude.ai {#installation}

0. Sign up for a Claude.ai account, or similar offering, if you have not already. (This skill was only tested with Anthropic's offering so far).
1. Download the skill using the button above (`ileap-skill.zip`).
2. In Claude.ai, open **Settings → Capabilities → Skills**.
3. Choose **Upload skill** and select the zip file.
4. Go to the Capabilities Tab (e.g. https://claude.ai/admin-settings/capabilities), and add `*.ileap.dev` to the *Domain allowlist*.
5. Start a chat and paste one of the example prompts below. The skill connects to
   the public iLEAP demo server automatically — no account or credentials needed to
   try it.

## Where does my data go?

The skill only runs the queries you ask for, against the iLEAP endpoint you choose.
By default, the skill pulls in example data from the public **demo server** (`api.preview.ileap.dev`). Below examples are based on the demo server.

**Point the skill at your own iLEAP endpoint to work with your own data.**

## What to expect {#disclaimer}

**Where we are.** The iLEAP Skill is **under active development** and offered as a community
preview. **Please do not use it for production purposes yet.**

**What is safe to do.** Explore iLEAP data, try out queries, and share feedback. Because AI
can make mistakes, always review the skill's output before you rely on it. When you pull in
data, make sure you have the rights to process it with the skill.

**Good to know.**

- This is **not an endorsement** of Claude, Anthropic, or any other AI approach or vendor.
- The skill has only been built for and tested with **Claude/Anthropic**. It should work with
  other providers, including local AI deployments, too.
- AI use carries a **significant environmental impact**. We take this seriously and weigh it
  against the emissions reductions the iLEAP Initiative is working to unlock.

We would love to hear from you. Please
<a href="mailto:team@ileap.global">send us your feedback</a>, and check back for updates.
