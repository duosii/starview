fn main() -> Result<(), std::io::Error> {
    let cli_result = starview_cli::run();

    if let Err(err) = cli_result {
        err.print()?;
    }

    Ok(())
}
