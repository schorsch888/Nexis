use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = nexis_cli::Cli::parse();
    match nexis_cli::run(cli).await {
        Ok(output) => {
            println!("{output}");
        }
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}
