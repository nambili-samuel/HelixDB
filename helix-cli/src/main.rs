use args::{CliError, HelixCLI};
use clap::Parser;
use helixdb::{
    helix_engine::graph_core::config::Config,
    helixc::{
        generator::generator::CodeGenerator,
        parser::helix_parser::{HelixParser, Source},
    },
    ingestion_engine::sqlite::SqliteIngestor,
};
use indicatif::{ProgressBar, ProgressStyle};
use instance_manager::InstanceManager;
use std::path::PathBuf;
use std::{
    collections::HashMap,
    fs::{self, DirEntry},
    io::{ErrorKind, Write},
    net::{SocketAddr, TcpListener},
    path::Path,
    process::{Command, Stdio},
    thread::sleep,
    time::Duration,
};
use tempfile::{NamedTempFile, TempDir};

pub mod args;
mod instance_manager;
const QUERIES_DIR: &str = "helixdb-queries/";

fn check_helix_installation() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or("Could not determine home directory")?;
    let repo_path = home_dir.join(".helix/repo/helix-db");
    let container_path = repo_path.join("helix-container");
    let cargo_path = container_path.join("Cargo.toml");

    // Check if main repo directory exists and is a directory
    if !repo_path.exists() || !repo_path.is_dir() {
        return Err("Helix repo is not installed. Please run `helix install` first.".to_string());
    }

    // Check if helix-container exists and is a directory
    if !container_path.exists() || !container_path.is_dir() {
        return Err(
            "Helix container is missing. Please run `helix install` to repair the installation."
                .to_string(),
        );
    }

    // Check if Cargo.toml exists in helix-container
    if !cargo_path.exists() {
        return Err("Helix container's Cargo.toml is missing. Please run `helix install` to repair the installation.".to_string());
    }

    Ok(container_path)
}

fn find_available_port(start_port: u16) -> Option<u16> {
    let mut port = start_port;
    while port < 65535 {
        let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>().unwrap();
        match TcpListener::bind(addr) {
            Ok(listener) => {
                drop(listener);
                let localhost = format!("127.0.0.1:{}", port).parse::<SocketAddr>().unwrap();
                match TcpListener::bind(localhost) {
                    Ok(local_listener) => {
                        drop(local_listener);
                        return Some(port);
                    }
                    Err(e) => {
                        //println!("Error binding to {}: {:?}", addr, e);
                        if e.kind() != ErrorKind::AddrInUse {
                            return None;
                        }
                        port += 1;
                        continue;
                    }
                }
            }
            Err(e) => {
                if e.kind() != ErrorKind::AddrInUse {
                    return None;
                }
                port += 1;
                continue;
            }
        }
    }
    None
}

fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{prefix:>12.cyan.bold} {spinner:.green} {wide_msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.set_prefix("🔄");
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner
}

fn finish_spinner_with_message(spinner: &ProgressBar, success: bool, message: &str) {
    let prefix = if success { "✅" } else { "❌" };
    spinner.set_prefix(prefix);
    spinner.finish_with_message(message.to_string());
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_is_uppercase = false;

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !prev_is_uppercase {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
            prev_is_uppercase = true;
        } else {
            result.push(c);
            prev_is_uppercase = false;
        }
    }
    result
}

fn update_cli(spinner: &ProgressBar) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("curl")
        .args(&["-sSL", "https://install.helix-db.com"])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| {
            finish_spinner_with_message(&spinner, false, "Failed to start curl");
            e
        })?
        .stdout
        .ok_or_else(|| {
            finish_spinner_with_message(&spinner, false, "Failed to capture curl output");
            "Failed to capture curl output"
        })?;

    let status = Command::new("bash").stdin(status).status().map_err(|e| {
        finish_spinner_with_message(&spinner, false, "Failed to execute install script");
        e
    })?;

    if status.success() {
        finish_spinner_with_message(&spinner, true, "Successfully updated Helix CLI");
        Ok(())
    } else {
        finish_spinner_with_message(&spinner, false, "Update script failed");
        Err(format!("Exit code: {}", status).into())
    }
}

