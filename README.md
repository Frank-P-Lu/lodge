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
   This creates the table and makes a new CLI subcommand available immediately.

3. **Use it.** The generated CLI is the primary interface:
   ```
   lodge gym_sessions add --date 2026-03-17 --duration 45 --notes "leg day"
   lodge gym_sessions query --where "date > 2026-03-01" --sort "date desc"
   lodge gym_sessions update --id 3 --notes "leg day, added squats"
   lodge gym_sessions delete --id 3
   ```

4. **Discover it.** Any agent in a future session runs `lodge help` and immediately sees what collections exist, what fields they have, and what commands are available. The help output is dynamically generated from the schema, so it's always accurate.

5. **Evolve it.** Add fields, create new collections. The CLI updates automatically:
   ```
   lodge alter gym_sessions --add "muscle_group:text"
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
```

**Project state over time.** Track status changes so the agent knows the full history, not just current state.
```
lodge create projects --fields "name:text, status:text, updated_at:datetime, notes:text"
lodge projects add --name myapp --status "launched" --notes "v1 shipped"
lodge projects query --where "name = 'myapp'" --sort "updated_at desc"
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
