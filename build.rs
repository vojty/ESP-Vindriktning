fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv_build::output(dotenv_build::Config::default())?;

    embuild::espidf::sysenv::output();
    Ok(())
}
