## Article Article https://x.com/zodchiii/article/2038909113795584094

# Conversation https://x.com/zodchiii [darkzodchi darkzodchi darkzodchi darkzodchi darkzodchi](https://x.com/zodchiii) [@zodchiii @zodchiii @zodchiii](https://x.com/zodchiii) https://x.com/zodchiii/article/2038909113795584094/media/2038908575700123648 [1.7M 1.7M 1.7M 1.7M 1.7M 1.7M](https://x.com/zodchiii/status/2038909113795584094/analytics) [https://t.me/zodchixquant https://t.me/zodchixquant https://t.me/zodchixquant](https://t.me/zodchixquant) https://x.com/zodchiii/article/2038909113795584094/media/2038890990229180416

## Getting started Getting started Getting started Getting started > 1. claude 1. claude 1. claude 1. claude https://x.com/zodchiii/article/2038909113795584094/media/2038891451581689857 > 2. claude "refactor the auth module" 2. claude "refactor the auth module" 2. claude "refactor the auth module" 2. claude "refactor the auth module" ```python
claude "find all TODO comments and create GitHub issues for each one" claude "review the last 3 commits for security issues" claude "add error handling to all API routes" claude "find all TODO comments and create GitHub issues for each one" claude "review the last 3 commits for security issues" claude "add error handling to all API routes"
```

> 3. claude -c and claude -r "name" 3. claude -c and claude -r "name" 3. claude -c and claude -r "name" 3. claude -c and claude -r "name" ```markdown
claude -n "payments-refactor" claude -n "payments-refactor"
```

> 4. /clear 4. /clear 4. /clear 4. /clear https://x.com/zodchiii/article/2038909113795584094/media/2038893559051612160 > 5. /compact 5. /compact 5. /compact 5. /compact https://x.com/zodchiii/article/2038909113795584094/media/2038893085971681280

## Speed tricks Speed tricks Speed tricks Speed tricks > 6. Esc to stop, Esc Esc to rewind 6. Esc to stop, Esc Esc to rewind 6. 6. Esc Esc to stop, to stop, Esc Esc Esc Esc to rewind to rewind > 7. !command 7. !command 7. !command 7. !command ```python
!git diff --stat !cat src/api/routes.ts !npm run lint !git diff - - stat !cat src / api / routes . ts
```

> 8. git diff main | claude -p "review for security issues" 8. git diff main | claude -p "review for security issues" 8. git diff main | claude -p "review for security issues" 8. git diff main | claude -p "review for security issues" ```python
cat error.log | claude -p "what caused this crash?" git diff main | claude -p "review for bugs and security issues" cat package.json | claude -p "are any dependencies outdated or vulnerable?" cat error . log | claude - p "what caused this crash?" git diff main | claude - p "review for bugs and security issues" cat package . json | claude - p "are any dependencies outdated or vulnerable?"
```

> 9. -p for scripting and automation 9. -p for scripting and automation 9. -p for scripting and automation 9. -p for scripting and automation ```python
# Daily error analysis at 6am 0 6 * * * tail -1000 /var/log/app.log | \ claude -p "analyze errors and patterns, output JSON" \ --output-format json > /tmp/daily-analysis.json # JSON output for downstream processing claude -p "list all TODO comments" --output-format json # Structured output with schema validation claude -p "extract function names from auth.py" \ --output-format json \ --json-schema '{"type":"object","properties":{"functions":{"type":"array","items":{"type":"string"}}}}' # Daily error analysis at 6am 0 6 * * * tail - 1000 / var / log / app . log | \ claude - p "analyze errors and patterns, output JSON" \ - - output - format json > / tmp / daily - analysis . json # JSON output for downstream processing claude - p "list all TODO comments" - - output - format json # Structured output with schema validation claude - p "extract function names from auth.py" \ - - output - format json \ - - json - schema '{"type":"object","properties":{"functions":{"type":"array","items":{"type":"string"}}}}'
```

> 10. /clear between tasks (yes, again) 10. /clear between tasks (yes, again) 10. /clear between tasks (yes, again) 10. /clear between tasks (yes, again) ```python
Task 1: implement feature → /clear Task 2: write tests → /clear Task 3: fix bug → /clear Task 4: review PR → /clear Task 1 : implement feature → / clear Task 2 : write tests → / clear Task 3 : fix bug → / clear Task 4 : review PR → / clear
```

## Power user Power user Power user Power user > 11. claude -w feature-branch 11. claude -w feature-branch 11. claude -w feature-branch 11. claude -w feature-branch ```python
claude -w "implement-oauth" claude - w "implement-oauth"
```

> 12. claude --permission-mode auto 12. claude --permission-mode auto 12. claude --permission-mode auto 12. claude --permission-mode auto > 13. --allowedTools for scoped permissions 13. --allowedTools for scoped permissions 13. --allowedTools for scoped permissions 13. --allowedTools for scoped permissions ```markdown
claude --allowedTools "Read" "Grep" "LS" "Bash(npm run test:*)" claude --allowedTools "Read" "Grep" "LS" "Bash(npm run test:*)"
``` ```json
{ "permissions": { "allow": ["Read", "Grep", "LS", "Bash(npm run test:*)"], "deny": ["WebFetch", "Bash(curl:*)", "Read(./.env)"] } } { "permissions" : { "allow" : [ "Read" , "Grep" , "LS" , "Bash(npm run test:*)" ] , "deny" : [ "WebFetch" , "Bash(curl:*)" , "Read(./.env)" ] } }
```

> 14. --max-budget-usd 5.00 14. --max-budget-usd 5.00 14. --max-budget-usd 5.00 14. --max-budget-usd 5.00 ```python
claude -p "refactor the API layer" --max-budget-usd 5.00 claude -p "fix the bug" --max-turns 3 claude - p "refactor the API layer" - - max - budget - usd 5.00 claude - p "fix the bug" - - max - turns 3
```

> 15. --add-dir for multi-repo context 15. --add-dir for multi-repo context 15. --add-dir for multi-repo context 15. --add-dir for multi-repo context ```markdown
claude --add-dir ./services/api ./packages/ui ./shared/types claude --add-dir ./services/api ./packages/ui ./shared/types
```

## Advanced workflows Advanced workflows Advanced workflows Advanced workflows > 16. CLAUDE.md 16. CLAUDE.md 16. CLAUDE.md 16. CLAUDE.md ```markdown
# Project context This is a Next.js 14 app with TypeScript, Prisma ORM, and Tailwind. API routes are in /src/app/api/. All database queries go through Prisma, never raw SQL. # Code style Use functional components with hooks, no class components. Error messages should be user-friendly, not technical. All API responses follow the {data, error, meta} format. # Testing Run `npm run test` after any changes. Fix failures before calling it done. # Project context # Code style # Testing Run `npm run test` after any changes.
```

> 17. Hooks for automatic formatting 17. Hooks for automatic formatting 17. Hooks for automatic formatting 17. Hooks for automatic formatting ```json
{ "hooks": { "PostToolUse": [ { "matcher": "Edit|Write", "hooks": [ { "type": "command", "command": "npx prettier --write \"$CLAUDE_FILE_PATH\" 2>/dev/null || true" } ] } ] } } { "hooks" : { "PostToolUse" : [ { "matcher" : "Edit|Write" , "hooks" : [ { "type" : "command" , "command" : "npx prettier --write \"$CLAUDE_FILE_PATH\" 2>/dev/null || true" } ] } ] } }
```

> 18. /install-github-app 18. /install-github-app 18. /install-github-app 18. /install-github-app > 19. TDD workflow 19. TDD workflow 19. TDD workflow 19. TDD workflow ```markdown
Before writing the implementation, write unit tests for a function called calculateShippingCost that takes an order weight and a destination zone (1-5) and returns a cost in dollars. Cover normal cases, edge cases (zero weight, max zone), and invalid inputs. Before writing the implementation, write unit tests for a function
``` ```markdown
Now implement the function to pass all those tests. Run the tests after implementation and fix any failures. Now implement the function to pass all those tests. Run the tests
```

> 20. Parallel sessions with agent teams 20. Parallel sessions with agent teams 20. Parallel sessions with agent teams 20. Parallel sessions with agent teams ```markdown
claude -w feature-auth --background claude -w feature-payments --background claude -w feature-notifications --background claude -w feature-auth --background
```

## The cheat sheet The cheat sheet The cheat sheet The cheat sheet ```markdown
GETTING STARTED claude start interactive session claude "prompt" start with initial task claude -c continue last session claude -r "name" resume named session /clear wipe context between tasks SPEED TRICKS Esc stop mid-action Esc Esc rewind to checkpoint !command run shell inline /compact compress context /context check token usage POWER USER claude -p "prompt" non-interactive mode git diff | claude -p "review" pipe anything in claude -w branch-name isolated worktree --permission-mode auto AI decides permissions --allowedTools "Read" "Grep" scoped permissions --max-budget-usd 5.00 spending limit ADVANCED CLAUDE.md project instructions hooks in settings.json auto-format on edit /install-github-app auto PR reviews TDD: tests first, then implement 2-3x quality boost -w branch --background parallel agents GETTING STARTED
```

https://x.com/zodchiii/article/2038909113795584094/media/2038897315017027584 https://x.com/zodchiii/article/2038909113795584094/media/2038897513609015297 [11:19 AM · Mar 31, 2026 11:19 AM · Mar 31, 2026](https://x.com/zodchiii/status/2038909113795584094) [1.7M Views 1.7M 1.7M 1.7M 1.7M Views Views](https://x.com/zodchiii/status/2038909113795584094/analytics)

# Trending now

## What’s happening What’s happening What’s happening [Show more Show more Show more](https://x.com/explore/tabs/for-you)