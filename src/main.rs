mod cli;
mod collection;
mod db;
mod error;
mod export;
mod fts;
mod import;
mod log;
mod output;
mod query_track;
mod record;
mod schema;
mod settings;
mod snapshot;
mod timeseries;
mod types;
mod view;

use clap::ArgMatches;
use error::LodgeError;
use output::Format;
use rusqlite::Connection;
use schema::Collection;
use settings::Settings;
use std::path::Path;
use std::process;

fn fields_to_json(
    conn: &Connection,
    collection_name: &str,
    fields: &[schema::Field],
    max_distinct: usize,
    ratio: f64,
) -> Vec<serde_json::Value> {
    let distinct_map =
        schema::load_all_distinct_values(conn, collection_name, fields, max_distinct, ratio)
            .unwrap_or_default();
    fields
        .iter()
        .map(|f| {
            let mut field_json =
                serde_json::json!({"name": f.name, "type": f.field_type.as_str()});
            if let Some(values) = distinct_map.get(&f.name) {
                field_json["values"] = serde_json::json!(values);
            }
            field_json
        })
        .collect()
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

/// Get a required string argument from clap matches.
fn get_arg<'a>(m: &'a ArgMatches, name: &str) -> error::Result<&'a String> {
    m.get_one::<String>(name)
        .ok_or_else(|| LodgeError::MissingArgument(name.to_string()))
}

/// Get the --format flag and parse it, falling back to the given default.
fn get_format(m: &ArgMatches, default_format: &str) -> Format {
    let source = m.value_source("format");
    if source == Some(clap::parser::ValueSource::CommandLine) {
        if let Some(s) = m.get_one::<String>("format") {
            if let Some(f) = Format::from_str(s) {
                return f;
            }
        }
    }
    Format::from_str(default_format).unwrap_or(Format::Json)
}

/// Parse a required string argument as i64.
fn get_id(m: &ArgMatches) -> error::Result<i64> {
    let raw = get_arg(m, "id")?;
    raw.parse().map_err(|_| LodgeError::InvalidValue {
        field: "id".to_string(),
        field_type: "int".to_string(),
        value: raw.clone(),
    })
}

fn run() -> error::Result<()> {
    let cwd = std::env::current_dir()?;

    // Try to load collections, view names, and settings from existing DB
    let (collections, view_names, settings) = if let Some(lodge_dir) = db::find_lodge_dir(&cwd) {
        let conn = db::open(&cwd)?;
        let colls = schema::load_collections(&conn)?;
        let views = view::load_view_names(&conn)?;
        let s = settings::load_settings(&lodge_dir);
        (colls, views, s)
    } else {
        (Vec::new(), Vec::new(), Settings::default())
    };

    let cmd = cli::build_cli(&collections, &view_names);
    let matches = cmd.get_matches();
    let df = &settings.default_format;

    match matches.subcommand() {
        Some(("init", _)) => {
            db::init(&cwd)?;
            let lodge_dir = db::lodge_dir(&cwd)?;
            settings::create_default_settings(&lodge_dir)?;
            println!("Initialized lodge database in .lodge/");
            Ok(())
        }
        Some(("guide", _)) => {
            print_guide();
            Ok(())
        }
        Some(("set", sub_m)) => handle_set(&cwd, sub_m),
        Some(("create", sub_m)) => handle_create(&cwd, sub_m),
        Some(("alter", sub_m)) => handle_alter(&cwd, sub_m),
        Some(("drop", sub_m)) => handle_drop(&cwd, sub_m),
        Some(("sql", sub_m)) => handle_sql(&cwd, sub_m, df),
        Some(("view", sub_m)) => handle_view(&cwd, sub_m, df, &settings),
        Some(("export", sub_m)) => handle_export(&cwd, sub_m, df),
        Some(("snapshot", sub_m)) => handle_snapshot(&cwd, sub_m),
        Some(("restore", sub_m)) => handle_restore(&cwd, sub_m),
        Some(("import", sub_m)) => handle_import(&cwd, sub_m),
        Some(("log", sub_m)) => handle_log(&cwd, sub_m, df),
        Some(("list", sub_m)) => handle_list(&cwd, sub_m, df, &settings),
        Some(("run", sub_m)) => handle_run_view(&cwd, sub_m, df, &settings),
        Some((collection_name, sub_m)) => {
            handle_collection(&cwd, collection_name, sub_m, df, &settings)
        }
        _ => unreachable!(),
    }
}

