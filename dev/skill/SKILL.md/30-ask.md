## 2. Ask

If the request doesn't say what the skill does and when it should load, ask that first, in one plain question. That answer is free text, so it doesn't go in a multiple-choice question.

Then ask the rest in one AskUserQuestion call. Leave out a question the request already answers, and put the recommended option first.

| Header | Question | Options |
|---|---|---|
| Name | What should the skill be called? | Two or three names you suggest: lowercase letters, digits and hyphens, at most 64 characters, named for the job |
| Loading | When should Claude use it? | Automatically when a task matches (recommended); only when typed as `/<name>` |
| Files | What does it need besides SKILL.md? (multiSelect) | Nothing else (recommended for a first draft); reference files Claude reads when needed; scripts for steps that must come out the same every time; assets such as templates |

"Only when typed" adds `disable-model-invocation: true` to the frontmatter.
