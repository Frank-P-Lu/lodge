use crate::schema::Collection;
use clap::{Arg, Command};

/// Build the top-level CLI command with dynamic subcommands for each collection.
pub fn build_cli(collections: &[Collection], view_names: &[String]) -> Command {
    let view_about: &'static str = if view_names.is_empty() {
        Box::leak(
            "Create, list, run, or delete saved views"
                .to_string()
                .into_boxed_str(),
        )
    } else {
        Box::leak(
            format!(
                "Create, list, run, or delete saved views. Existing views: {}",
                view_names.join(", ")
            )
            .into_boxed_str(),
        )
    };

    let mut cmd = Command::new("lodge")
        .about("A local SQLite database with dynamically generated subcommands for AI agents")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("init").about("Initialize a new lodge database in the current directory"),
        )
        .subcommand(
            Command::new("create")
                .about("Create a new collection")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the collection"),
                )
                .arg(
                    Arg::new("fields")
                        .long("fields")
                        .required(true)
                        .help("Field definitions (e.g. \"title:text, priority:int\")"),
                )
                .arg(Arg::new("fts").long("fts").help(
                    "Enable full-text search on these fields (comma-separated, must be text type)",
                )),
        )
        .subcommand(
            Command::new("alter")
                .about("Add fields to an existing collection")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the collection"),
                )
                .arg(
                    Arg::new("add-fields")
                        .long("add-fields")
                        .help("New field definitions to add (e.g. \"status:text, due:date\")"),
                )
                .arg(Arg::new("enable-fts").long("enable-fts").help(
                    "Enable full-text search on these fields (comma-separated, must be text type)",
                )),
        )
        .subcommand(
            Command::new("sql")
                .about("Execute a raw SQL query")
                .arg(
                    Arg::new("query")
                        .required(true)
                        .help("SQL query to execute"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        )
        .subcommand(
            Command::new("view")
                .about(view_about)
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("create")
                        .about("Create a saved view")
                        .arg(Arg::new("name").required(true).help("Name of the view"))
                        .arg(
                            Arg::new("collection")
                                .long("collection")
                                .required(true)
                                .help("Collection to query"),
                        )
                        .arg(Arg::new("where").long("where").help("SQL WHERE clause"))
                        .arg(Arg::new("sort").long("sort").help("ORDER BY clause"))
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .help("Maximum number of records"),
                        ),
                )
                .subcommand(
                    Command::new("list").about("List all saved views").arg(
                        Arg::new("format")
                            .long("format")
                            .default_value("json")
                            .help("Output format: json, table, csv"),
                    ),
                )
                .subcommand(
                    Command::new("run")
                        .about("Run a saved view")
                        .arg(
                            Arg::new("name")
                                .required(true)
                                .help("Name of the view to run"),
                        )
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .default_value("json")
                                .help("Output format: json, table, csv"),
                        ),
                )
                .subcommand(
                    Command::new("delete").about("Delete a saved view").arg(
                        Arg::new("name")
                            .required(true)
                            .help("Name of the view to delete"),
                    ),
                ),
        )
        .subcommand(
            Command::new("export")
                .about("Export collection data")
                .arg(Arg::new("collection").help("Collection to export"))
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Export all collections"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, csv"),
                ),
        )
        .subcommand(
            Command::new("import")
                .about("Import data into a collection")
                .arg(Arg::new("collection").help("Collection to import into"))
                .arg(Arg::new("file").help("File to import from (positional, after collection)"))
                .arg(
                    Arg::new("import-file")
                        .long("file")
                        .help("Import a full export file (all collections)"),
                ),
        )
        .subcommand(
            Command::new("snapshot")
                .about("Create a snapshot of the entire database")
                .arg(
                    Arg::new("output")
                        .long("output")
                        .help("Custom output path for the snapshot file"),
                ),
        )
        .subcommand(
            Command::new("restore")
                .about("Restore from a snapshot file")
                .arg(
                    Arg::new("path")
                        .required(true)
                        .help("Path to the snapshot JSON file"),
                ),
        );

    // Add dynamic subcommands for each collection
    // We leak the strings so clap gets 'static &str references.
    // This is fine — the CLI runs once per invocation.
    for collection in collections {
        let coll_name: &'static str = Box::leak(collection.name.clone().into_boxed_str());
        let about_str: &'static str =
            Box::leak(format!("Manage '{}' records", collection.name).into_boxed_str());

        let mut coll_cmd = Command::new(coll_name)
            .about(about_str)
            .subcommand_required(true)
            .arg_required_else_help(true);

        // add subcommand
        let mut add_cmd = Command::new("add").about("Add a new record");
        for field in &collection.fields {
            let fname: &'static str = Box::leak(field.name.clone().into_boxed_str());
            let help: &'static str = Box::leak(
                format!("({}) {}", field.field_type.as_str(), field.name).into_boxed_str(),
            );
            add_cmd = add_cmd.arg(Arg::new(fname).long(fname).help(help));
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
            let help: &'static str =
                Box::leak(format!("({}) new value", field.field_type.as_str()).into_boxed_str());
            update_cmd = update_cmd.arg(Arg::new(fname).long(fname).help(help));
        }
        coll_cmd = coll_cmd.subcommand(update_cmd);

        // delete subcommand
        coll_cmd = coll_cmd.subcommand(
            Command::new("delete")
                .about("Delete a record by id")
                .arg(Arg::new("id").required(true).help("Record ID to delete")),
        );

        // search subcommand (FTS)
        coll_cmd = coll_cmd.subcommand(
            Command::new("search")
                .about("Full-text search records")
                .arg(Arg::new("query").required(true).help("Search query"))
                .arg(
                    Arg::new("limit")
                        .long("limit")
                        .help("Maximum number of results"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        );

        // streak subcommand (time-series)
        coll_cmd = coll_cmd.subcommand(
            Command::new("streak")
                .about("Compute consecutive-day streaks for a date field")
                .arg(
                    Arg::new("field")
                        .long("field")
                        .required(true)
                        .help("Date field to analyze"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        );

        // gaps subcommand (time-series)
        coll_cmd = coll_cmd.subcommand(
            Command::new("gaps")
                .about("Find gaps in date sequences exceeding a threshold")
                .arg(
                    Arg::new("field")
                        .long("field")
                        .required(true)
                        .help("Date field to analyze"),
                )
                .arg(
                    Arg::new("threshold")
                        .long("threshold")
                        .default_value("1")
                        .help("Minimum gap in days to report (default: 1)"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        );

        // rolling-avg subcommand (time-series)
        coll_cmd = coll_cmd.subcommand(
            Command::new("rolling-avg")
                .about("Compute rolling average of a numeric field over a date field")
                .arg(
                    Arg::new("field")
                        .long("field")
                        .required(true)
                        .help("Numeric field to average"),
                )
                .arg(
                    Arg::new("over")
                        .long("over")
                        .required(true)
                        .help("Date field to order by"),
                )
                .arg(
                    Arg::new("window")
                        .long("window")
                        .default_value("7")
                        .help("Window size in rows (default: 7)"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        );

        cmd = cmd.subcommand(coll_cmd);
    }

    cmd
}
