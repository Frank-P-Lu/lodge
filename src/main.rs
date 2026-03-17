mod cli;
mod collection;
mod db;
mod error;
mod export;
mod import;
mod output;
mod record;
mod schema;
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
            let add_fields = sub_m.get_one::<String>("add-fields").unwrap();
            collection::alter_collection(&conn, name, add_fields)?;
            let coll = schema::load_collection(&conn, name)?.unwrap();
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
                Some(("run", run_m)) => {
                    let name = run_m.get_one::<String>("name").unwrap();
                    let format_str = run_m.get_one::<String>("format").unwrap();
                    let format = Format::from_str(format_str).unwrap_or(Format::Json);
                    let result = view::run_view(&conn, name, &format)?;
                    println!("{result}");
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
                let name = sub_m.get_one::<String>("collection").ok_or_else(|| {
                    LodgeError::InvalidFieldsFormat(
                        "specify a collection name or use --all".to_string(),
                    )
                })?;
                let format_str = sub_m.get_one::<String>("format").unwrap();
                let format = Format::from_str(format_str).unwrap_or(Format::Json);
                let result = export::export_collection(&conn, name, &format)?;
                println!("{result}");
            }
            Ok(())
        }
        Some(("import", sub_m)) => {
            let conn = db::open(&cwd)?;
            if let Some(file_path) = sub_m.get_one::<String>("import-file") {
                let data = std::fs::read_to_string(file_path)?;
                let results = import::import_full(&conn, &data)?;
                for (name, count) in &results {
                    println!("Imported {count} records into '{name}'");
                }
                Ok(())
            } else {
                let name = sub_m.get_one::<String>("collection").ok_or_else(|| {
                    LodgeError::InvalidFieldsFormat(
                        "specify a collection name or use --file".to_string(),
                    )
                })?;
                let file_path = sub_m.get_one::<String>("file").ok_or_else(|| {
                    LodgeError::InvalidFieldsFormat("specify a file to import".to_string())
                })?;
                let data = std::fs::read_to_string(file_path)?;
                let count = import::import_collection(&conn, name, &data)?;
                println!("Imported {count} records into '{name}'");
                Ok(())
            }
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
                    let result = record::add_record(&conn, &coll, &values)?;
                    println!("{}", output::format_single(&result, &Format::Json));
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
                        return Err(LodgeError::InvalidFieldsFormat(
                            "no fields to update".to_string(),
                        ));
                    }
                    let result = record::update_record(&conn, &coll, id, &values)?;
                    println!("{}", output::format_single(&result, &Format::Json));
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
                    let result = record::delete_record(&conn, &coll, id)?;
                    println!("{}", output::format_single(&result, &Format::Json));
                    Ok(())
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}
