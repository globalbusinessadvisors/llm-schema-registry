use clap::Parser;

#[derive(Parser)]
#[command(name = "schema-cli")]
#[command(about = "Schema Registry CLI", long_about = None)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
    println!("Schema Registry CLI");
}
