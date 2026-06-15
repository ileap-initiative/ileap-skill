# ADR-0010: A minimal Zola microsite for the iLEAP skill

## Status

Proposed (2026-06-16). **First non-code, product-facing ADR in this repo** — it
governs a website artifact, not the CLI.
## Context

We want a **minimal microsite** on a subdomain of `ileap.global` that (1) explains
the iLEAP skill, (2) gives install instructions, and (3) shows example prompts and
their outputs. Decisions below were settled in a briefing review plus four scoping
answers (2026-06-15/16). The demo artifacts referenced here were **built and
verified during this session** (see Implementation).

## Decision

1. **Primary audience: non-technical decarbonization end-users.** Lead with the
   dashboard visual, plain language, terminology expanded (TOC = Transport
   Operation **Category**, HOC = Hub Operation **Category**), and a screenshot-led
   install walkthrough. A short "Where does my data go?" privacy note is mandatory.

2. **Vendor-neutral framing.** Position as *"works with Claude today; portable to
   any agent that supports skills / MCP."* No claim of endorsement by the Smart
   Freight Centre; Claude.ai is the first, not the only, supported path. State that
   the skill is **provided by SINE and Smart Freight Centre as part of the iLEAP Initiative**.

3. **Implementation: Zola (Rust) static-site generator, search disabled**
   (`build_search_index = false`). Chosen over mdbook (book-shaped, wrong for a
   landing page) and over a single hand-authored HTML file (not human-editable).
   Content lives in **markdown** (non-technical-editable); structure/CSS in
   templates + `styles.css`; demos are **separate committed files**. In this repo
   under `site/`. No Node toolchain — one `cargo install zola`, static output.

4. **Show all three artifacts as "prompt → result," with the result pre-generated
   and committed** (not live). Each demo block shows the verbatim prompt and the
   artifact:
   - **Dashboard** — embedded `<iframe>` (hero demo).
   - **Scenario planner** — embedded `<iframe>`.
   - **TOC/HOC decarbonization table** — rendered inline as a table **plus**
     downloadable `.xlsx` and `.md`.

5. **Three global disclaimers** (replace per-demo provenance notes):
   (a) provided by SINE and Smart Freight Centre as part of the iLEAP Initiative (doubles as the neutrality line);
   (b) **preview**; (c) output may diverge across agents, environments and runs.
   (a)+(b) live in a persistent footer + a small "Preview" banner; (c) sits with
   the demos. Artifacts with illustrative metrics (scenario planner, decarb table)
   additionally label those figures inline as indicative, not advice.

6. **Single source of truth for name and download.** Every reference is `ileap`
   (never `ileap-cli`); the install download points to the released
   `ileap-skill.zip` asset, not a drift-prone path. Truth-sensitive strings (skill
   name, zip name, demo URL, example prompts) are tracked in `docs/DOC-DRIFT.md`.

7. Applies the general styling from https://ileap.global/ (logo, colors, fonts) but keeps the layout simple and content-focused.

## Considered Options

- **mdbook.** On the Rust toolchain, but a *book* generator (SUMMARY.md → chapters
  → sidebar nav). To get a landing-page layout you fight the theme the whole way,
  and the docs-book aesthetic reads as developer docs to a non-technical audience.
  **Rejected** — diagnostic: no chapters → not a book. (Revisit if this grows into
  multi-page documentation.)
- **Single self-contained HTML file** (the `ileap-dashboard.html` idiom).
  Truly minimal, no build, but a 1,000-line inline file is not human-editable by a
  non-developer. **Rejected** for the maintenance goal.
- **Live demos against the API from the website.** Most convincing, but needs a
  backend that holds/proxies credentials. **Rejected** — the demo server +
  "install and run it yourself" delivers live-ness on the user's side.
- **Astro / Eleventy (Node SSG).** Capable, but reintroduces a Node toolchain to a
  Rust repo. **Rejected** in favour of Zola (single Rust binary).
- **Separate repository for the site.** Splits the rename and drift discipline
  across two repos. **Rejected** — keep in `site/` here. Revisit if the site grows.

## Consequences

**Positive**

- A real "install and try it now" path: demo server + public creds + auto-login =
  zero credential friction for a non-technical visitor.
- Content in markdown → editable by non-developers; Zola stays on the Rust
  toolchain (no Node), build-time HTML (good first render), search off (one fewer
  moving part, per the explicit decision).
- Hero dashboard and scenario planner are interactive at near-zero cost
  (self-contained HTML embedded via iframe).
- Honest provenance: pre-generated artifacts + the three disclaimers pre-empt the
  "did Claude really make this?" credibility hit.
- Neutral framing protects the standard and the Smart Freight Centre relationship.

**Negative / risks**

- **Scenario planner and decarb table are not reproducible procedures.** A visitor
  re-running those prompts may get a different layout/figures. Mitigated by the
  disclaimers (§5) but a real expectation gap.
- **Illustrative metrics.** The decarbonization "potential" and the planner's
  electric-road / load-factor factors are derived/placeholder, not measured TOCs;
  if read as advice they mislead. Mitigated by inline labels.