fn handle_set(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let lodge_dir = db::lodge_dir(cwd)?;
    let key = get_arg(sub_m, "key")?;
    let value = get_arg(sub_m, "value")?;
    settings::set_setting(&lodge_dir, key, value)?;
    println!("Set {key} = {value}");
    Ok(())
}

fn handle_create(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let name = get_arg(sub_m, "name")?;
    let fields = get_arg(sub_m, "fields")?;
    collection::create_collection(&conn, name, fields)?;
    let coll = schema::load_collection(&conn, name)?.unwrap();
    let fields_desc: Vec<String> = coll
        .fields
        .iter()
        .map(|f| format!("{}:{}", f.name, f.field_type.as_str()))
        .collect();
    // Auto-enable FTS for all text fields
    let text_fields: Vec<String> = coll
        .fields
        .iter()
        .filter(|f| f.field_type == types::FieldType::Text)
        .map(|f| f.name.clone())
        .collect();
    if !text_fields.is_empty() {
        fts::create_fts_table(&conn, name, &text_fields)?;
    }
    println!(
        "Created collection '{}' with fields: {}",
        name,
        fields_desc.join(", ")
    );
    Ok(())
}

fn handle_alter(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let name = get_arg(sub_m, "name")?;

    // Capture text fields before any changes for FTS rebuild logic
    let coll_before = schema::load_collection(&conn, name)?
        .ok_or_else(|| LodgeError::CollectionNotFound(name.to_string()))?;
    let prev_text_fields: Vec<String> = coll_before
        .fields
        .iter()
        .filter(|f| f.field_type == types::FieldType::Text)
        .map(|f| f.name.clone())
        .collect();

    // Drop FTS before structural changes (triggers reference columns)
    let had_fts = fts::has_fts(&conn, name)?;
    if had_fts {
        fts::drop_fts_table(&conn, name)?;
    }

    // --add-fields
    if let Some(fields_spec) = sub_m.get_one::<String>("add-fields") {
        collection::alter_collection(&conn, name, fields_spec)?;
    }

    // --rename-field
    if let Some(rename_spec) = sub_m.get_one::<String>("rename-field") {
        let parts: Vec<&str> = rename_spec.splitn(2, ':').collect();
        if parts.len() != 2 || parts[0].trim().is_empty() || parts[1].trim().is_empty() {
            return Err(LodgeError::InvalidFieldsFormat(
                "rename format must be \"old_name:new_name\"".to_string(),
            ));
        }
        collection::rename_field(&conn, name, parts[0].trim(), parts[1].trim())?;
    }

    // --drop-fields
    if let Some(drop_spec) = sub_m.get_one::<String>("drop-fields") {
        let field_names: Vec<String> = drop_spec
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        collection::drop_fields(&conn, name, &field_names)?;
    }

    // Rebuild FTS with current text fields
    let coll = schema::load_collection(&conn, name)?.unwrap();
    let new_text_fields: Vec<String> = coll
        .fields
        .iter()
        .filter(|f| f.field_type == types::FieldType::Text)
        .map(|f| f.name.clone())
        .collect();

    if (had_fts || prev_text_fields != new_text_fields) && !new_text_fields.is_empty() {
        fts::create_fts_table(&conn, name, &new_text_fields)?;
    }

    let fields_desc: Vec<String> = coll
        .fields
        .iter()
        .map(|f| format!("{}:{}", f.name, f.field_type.as_str()))
        .collect();
    println!(
        "Altered collection '{}', fields now: {}",
        name,
        fields_desc.join(", ")
    );
    Ok(())
}

fn handle_drop(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let name = get_arg(sub_m, "name")?;
    collection::drop_collection(&conn, name)?;
    println!("Dropped collection '{name}'");
    Ok(())
}

fn handle_sql(cwd: &Path, sub_m: &ArgMatches, df: &str) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let collections_now = schema::load_collections(&conn)?;
    let query = get_arg(sub_m, "query")?;
    let format = get_format(sub_m, df);
    let results = record::execute_sql(&conn, query, &collections_now)?;
    println!("{}", output::format_output(&results, &format)?);
    Ok(())
}

