//! Pure generic source provider; private language owners compose through `input_service`.
use std::{io::Read, path::Path};
async fn run(plan: &Path, socket: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(plan)?
        .take(131_073)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 131_072 {
        return Err("SourceInputLimit".into());
    }
    let plan = serde_json::from_slice(&bytes)?;
    hat_source_curator::input_service::serve(plan, socket, |tree| {
        serde_json::to_value(tree).map_err(|_| "SourceInputRejected")
    })
    .await
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [plan, socket] => run(Path::new(plan), Path::new(socket)).await,
        _ => Err("SourceInputArgumentsInvalid".into()),
    };
    if result.is_err() {
        eprintln!("SourceProviderUnavailable");
        std::process::exit(1);
    }
}
