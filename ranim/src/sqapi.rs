use std::{fs, path::PathBuf};

use high::squirrel::{UserData, UserDataRef};
use rrplug::prelude::*;

use crate::{bindings::RecordedAnimation, recording_impl::SavedRecordedAnimation, NS_DIR};

const USER_DATA_ID: u64 = 18444492235241160706;

pub fn register_sq_function() {
    register_sq_functions(save_recorded_animation);
    register_sq_functions(read_recorded_animation);
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "RSaveRecordedAnimation")]
fn save_recorded_animation(
    recording: UserDataRef<RecordedAnimation, true, USER_DATA_ID>,
    name: String,
) -> Result<(), String> {
    let recording: SavedRecordedAnimation = recording.clone().into();

    fs::write(
        name_to_path(name)?,
        bincode::serialize(&recording).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;

    Ok(())
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "RReadRecordedAnimation")]
fn read_recorded_animation(name: String) -> Result<UserData<RecordedAnimation, true>, String> {
    bincode::deserialize::<SavedRecordedAnimation>(
        &fs::read(name_to_path(name)?).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?
    .try_into()
    .map(UserData::new)
    .map_err(|err: &str| err.to_string())
}

fn name_to_path(name: String) -> Result<PathBuf, String> {
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
    path.set_extension(".anim");

    if path.parent().and_then(|path| path.as_os_str().to_str()) != Some("recordings") {
        Err("path somehow ended up in the wrong dir".to_string())
    } else {
        Ok(path)
    }
}