fn handle_view(cwd: &Path, sub_m: &ArgMatches, df: &str, settings: &Settings) -> error::Result<()> {
    let conn = db::open(cwd)?;
    match sub_m.subcommand() {
        Some(("create", create_m)) => {
            let name = get_arg(create_m, "name")?;
            let description = create_m
                .get_one::<String>("description")
                .map(|s| s.as_str());
            if let Some(sql) = create_m.get_one::<String>("sql") {
                view::create_sql_view(&conn, name, sql, description)?;
            } else {
                let collection = get_arg(create_m, "collection")?;
                let where_clause = create_m.get_one::<String>("where").map(|s| s.as_str());
                let sort = create_m.get_one::<String>("sort").map(|s| s.as_str());
                let limit = create_m
                    .get_one::<String>("limit")
                    .and_then(|s| s.parse::<i64>().ok());
                view::create_view(
                    &conn,
                    name,
                    collection,
                    where_clause,
                    sort,
                    limit,
                    description,
                )?;
            }
            println!("Created view '{name}'");
            Ok(())
        }
        Some(("list", list_m)) => {
            let format = get_format(list_m, df);
            let views = view::list_views(&conn)?;
            println!("{}", output::format_output(&views, &format)?);
            Ok(())
        }
        Some(("show", show_m)) => {
            let name = get_arg(show_m, "name")?;
            let v = view::show_view(&conn, name)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&v).map_err(|e| LodgeError::Sql(e.to_string()))?
            );
            Ok(())
        }
        Some(("update", update_m)) => {
            let name = get_arg(update_m, "name")?;
            let where_clause = update_m.get_one::<String>("where").map(|s| s.as_str());
            let sort = update_m.get_one::<String>("sort").map(|s| s.as_str());
            let limit = update_m
                .get_one::<String>("limit")
                .and_then(|s| s.parse::<i64>().ok());
            let description = update_m
                .get_one::<String>("description")
                .map(|s| s.as_str());
            let sql = update_m.get_one::<String>("sql").map(|s| s.as_str());
            view::update_view(&conn, name, where_clause, sort, limit, description, sql)?;
            println!("Updated view '{name}'");
            Ok(())
        }
        Some(("run", run_m)) => run_view_inner(&conn, run_m, df, settings.view_suggest_threshold),
        Some(("delete", delete_m)) => {
            let name = get_arg(delete_m, "name")?;
            view::delete_view(&conn, name)?;
            println!("Deleted view '{name}'");
            Ok(())
        }
        _ => unreachable!(),
    }
}

fn handle_export(cwd: &Path, sub_m: &ArgMatches, df: &str) -> error::Result<()> {
    let conn = db::open(cwd)?;
    if sub_m.get_flag("all") {
        let result = export::export_all(&conn)?;
        println!("{result}");
    } else {
        let name = get_arg(sub_m, "collection")?;
        let format = get_format(sub_m, df);
        let result = export::export_collection(&conn, name, &format)?;
        println!("{result}");
    }
    Ok(())
}

fn handle_snapshot(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let lodge_dir = db::lodge_dir(cwd)?;
    let output_path = sub_m.get_one::<String>("output").map(|s| s.as_str());
    let path = snapshot::create_snapshot(&conn, &lodge_dir, output_path)?;
    println!("Snapshot saved to {}", path.display());
    Ok(())
}

fn handle_restore(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let path = get_arg(sub_m, "path")?;
    snapshot::restore_snapshot(&conn, path)?;
    println!("Restored from {path}");
    Ok(())
}

fn handle_import(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let file_path = get_arg(sub_m, "file")?;
    let data = std::fs::read_to_string(file_path)?;
    if sub_m.get_flag("all") {
        let results = import::import_full(&conn, &data)?;
        for (name, count) in &results {
            println!("Imported {count} records into '{name}'");
        }
    } else {
        let name = sub_m.get_one::<String>("collection").ok_or_else(|| {
            LodgeError::MissingArgument("specify a collection name or use --all".to_string())
        })?;
        let count = import::import_collection(&conn, name, &data)?;
        println!("Imported {count} records into '{name}'");
    }
    Ok(())
}

