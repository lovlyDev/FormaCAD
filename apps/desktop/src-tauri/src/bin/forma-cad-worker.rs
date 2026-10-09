//! Parent owns permissions and workspace. Worker accepts bounded fixed-name jobs.
fn main() -> std::io::Result<()> {
    forma_core::worker_protocol::run(std::io::stdin(), &std::env::current_dir()?)
}
