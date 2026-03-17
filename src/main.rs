mod cli;
mod collection;
mod db;
mod error;
mod export;
mod fts;
mod import;
mod output;
mod record;
mod schema;
mod snapshot;
mod timeseries;
mod types;
mod view;

use clap::ArgMatches;
use error::LodgeError;
use output::Format;
use rusqlite::Connection;
use schema::Collection;
use std::path::Path;
use std::process;

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

/// Get the --format flag and parse it, defaulting to JSON.
fn get_format(m: &ArgMatches) -> Format {
    m.get_one::<String>("format")
        .and_then(|s| Format::from_str(s))
        .unwrap_or(Format::Json)
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

    // Try to load collections and view names from existing DB (for dynamic subcommands)
    let (collections, view_names) = if let Ok(conn) = db::open(&cwd) {
        let colls = schema::load_collections(&conn)?;
        let views = view::load_view_names(&conn)?;
        (colls, views)
    } else {
        (Vec::new(), Vec::new())
    };

    let cmd = cli::build_cli(&collections, &view_names);
    let matches = cmd.get_matches();

    match matches.subcommand() {
        Some(("init", _)) => {
            db::init(&cwd)?;
            println!("Initialized lodge database in .lodge/");
            Ok(())
        }
        Some(("create", sub_m)) => handle_create(&cwd, sub_m),
        Some(("alter", sub_m)) => handle_alter(&cwd, sub_m),
        Some(("sql", sub_m)) => handle_sql(&cwd, sub_m),
        Some(("view", sub_m)) => handle_view(&cwd, sub_m),
        Some(("export", sub_m)) => handle_export(&cwd, sub_m),
        Some(("snapshot", sub_m)) => handle_snapshot(&cwd, sub_m),
        Some(("restore", sub_m)) => handle_restore(&cwd, sub_m),
        Some(("import", sub_m)) => handle_import(&cwd, sub_m),
        Some(("list", sub_m)) => handle_list(&cwd, sub_m),
        Some(("run", sub_m)) => handle_run_view(&cwd, sub_m),
        Some((collection_name, sub_m)) => handle_collection(&cwd, collection_name, sub_m),
        _ => unreachable!(),
    }
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

fn handle_sql(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let collections_now = schema::load_collections(&conn).unwrap_or_default();
    let query = get_arg(sub_m, "query")?;
    let format = get_format(sub_m);
    let results = record::execute_sql(&conn, query, &collections_now)?;
    println!("{}", output::format_output(&results, &format));
    Ok(())
}

fn handle_view(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    match sub_m.subcommand() {
        Some(("create", create_m)) => {
            let name = get_arg(create_m, "name")?;
            let collection = get_arg(create_m, "collection")?;
            let where_clause = create_m.get_one::<String>("where").map(|s| s.as_str());
            let sort = create_m.get_one::<String>("sort").map(|s| s.as_str());
            let limit = create_m
                .get_one::<String>("limit")
                .and_then(|s| s.parse::<i64>().ok());
            view::create_view(&conn, name, collection, where_clause, sort, limit)?;
            println!("Created view '{name}'");
            Ok(())
        }
        Some(("list", list_m)) => {
            let format = get_format(list_m);
            let views = view::list_views(&conn)?;
            println!("{}", output::format_output(&views, &format));
            Ok(())
        }
        Some(("show", show_m)) => {
            let name = get_arg(show_m, "name")?;
            let v = view::show_view(&conn, name)?;
            println!("{v}");
            Ok(())
        }
        Some(("update", update_m)) => {
            let name = get_arg(update_m, "name")?;
            let where_clause = update_m.get_one::<String>("where").map(|s| s.as_str());
            let sort = update_m.get_one::<String>("sort").map(|s| s.as_str());
            let limit = update_m
                .get_one::<String>("limit")
                .and_then(|s| s.parse::<i64>().ok());
            view::update_view(&conn, name, where_clause, sort, limit)?;
            println!("Updated view '{name}'");
            Ok(())
        }
        Some(("run", run_m)) => {
            run_view_inner(&conn, run_m)
        }
        Some(("delete", delete_m)) => {
            let name = get_arg(delete_m, "name")?;
            view::delete_view(&conn, name)?;
            println!("Deleted view '{name}'");
            Ok(())
        }
        _ => unreachable!(),
    }
}

fn handle_export(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    if sub_m.get_flag("all") {
        let result = export::export_all(&conn)?;
        println!("{result}");
    } else {
        let name = get_arg(sub_m, "collection")?;
        let format = get_format(sub_m);
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
            LodgeError::MissingArgument(
                "specify a collection name or use --all".to_string(),
            )
        })?;
        let count = import::import_collection(&conn, name, &data)?;
        println!("Imported {count} records into '{name}'");
    }
    Ok(())
}