fn handle_log(cwd: &Path, sub_m: &ArgMatches, df: &str) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let collection = sub_m.get_one::<String>("collection").map(|s| s.as_str());
    let limit: i64 = sub_m
        .get_one::<String>("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let verbose = sub_m.get_flag("verbose");
    let since = sub_m.get_one::<String>("since").map(|s| s.as_str());
    if let Some(s) = since {
        log::validate_since(s)?;
    }
    let format = get_format(sub_m, df);
    let results = log::query_log(&conn, collection, limit, verbose, since)?;
    // Strip before/after from non-JSON formats — inline JSON makes table/csv too wide
    // But preserve them when --verbose is explicitly requested
    let results = match format {
        output::Format::Json => results,
        _ if verbose => results,
        _ => results
            .into_iter()
            .map(|v| {
                if let serde_json::Value::Object(mut map) = v {
                    map.remove("before");
                    map.remove("after");
                    serde_json::Value::Object(map)
                } else {
                    v
                }
            })
            .collect(),
    };
    println!("{}", output::format_output(&results, &format)?);
    Ok(())
}

fn handle_list(cwd: &Path, sub_m: &ArgMatches, df: &str, settings: &Settings) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let format = get_format(sub_m, df);
    let colls = schema::load_collections(&conn)?;
    let max_distinct = settings.distinct_max;
    let ratio = settings.distinct_ratio;
    let result: Vec<serde_json::Value> = colls
        .iter()
        .map(|c| {
            let fields = fields_to_json(&conn, &c.name, &c.fields, max_distinct, ratio);
            serde_json::json!({"name": c.name, "fields": fields})
        })
        .collect();
    println!("{}", output::format_output(&result, &format)?);
    Ok(())
}

fn handle_run_view(
    cwd: &Path,
    run_m: &ArgMatches,
    df: &str,
    settings: &Settings,
) -> error::Result<()> {
    let conn = db::open(cwd)?;
    run_view_inner(&conn, run_m, df, settings.view_suggest_threshold)
}

/// Shared logic for `lodge run <view>` and `lodge view run <view>`.
fn run_view_inner(
    conn: &Connection,
    run_m: &ArgMatches,
    df: &str,
    threshold: i64,
) -> error::Result<()> {
    let name = get_arg(run_m, "name")?;
    let format = get_format(run_m, df);
    let (collection_name, records) = view::run_view(conn, name)?;
    if run_m.get_flag("meta") {
        let collection_val = match &collection_name {
            Some(c) => serde_json::Value::String(c.clone()),
            None => serde_json::Value::Null,
        };
        let wrapped = serde_json::json!({
            "view": name,
            "collection": collection_val,
            "records": records,
        });
        println!("{wrapped}");
    } else {
        println!("{}", output::format_output(&records, &format)?);
    }

    // Track view run (never suggests)
    let fingerprint = query_track::build_view_run_fingerprint(name);
    let _ = query_track::track_query(conn, "view_run", name, &fingerprint, threshold);

    Ok(())
}

fn handle_collection(
    cwd: &Path,
    collection_name: &str,
    sub_m: &ArgMatches,
    df: &str,
    settings: &Settings,
) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let coll = schema::load_collection(&conn, collection_name)?
        .ok_or_else(|| LodgeError::CollectionNotFound(collection_name.to_string()))?;

    match sub_m.subcommand() {
        Some(("add", add_m)) => handle_collection_add(&conn, &coll, collection_name, add_m, df),
        Some(("query", query_m)) => handle_collection_query(&conn, &coll, query_m, df, settings),
        Some(("update", update_m)) => {
            handle_collection_update(&conn, &coll, collection_name, update_m, df)
        }
        Some(("delete", delete_m)) => {
            handle_collection_delete(&conn, &coll, collection_name, delete_m, df)
        }
        Some(("search", search_m)) => {
            handle_collection_search(&conn, &coll, search_m, df, settings)
        }
        Some(("streak", streak_m)) => handle_collection_streak(&conn, &coll, streak_m, df),
        Some(("gaps", gaps_m)) => handle_collection_gaps(&conn, &coll, gaps_m, df),
        Some(("rolling-avg", avg_m)) => handle_collection_rolling_avg(&conn, &coll, avg_m, df),
        Some(("schema", schema_m)) => {
            handle_collection_schema(&conn, &coll, schema_m, df, settings)
        }
        _ => unreachable!(),
    }
}

