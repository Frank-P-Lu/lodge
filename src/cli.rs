use crate::schema::Collection;
use clap::{Arg, Command};

/// Build the top-level CLI command with dynamic subcommands for each collection.
pub fn build_cli(collections: &[Collection], view_names: &[String]) -> Command {
    let view_about: &'static str = if view_names.is_empty() {
        "Create, list, run, or delete saved views"
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
        .about("A local SQLite database with dynamically generated subcommands for AI agents. Run 'lodge guide' to learn when to use Lodge vs files.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("init").about("Initialize a new lodge database in the current directory"),
        )
        .subcommand(
            Command::new("guide").about("When to use Lodge vs files — a decision framework for agents"),
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
                ),
        )
        .subcommand(
            Command::new("alter")
                .about("Alter an existing collection: add, rename, or drop fields")
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
                .arg(
                    Arg::new("rename-field")
                        .long("rename-field")
                        .help("Rename a field (e.g. \"old_name:new_name\")"),
                )
                .arg(
                    Arg::new("drop-fields")
                        .long("drop-fields")
                        .help("Drop fields (comma-separated, e.g. \"field1,field2\")"),
                )
                .group(
                    clap::ArgGroup::new("alter_action")
                        .args(["add-fields", "rename-field", "drop-fields"])
                        .required(true)
                        .multiple(true),
                ),
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
                    Command::new("show")
                        .alias("schema")
                        .about("Show a single view's definition")
                        .arg(
                            Arg::new("name")
                                .required(true)
                                .help("Name of the view to show"),
                        ),
                )
                .subcommand(
                    Command::new("update")
                        .about("Update a saved view's filter, sort, or limit")
                        .arg(
                            Arg::new("name")
                                .required(true)
                                .help("Name of the view to update"),
                        )
                        .arg(Arg::new("where").long("where").help("New SQL WHERE clause"))
                        .arg(Arg::new("sort").long("sort").help("New ORDER BY clause"))
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .help("New maximum number of records"),
                        )
                        .group(
                            clap::ArgGroup::new("update_fields")
                                .args(["where", "sort", "limit"])
                                .required(true)
                                .multiple(true),
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
                        )
                        .arg(
                            Arg::new("meta")
                                .long("meta")
                                .action(clap::ArgAction::SetTrue)
                                .help("Wrap output with view and collection context"),
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
                .arg(
                    Arg::new("collection")
                        .help("Collection to export")
                        .required_unless_present("all")
                        .conflicts_with("all"),
                )
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
                .arg(
                    Arg::new("file")
                        .long("file")
                        .required(true)
                        .help("File to import from"),
                )
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Import all collections from a full export file"),
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
        )
        .subcommand(
            Command::new("log")
                .about("Show mutation log (adds, updates, deletes)")
                .arg(
                    Arg::new("collection")
                        .help("Filter by collection name"),
                )
                .arg(
                    Arg::new("limit")
                        .long("limit")
                        .default_value("20")
                        .help("Maximum number of log entries (default: 20)"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("List all collections and their schemas")
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        )
        .subcommand(
            Command::new("run")
                .about("Run a saved view by name")
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
                )
                .arg(
                    Arg::new("meta")
                        .long("meta")
                        .action(clap::ArgAction::SetTrue)
                        .help("Wrap output with view and collection context"),
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
            add_cmd = add_cmd.arg(Arg::new(fname).long(fname).help(help).allow_hyphen_values(true));
        }
        add_cmd = add_cmd.arg(
            Arg::new("format")
                .long("format")
                .default_value("json")
                .help("Output format: json, table, csv"),
        );
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
                    Arg::new("fields")
                        .long("fields")
                        .help("Comma-separated list of fields to return (e.g. \"id,title,status\")"),
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
            update_cmd = update_cmd.arg(Arg::new(fname).long(fname).help(help).allow_hyphen_values(true));

            let clear_name: &'static str =
                Box::leak(format!("clear-{}", field.name).into_boxed_str());
            let clear_help: &'static str =
                Box::leak(format!("Set {} to null", field.name).into_boxed_str());
            update_cmd = update_cmd.arg(
                Arg::new(clear_name)
                    .long(clear_name)
                    .help(clear_help)
                    .action(clap::ArgAction::SetTrue)
                    .conflicts_with(fname),
            );
        }
        update_cmd = update_cmd.arg(
            Arg::new("format")
                .long("format")
                .default_value("json")
                .help("Output format: json, table, csv"),
        );
        coll_cmd = coll_cmd.subcommand(update_cmd);

        // delete subcommand
        coll_cmd = coll_cmd.subcommand(
            Command::new("delete")
                .about("Delete a record by id")
                .arg(Arg::new("id").required(true).help("Record ID to delete"))
                .arg(
                    Arg::new("format")
                        .long("format")
                        .default_value("json")
                        .help("Output format: json, table, csv"),
                ),
        );

        // search subcommand (FTS)
        coll_cmd = coll_cmd.subcommand(
            Command::new("search")
                .about("Full-text search records (results ordered by relevance; --where/--sort not supported — use query for filtered/sorted access)")
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

        // schema subcommand
        coll_cmd = coll_cmd.subcommand(
            Command::new("schema")
                .about("Show field definitions for this collection")
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
