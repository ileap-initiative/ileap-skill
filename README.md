# iLEAP Skill

<img src="site/static/ileap-logo.png" alt="iLEAP logo" width="120" style="margin: 1rem; float: right"/>

This repository contains the iLEAP Skill to interact with the iLEAP API using tools such
as Claude.ai, Claude, or similar software offerings.

Consider the development status being a **community preview**, such that we can gather
feedback and further iterate on the skill before deciding on a stable release.

**You can download the latest version of the iLEAP Skill [here](https://github.com/sine-fdn/ileap-cli/releases/latest/download/ileap-skill.zip).**

## What it does

The iLEAP skill lets you explore and make use of iLEAP Data. It describes and implements
everything that is needed for AI to query, to visualize, and to work with the data.

If you are looking for some example outputs, you can find them on the skill's website at [https://skill-preview.ileap.dev](https://skill-preview.ileap.dev).

## The relationship with the iLEAP Initiative

The iLEAP Skill is made available through the iLEAP Initiative, with its main mission
 being to leverage digitalization to decarbonize logistics at scale.

This skill builds on top of a core deliverable of this initiative, the iLEAP Technical
Specifications. These describe how to make logistics emissions data available, how the data
is structured, and, most importantly what it means – i.e. how it needs to be calculated to be
representative, complete, that the quality of the data can assesd, and more.

The iLEAP Skill builds on top of a core virtue of the iLEAP Technical Specifications:
being a so-called interoperability standard. What these fany words mean is that *any* tool
provider can implement them, and that *everyone* can use the skill to work with
iLEAP-aligned data immediately.

## Installation Instructions

The following instructions apply to Claude.ai only, the only platform we have tested so far.

You can find them right on the iLEAP Skill's website at [https://skill-preview.ileap.dev](https://skill-preview.ileap.dev/#installation).

## Important Disclaimer

1. As with AI in general, also the iLEAP Skill _can_ make mistakes. Always review its output.
2. **Do not use it for production purposes yet.**
3. When you pull in data, make sure you have the rights to process that data with the skill.
4. This is **not an endorsement** of Claude, Anthropic, or any other AI approach or vendor.
5. AI use carries a **significant environmental impact**.
6. The skill has only been built for and tested with **Claude/Anthropic**. It should work with
     other providers, including local AI deployments, too.
7. The iLEAP skill is **under active development**.
     Please <a href="mailto:team@ileap.global">provide feedback</a>, and check back for updates.


## Technical Details

1. the iLEAP CLI is implemented in Rust (see [src/](src/)),
2. The skill uses prebuilt static Linux binaries of it. With every release,
     an `ileap-skill.zip` with pre-built binaries is created.
3. To build them locally and after cloning the repo, you can either
     run `cargo build --release`, or `cargo install --path .` to install the CLI on your machine
4. The skill is available in [.agents/skills/ileap](.agents/skills/ileap) and can be used as a
     template for building your own skills on top of the iLEAP CLI.