fn handle_collection_add(
    conn: &Connection,
    coll: &Collection,
    collection_name: &str,
    add_m: &ArgMatches,
    df: &str,
) -> error::Result<()> {
    let mut values = Vec::new();
    for field in &coll.fields {
        if let Some(val) = add_m.get_one::<String>(&field.name) {
            values.push((field.name.clone(), val.clone()));
        }
    }
    if values.is_empty() {
        return Err(LodgeError::MissingArgument(
            "no fields provided -- specify at least one field".into(),
        ));
    }
    let format = get_format(add_m, df);
    let result = record::add_record(conn, coll, &values)?;
    println!("Added record {} to '{}'", result["id"], collection_name);
    println!("{}", output::format_single(&result, &format)?);
    Ok(())
}

fn handle_collection_query(
    conn: &Connection,
    coll: &Collection,
    query_m: &ArgMatches,
    df: &str,
    settings: &Settings,
) -> error::Result<()> {
    let where_clause = query_m.get_one::<String>("where").map(|s| s.as_str());
    let sort = query_m.get_one::<String>("sort").map(|s| s.as_str());
    let limit = query_m
        .get_one::<String>("limit")
        .and_then(|s| s.parse::<i64>().ok());
    let fields_str = query_m.get_one::<String>("fields");
    let fields_vec: Option<Vec<&str>> =
        fields_str.map(|s| s.split(',').map(|f| f.trim()).collect());
    let fields_slice = fields_vec.as_deref();
    let format = get_format(query_m, df);
    let results =
        record::query_records_with_fields(conn, coll, where_clause, sort, limit, fields_slice)?;
    println!("{}", output::format_output(&results, &format)?);

    // Track query
    let fingerprint = query_track::build_query_fingerprint(
        &coll.name,
        where_clause,
        sort,
        limit,
        fields_str.map(|s| s.as_str()),
    );
    let track = query_track::track_query(
        conn,
        "query",
        &coll.name,
        &fingerprint,
        settings.view_suggest_threshold,
    )?;
    if track.newly_suggested {
        let cmd = query_track::build_suggestion_command(&coll.name, where_clause, sort, limit);
        eprintln!(
            "Hint: This query has been run {} times. Save it as a view: {}",
            track.call_count, cmd
        );
    }

    Ok(())
}

fn handle_collection_update(
    conn: &Connection,
    coll: &Collection,
    collection_name: &str,
    update_m: &ArgMatches,
    df: &str,
) -> error::Result<()> {
    let id = get_id(update_m)?;
    let mut values = Vec::new();
    let mut clear_fields = Vec::new();
    for field in &coll.fields {
        let clear_name = format!("clear-{}", field.name);
        if update_m.get_flag(&clear_name) {
            clear_fields.push(field.name.clone());
        } else if let Some(val) = update_m.get_one::<String>(&field.name) {
            values.push((field.name.clone(), val.clone()));
        }
    }
    if values.is_empty() && clear_fields.is_empty() {
        return Err(LodgeError::MissingArgument(
            "no fields to update".to_string(),
        ));
    }
    let format = get_format(update_m, df);
    let result = record::update_record(conn, coll, id, &values, &clear_fields)?;
    println!("Updated record {id} in '{collection_name}'");
    println!("{}", output::format_single(&result, &format)?);
    Ok(())
}

fn handle_collection_delete(
    conn: &Connection,
    coll: &Collection,
    collection_name: &str,
    delete_m: &ArgMatches,
    df: &str,
) -> error::Result<()> {
    let id = get_id(delete_m)?;
    let format = get_format(delete_m, df);
    let result = record::delete_record(conn, coll, id)?;
    println!("Deleted record {id} from '{collection_name}'");
    println!("{}", output::format_single(&result, &format)?);
    Ok(())
}