fn main() {
    let args = HelixCLI::parse();
    match args.command {
        args::CommandType::Update(_) => {
            unimplemented!();
            return;
            let spinner = create_spinner("Updating Helix CLI");
            if let Err(e) = update_cli(&spinner) {
                println!("\t└── {}", e);
            }
        }
        args::CommandType::Deploy(command) => {
            // check if cargo is installed
            let mut runner = Command::new("cargo");
            match runner.output() {
                Ok(_) => {}
                Err(e) => {
                    println!("\t❌ Cargo is not installed");
                    println!("\t|");
                    println!("\t└── {}", e);
                    return;
                }
            }

            // Check helix installation
            let container_path = match check_helix_installation() {
                Ok(path) => path,
                Err(e) => {
                    println!("\t❌ Helix is not installed. Please run `helix install` first.");
                    return;
                }
            };

            // path to project
            let path = match command.path {
                Some(path) => path,
                None => QUERIES_DIR.to_string(),
            };

            // output path
            let output = match command.output {
                Some(output) => output,
                None => dirs::home_dir()
                    .map(|path| {
                        path.join(".helix/repo/helix-db/helix-container")
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|| "./.helix/repo/helix-db/helix-container".to_string()),
            };

            let start_port = match command.port {
                Some(port) => port,
                None => 6969,
            };

            let port = match find_available_port(start_port) {
                Some(port) => {
                    if port != start_port {
                        println!(
                            "\t⚠️  Port {} is in use, using port {} instead",
                            start_port, port
                        );
                    }
                    port
                }
                None => {
                    println!("\t❌ No available ports found starting from {}", start_port);
                    return;
                }
            };

            // local flag
            let local = command.local;

            // TODO: remove this once remote instance is supported
            if !local {
                println!("Building for remote instance is not supported yet, use --local flag to build for local machine");
                println!("Example: helix deploy --local");
                println!();
                println!("Building for local machine will be available within the next 2 weeks");
                return;
            }

            // check and read files
            let files = match check_and_read_files(&path) {
                Ok(files) => files,
                Err(e) => {
                    println!("{}", e);
                    return;
                }
            };
            if files.is_empty() {
                println!("No queries found, nothing to compile");
                return;
            }

            // create progress spinner
            let spinner = create_spinner("Compiling Helix queries");
            // spinner.set_style(
            //     ProgressStyle::default_spinner()
            //         .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈")
            //         .template("{spinner:.green.bold} {msg}")
            //         .unwrap(),
            // );
            // number of files
            let numb_of_files = files.len();
            let mut successes = HashMap::new();
            let mut errors = HashMap::new();
            let mut code = String::new();
            let mut generator = CodeGenerator::new();
            code.push_str(&generator.generate_headers());
            for file in files {
                let contents = match fs::read_to_string(file.path()) {
                    Ok(contents) => contents,
                    Err(e) => {
                        spinner.finish_with_message("❌ Failed to read files");
                        println!("{}", e);
                        return;
                    }
                };
                match HelixParser::parse_source(&contents) {
                    Ok(source) => {
                        code.push_str(&generator.generate_source(&source));
                        successes.insert(file.file_name().to_string_lossy().into_owned(), source);
                    }
                    Err(e) => {
                        errors.insert(file.file_name().to_string_lossy().into_owned(), e);
                    }
                }
            }

            if !errors.is_empty() {
                finish_spinner_with_message(&spinner, false, "Failed to compile some queries");
                for (name, error) in errors {
                    println!("\t❌ {}: {}", name, error);
                }
                return;
            }

            finish_spinner_with_message(
                &spinner,
                true,
                &format!("Successfully compiled {} queries", numb_of_files),
            );

            let cache_dir = PathBuf::from(&output);
            fs::create_dir_all(&cache_dir).unwrap();

            // if local overwrite queries file in ~/.helix/repo/helix-container/src/queries.rs
            if local {
                let spinner = create_spinner("Building Helix");
                // spinner.set_style(
                //     ProgressStyle::default_spinner()
                //         .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈")
                //         .template("{spinner:.green.bold} {msg}")
                //         .unwrap(),
                // );
                let file_path = PathBuf::from(&output).join("src/queries.rs");
                match fs::write(file_path, code) {
                    Ok(_) => {
                        finish_spinner_with_message(
                            &spinner,
                            true,
                            "Successfully wrote queries file",
                        );
                    }
                    Err(e) => {
                        finish_spinner_with_message(
                            &spinner,
                            false,
                            "Failed to write queries file",
                        );
                        println!("\t└── {}", e);
                        return;
                    }
                }

                // copy config.hx.json to ~/.helix/repo/helix-db/helix-container/config.hx.json
                let config_path = PathBuf::from(&output).join("src/config.hx.json");
                fs::copy(PathBuf::from(path + "config.hx.json"), config_path).unwrap();

                // check rust code
                let mut runner = Command::new("cargo");
                runner
                    .arg("check")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .current_dir(PathBuf::from(&output));
                match runner.output() {
                    Ok(_) => {}
                    Err(e) => {
                        finish_spinner_with_message(&spinner, false, "Failed to check Rust code");
                        println!("\t└── {}", e);
                        return;
                    }
                }

                let mut runner = Command::new("cargo");
                runner
                    .arg("build")
                    .arg("--release")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .current_dir(PathBuf::from(&output));

                match runner.output() {
                    Ok(_) => {
                        finish_spinner_with_message(&spinner, true, "Successfully built Helix");
                    }
                    Err(e) => {
                        finish_spinner_with_message(&spinner, false, "Failed to build Helix");
                        println!("\t└── {}", e);
                        return;
                    }
                }

                let spinner = create_spinner("Starting Helix instance");
                // spinner.set_style(
                //     ProgressStyle::default_spinner()
                //         .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈")
                //         .template("{spinner:.green.bold} {msg}")
                //         .unwrap(),
                // );
                let instance_manager = InstanceManager::new().unwrap();

                let binary_path = dirs::home_dir()
                    .map(|path| path.join(".helix/repo/helix-db/target/release/helix-container"))
                    .unwrap();

                let endpoints: Vec<String> = successes
                    .values()
                    .flat_map(|source| source.queries.iter().map(|q| to_snake_case(&q.name)))
                    .collect();

                match instance_manager.start_instance(&binary_path, port, endpoints) {
                    Ok(instance) => {
                        finish_spinner_with_message(
                            &spinner,
                            true,
                            "Successfully started Helix instance",
                        );
                        println!(" ");
                        println!("\t└── Instance ID: {}", instance.id);
                        println!("\t└── Port: {}", instance.port);
                        println!("\t└── Available endpoints:");
                        for endpoint in instance.available_endpoints {
                            println!("\t    └── /{}", endpoint);
                        }
                    }
                    Err(e) => {
                        finish_spinner_with_message(
                            &spinner,
                            false,
                            "Failed to start Helix instance",
                        );
                        println!("\t└── {}", e);
                    }
                }
            }
        }
        args::CommandType::Instances(_) => {
            let instance_manager = InstanceManager::new().unwrap();
            match instance_manager.list_instances() {
                Ok(instances) => {
                    if instances.is_empty() {
                        println!("No running Helix instances");
                        return;
                    }
                    println!("Running Helix instances:");
                    for instance in instances {
                        println!("ID: {}", instance.id);
                        println!("  Port: {}", instance.port);
                        println!("  Started at: {}", instance.started_at);
                        println!("  Available endpoints:");
                        for endpoint in instance.available_endpoints {
                            println!("    /{}", endpoint);
                        }
                        println!();
                    }
                }
                Err(e) => {
                    println!("Failed to list instances: {}", e);
                }
            }
        }
        args::CommandType::Stop(command) => {
            let instance_manager = InstanceManager::new().unwrap();
            if command.all {
                match instance_manager.stop_all_instances() {
                    Ok(_) => println!("\t✅ Stopped all Helix instances"),
                    Err(e) => println!("\t❌ Failed to stop instances: {}", e),
                }
            } else if let Some(instance_id) = command.instance_id {
                match instance_manager.stop_instance(&instance_id) {
                    Ok(_) => println!("\t✅ Stopped instance {}", instance_id),
                    Err(e) => println!("\t❌ Failed to stop instance: {}", e),
                }
            } else {
                println!("Please specify --all or provide an instance ID");
            }
        }
        args::CommandType::Start(command) => {
            let instance_manager = InstanceManager::new().unwrap();
            let spinner = create_spinner("Starting Helix instance");

            match instance_manager.restart_instance(&command.instance_id) {
                Ok(Some(instance)) => {
                    finish_spinner_with_message(
                        &spinner,
                        true,
                        "Successfully restarted Helix instance",
                    );
                    println!("\t└── Instance ID: {}", instance.id);
                    println!("\t└── Port: {}", instance.port);
                    println!("\t└── Available endpoints:");
                    for endpoint in instance.available_endpoints {
                        println!("\t    └── /{}", endpoint);
                    }
                }
                Ok(None) => {
                    finish_spinner_with_message(
                        &spinner,
                        false,
                        "Instance not found or binary missing",
                    );
                    println!(
                        "\t└── Could not find instance with ID: {}",
                        command.instance_id
                    );
                }
                Err(e) => {
                    finish_spinner_with_message(&spinner, false, "Failed to restart instance");
                    println!("\t└── {}", e);
                }
            }
        }
        args::CommandType::Compile(command) => {
            let path = match &command.path {
                Some(path) => {
                    // call parser
                    path
                }
                None => {
                    // current directory
                    QUERIES_DIR
                }
            };
            let output = match &command.output {
                Some(output) => output.to_owned(),
                None => dirs::home_dir()
                    .map(|path| {
                        path.join(".helix/cache/generated/")
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|| "./.helix/cache/generated/".to_string()),
            };
            let files = match check_and_read_files(&path) {
                Ok(files) => files,
                Err(e) => {
                    println!("{}", e);
                    return;
                }
            };

            if files.is_empty() {
                println!("No queries found, nothing to compile");
                return;
            }

            let source = match compile_hql_to_source(&files) {
                Ok(source) => source,
                Err(e) => {
                    println!("\n❌ Failed to parse source");
                    println!("|");
                    println!("└─ {}", e);
                    return;
                }
            };

            generate_rust_from_source(&source, &output, files.len());
            if command.gen_py {
                generate_python_bindings(&source, &output);
            }
        }

        args::CommandType::Check(command) => {
            let path = match &command.path {
                Some(path) => {
                    // call parser
                    path
                }
                None => {
                    // current directory
                    QUERIES_DIR
                }
            };

            let files = match check_and_read_files(&path) {
                Ok(files) => files,
                Err(e) => {
                    println!("{}", e);
                    return;
                }
            };

            if files.is_empty() {
                println!("No queries found, nothing to compile");
                return;
            }

            match compile_hql_to_source(&files) {
                Ok(_) => {
                    println!("✅ Successfully parsed source");
                }
                Err(e) => {
                    println!("\n❌ Failed to parse source");
                    println!("|");
                    println!("└─ {}", e);
                    return;
                }
            }
        }
        args::CommandType::Install(_command) => {
            // check if cargo is installed
            let mut runner = Command::new("cargo");
            runner.arg("check");
            match runner.output() {
                Ok(_) => {}
                Err(e) => {
                    println!("\t❌ Cargo is not installed");
                    println!("\t|");
                    println!("\t└── {}", e);
                }
            }

            // check if git is installed
            let mut runner = Command::new("git");
            runner.arg("version");
            match runner.output() {
                Ok(_) => {}
                Err(e) => {
                    println!("\t❌ Git is not installed");
                    println!("\t|");
                    println!("\t└── {}", e);
                }
            }

            let repo_path = match _command.path {
                Some(path) => {
                    let path = PathBuf::from(path);
                    if !path.is_dir() {
                        println!("\t❌ Path is not a directory");
                        return;
                    }
                    if !path.exists() {
                        println!("\t❌ Path does not exist");
                        return;
                    }
                    path
                }
                None => {
                    // check if helix repo exists
                    let home_dir = match dirs::home_dir() {
                        Some(dir) => dir,
                        None => {
                            println!("\t❌ Could not determine home directory");
                            return;
                        }
                    };
                    home_dir.join(".helix/repo")
                }
            };

            if repo_path.clone().join("helix-db").exists()
                && repo_path.clone().join("helix-db").is_dir()
            {
                println!(
                    "\t✅ Helix repo already exists at {}",
                    repo_path.join("helix-db").display()
                );
                return;
            }

            // Create the directory structure if it doesn't exist
            match fs::create_dir_all(&repo_path) {
                Ok(_) => println!(
                    "\t✅ Created directory structure at {}",
                    repo_path.display()
                ),
                Err(e) => {
                    println!("\t❌ Failed to create directory structure");
                    println!("\t|");
                    println!("\t└── {}", e);
                    return;
                }
            }

            let mut runner = Command::new("git");
            runner.arg("clone");
            runner.arg("https://github.com/HelixDB/helix-db.git");
            runner.current_dir(&repo_path);

            match runner.output() {
                Ok(_) => {
                    let home_dir = dirs::home_dir().unwrap();
                    println!(
                        "\t✅ Helix repo installed at {}",
                        home_dir.join(".helix/repo/").to_string_lossy()
                    );
                    println!("\t|");
                    println!("\t└── To get started, begin writing helix queries in your project.");
                    println!("\t|");
                    println!("\t└── Then run `helix check --path <path-to-project>` to check your queries.");
                    println!("\t|");
                    println!("\t└── Then run `helix deploy --path <path-to-project> --local` to build your queries.");
                }
                Err(e) => {
                    println!("\t❌ Failed to install Helix repo");
                    println!("\t|");
                    println!("\t└── {}", e);
                    return;
                }
            }
        }
        args::CommandType::Test(command) => {
            match command.path {
                Some(path) => {
                    // parse files
                    let files = match check_and_read_files(&path) {
                        Ok(files) => files,
                        Err(e) => {
                            println!("{}", e);
                            return;
                        }
                    };

                    let temp_dir = TempDir::new().unwrap();

                    // parse

                    // interpret

                    // generate rust code

                    // run against rocksdb
                }
                None => println!("No path provided"),
            };

            match command.test {
                Some(test) => println!("Testing: {:?}", test),
                None => println!("No test provided"),
            }
        }
        args::CommandType::Init(command) => {
            println!("Initialising Helix project...");
            let path = match command.path {
                Some(path) => PathBuf::from(path),
                None => PathBuf::from("."),
            };

            // create directory
            fs::create_dir_all(&path).unwrap();

            // create queries directory
            let queries_dir = path.join(QUERIES_DIR);
            fs::create_dir_all(&queries_dir).unwrap();

            // create schema.hx
            let schema_path = queries_dir.join("schema.hx");
            fs::write(
                &schema_path,
                r#"// Start building your schema here.
//
// The schema is used to to ensure a level of type safety in your queries.
//
// The schema is made up of Node types, denoted by N::,
// and Edge types, denoted by E::
//
// Under the Node types you can define fields that
// will be stored in the database.
//
// Under the Edge types you can define what type of node
// the edge will connect to and from, and also the
// properties that you want to store on the edge.
//
// Example:
//
// N::User {
//     Name: String,
//     Label: String,
//     Age: Integer,
//     IsAdmin: Boolean,
// }
//
// E::Knows {
//     From: User,
//     To: User,
//     Properties: {
//         Since: Integer,
//     }
// }
//
//
// For more information on how to write queries,
// see the documentation at https://docs.helix-db.com
// or checkout our GitHub at https://github.com/HelixDB/helix-db
"#,
            )
            .unwrap();

            // create queries/main.hx
            let main_path = queries_dir.join("main.hx");
            fs::write(
                main_path,
                r#"// Start writing your queries here.
//
// You can use the schema to help you write your queries.
//
// Queries take the form:
//     QUERY {query name}({input name}: {input type}) =>
//         {variable} <- {traversal}
//         RETURN {variable}
//
// Example:
//     QUERY GetUserFriends(user_id: String) =>
//         friends <- N<User>(user_id)::Out<Knows>
//         RETURN friends
//
//
// For more information on how to write queries,
// see the documentation at https://docs.helix-db.com
// or checkout our GitHub at https://github.com/HelixDB/helix-db

QUERY hnswinsert(vector: [Float]) =>
    AddV<Vector>(vector)
    RETURN "Success"

QUERY hnswload(vectors: [[Float]]) =>
    res <- BatchAddV<Type>(vectors)
    RETURN res::{ID}

QUERY hnswsearch(query: [Float], k: Integer) =>
    res <- SearchV<Type>(query, k)
    RETURN res

QUERY size() =>
	size <- V::COUNT
	RETURN size
"#,
            )
            .unwrap();
            /*
            QUERY ingestnodes() =>
                AddN<Type>({ field: val })
                return "Success"

            QUERY ingestedges() =>
                AddE<Type>({ field: val })::To(node1)::From(node2)
                return "Sucess"
            */

            let config_path = queries_dir.join("config.hx.json");
            fs::write(config_path, Config::init_config()).unwrap();

            println!("Helix project initialised at {}", path.display());
        }
        args::CommandType::IngestSqlite(command) => {
            let path_str = command.path; // already required in clap object
            let instance = command.instance; // already required in clap object

            let path = Path::new(&path_str);
            if !path.exists() {
                println!("❌The file '{}' does not exist", path.display());
                return;
            }

            let valid_extensions = [".sqlite", ".db", ".sqlite3"];
            let is_valid_extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| valid_extensions.iter().any(|&valid_ext| valid_ext == ext))
                .unwrap_or(false);

            if !is_valid_extension {
                println!(
                    "❌The file '{}' must have a .sqlite, .db, or .sqlite3 extension.",
                    path.display()
                );
                return;
            }

            let instance_manager = InstanceManager::new().unwrap();
            match instance_manager.list_instances() {
                Ok(instances) => {
                    if instances.is_empty() {
                        println!("There are no running Helix instances!");
                        return;
                    }
                    let mut is_valid_instance = false;
                    for iter_instance in instances {
                        if iter_instance.id == instance {
                            is_valid_instance = true;
                            break;
                        }
                    }
                    if !is_valid_instance {
                        println!("No Helix instance found with id: '{}'!", instance);
                        return;
                    } else {
                        println!("Helix instance found with id: '{}'!", instance);
                    }
                }
                Err(e) => {
                    println!("Error while searching for Helix instances: {}", e);
                }
            }

            let mut ingestor = SqliteIngestor::new(&path_str, None, 5).unwrap();
            // ingestor.verify_sqlite().unwrap();
            //ingestor.ingest().unwrap(); // TODO: catch error here if throws
            // ingestor.verify();

            // TODO: ingestor verify db file is not corrupted or anything
            // TODO: ingestor.ingest()?;
            // TODO: verify ingestion and schema
        }
    }
}

fn check_and_read_files(path: &str) -> Result<Vec<DirEntry>, CliError> {
    // check there is schema and at least one query
    if !fs::read_dir(&path)
        .map_err(CliError::Io)?
        .any(|file| file.unwrap().file_name() == "schema.hx")
    {
        println!("{}", CliError::from("No schema file found"));
        // return Err(CliError::from("No schema file found"));
    }

    let files: Vec<DirEntry> = fs::read_dir(&path)?
        .filter_map(|entry| entry.ok())
        .filter(|file| file.file_name().to_string_lossy().ends_with(".hx"))
        .collect();

    Ok(files)
}

fn check_is_dir(path: &str) -> bool {
    match fs::metadata(&path) {
        Ok(metadata) => metadata.is_dir(),
        Err(e) => {
            println!("{}", CliError::Io(e));
            return false;
        }
    }
}

fn format_rust_file(file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("rustfmt").arg(file_path).status()?;

    if !status.success() {
        return Err(format!("rustfmt failed with exit code: {}", status).into());
    }

    Ok(())
}

fn check_hql_files(files: &Vec<DirEntry>) -> Result<(), CliError> {
    for file in files {
        let contents = fs::read_to_string(file.path()).unwrap();
        match HelixParser::parse_source(&contents) {
            Ok(_) => (),
            Err(e) => {
                return Err(CliError::from(format!("{}\n", e)));
            }
        }
    }
    Ok(())
}

fn compile_hql_to_source(files: &Vec<DirEntry>) -> Result<Source, CliError> {
    // let numb_of_files = files.len();
    // let mut code = String::new();
    // let mut generator = CodeGenerator::new();

    let contents: String = files
        .iter()
        .map(|file| -> String {
            match fs::read_to_string(file.path()) {
                Ok(contents) => contents,
                Err(e) => {
                    panic!("{}", e);
                }
            }
        })
        .fold(String::new(), |acc, contents| acc + &contents);

    let source = match HelixParser::parse_source(&contents) {
        Ok(source) => {
            // println!("{:?}", parser);
            source
        }
        Err(e) => {
            return Err(CliError::from(format!("{}\n", e)));
        }
    };

    Ok(source)
}

fn generate_rust_from_source(source: &Source, output_path: &String, numb_of_files: usize) {
    let mut generator = CodeGenerator::new();
    let mut code = String::new();
    code.push_str(&generator.generate_headers());
    code.push_str(&generator.generate_source(&source));

    let cache_dir = PathBuf::from(&output_path);
    fs::create_dir_all(&cache_dir).unwrap();

    let file_path = cache_dir.join(format!("queries.rs",));
    fs::write(&file_path, code).unwrap();
    match format_rust_file(&file_path) {
        Ok(_) => println!("\nCompiled and formatted {} files!\n", numb_of_files),
        Err(e) => println!(
            "\nCompiled {} files! (formatting failed: {})\n",
            numb_of_files, e
        ),
    };
}

struct Query {
    name: String,
    query: String,
    inputs: Vec<String>,
}

fn generate_python_bindings(source: &Source, output_path: &String) {
    let mut code = String::new();

    for query in &source.queries {
        code.push_str(&format!(
            "def {}(helixDB, {}):\n",
            query.name,
            query
                .parameters
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        ));
        code.push_str("\tdata = {\n");
        for input in &query.parameters {
            code.push_str(&format!("\t'{}': {},\n", input.name, input.name));
        }
        code.push_str("\t}\n");
        code.push_str("\tjson_string = json.dumps(data)\n");
        code.push_str(&format!(
            "\treturn helixDB.query(\"{}\", json_string)\n",
            query.name
        ));
    }

    let file_path = PathBuf::from(output_path).join(format!("queries.py",));
    fs::write(&file_path, code).unwrap();
}
