use vergen_git2::{BuildBuilder, CargoBuilder, Emitter, Git2Builder, RustcBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let git = Git2Builder::default()
        .branch(true)
        .commit_date(true)
        .commit_message(true)
        .describe(true, true, None)
        .sha(false)
        .dirty(true)
        .build()?;
    let build = BuildBuilder::all_build()?;
    let cargo = CargoBuilder::all_cargo()?;
    let rustc = RustcBuilder::all_rustc()?;

    Emitter::default()
        .add_instructions(&git)?
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .emit()?;

    if let Ok(val) = std::env::var("VACS_VERSION_OVERRIDE") {
        println!("cargo:rustc-env=VACS_VERSION_OVERRIDE={val}");
    }

    tauri_build::build();
    Ok(())
}
