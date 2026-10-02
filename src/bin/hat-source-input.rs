//! External pure source-input service. Hatter never spawns this executable.
//! The local owner supplies a private plan; only its declaration leaves on describe.
use crowsi_transport_foundation::{
    Connection, Limits,
    io::{Reader, Writer},
};
use hat_source_curator::input::OwnerInput;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::Read,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    time::Duration,
};
use tokio::net::UnixListener;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
enum Request {
    Describe,
    Bind {
        generation: String,
        submission: zixcel_interaction::InvokeRequest,
    },
}
fn limits() -> Limits {
    Limits {
        accepted: 131_072,
        buffered: 131_073,
        frame: 131_074,
        pending_bytes: 131_074,
        pending_messages: 1,
    }
}
fn response(plan: &OwnerInput, generation: &str, request: Request) -> Result<Value, &'static str> {
    match request {
        Request::Describe => Ok(
            json!({"generation":generation,"input":plan.declaration,"sourceRevision":plan.source_revision}),
        ),
        Request::Bind {
            generation: expected,
            submission,
        } => {
            if expected != generation {
                return Err("SourceProviderStale");
            }
            let input = plan.bind(&submission).map_err(|_| "SourceInputRejected")?;
            Ok(json!({"generation":generation,"input":input}))
        }
    }
}
async fn run(plan_path: &Path, socket: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(plan_path)?
        .take(131_073)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 131_072 {
        return Err("SourceInputLimit".into());
    }
    let plan: OwnerInput = serde_json::from_slice(&bytes)?;
    plan.validate()?;
    let parent = socket.parent().ok_or("SourceEndpointInvalid")?;
    let metadata = std::fs::symlink_metadata(parent)?;
    if !socket.is_absolute() || !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
        return Err("SourceEndpointInvalid".into());
    }
    // Binding fails if anything already occupies the endpoint; never unlink another owner.
    let listener = UnixListener::bind(socket)?;
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))?;
    let socket_identity = std::fs::symlink_metadata(socket)?;
    let generation = uuid::Uuid::new_v4().to_string();
    println!("{}", json!({"generation":generation,"ready":true}));
    let stopping = tokio::signal::ctrl_c();
    tokio::pin!(stopping);
    loop {
        let stream = tokio::select! {
            result = listener.accept() => result?.0,
            _ = &mut stopping => break,
        };
        // One in-flight pure request. No worker thread, effects, file mutation or retry queue.
        let connection = Connection::new()?;
        connection.open()?;
        let (read, write) = stream.into_split();
        let mut reader = Reader::new(read, limits(), connection.clone())?;
        let writer = Writer::new(write, limits(), connection.clone(), Duration::from_secs(4))?;
        let exchange = async {
            let frame = reader
                .next_frame()
                .await?
                .ok_or(crowsi_transport_foundation::Outcome::ConnectionClosed)?;
            let result = serde_json::from_slice::<Request>(&frame.payload)
                .map_err(|_| "SourceInputInvalid")
                .and_then(|r| response(&plan, &generation, r));
            let value = match result {
                Ok(value) => value,
                Err(code) => json!({"error":{"code":code}}),
            };
            writer
                .send(|out| serde_json::to_writer(out, &value).map_err(std::io::Error::other))
                .await
        };
        let stopped = tokio::select! {
            _ = &mut stopping => true,
            _ = tokio::time::timeout(Duration::from_secs(4), exchange) => false,
        };
        connection.close();
        if stopped {
            break;
        }
    }
    drop(listener);
    let current = std::fs::symlink_metadata(socket)?;
    if (current.dev(), current.ino()) == (socket_identity.dev(), socket_identity.ino()) {
        std::fs::remove_file(socket)?;
    }
    Ok(())
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
