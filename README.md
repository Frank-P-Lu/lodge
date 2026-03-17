# Lodge

A structured data tool for AI agents.

Named after the Black Lodge in Twin Peaks. A place between worlds where all information is stored and retrieved, if you know how to ask.

Lodge gives agents a local database they can configure themselves. No predefined schemas, no opinions about what you track. The agent creates collections, and Lodge generates a typed CLI interface from the schema. Future sessions run `lodge help` to discover what exists and start working immediately.

Think of it as the storage layer that makes agents useful over time. Without structured persistence, every conversation starts from zero.

## Why

Agents are good at conversation but bad at remembering things. Most agent setups hack around this with markdown files, but markdown doesn't filter, sort, aggregate, or scale. You end up with a pile of text the agent has to re-read every session.

Lodge replaces that with a real database (SQLite) behind a CLI the agent operates. The agent never writes raw SQL. It uses typed commands that Lodge generates from the schema. If something isn't covered by the generated commands, the agent can drop down to raw SQL as an escape hatch.

## How it works

1. **Init.** `lodge init` creates a `.lodge/` directory with a SQLite database.

2. **Create a collection.** The agent (or you) defines what to track:
   ```
   lodge create gym_sessions --fields "date:date, duration:int, notes:text"
   ```
   This creates the table and makes a new CLI subcommand available immediately. Text fields automatically get full-text search enabled.

3. **Use it.** The generated CLI is the primary interface:
   ```
   lodge gym_sessions add --date 2026-03-17 --duration 45 --notes "leg day"
   lodge gym_sessions query --where "date > 2026-03-01" --sort "date desc"
   lodge gym_sessions update --id 3 --notes "leg day, added squats"
   lodge gym_sessions update --id 3 --clear-notes   # set a field to null
   lodge gym_sessions delete --id 3
   ```

4. **Search text fields.** Full-text search is automatically enabled for all text fields:
   ```
   lodge gym_sessions search "leg day"
   lodge gym_sessions search "squats" --limit 5
   ```

5. **Inspect a collection.** View the schema for a specific collection:
   ```
   lodge gym_sessions schema
   ```

6. **Discover it.** Any agent in a future session runs `lodge help` and immediately sees what collections exist, what fields they have, and what commands are available. The help output is dynamically generated from the schema, so it's always accurate.

   List all collections and their fields:
   ```
   lodge list
   ```

7. **Evolve it.** Add, rename, or drop fields. The CLI updates automatically:
   ```
   lodge alter gym_sessions --add-fields "muscle_group:text"
   lodge alter gym_sessions --rename-field "notes:description"
   lodge alter gym_sessions --drop-fields "muscle_group"
   ```

8. **Save views.** Bookmark queries the agent runs repeatedly:
   ```
   lodge view create recent_sessions --collection gym_sessions --where "date > '2026-03-01'" --sort "date desc" --limit 10
   lodge view run recent_sessions
   lodge view list
   lodge view show recent_sessions
   lodge view update recent_sessions --limit 20
   lodge view delete recent_sessions
   ```

   Shorthand for running a view without the `view` prefix:
   ```
   lodge run recent_sessions
   ```

9. **Time-series analysis.** Built-in commands for streak tracking and trend analysis on date fields:
   ```
   lodge gym_sessions streak --field date
   lodge gym_sessions gaps --field date --threshold 2
   lodge gym_sessions rolling-avg --field duration --over date --window 7
   ```

10. **Export and import.** Snapshot data or move it between contexts:
    ```
    lodge export gym_sessions
    lodge export --all > backup.json
    lodge import --collection gym_sessions --file data.json
    lodge import --all --file backup.json
    ```

11. **Snapshot and restore.** Save and restore the full database:
    ```
    lodge snapshot
    lodge snapshot --output my_backup.db
    lodge restore path/to/snapshot.db
    ```

12. **Audit log.** Every write operation is logged automatically:
    ```
    lodge log
    lodge log --collection gym_sessions
    lodge log --limit 50
    ```

## The metaprogramming bit

Lodge doesn't have hardcoded knowledge of your data. When you create a collection, Lodge reads the SQLite schema at runtime and dynamically builds the CLI interface from it. The schema is the program definition.

`lodge help` reads the same schema to generate its output, so documentation can never go stale. There are no external docs to maintain. The tool describes itself.

## Use cases

**Habit and streak tracking.** Gym sessions, sleep times, daily routines. Query streaks, aggregates, trends over time.
```
lodge create habits --fields "name:text, date:date, value:text, notes:text"
lodge habits add --name gym --date 2026-03-17 --value "45min"
lodge habits query --where "name = 'gym'" --sort "date desc" --limit 7
lodge habits streak --field date
```

**Project state over time.** Track status changes so the agent knows the full history, not just current state.
```
lodge create projects --fields "name:text, status:text, updated_at:datetime, notes:text"
lodge projects add --name myapp --status "launched" --notes "v1 shipped"
lodge projects query --where "name = 'myapp'" --sort "updated_at desc"
lodge projects search "launched"
```

**Tasks with priorities and deadlines.** Replace unstructured todo lists with something queryable.
```
lodge create tasks --fields "title:text, priority:int, deadline:date, status:text, project:text, notes:text"
lodge tasks query --where "status = 'open'" --sort "priority asc, deadline asc"
```

**People and interactions.** Who you talked to, when, what about. Build context over time.
```
lodge create people --fields "name:text, last_contact:date, context:text, notes:text"
lodge people query --where "last_contact < 2026-03-01" --sort "last_contact asc"
lodge people search "project kickoff"
```

**Anything else.** Decisions log, metrics over time, reading lists, whatever. The agent creates what it needs.

## What Lodge is not

- Not a web app. No server, no HTTP, no auth. It's a binary and a SQLite file.
- Not opinionated. It doesn't know what "habits" or "projects" are. You define the schema, Lodge provides the interface.
- Not a human-facing tool (primarily). The CLI is designed for agents to operate. Humans can use it too, but the interface is optimized for LLM consumption.

## The thesis

Most productivity tools (Notion, Asana, Jira) are complex because they need to be operable by humans. Views, drag-and-drop, formulas, templates, the whole GUI. When an agent is the operator, you don't need any of that. You just need structured storage with good discoverability.

Lodge is what you get when you ask: what does Notion look like if no human ever touches the data layer?

The answer is a SQLite database, a CLI, and a good help command.

## Install

```
cargo install lodge
```

Or build from source:
```
git clone https://github.com/user/lodge
cd lodge
cargo build --release
```

## License

MIT
