# Rustquty User Experience Survey

> **Objective**: Measure user satisfaction, perceived usefulness, ease of use, and
> enjoyment of rustquty using validated instruments from HCI and survey methodology
> research. Results will guide prioritization of features and usability improvements.

---

## Methodology Notes

This questionnaire draws on three established frameworks:

| Framework | What it measures | Source |
|---|---|---|
| **TAM** (Technology Acceptance Model) | Perceived Usefulness (PU) + Perceived Ease of Use (PEOU) | Davis (1989) |
| **SUS** (System Usability Scale) | Global usability | Brooke (1996) |
| **IMI** (Intrinsic Motivation Inventory) | Enjoyment / Interest subscale | Ryan & Deci (2000) |

**Scale design principles applied:**

- 5-point Likert scales with labeled anchors (avoids the ambiguity of unlabeled
  intermediate points; reduces respondent fatigue vs. 7-point).
- One construct per item (no double-barreled questions).
- Reverse-coded items interspersed to detect acquiescence bias (marked ⚡).
- Neutral midpoint included to avoid forced-choice distortion.
- Items randomized within each block when administered electronically.
- Open-ended items placed after structured blocks to prevent anchoring effects on
  qualitative responses.
- Demographic items placed at end to reduce stereotype threat.

---

## Section A — Usage Context

> *Purpose*: Segment respondents by experience level and usage pattern. These
> variables are expected moderators of satisfaction (power users vs. newcomers).

**A1.** How did you first learn about rustquty?

- [ ] crates.io search
- [ ] GitHub / awesome-rust list
- [ ] Blog post or article
- [ ] Recommendation from a colleague
- [ ] Conference talk or meetup
- [ ] Social media (Reddit, Twitter/X, Mastodon, etc.)
- [ ] Other: _______________

**A2.** How long have you been using rustquty?

- [ ] Less than 1 week
- [ ] 1–4 weeks
- [ ] 1–6 months
- [ ] More than 6 months

**A3.** How would you describe your Rust experience?

- [ ] Beginner (learning the language)
- [ ] Intermediate (comfortable in small-to-medium projects)
- [ ] Advanced (contributing to libraries or complex systems)
- [ ] Expert (teaching Rust, core team, or equivalent)

**A4.** In what context do you primarily use rustquty?

- [ ] Personal / hobby projects
- [ ] Professional / work projects
- [ ] Open-source maintainer
- [ ] CI/CD pipeline integration
- [ ] Other: _______________

**A5.** How many Rust projects in your workspace use rustquty?

- [ ] 1
- [ ] 2–5
- [ ] 6–10
- [ ] More than 10

**A6.** Which scan profile do you use most frequently?

- [ ] `fast` (fmt + clippy only)
- [ ] `full` (all except mutants — default)
- [ ] `deep` (all 12 collectors including mutation testing)

**A7.** Have you integrated rustquty into a CI/CD pipeline?

- [ ] No
- [ ] Yes, on every push/PR
- [ ] Yes, on a schedule (e.g., nightly)
- [ ] I plan to

---

## Section B — Perceived Usefulness

> *Construct*: TAM Perceived Usefulness (PU).
> *Instruction*: "Please indicate how much you agree with each statement about
> rustquty's usefulness."
> *Anchors*: 1 = Strongly disagree, 2 = Disagree, 3 = Neutral, 4 = Agree,
> 5 = Strongly agree

| # | Item | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| **B1** | Using rustquty helps me catch quality issues I would otherwise miss. | ○ | ○ | ○ | ○ | ○ |
| **B2** | rustquty makes my code reviews more effective. | ○ | ○ | ○ | ○ | ○ |
| **B3** | The quality gate (pass/fail) gives me confidence before merging changes. | ○ | ○ | ○ | ○ | ○ |
| **B4** | rustquty has improved the overall quality of my Rust codebases. | ○ | ○ | ○ | ○ | ○ |
| **B5** ⚡ | I could achieve the same results using separate Cargo tools without rustquty. | ○ | ○ | ○ | ○ | ○ |
| **B6** | The ratchet model (preventing regressions) is a useful approach to quality enforcement. | ○ | ○ | ○ | ○ | ○ |

---

## Section C — Perceived Ease of Use

