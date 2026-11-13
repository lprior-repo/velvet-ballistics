//! CLI argument parsing for the closure-evidence checker.

pub(crate) const DEFAULT_PARENT: &str = "vb-jpq7";
pub(crate) const DEFAULT_MANIFEST: &str = ".beads/vb-jpq7/closure-evidence-manifest.jsonl";
pub(crate) const DEFAULT_BD_WORKDIR: &str = ".";

pub(crate) struct CliArgs {
    pub(crate) parent: String,
    pub(crate) manifest: String,
    pub(crate) bd_workdir: String,
    pub(crate) self_test: bool,
}

pub(crate) fn parse() -> Result<CliArgs, String> {
    let args: Vec<String> = std::env::args().collect();

    let mut parent = DEFAULT_PARENT.to_string();
    let mut manifest = DEFAULT_MANIFEST.to_string();
    let mut bd_workdir = DEFAULT_BD_WORKDIR.to_string();
    let mut self_test = false;

    let mut iter = args.into_iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--parent" => {
                parent = iter.next().ok_or("--parent requires a value".to_string())?;
            }
            "--manifest" => {
                manifest = iter
                    .next()
                    .ok_or("--manifest requires a value".to_string())?;
            }
            "--bd-workdir" => {
                bd_workdir = iter
                    .next()
                    .ok_or("--bd-workdir requires a value".to_string())?;
            }
            "--self-test" => {
                self_test = true;
            }
            _ => {
                return Err(format!("unknown option: {}", arg));
            }
        }
    }

    Ok(CliArgs {
        parent,
        manifest,
        bd_workdir,
        self_test,
    })
}
