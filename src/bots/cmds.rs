use rrplug::{bindings::entity::SignonState, high::vector::Vector3};
use std::ops::Deref;

// buttons
// moveleft : 0x200

use super::SIMULATE_CONVAR;
use crate::{
    bindings::{CUserCmd, ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::iterate_c_array_sized,
};

pub fn run_bots_cmds() {
    if SIMULATE_CONVAR.wait().get_value_i32() != 1 {
        return;
    }

    let server_functions = SERVER_FUNCTIONS.wait();
    let engine_functions = ENGINE_FUNCTIONS.wait();
    let player_by_index = server_functions.get_player_by_index;
    let proccess_user_cmds = server_functions.proccess_user_cmds;
    let run_null_command = server_functions.run_null_command;
    let get_eye_pos = server_functions.get_eye_pos;
    let eye_angles = server_functions.eye_angles;
    let globals =
        unsafe { engine_functions.globals.as_ref() }.expect("globals were null for some reason");

    for (i, client) in
        unsafe { iterate_c_array_sized::<_, 32>(engine_functions.client_array.into()).enumerate() }
    {
        unsafe {
            if *client.signon.deref().deref() == SignonState::FULL && **client.fake_player {
                let player = player_by_index((i + 1) as i32)
                    .as_mut()
                    .expect("invalid playe for valid client");

                let mut v = Vector3::ZERO;

                let angles = *eye_angles(player, &mut v);

                let cmd = CUserCmd {
                    move_: Vector3::new(0., -1., 0.),
                    tick_count: **globals.tick_count,
                    frame_time: **globals.absolute_frame_time,
                    command_time: **globals.cur_time,
                    command_number: **player.rank as u32,
                    world_view_angles: angles,
                    local_view_angles: angles,
                    attackangles: angles,
                    buttons: 0x200,
                    impulse: 0,
                    weaponselect: 0,
                    meleetarget: 0,
                    camera_pos: *get_eye_pos(player, &mut v),
                    camera_angles: angles,
                    tick_something: **globals.tick_count as i32,
                    dword90: **globals.tick_count + 4,
                    ..Default::default()
                };

                **player.rank += 1; // using this for command number

                proccess_user_cmds(player, 1, &cmd, 1, 1, 0, 0);
                run_null_command(player)
            }
        }
    }
}