- **New surface to keep true.** The site adds drift risk against the skill (the
  rename is the first instance). Mitigated by §6 + the drift ledger.
- **Hosting/DNS ownership and the subdomain are open** — a coordination dependency
  on whoever administers `ileap.global`.

**Neutral**

- No change to the CLI or the skill; this ADR governs only the website.
- **Source `ileap-dashboard.html` defects** (emoji + "Characteristics") are out of
  scope here — logged as a `docs/DOC-DRIFT.md` item / chore; the site ships a
  sanitized copy regardless.
- Subdomain name and the hosting account are deferred to the implementer once
  confirmed; they do not change the architecture.

## Changes (for the implementer)

1. **`site/`** — initialize a Zola project: `config.toml`
   (`build_search_index = false`, base_url = the chosen subdomain), `content/`
   (markdown), `templates/`, `static/`, `sass/` or `static/styles.css`.
2. **`site/content/_index.md`** — the single page, in order: hero (dashboard
   visual + one-line value prop) → "what it does" in plain language (TOC/HOC
   expanded) → **install walkthrough** (screenshot-led: download `ileap-skill.zip`
   → Claude.ai → Settings → Capabilities → Skills → upload → paste the demo prompt)
   → the three demo blocks (prompt → result) → privacy note → footer (repo link,
   license, NGO/neutrality line, MCP/SDK portability).
3. **`site/static/demos/`** — **already produced this session** (see
   Implementation): `dashboard.html`, `scenario-planner.html`,
   `toc-hoc-decarbonization.{md,xlsx}`. Embed the two HTML files via `<iframe>`;
   link the `.xlsx`/`.md` as downloads.
4. **`site/static/`** — install-walkthrough screenshots; `ileap-logo.png`
   (already in repo).
5. **Naming** — `grep -rn "ileap-cli" site/` must be empty; install download
   resolves to the released `ileap-skill.zip`.
6. **`docs/DOC-DRIFT.md`** — add (a) the site's truth-sensitive strings item and
   (b) the `ileap-dashboard.html` terminology/emoji defect.

### Open items to resolve before launch

- [ ] Pick the `*.ileap.global` subdomain.
- [ ] Confirm who administers `ileap.global` DNS / the hosting account.
- [ ] Fix or regenerate the source `ileap-dashboard.html` (terminology + emoji).

### Verify

- `cd site && zola build` succeeds; output is static, no search index emitted.
- `grep -rn "ileap-cli" site/` is empty; no emojis and no "Characteristics" in any
  `site/static/demos/*` artifact.
- Each demo block shows both the prompt and the artifact; the download links work.
- The install download resolves to the current `ileap-skill.zip`.

## Implementation

**Demo artifacts produced and verified this session (2026-06-16), against the live
demo server `https://api.preview.ileap.dev`:**

- Data fetched via the bundled `ileap` binary (Darwin-arm64), public creds,
  exit 0 for all six resources.
- **`site/static/demos/dashboard.html`** — sanitized from the repo's
  `ileap-dashboard.html`: "Operation Characteristics" → "Operation Categories",
  all emojis stripped (verified: 0 residual, no "Characteristics"). Self-contained
  (only external resource: the Chart.js CDN).
- **`site/static/demos/scenario-planner.html`** — authored: interactive levers
  (transport activity, Road→Rail modal shift, road electrification share, load-
  factor improvement) recomputing CO2e live, baseline-vs-scenario bar chart.
  Calc anchored on real demo TOC intensities (Road 0.116, Rail 0.007 kg CO2e/tkm);
  baseline at 16,920 tkm = 1,962.72 kg, matching the demo's reported WTW.
  Electric-road 0.030 and the load-factor effect are labelled illustrative.
  Self-contained; no emojis.
- **`site/static/demos/toc-hoc-decarbonization.md` + `.xlsx`** — listing of all 4
  TOCs and 2 HOCs with WTW/TTW intensity and an illustrative decarbonization
  potential (% reduction vs the cleanest same-type entry) + an indicative lever.
  `.xlsx` written with stdlib `zipfile`/OOXML (openpyxl was unavailable in the
  sandbox); validated as a well-formed archive.

**Suggested reproduction prompts** (for the site's demo blocks):
- Dashboard: *"Using the iLEAP skill, build an emissions dashboard from the demo
  server."*
- Scenario planner: *"Using the iLEAP skill, fetch the TOCs and build an
  interactive scenario planner showing how shifting road freight to rail cuts
  CO2e."*
- Table: *"Using the iLEAP skill, list all TOCs and HOCs with their CO2e
  intensities and an indicative decarbonization potential, and export it as an
  Excel file."*

**Process note (orchestration):** three subagents were dispatched to build the
artifacts in parallel (2× Sonnet for the HTML, 1× Haiku for the table) but were
**blocked — subagents in this session can only Write to `/tmp/*`** (per
`.claude/settings.json`) and could not write into `site/`. The main agent produced
all artifacts instead, keeping large outputs out of context by transforming via
scripts (sanitize/table) where possible. Lesson recorded for the operating memory.
