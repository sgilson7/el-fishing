# Decisions

Dated entries. Each quotes the person's request word for word, then says what the agent decided and where it is recorded. Grants of autonomy go here too ("you may publish"), with their scope: this run only, or until withdrawn.

## 2026-10-17

> then, stop and allow me to review both how you will implement the hyper realistic water simulation for fishing, and another tab for the mockups

The agent decided: the plan is not approved in advance. This round ends after the brief, glossary, research report, paper and mockups, all shown in the design journal (`../journal/fishing-journal.html`). No game code is written and nothing is published. Scope: this round only.

## 2026-10-17, later the same day

> fyi i give you approval to create a public github repo, and a public github pages when the time comes to store and deploy

The agent decided: the grant is written into `BRIEF.md`'s Autonomy paragraph word for word. It covers creating one public GitHub repository for this game and publishing it to GitHub Pages with `./scripts/publish.sh` after the plan is read, the build exists and `./scripts/check.sh` passes. It does not cover this design round, in which nothing is pushed. Scope: until withdrawn.

## 2026-10-17, round 2

> name the game "Extremely Lifelike Fishing", take your recomemndation to answer every question, begin the build, dont stop till the mvp is deployed, i give explicit permision to make deploy the website repo with a new chip when the mvp is deployed. start by getting just the first open ocean swells map ready for me to test, don't do the rest of whats in the mockup; just everything necessary to get the first level working and able to catch fish in it
>
> the tag for website should be el-fishing
>
> as in the website extension sgilson7.github.io/el-fishing

The agent decided: the plan is approved in advance for this MVP, and every open question takes the agent's recommendation (listed in `PLAN.md`, "Answers"). Where a question had no recommendation, the agent chose and says so there. The build is the Open Swell only. Publishing is granted: repository `el-fishing`, GitHub Pages at `sgilson7.github.io/el-fishing`, then a chip tagged `el-fishing` on the person's website, deployed. Scope: this MVP.

## 2026-10-17, round 2: published

Under the grant above: the public repository `sgilson7/el-fishing` was created, `dist/` was published from 95faae9 to GitHub Pages at `https://sgilson7.github.io/el-fishing/`, and the browser check passed against the live address. The website lists the game with the route `el-fishing` (local `website` commit e77223c, build 651a8ae4), deployed and gated live. Scope of the grant: used.