fn handle_list(cwd: &Path, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let format = get_format(sub_m);
    let colls = schema::load_collections(&conn)?;
    let result: Vec<serde_json::Value> = colls
        .iter()
        .map(|c| {
            let fields: Vec<serde_json::Value> = c
                .fields
                .iter()
                .map(|f| serde_json::json!({"name": f.name, "type": f.field_type.as_str()}))
                .collect();
            serde_json::json!({"name": c.name, "fields": fields})
        })
        .collect();
    println!("{}", output::format_output(&result, &format));
    Ok(())
}

fn handle_run_view(cwd: &Path, run_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    run_view_inner(&conn, run_m)
}

/// Shared logic for `lodge run <view>` and `lodge view run <view>`.
fn run_view_inner(conn: &Connection, run_m: &ArgMatches) -> error::Result<()> {
    let name = get_arg(run_m, "name")?;
    let format = get_format(run_m);
    let (collection_name, records) = view::run_view(conn, name)?;
    if run_m.get_flag("meta") {
        let wrapped = serde_json::json!({
            "view": name,
            "collection": collection_name,
            "records": records,
        });
        println!("{wrapped}");
    } else {
        println!("{}", output::format_output(&records, &format));
    }
    Ok(())
}

fn handle_collection(cwd: &Path, collection_name: &str, sub_m: &ArgMatches) -> error::Result<()> {
    let conn = db::open(cwd)?;
    let coll = schema::load_collection(&conn, collection_name)?
        .ok_or_else(|| LodgeError::CollectionNotFound(collection_name.to_string()))?;

    match sub_m.subcommand() {
        Some(("add", add_m)) => handle_collection_add(&conn, &coll, collection_name, add_m),
        Some(("query", query_m)) => handle_collection_query(&conn, &coll, query_m),
        Some(("update", update_m)) => handle_collection_update(&conn, &coll, collection_name, update_m),
        Some(("delete", delete_m)) => handle_collection_delete(&conn, &coll, collection_name, delete_m),
        Some(("search", search_m)) => handle_collection_search(&conn, &coll, search_m),
        Some(("streak", streak_m)) => handle_collection_streak(&conn, &coll, streak_m),
        Some(("gaps", gaps_m)) => handle_collection_gaps(&conn, &coll, gaps_m),
        Some(("rolling-avg", avg_m)) => handle_collection_rolling_avg(&conn, &coll, avg_m),
        Some(("schema", schema_m)) => handle_collection_schema(&coll, schema_m),
        _ => unreachable!(),
    }
}

fn handle_collection_add(
    conn: &Connection,
    coll: &Collection,
    collection_name: &str,
    add_m: &ArgMatches,
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
    let format = get_format(add_m);
    let result = record::add_record(conn, coll, &values)?;
    println!("Added record {} to '{}'", result["id"], collection_name);
    println!("{}", output::format_single(&result, &format));
    Ok(())
}

fn handle_collection_query(
    conn: &Connection,
    coll: &Collection,
    query_m: &ArgMatches,
) -> error::Result<()> {
    let where_clause = query_m.get_one::<String>("where").map(|s| s.as_str());
    let sort = query_m.get_one::<String>("sort").map(|s| s.as_str());
    let limit = query_m
        .get_one::<String>("limit")
        .and_then(|s| s.parse::<i64>().ok());
    let format = get_format(query_m);
    let results = record::query_records(conn, coll, where_clause, sort, limit)?;
    println!("{}", output::format_output(&results, &format));
    Ok(())
}

