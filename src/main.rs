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

use error::LodgeError;
use output::Format;
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run() -> error::Result<()> {
    let cwd = std::env::current_dir()?;

    // Try to load collections and view names from existing DB (for dynamic subcommands)
    let (collections, view_names) = if let Ok(conn) = db::open(&cwd) {
        let colls = schema::load_collections(&conn).unwrap_or_default();
        let views = view::load_view_names(&conn).unwrap_or_default();
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
        Some(("create", sub_m)) => {
            let conn = db::open(&cwd)?;
            let name = sub_m.get_one::<String>("name").unwrap();
            let fields = sub_m.get_one::<String>("fields").unwrap();
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
        Some(("alter", sub_m)) => {
            let conn = db::open(&cwd)?;
            let name = sub_m.get_one::<String>("name").unwrap();
            let fields_spec = sub_m.get_one::<String>("add-fields").unwrap();
            let coll_before = schema::load_collection(&conn, name)?
                .ok_or_else(|| LodgeError::CollectionNotFound(name.to_string()))?;
            let prev_text_count = coll_before
                .fields
                .iter()
                .filter(|f| f.field_type == types::FieldType::Text)
                .count();
            collection::alter_collection(&conn, name, fields_spec)?;
            let coll = schema::load_collection(&conn, name)?.unwrap();
            let all_text_fields: Vec<String> = coll
                .fields
                .iter()
                .filter(|f| f.field_type == types::FieldType::Text)
                .map(|f| f.name.clone())
                .collect();
            // If new text fields were added, rebuild FTS index
            if all_text_fields.len() > prev_text_count {
                if fts::has_fts(&conn, name)? {
                    fts::drop_fts_table(&conn, name)?;
                }
                fts::create_fts_table(&conn, name, &all_text_fields)?;
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
        Some(("sql", sub_m)) => {
            let conn = db::open(&cwd)?;
            let query = sub_m.get_one::<String>("query").unwrap();
            let format_str = sub_m.get_one::<String>("format").unwrap();
            let format = Format::from_str(format_str).unwrap_or(Format::Json);
            let results = record::execute_sql(&conn, query)?;
            println!("{}", output::format_output(&results, &format));
            Ok(())
        }
        Some(("view", sub_m)) => {
            let conn = db::open(&cwd)?;
            match sub_m.subcommand() {
                Some(("create", create_m)) => {
                    let name = create_m.get_one::<String>("name").unwrap();
                    let collection = create_m.get_one::<String>("collection").unwrap();
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
                    let format_str = list_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let views = view::list_views(&conn)?;
                    println!("{}", output::format_output(&views, &format));
                    Ok(())
                }
                Some(("show", show_m)) => {
                    let name = show_m.get_one::<String>("name").unwrap();
                    let v = view::show_view(&conn, name)?;
                    println!("{v}");
                    Ok(())
                }
                Some(("update", update_m)) => {
                    let name = update_m.get_one::<String>("name").unwrap();
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
                    let name = run_m.get_one::<String>("name").unwrap();
                    let format_str = run_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let (collection_name, records) = view::run_view(&conn, name)?;
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
                Some(("delete", delete_m)) => {
                    let name = delete_m.get_one::<String>("name").unwrap();
                    view::delete_view(&conn, name)?;
                    println!("Deleted view '{name}'");
                    Ok(())
                }
                _ => unreachable!(),
            }
        }
        Some(("export", sub_m)) => {
            let conn = db::open(&cwd)?;
            if sub_m.get_flag("all") {
                let result = export::export_all(&conn)?;
                println!("{result}");
            } else {
                let name = sub_m.get_one::<String>("collection").unwrap();
                let format_str = sub_m.get_one::<String>("format").unwrap();
                let format = Format::from_str(format_str).unwrap_or(Format::Json);
                let result = export::export_collection(&conn, name, &format)?;
                println!("{result}");
            }
            Ok(())
        }
        Some(("snapshot", sub_m)) => {
            let conn = db::open(&cwd)?;
            let lodge_dir = db::lodge_dir(&cwd)?;
            let output_path = sub_m.get_one::<String>("output").map(|s| s.as_str());
            let path = snapshot::create_snapshot(&conn, &lodge_dir, output_path)?;
            println!("Snapshot saved to {}", path.display());
            Ok(())
        }
        Some(("restore", sub_m)) => {
            let conn = db::open(&cwd)?;
            let path = sub_m.get_one::<String>("path").unwrap();
            snapshot::restore_snapshot(&conn, path)?;
            println!("Restored from {path}");
            Ok(())
        }
        Some(("import", sub_m)) => {
            let conn = db::open(&cwd)?;
            let file_path = sub_m.get_one::<String>("file").unwrap();
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
        Some(("run", run_m)) => {
            let conn = db::open(&cwd)?;
            let name = run_m.get_one::<String>("name").unwrap();
            let format_str = run_m.get_one::<String>("format").unwrap();
            let format = Format::from_str(format_str).unwrap_or(Format::Json);
            let (collection_name, records) = view::run_view(&conn, name)?;
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
        Some((collection_name, sub_m)) => {
            let conn = db::open(&cwd)?;
            let coll = schema::load_collection(&conn, collection_name)?
                .ok_or_else(|| LodgeError::CollectionNotFound(collection_name.to_string()))?;

            match sub_m.subcommand() {
                Some(("add", add_m)) => {
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
                    let format_str = add_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let result = record::add_record(&conn, &coll, &values)?;
                    println!("Added record {} to '{}'", result["id"], collection_name);
                    println!("{}", output::format_single(&result, &format));
                    Ok(())
                }
                Some(("query", query_m)) => {
                    let where_clause = query_m.get_one::<String>("where").map(|s| s.as_str());
                    let sort = query_m.get_one::<String>("sort").map(|s| s.as_str());
                    let limit = query_m
                        .get_one::<String>("limit")
                        .and_then(|s| s.parse::<i64>().ok());
                    let format_str = query_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let results = record::query_records(&conn, &coll, where_clause, sort, limit)?;
                    println!("{}", output::format_output(&results, &format));
                    Ok(())
                }
                Some(("update", update_m)) => {
                    let id: i64 =
                        update_m
                            .get_one::<String>("id")
                            .unwrap()
                            .parse()
                            .map_err(|_| LodgeError::InvalidValue {
                                field: "id".to_string(),
                                field_type: "int".to_string(),
                                value: update_m.get_one::<String>("id").unwrap().clone(),
                            })?;
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
                    let format_str = update_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let result = record::update_record(&conn, &coll, id, &values)?;
                    println!("Updated record {id} in '{collection_name}'");
                    println!("{}", output::format_single(&result, &format));
                    Ok(())
                }
                Some(("delete", delete_m)) => {
                    let id: i64 =
                        delete_m
                            .get_one::<String>("id")
                            .unwrap()
                            .parse()
                            .map_err(|_| LodgeError::InvalidValue {
                                field: "id".to_string(),
                                field_type: "int".to_string(),
                                value: delete_m.get_one::<String>("id").unwrap().clone(),
                            })?;
                    let format_str = delete_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let result = record::delete_record(&conn, &coll, id)?;
                    println!("Deleted record {id} from '{collection_name}'");
                    println!("{}", output::format_single(&result, &format));
                    Ok(())
                }
                Some(("search", search_m)) => {
                    let query = search_m.get_one::<String>("query").unwrap();
                    let limit = search_m
                        .get_one::<String>("limit")
                        .and_then(|s| s.parse::<i64>().ok());
                    let format_str = search_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let results = fts::search_records(&conn, &coll, query, limit)?;
                    println!("{}", output::format_output(&results, &format));
                    Ok(())
                }
                Some(("streak", streak_m)) => {
                    let field = streak_m.get_one::<String>("field").unwrap();
                    let format_str = streak_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let result = timeseries::streak(&conn, &coll, field)?;
                    println!("{}", output::format_single(&result, &format));
                    Ok(())
                }
                Some(("gaps", gaps_m)) => {
                    let field = gaps_m.get_one::<String>("field").unwrap();
                    let threshold: i64 = gaps_m
                        .get_one::<String>("threshold")
                        .unwrap()
                        .parse()
                        .map_err(|_| LodgeError::InvalidValue {
                            field: "threshold".to_string(),
                            field_type: "int".to_string(),
                            value: gaps_m.get_one::<String>("threshold").unwrap().clone(),
                        })?;
                    let format_str = gaps_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let results = timeseries::gaps(&conn, &coll, field, threshold)?;
                    println!("{}", output::format_output(&results, &format));
                    Ok(())
                }
                Some(("rolling-avg", avg_m)) => {
                    let field = avg_m.get_one::<String>("field").unwrap();
                    let over = avg_m.get_one::<String>("over").unwrap();
                    let window: i64 =
                        avg_m
                            .get_one::<String>("window")
                            .unwrap()
                            .parse()
                            .map_err(|_| LodgeError::InvalidValue {
                                field: "window".to_string(),
                                field_type: "int".to_string(),
                                value: avg_m.get_one::<String>("window").unwrap().clone(),
                            })?;
                    let format_str = avg_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let results = timeseries::rolling_average(&conn, &coll, field, over, window)?;
                    println!("{}", output::format_output(&results, &format));
                    Ok(())
                }
                Some(("schema", _)) => {
                    let fields: Vec<serde_json::Value> = coll
                        .fields
                        .iter()
                        .map(|f| serde_json::json!({"name": f.name, "type": f.field_type.as_str()}))
                        .collect();
                    let out = serde_json::json!({
                        "collection": coll.name,
                        "fields": fields,
                    });
                    println!("{out}");
                    Ok(())
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}
