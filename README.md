# iLEAP Skill

<img src="site/static/ileap-logo.png" alt="iLEAP logo" width="120" style="margin: 1rem; float: right"/>

**Talk to your logistics emissions data in plain language.**

This repository contains the iLEAP Skill, which lets you interact with the iLEAP API using
tools such as Claude.ai, Claude, or similar software offerings.

> **Status: community preview.** The skill works today, and we are sharing it early to gather
> feedback before committing to a stable release. Expect changes between now and then.

**You can download the latest version of the iLEAP Skill [here](https://github.com/ileap-initiative/ileap-skill/releases/latest/download/ileap-skill.zip).**

## Who is this for?

- **Software engineers:** a Rust CLI plus a packaged skill you can run, inspect, and use as a template (see [Technical Details](#technical-details)).
- **Product managers:** a community preview that shows what natural-language access to logistics emissions data looks like (see [What to expect](#what-to-expect)).
- **Sustainability managers:** a no-lock-in way to explore iLEAP-aligned emissions data and understand how it is calculated (see [the iLEAP Initiative](#the-relationship-with-the-ileap-initiative)).

## What it does

The iLEAP skill lets you explore **transport emissions data** (shipments, Transport Operation
Categories (TOCs), Hub Operation Categories (HOCs), footprints, and more) just by asking Claude
in plain language. No spreadsheets, no API calls. Ask for a dashboard, a what-if scenario, or
an export, and Claude fetches the data and builds it for you.

If you are looking for some example outputs, you can find them on the skill's website at [https://skill-preview.ileap.dev](https://skill-preview.ileap.dev).

## The relationship with the iLEAP Initiative

The iLEAP Initiative exists to decarbonize logistics at scale, using digitalization to get
there. The iLEAP Skill is one way we put that mission into practice.

The skill builds on a core deliverable of the initiative, the iLEAP Technical Specifications.
These describe how to make logistics emissions data available, how the data is structured,
and what it means. That last part matters most: it covers how the data must be calculated to
be representative and complete, how its quality can be assessed, and more.

The Specifications are an interoperability standard, and that brings a concrete benefit: there
is no lock-in. Any tool provider can implement them, and anyone can use this skill to work with
iLEAP-aligned data right away, no matter who produced it.

## Installation Instructions

The following instructions apply to Claude.ai only, the only platform we have tested so far.

You can find them right on the iLEAP Skill's website at [https://skill-preview.ileap.dev](https://skill-preview.ileap.dev/#installation).

## What to expect

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


## Technical Details

1. The iLEAP CLI is implemented in Rust (see [cli/](cli/)).
2. The skill uses prebuilt static Linux binaries of it. With every release,
     an `ileap-skill.zip` with pre-built binaries is created.
3. To build them locally and after cloning the repo, you can either
     run `cargo build --release`, or `cargo install --path cli` to install the CLI on your machine.
4. The skill is available in [ileap](ileap) and can be used as a
     template for building your own skills on top of the iLEAP CLI.