fn handle_collection_update(
    conn: &Connection,
    coll: &Collection,
    collection_name: &str,
    update_m: &ArgMatches,
) -> error::Result<()> {
    let id = get_id(update_m)?;
    let mut values = Vec::new();
    for field in &coll.fields {
        if let Some(val) = update_m.get_one::<String>(&field.name) {
            values.push((field.name.clone(), val.clone()));
        }
    }
    if values.is_empty() {
        return Err(LodgeError::MissingArgument(
            "no fields to update".to_string(),
        ));
    }
    let format = get_format(update_m);
    let result = record::update_record(conn, coll, id, &values)?;
    println!("Updated record {id} in '{collection_name}'");
    println!("{}", output::format_single(&result, &format));
    Ok(())
}

fn handle_collection_delete(
    conn: &Connection,
    coll: &Collection,
    collection_name: &str,
    delete_m: &ArgMatches,
) -> error::Result<()> {
    let id = get_id(delete_m)?;
    let format = get_format(delete_m);
    let result = record::delete_record(conn, coll, id)?;
    println!("Deleted record {id} from '{collection_name}'");
    println!("{}", output::format_single(&result, &format));
    Ok(())
}

fn handle_collection_search(
    conn: &Connection,
    coll: &Collection,
    search_m: &ArgMatches,
) -> error::Result<()> {
    let query = get_arg(search_m, "query")?;
    let limit = search_m
        .get_one::<String>("limit")
        .and_then(|s| s.parse::<i64>().ok());
    let format = get_format(search_m);
    let results = fts::search_records(conn, coll, query, limit)?;
    println!("{}", output::format_output(&results, &format));
    Ok(())
}

fn handle_collection_streak(
    conn: &Connection,
    coll: &Collection,
    streak_m: &ArgMatches,
) -> error::Result<()> {
    let field = get_arg(streak_m, "field")?;
    let format = get_format(streak_m);
    let result = timeseries::streak(conn, coll, field)?;
    println!("{}", output::format_single(&result, &format));
    Ok(())
}

fn handle_collection_gaps(
    conn: &Connection,
    coll: &Collection,
    gaps_m: &ArgMatches,
) -> error::Result<()> {
    let field = get_arg(gaps_m, "field")?;
    let raw_threshold = get_arg(gaps_m, "threshold")?;
    let threshold: i64 = raw_threshold.parse().map_err(|_| LodgeError::InvalidValue {
        field: "threshold".to_string(),
        field_type: "int".to_string(),
        value: raw_threshold.clone(),
    })?;
    let format = get_format(gaps_m);
    let results = timeseries::gaps(conn, coll, field, threshold)?;
    println!("{}", output::format_output(&results, &format));
    Ok(())
}

fn handle_collection_rolling_avg(
    conn: &Connection,
    coll: &Collection,
    avg_m: &ArgMatches,
) -> error::Result<()> {
    let field = get_arg(avg_m, "field")?;
    let over = get_arg(avg_m, "over")?;
    let raw_window = get_arg(avg_m, "window")?;
    let window: i64 = raw_window.parse().map_err(|_| LodgeError::InvalidValue {
        field: "window".to_string(),
        field_type: "int".to_string(),
        value: raw_window.clone(),
    })?;
    let format = get_format(avg_m);
    let results = timeseries::rolling_average(conn, coll, field, over, window)?;
    println!("{}", output::format_output(&results, &format));
    Ok(())
}

fn handle_collection_schema(coll: &Collection, sub_m: &ArgMatches) -> error::Result<()> {
    let fmt_str = sub_m
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("json");
    let format = output::Format::from_str(fmt_str)
        .ok_or_else(|| LodgeError::Sql(format!("Unknown format: {fmt_str}")))?;
    let fields: Vec<serde_json::Value> = coll
        .fields
        .iter()
        .map(|f| serde_json::json!({"name": f.name, "type": f.field_type.as_str()}))
        .collect();
    match format {
        output::Format::Json => {
            let out = serde_json::json!({
                "collection": coll.name,
                "fields": fields,
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
        }
        _ => println!("{}", output::format_output(&fields, &format)),
    }
    Ok(())
}
