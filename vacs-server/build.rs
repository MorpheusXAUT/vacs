use vergen_git2::{Build, Cargo, Emitter, Git2, Rustc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let git = Git2::builder()
        .branch(true)
        .commit_date(true)
        .commit_message(true)
        .describe(true, true, None)
        .sha(false)
        .dirty(true)
        .build();
    let build = Build::all_build();
    let cargo = Cargo::all_cargo();
    let rustc = Rustc::all_rustc();

    Emitter::default()
        .add_instructions(&git)?
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .emit()?;

    Ok(())
}