> *Construct*: TAM Perceived Ease of Use (PEOU).
> *Anchors*: 1 = Strongly disagree, 2 = Disagree, 3 = Neutral, 4 = Agree,
> 5 = Strongly agree

| # | Item | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| **C1** | Setting up rustquty for a new project is straightforward. | ○ | ○ | ○ | ○ | ○ |
| **C2** | The CLI commands are easy to remember and use. | ○ | ○ | ○ | ○ | ○ |
| **C3** | The output (terminal + JSON) is clear and actionable. | ○ | ○ | ○ | ○ | ○ |
| **C4** | Configuring rustquty via `rustquty.toml` is intuitive. | ○ | ○ | ○ | ○ | ○ |
| **C5** ⚡ | I often need to consult the documentation to accomplish what I want. | ○ | ○ | ○ | ○ | ○ |
| **C6** | The three scan profiles (`fast`, `full`, `deep`) cover my needs well. | ○ | ○ | ○ | ○ | ○ |

---

## Section D — System Usability (SUS)

> *Construct*: System Usability Scale (Brooke, 1996) — adapted for CLI tools.
> *Anchors*: 1 = Strongly disagree, 5 = Strongly agree.
> *Scoring*: Odd items: score − 1. Even items: 5 − score. Sum × 2.5 → 0–100 scale.

| # | Item | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| **D1** | I think I would like to use rustquty frequently. | ○ | ○ | ○ | ○ | ○ |
| **D2** ⚡ | I found rustquty unnecessarily complex. | ○ | ○ | ○ | ○ | ○ |
| **D3** | I thought rustquty was easy to use. | ○ | ○ | ○ | ○ | ○ |
| **D4** ⚡ | I think I would need support from a technical person to use rustquty. | ○ | ○ | ○ | ○ | ○ |
| **D5** | I found the various collectors in rustquty were well integrated. | ○ | ○ | ○ | ○ | ○ |
| **D6** ⚡ | I thought there was too much inconsistency in rustquty's behavior. | ○ | ○ | ○ | ○ | ○ |
| **D7** | I would imagine that most Rust developers would learn to use rustquty quickly. | ○ | ○ | ○ | ○ | ○ |
| **D8** ⚡ | I found rustquty very cumbersome to use. | ○ | ○ | ○ | ○ | ○ |
| **D9** | I felt very confident using rustquty. | ○ | ○ | ○ | ○ | ○ |
| **D10** ⚡ | I needed to learn a lot of things before I could get going with rustquty. | ○ | ○ | ○ | ○ | ○ |

---

## Section E — Enjoyment & Intrinsic Motivation

> *Construct*: IMI Interest/Enjoyment subscale.
> *Anchors*: 1 = Strongly disagree, 2 = Disagree, 3 = Neutral, 4 = Agree,
> 5 = Strongly agree

| # | Item | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| **E1** | I enjoy using rustquty as part of my development workflow. | ○ | ○ | ○ | ○ | ○ |
| **E2** | Running rustquty and seeing the results is satisfying. | ○ | ○ | ○ | ○ | ○ |
| **E3** ⚡ | Using rustquty feels like a chore. | ○ | ○ | ○ | ○ | ○ |
| **E4** | I find the feedback rustquty provides to be motivating (e.g., watching scores improve). | ○ | ○ | ○ | ○ | ○ |
| **E5** ⚡ | I would rather use a different quality tool if one were available. | ○ | ○ | ○ | ○ | ○ |

---

## Section F — Feature Evaluation

> *Purpose*: Identify which specific features drive (or hinder) satisfaction.
> *Anchors*: 1 = Not at all valuable, 2 = Slightly valuable, 3 = Moderately
> valuable, 4 = Very valuable, 5 = Extremely valuable

**F1.** How valuable is each collector to you?