fn handle_collection_search(
    conn: &Connection,
    coll: &Collection,
    search_m: &ArgMatches,
    df: &str,
    settings: &Settings,
) -> error::Result<()> {
    let query = get_arg(search_m, "query")?;
    let limit = search_m
        .get_one::<String>("limit")
        .and_then(|s| s.parse::<i64>().ok());
    let format = get_format(search_m, df);
    let results = fts::search_records(conn, coll, query, limit)?;
    println!("{}", output::format_output(&results, &format)?);

    // Track search (never suggests)
    let fingerprint = query_track::build_search_fingerprint(&coll.name, query, limit);
    let _ = query_track::track_query(
        conn,
        "search",
        &coll.name,
        &fingerprint,
        settings.view_suggest_threshold,
    );

    Ok(())
}

fn handle_collection_streak(
    conn: &Connection,
    coll: &Collection,
    streak_m: &ArgMatches,
    df: &str,
) -> error::Result<()> {
    let field = get_arg(streak_m, "field")?;
    let format = get_format(streak_m, df);
    let result = timeseries::streak(conn, coll, field)?;
    println!("{}", output::format_single(&result, &format)?);
    Ok(())
}

fn handle_collection_gaps(
    conn: &Connection,
    coll: &Collection,
    gaps_m: &ArgMatches,
    df: &str,
) -> error::Result<()> {
    let field = get_arg(gaps_m, "field")?;
    let raw_threshold = get_arg(gaps_m, "threshold")?;
    let threshold: i64 = raw_threshold
        .parse()
        .map_err(|_| LodgeError::InvalidValue {
            field: "threshold".to_string(),
            field_type: "int".to_string(),
            value: raw_threshold.clone(),
        })?;
    let format = get_format(gaps_m, df);
    let results = timeseries::gaps(conn, coll, field, threshold)?;
    println!("{}", output::format_output(&results, &format)?);
    Ok(())
}

fn handle_collection_rolling_avg(
    conn: &Connection,
    coll: &Collection,
    avg_m: &ArgMatches,
    df: &str,
) -> error::Result<()> {
    let field = get_arg(avg_m, "field")?;
    let over = get_arg(avg_m, "over")?;
    let raw_window = get_arg(avg_m, "window")?;
    let window: i64 = raw_window.parse().map_err(|_| LodgeError::InvalidValue {
        field: "window".to_string(),
        field_type: "int".to_string(),
        value: raw_window.clone(),
    })?;
    let format = get_format(avg_m, df);
    let results = timeseries::rolling_average(conn, coll, field, over, window)?;
    println!("{}", output::format_output(&results, &format)?);
    Ok(())
}

fn print_guide() {
    println!(
        "\
Lodge is for data you ACT on. Markdown is for context you READ.

USE LODGE WHEN YOU HAVE:
  - Multiple records of the same type
  - You need to filter, sort, or search across them
  - You update individual records frequently
  - Examples: tasks, habits, logs, contacts, inventory

USE FILES WHEN:
  - The content is mostly narrative or prose
  - There are few instances (one project doc, one person profile)
  - You read the whole thing when you need it
  - The structure is loose or evolving

LITMUS TEST:
  \"Am I reading this whole file just to find one thing?\"
    -> It should probably be a collection.
  \"Am I reading this file because I need all of it?\"
    -> It's fine as a file.

DON'T USE LODGE FOR:
  - Config or settings (single document, read whole)
  - Fewer than ~3 records of a type
  - Free-form notes with no consistent structure"
    );
}

fn handle_collection_schema(
    conn: &Connection,
    coll: &Collection,
    sub_m: &ArgMatches,
    df: &str,
    settings: &Settings,
) -> error::Result<()> {
    let format = get_format(sub_m, df);
    let max_distinct = settings.distinct_max;
    let ratio = settings.distinct_ratio;
    let fields = fields_to_json(conn, &coll.name, &coll.fields, max_distinct, ratio);
    match format {
        output::Format::Json => {
            let out = serde_json::json!({
                "collection": coll.name,
                "fields": fields,
            });
            let json = serde_json::to_string_pretty(&out)
                .map_err(|e| LodgeError::Serialization(e.to_string()))?;
            println!("{json}");
        }
        _ => println!("{}", output::format_output(&fields, &format)?),
    }
    Ok(())
}
