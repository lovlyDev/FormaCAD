//! Parent owns permissions and workspace. Worker accepts bounded fixed-name jobs.
fn main() -> std::io::Result<()> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() == 1 && arguments[0] == "--health-check" {
        let response = forma_core::native::health::kernel_readiness()
            .map_err(|error| std::io::Error::other(format!("{}: {}", error.code, error.detail)))?;
        serde_json::to_writer(std::io::stdout(), &response)?;
        return Ok(());
    }
    if !arguments.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Unsupported native worker arguments",
        ));
    }
    forma_core::worker_protocol::run(std::io::stdin(), &std::env::current_dir()?)
}
