//! Bounded source-owner CLI. Does not start a worker, persist meaning or call a model.
use std::io::Read;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = Vec::new();
    std::io::stdin().take(131_073).read_to_end(&mut input)?;
    if input.len() > 131_072 {
        return Err("source-input-limit".into());
    }
    match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [mode] if mode == "package" => {
            print!("{}", include_str!("../../hat.package.json"));
        }
        [mode] if mode == "input-seal" => {
            let mut plan: hat_source_curator::input::OwnerInput = serde_json::from_slice(&input)?;
            plan.declaration.action.contract_revision = plan.contract_ref()?;
            plan.validate()?;
            serde_json::to_writer(std::io::stdout(), &plan)?;
        }
        [] => {
            let request: hat_source_curator::structure::Decomposition =
                serde_json::from_slice(&input)?;
            let output = hat_source_curator::structure::decompose(&request)?;
            serde_json::to_writer(std::io::stdout(), &output)?;
        }
        [mode] if mode == "contribution" => {
            let request = serde_json::from_slice(&input)?;
            let output = hat_source_curator::interpretation::interpret(request)?;
            serde_json::to_writer(std::io::stdout(), &output)?;
        }
        [mode] if mode == "graph" => {
            let request = serde_json::from_slice(&input)?;
            let output = hat_source_curator::structure::decompose_graph(&request)?;
            serde_json::to_writer(std::io::stdout(), &output)?;
        }
        [mode] if mode == "input-contract" => {
            let plan: hat_source_curator::input::OwnerInput = serde_json::from_slice(&input)?;
            plan.validate()?;
            // Public output omits the source, its paths, bindings and fixed values.
            serde_json::to_writer(std::io::stdout(), &plan.declaration)?;
        }
        [mode] if mode == "input-bind" => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Request {
                plan: hat_source_curator::input::OwnerInput,
                submission: zixcel_interaction::InvokeRequest,
            }
            let request: Request = serde_json::from_slice(&input)?;
            let output = request.plan.bind(&request.submission)?;
            serde_json::to_writer(std::io::stdout(), &output)?;
        }
        _ => return Err("source-command-invalid".into()),
    }
    Ok(())
}
