# Second-order notebook

Stage 6, kept by the agent as the work happens, not afterwards. `B3 Build in self-questioning`.

The code and tests are the work; this file is the record about the work. It is where a decision the agent made alone, an assumption that turned out wrong, or something only the person can do becomes visible to the person. With the plan approved in advance, it is the main place those decisions surface before the person uses the tool. [`docs/procedure.md`](https://github.com/sgilson7/builder-setting/blob/main/docs/procedure.md#the-second-order-notebook) says why.

One row per assumption, surprise or open item. **Kind** is *divergence* (what was assumed, what turned out to be true, what changed), *finding*, or *worklist* (the person's to do). Number rows by round (R1.1, R1.2, then R2.1 after the first triage) so a later round does not renumber an earlier one. **Rule** is the rule the row produced for later sessions, and where it now lives (`CLAUDE.md`, a test, `tests/breaks.json`), or blank. **Status** is *done*, *open*, or *person*; every *person* row is named in `HANDOFF.md`. `scripts/notebook_check.py` checks the form of each row and that the notebook is not empty once `PLAN.md` holds a decision, not what the rows say.

| # | Kind | Assumed or found | What is true | What changed | Rule | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R1.1 | worklist | The description gives no name for the game. | A name is the person's (brief, decisions that stay with the person), and the publishing grant needs a repository name. | The project folder is `fishing-game`; no string names the game yet; no repository is created until a name is chosen. | | person |
| R1.2 | divergence | "chi games cs ed conference" names one venue. | No single venue has that name; CHI PLAY is ACM's games-and-HCI venue, and CS-education venues (SIGCSE, ITiCSE, ICER) are separate. | The paper is drafted in the ACM format for CHI PLAY, with the CS-education contribution (the Builder procedure as a way to teach simulation work) as its second claim. The venue is the person's to confirm. | | person |
| R1.3 | finding | The template's autonomy example approves the plan in advance. | The description says to stop for review after the research, paper and mockups. | The brief's Autonomy paragraph withholds approval; `PLAN.md` is not written this round. The water design is in the journal for the person to read first. | `BRIEF.md` Autonomy | done |
| R1.4 | finding | The first glossary draft put "strike" on the hook set's `Not:` line. | "Strike" is its own term (a fish's rush), so the copy test would have banned the word everywhere, including where it is right. | Hook set's `Not:` line is "yank, jerk". A word that is another entry's name never goes on a `Not:` line. | this row | done |
| R1.5 | worklist | The glossary's eighteen terms and their `Not:` lines were written by the agent from fishing usage. | A generated list a reader will see is the person's to check (`CLAUDE.md` rule 12). | Listed in `GLOSSARY.md` for the person to read. | | person |
| R1.6 | divergence | The style brief asks for Ape Race's faceted look everywhere. | Faceted, flat-shaded water would undo the brief's first goal, realistic water. | Land, kayak, angler, fish and props are faceted; the water is smooth with reflection, refraction and foam. Recorded in the brief's Style section for the person to confirm. | `BRIEF.md` Style | person |

## Strings written by the agent

Each reader-visible string the agent wrote, with its key, for the person to read.

| Key | String |
| --- | --- |