| Collector | What it checks | 1 | 2 | 3 | 4 | 5 | N/A |
|---|---|---|---|---|---|---|---|
| `fmt` | Code formatting | ○ | ○ | ○ | ○ | ○ | ○ |
| `clippy` | Linting warnings | ○ | ○ | ○ | ○ | ○ | ○ |
| `tests` | Test pass/fail/ignore | ○ | ○ | ○ | ○ | ○ | ○ |
| `coverage` | Line coverage % | ○ | ○ | ○ | ○ | ○ | ○ |
| `deny` | Banned crates & licenses | ○ | ○ | ○ | ○ | ○ | ○ |
| `audit` | Security vulnerabilities | ○ | ○ | ○ | ○ | ○ | ○ |
| `hack` | Feature powerset checks | ○ | ○ | ○ | ○ | ○ | ○ |
| `mutants` | Mutation testing score | ○ | ○ | ○ | ○ | ○ | ○ |
| `duplicates` | Duplicate line detection | ○ | ○ | ○ | ○ | ○ | ○ |
| `loc` | Lines of code metrics | ○ | ○ | ○ | ○ | ○ | ○ |
| `size` | Per-function line/param counts | ○ | ○ | ○ | ○ | ○ | ○ |
| `complexity` | Cyclomatic complexity | ○ | ○ | ○ | ○ | ○ | ○ |

**F2.** Which single feature of rustquty do you value **most**?

- [ ] Local-first / no network requirement
- [ ] Ratchet model (prevent regressions)
- [ ] Single-command quality scan (`qa`)
- [ ] JSON output for CI integration
- [ ] Absolute thresholds (`[gate.defaults]`)
- [ ] Built-in collectors (no external tool needed)
- [ ] Parallel multi-collector execution
- [ ] The `doctor` command (availability check)
- [ ] Verbose output with file:line violations
- [ ] Other: _______________

---

## Section G — Friction & Pain Points

> *Purpose*: Identify barriers to adoption and continued use.
> *Anchors*: 1 = Not a problem at all, 2 = Minor problem, 3 = Moderate problem,
> 4 = Serious problem, 5 = Blocker (prevents use)

| # | Item | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| **G1** | Installing/updating rustquty (cargo install, build time, dependencies). | ○ | ○ | ○ | ○ | ○ |
| **G2** | Setting up external tools (cargo-deny, cargo-audit, cargo-llvm-cov, etc.). | ○ | ○ | ○ | ○ | ○ |
| **G3** | Understanding what each collector does and when to use it. | ○ | ○ | ○ | ○ | ○ |
| **G4** | Configuring thresholds to match my project's standards. | ○ | ○ | ○ | ○ | ○ |
| **G5** | Interpreting gate failures and deciding what to fix. | ○ | ○ | ○ | ○ | ○ |
| **G6** | False positives or overly strict default thresholds. | ○ | ○ | ○ | ○ | ○ |
| **G7** | Scan speed — rustquty takes too long to run. | ○ | ○ | ○ | ○ | ○ |
| **G8** | The baseline / ratchet workflow (init, update, diff) is confusing. | ○ | ○ | ○ | ○ | ○ |
| **G9** | Verbose output is too noisy or hard to read. | ○ | ○ | ○ | ○ | ○ |
| **G10** | Lack of IDE / editor integration. | ○ | ○ | ○ | ○ | ○ |
| **G11** | Lack of GitHub/GitLab PR annotations or inline comments. | ○ | ○ | ○ | ○ | ○ |

---

## Section H — Net Promoter Score

> *Construct*: NPS (Reichheld, 2003) — single-item loyalty measure.

**H1.** How likely are you to recommend rustquty to another Rust developer?

| 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|---|---|---|---|---|
| Not at all likely | ○ | ○ | ○ | ○ | ○ | ○ | ○ | ○ | ○ | ○ | Extremely likely |

*Scoring*:
- **Promoters** (9–10): Loyal enthusiasts who will fuel growth.
- **Passives** (7–8): Satisfied but unenthusiastic customers.
- **Detractors** (0–6): Unhappy customers who may impede growth.
- **NPS = %Promoters − %Detractors** (range: −100 to +100)

---

## Section I — Open-Ended Feedback

> *Purpose*: Capture themes not covered by structured items. Placed last to avoid
> anchoring on qualitative responses when answering Likert items.

**I1.** What motivated you to try rustquty instead of other quality tools (e.g.,
SonarQube, CodeClimate, custom CI scripts)?

____________________________________________________________
____________________________________________________________
____________________________________________________________

**I2.** Describe a moment when rustquty was *most* valuable to you or your team.

