use mid::northstar::NORTHSTAR_DATA;
use rrplug::prelude::*;
use std::{fs, io::Write, path::PathBuf};

use crate::{bindings::RecordedAnimation, recording_impl::SavedRecordedAnimation, NS_DIR};

#[allow(unused)]
const USER_DATA_ID: u64 = 18444492235241160706;

pub fn register_sq_function() {
    register_sq_functions(save_recorded_animation);
    register_sq_functions(read_recorded_animation);
    register_sq_functions(unload_thyself);
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "RSaveRecordedAnimation")]
fn save_recorded_animation(recording: &mut RecordedAnimation, name: String) -> Result<(), String> {
    let org = recording.clone();
    let recording: SavedRecordedAnimation = recording.clone().into();

    fs::File::create(name_to_path(name.clone())?)
        .map_err(|err| err.to_string())?
        .write_all(&bincode::serialize(&recording).map_err(|err| err.to_string())?)
        .map_err(|err| err.to_string())?;

    assert_eq!(
        org,
        bincode::deserialize::<SavedRecordedAnimation>(
            &fs::read(name_to_path(name)?).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?
        .try_into()
        .map_err(|err: &str| err.to_string())?
    );

    Ok(())
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "RReadRecordedAnimation")]
fn read_recorded_animation(name: String) -> Result<&mut RecordedAnimation, String> {
    bincode::deserialize::<SavedRecordedAnimation>(
        &fs::read(name_to_path(name)?).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?
    .try_into()
    .map_err(|err: &str| err.to_string())
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "RUnload")]
fn unload_thyself() {
    unsafe {
        NORTHSTAR_DATA
            .wait()
            .sys()
            .unload(NORTHSTAR_DATA.wait().handle())
    }
}

fn name_to_path(name: impl ToString) -> Result<PathBuf, String> {
    let name = name.to_string();
    if name
        .chars()
        .any(|c| !c.is_alphabetic() && c != '_' && c != '-')
    {
        return Err(
            "name didn't pass the filter, should be a only have alphanumric values, _ or -"
                .to_string(),
        );
    }

    let mut path = NS_DIR
        .get()
        .expect("NS_DIR should be init")
        .join("recordings")
        .join(name);
    path.set_extension("anim");

    Ok(path)
}
