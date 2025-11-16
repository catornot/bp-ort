use std::sync::LazyLock;

use rrplug::{
    bindings::{
        class_types::cplayer::CPlayer,
        cvar::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL, FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS},
    },
    prelude::*,
};
use shared::utils::{get_c_char_array, send_client_print, set_c_char_array};

use crate::{
    admin_abuse::{
        admin_check, completion_append_player_names, execute_for_matches, forward_to_server,
    },
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
};

const DISGUISE_TYPES_ARR: &[DisguiseTypes] = const {
    use DisguiseTypes::*;
    &[Name, Tag, Traversal, Edict, Generation, Level]
};
static DISGUISE_TYPES_STR: LazyLock<Box<[&'static str]>> = LazyLock::new(|| {
    DISGUISE_TYPES_ARR
        .iter()
        .map(|ty| &*format!("{ty:?}").to_lowercase().leak())
        .collect::<Vec<_>>()
        .into_boxed_slice()
});

#[derive(Debug, Clone, Copy)] // IMPORTANT: don't forget to update the array above
enum DisguiseTypes {
    Name,
    Tag,
    Traversal,
    Edict,
    Generation,
    Level,
}

pub fn register_disguise_command(engine_data: &EngineData, token: EngineToken) {
    _ = engine_data.register_concommand_with_completion(
        "disguise",
        forward_to_server,
        "dones various things to cplayer and cclient",
        FCVAR_CLIENTDLL as i32,
        disguise_completion,
        token,
    );

    _ = engine_data.register_concommand(
        "disguise_server",
        disguise_server_command,
        "",
        FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
        token,
    );
}

#[rrplug::concommand]
fn disguise_server_command(command: CCommandResult) {
    if command.get_arg(0).is_none() {
        log::warn!("Usage: {} < name >", command.get_command());
        return;
    }

    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();

    let (is_admin, Some(player)) = admin_check(&command, engine, funcs) else {
        return;
    };

    if !is_admin {
        return;
    }

    let Some(ty) = command.get_arg(0).and_then(disguise_str_to_ty) else {
        _ = send_client_print(
            player,
            &format!("{:?} is not a valid type", command.get_arg(0)),
        );
        log::info!(
            "player emited err: {:?} is not a valid type",
            command.get_arg(0)
        );
        return;
    };

    execute_for_matches(
        command.get_arg(1),
        |player| {
            if let Err(err) = disguise(player, ty, command.get_args().get(2..).unwrap_or_default())
            {
                _ = send_client_print(player, err);
                log::info!("player emited err: {err}");
            }
        },
        false,
        funcs,
        engine,
    );
}

