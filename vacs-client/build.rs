use vergen_git2::{BuildBuilder, CargoBuilder, Emitter, Git2Builder, RustcBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let git = Git2Builder::all_git()?;
    let build = BuildBuilder::all_build()?;
    let cargo = CargoBuilder::all_cargo()?;
    let rustc = RustcBuilder::all_rustc()?;

    Emitter::default()
        .add_instructions(&git)?
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .emit()?;

    tauri_build::build();

    Ok(())
}
