use crate::schema::Collection;
use clap::{Arg, Command};

/// Build the top-level CLI command with dynamic subcommands for each collection.
pub fn build_cli(collections: &[Collection]) -> Command {
    let mut cmd = Command::new("lodge")
        .about("A local SQLite database with dynamically generated subcommands for AI agents")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("init").about("Initialize a new lodge database in the current directory"))
        .subcommand(
            Command::new("create")
                .about("Create a new collection")
                .arg(Arg::new("name").required(true).help("Name of the collection"))
                .arg(
                    Arg::new("fields")
                        .long("fields")
                        .required(true)
                        .help("Field definitions (e.g. \"title:text, priority:int\")"),
                ),
        )
        .subcommand(
            Command::new("alter")
                .about("Add fields to an existing collection")
                .arg(Arg::new("name").required(true).help("Name of the collection"))
                .arg(
                    Arg::new("add-fields")
                        .long("add-fields")
                        .required(true)
                        .help("New field definitions to add (e.g. \"status:text, due:date\")"),
                ),
        )
        .subcommand(
            Command::new("sql")
                .about("Execute a raw SQL query")
                .arg(Arg::new("query").required(true).help("SQL query to execute"))
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        );

    // Add dynamic subcommands for each collection
    // We leak the strings so clap gets 'static &str references.
    // This is fine — the CLI runs once per invocation.
    for collection in collections {
        let coll_name: &'static str = Box::leak(collection.name.clone().into_boxed_str());
        let about_str: &'static str = Box::leak(format!("Manage '{}' records", collection.name).into_boxed_str());

        let mut coll_cmd = Command::new(coll_name)
            .about(about_str)
            .subcommand_required(true)
            .arg_required_else_help(true);

        // add subcommand
        let mut add_cmd = Command::new("add").about("Add a new record");
        for field in &collection.fields {
            let fname: &'static str = Box::leak(field.name.clone().into_boxed_str());
            let help: &'static str = Box::leak(format!("({}) {}", field.field_type.as_str(), field.name).into_boxed_str());
            add_cmd = add_cmd.arg(
                Arg::new(fname)
                    .long(fname)
                    .help(help),
            );
        }
        coll_cmd = coll_cmd.subcommand(add_cmd);

        // query subcommand
        coll_cmd = coll_cmd.subcommand(
            Command::new("query")
                .about("Query records")
                .arg(Arg::new("where").long("where").help("SQL WHERE clause"))
                .arg(Arg::new("sort").long("sort").help("ORDER BY clause"))
                .arg(
                    Arg::new("limit")
                        .long("limit")
                        .help("Maximum number of records"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        );

        // update subcommand
        let mut update_cmd = Command::new("update")
            .about("Update a record by id")
            .arg(Arg::new("id").required(true).help("Record ID to update"));
        for field in &collection.fields {
            let fname: &'static str = Box::leak(field.name.clone().into_boxed_str());
            let help: &'static str = Box::leak(format!("({}) new value", field.field_type.as_str()).into_boxed_str());
            update_cmd = update_cmd.arg(
                Arg::new(fname)
                    .long(fname)
                    .help(help),
            );
        }
        coll_cmd = coll_cmd.subcommand(update_cmd);

        // delete subcommand
        coll_cmd = coll_cmd.subcommand(
            Command::new("delete")
                .about("Delete a record by id")
                .arg(Arg::new("id").required(true).help("Record ID to delete")),
        );

        cmd = cmd.subcommand(coll_cmd);
    }

    cmd
}