fn disguise(
    player: &mut CPlayer,
    ty: DisguiseTypes,
    extra_cmds: &[String],
) -> Result<(), &'static str> {
    let client = unsafe {
        ENGINE_FUNCTIONS
            .wait()
            .client_array
            .add(
                player
                    .pl
                    .index
                    .abs()
                    .saturating_sub(1)
                    .try_into()
                    .ok()
                    .ok_or("woops")?,
            )
            .as_mut()
            .ok_or("no cclient how?")?
    };

    match ty {
        DisguiseTypes::Name => {
            // if we have smth
            let _ = extra_cmds.first().ok_or("no name provided rip")?;
            let name = extra_cmds
                .iter()
                .enumerate()
                .flat_map(|(i, s)| {
                    if i == extra_cmds.len().saturating_sub(1) {
                        [s.as_str(), ""]
                    } else {
                        [s.as_str(), " "]
                    }
                })
                .collect::<String>();

            if name.len() >= client.m_szServerName.len()
                || name.is_char_boundary(client.m_szServerName.len() - 1)
            {
                Err("too long")?;
            }

            unsafe {
                // HACK: setting player name to nothing tricks the game into running setname
                set_c_char_array(&mut client.m_szServerName, "");
                (ENGINE_FUNCTIONS.wait().cclient_setname)(
                    client,
                    (name.to_string() + "\0").as_ptr().cast(),
                );
            }
        }
        DisguiseTypes::Tag => {
            // if we have smth
            let _ = extra_cmds.first().ok_or("no tag provided rip")?;
            let tag = extra_cmds
                .iter()
                .enumerate()
                .flat_map(|(i, s)| {
                    if i == extra_cmds.len().saturating_sub(1) {
                        [s.as_str(), ""]
                    } else {
                        [s.as_str(), " "]
                    }
                })
                .collect::<String>();

            if tag.len() >= client.m_szServerName.len()
                || tag.is_char_boundary(client.m_szServerName.len() - 1)
            {
                Err("too long")?;
            }

            let name = get_c_char_array(&client.m_szServerName)
                .ok_or("failed to get name")?
                .to_string();
            unsafe {
                // HACK: setting the player name also updates the clan tag
                set_c_char_array(&mut client.m_szServerName, "");
                set_c_char_array(&mut client.m_szClanTag, &tag);
                set_c_char_array(&mut player.m_communityClanTag, &tag);
                (ENGINE_FUNCTIONS.wait().cclient_setname)(client, (name + "\0").as_ptr().cast());
            }
        }
        DisguiseTypes::Traversal => {
            let state: i32 = extra_cmds
                .first()
                .ok_or("no state")?
                .parse()
                .ok()
                .ok_or("bad state")?;

            player.m_traversalType = state;
        }
        DisguiseTypes::Edict => {
            log::info!("client.handle {}", client.m_nHandle);

            let edict: u16 = extra_cmds
                .first()
                .ok_or("no edict")?
                .parse()
                .ok()
                .ok_or("bad edict")?;

            client.m_nHandle = edict;

            log::info!("new client.handle {}", client.m_nHandle);
        }
        DisguiseTypes::Generation => {
            let generation: i32 = extra_cmds
                .first()
                .ok_or("no generation")?
                .parse()
                .ok()
                .ok_or("bad generation")?;

            player.m_generation = generation;
        }
        DisguiseTypes::Level => {
            let level: i32 = extra_cmds
                .first()
                .ok_or("no level")?
                .parse()
                .ok()
                .ok_or("bad level")?;

            player.m_rank = level;
        }
    }

    Ok(())
}

#[rrplug::completion]
fn disguise_completion(current: CurrentCommand, suggestions: CommandCompletion) {
    let Some((prev, next)) = current.partial.split_once(' ') else {
        for ty in DISGUISE_TYPES_STR
            .iter()
            .filter(|ty| ty.starts_with(current.partial))
        {
            _ = suggestions.push(&format!("{} {}", current.cmd, ty))
        }

        return;
    };

    let prev_prev = prev;
    let Some((prev, next)) = next.split_once(' ') else {
        completion_append_player_names(next, |name| {
            _ = suggestions.push(&format!("{} {} {}", current.cmd, prev, name))
        });

        return;
    };

    _ = suggestions.push(&format!(
        "{} {prev_prev} {} {}",
        current.cmd, prev, "enter the valid value for the disguise type"
    ));
    _ = suggestions.push(&format!(
        "{} {prev_prev} {} {}",
        current.cmd, prev, "below are all the names and factions"
    ));

    if "all".starts_with(next) {
        _ = suggestions.push(&format!("{} {prev_prev} {} all", current.cmd, prev))
    }

    if "imc".starts_with(next) {
        _ = suggestions.push(&format!("{} {prev_prev} {} imc", current.cmd, prev))
    }

    if "militia".starts_with(next) {
        _ = suggestions.push(&format!("{} {prev_prev} {} militia", current.cmd, prev))
    }

    completion_append_player_names(next, |name| {
        _ = suggestions.push(&format!("{} {prev_prev} {} {}", current.cmd, prev, name))
    });
}

fn disguise_str_to_ty(str: &str) -> Option<DisguiseTypes> {
    DISGUISE_TYPES_ARR
        .iter()
        .copied()
        .zip(DISGUISE_TYPES_STR.iter().copied())
        .find_map(|(ty, str_ty)| (str_ty == str).then_some(ty))
}
