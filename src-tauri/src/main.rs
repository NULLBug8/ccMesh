#[tokio::main]
async fn main() {
    if let Err(error) = ccmesh_lib::run_server().await {
        eprintln!("ccMesh startup failed: {error}");
        std::process::exit(1);
    }
}
