+++
title = "iLEAP Skill for AI assistants"
template = "index.html"
+++

## What it does

The iLEAP skill lets you explore **transport emissions data** – shipments,
Transport Operation Categories (TOCs), Hub Operation Categories (HOCs), footprints
and more – just by asking an AI assistant in plain language. No spreadsheets, no API calls.
Ask for a dashboard, a what-if scenario, or an export, and the AI fetches the data
and builds it for you.

(TOC = Transport Operation Category. HOC = Hub Operation Category.)

## Installing at Claude.ai {#installation}

0. Sign up for a Claude.ai account, or a similar offering, if you have not already. (This skill has only been tested with Anthropic's offering so far.)
1. Download the skill using the button above (`ileap-skill.zip`).
2. In Claude.ai, open **Customize → Skills** and click the **+ button**.
3. Click **Create skill → Upload a skill**, and select the zip file you downloaded.
4. If you have a Team subscription (instead of an individual "Pro Plan" etc.),
     a team admin then also needs to update the *Domain allowlist*
     (https://claude.ai/admin-settings/capabilities), adding `*.ileap.dev` to it.
     This is required for the skill to be able to call the iLEAP API on the demo server.
5. Start a chat and paste one of the example prompts below. The skill connects to
   the public iLEAP demo server automatically — no account or credentials needed to
   try it.

## Where does my data go?

The skill only runs the queries you ask for, against the iLEAP endpoint you choose.
By default, the skill pulls in example data from the public **demo server** (`api.preview.ileap.dev`). The examples below are based on the demo server.

**Point the skill at your own iLEAP endpoint to work with your own data.**

## What to expect {#disclaimer}

<dl class="expect">
  <dt>Where we are</dt>
  <dd>The iLEAP Skill is <strong>under active development</strong> and offered as a community preview. <strong>Please do not use it for production purposes yet.</strong></dd>
  <dt>What is safe to do</dt>
  <dd>Explore iLEAP data, try out queries, and share feedback. Because AI can make mistakes, always review the skill's output before you rely on it. When you pull in data, make sure you have the rights to process it with the skill.</dd>
  <dt>Good to know</dt>
  <dd>
    <ul>
      <li>This is <strong>not an endorsement</strong> of Claude, Anthropic, or any other AI approach or vendor.</li>
      <li>The skill has only been built for and tested with <strong>Claude/Anthropic</strong>. It should work with other providers, including local AI deployments, too.</li>
      <li>AI use carries a <strong>significant environmental impact</strong>. We take this seriously and weigh it against the emissions reductions the iLEAP Initiative is working to unlock.</li>
    </ul>
  </dd>
</dl>

We would love to hear from you. Please
<a href="mailto:team@ileap.global">send us your feedback</a>, and check back for updates.