____________________________________________________________
____________________________________________________________
____________________________________________________________

**I3.** Describe a moment when rustquty was *frustrating* or got in your way.

____________________________________________________________
____________________________________________________________
____________________________________________________________

**I4.** If you could change **one thing** about rustquty, what would it be?

____________________________________________________________
____________________________________________________________
____________________________________________________________

**I5.** What feature or improvement would most increase your likelihood of
recommending rustquty to others?

____________________________________________________________
____________________________________________________________
____________________________________________________________

**I6.** (Optional) Is there anything else you'd like us to know?

____________________________________________________________
____________________________________________________________
____________________________________________________________

---

## Section J — Demographics (Optional)

> *Purpose*: Contextualize responses. Placed at end per best practice — reduces
> stereotype threat and survey abandonment. All items are optional.

**J1.** What is your primary development operating system?

- [ ] Linux
- [ ] macOS
- [ ] Windows
- [ ] Other: _______________
- [ ] Prefer not to say

**J2.** In what type of organization do you primarily write Rust?

- [ ] Solo / freelance
- [ ] Startup (< 50 employees)
- [ ] Small/medium company (50–500 employees)
- [ ] Large company (> 500 employees)
- [ ] Academic / research institution
- [ ] Open source (not employment-related)
- [ ] Prefer not to say

**J3.** How large is the largest Rust codebase you use rustquty on?

- [ ] < 1,000 lines
- [ ] 1,000–10,000 lines
- [ ] 10,000–100,000 lines
- [ ] 100,000–1,000,000 lines
- [ ] > 1,000,000 lines
- [ ] I don't know

**J4.** (Optional) If you're open to a follow-up conversation, enter your email
or GitHub handle:

____________________________

---

## Administration Notes

### Distribution channels
- Embed in rustquty README as a link (GitHub-flavored markdown)
- Post-release announcement on Reddit (`r/rust`), Rust users forum, and Discord
- Include a one-line prompt in `rustquty --help` output or after `qa` runs
  (e.g., "Help improve rustquty: <URL>")

### Timing
- Deploy after each minor (0.x) release to track satisfaction trajectory
- Avoid deploying more than quarterly to prevent survey fatigue in the community

### Analysis plan
1. **SUS score**: Compute per Section D scoring formula → benchmark against
   Sauro-Lewis curved grading scale (≥ 68 = "C", ≥ 80.3 = "A").
2. **TAM regression**: Model PU and PEOU as predictors of NPS (H1) and
   Enjoyment (E1) using ordinal logistic regression.
3. **NPS segmentation**: Cross-tabulate promoters vs. detractors against usage
   context (A4), experience (A3), and pain points (G) to identify leverage
   points.
4. **Thematic analysis**: Code open-ended responses (I1–I6) using emergent
   coding; inter-rater reliability with two coders; resolve disagreements by
   consensus.
5. **Feature prioritization**: Map F1 value ratings × G item severity onto an
   impact-effort matrix.

### Ethical considerations
- All demographic items are optional and include "Prefer not to say."
- No telemetry, no tracking IDs, no IP logging.
- Raw data stored privately; only aggregated results published.
- Open-ended responses may be quoted anonymously in public reports (with
  explicit opt-in from J4).

---

## References

1. Davis, F. D. (1989). Perceived usefulness, perceived ease of use, and user
   acceptance of information technology. *MIS Quarterly*, 13(3), 319–340.
2. Brooke, J. (1996). SUS: A "quick and dirty" usability scale. In *Usability
   Evaluation in Industry* (pp. 189–194). Taylor & Francis.
3. Ryan, R. M., & Deci, E. L. (2000). Self-determination theory and the
   facilitation of intrinsic motivation, social development, and well-being.
   *American Psychologist*, 55(1), 68–78.
4. Reichheld, F. F. (2003). The one number you need to grow. *Harvard Business
   Review*, 81(12), 46–54.
5. Sauro, J., & Lewis, J. R. (2016). *Quantifying the User Experience* (2nd
   ed.). Morgan Kaufmann.
6. Krosnick, J. A., & Presser, S. (2010). Question and questionnaire design. In
   *Handbook of Survey Research* (pp. 263–313). Emerald.
