use rrplug::{bindings::class_types::globalvars::CGlobalVars, prelude::*};

use crate::bindings::{CUserCmd, EngineFunctions, ServerFunctions};

#[derive(Clone)]
pub struct CUserCmdHelper<'a> {
    pub globals: &'a CGlobalVars,
    pub angles: Vector3,
    pub cmd_num: u32,
    pub sv_funcs: &'a ServerFunctions,
    pub engine_funcs: &'a EngineFunctions,
}

impl<'a> CUserCmdHelper<'a> {
    pub fn new(
        globals: &'a CGlobalVars,
        angles: Vector3,
        cmd_num: u32,
        sv_funcs: &'a ServerFunctions,
        engine_funcs: &'a EngineFunctions,
    ) -> CUserCmdHelper<'a> {
        Self {
            globals,
            angles,
            cmd_num,
            sv_funcs,
            engine_funcs,
        }
    }

    pub fn construct_from_global(s: &Self) -> Self {
        s.clone()
    }
}

impl CUserCmd {
    pub fn new_basic_move(move_: Vector3, buttons: u32, helper: &CUserCmdHelper) -> Self {
        CUserCmd {
            move_,
            tick_count: helper.globals.tickCount,
            frame_time: helper.globals.absoluteFrameTime,
            command_time: helper.globals.curTime,
            command_number: helper.cmd_num,
            world_view_angles: helper.angles,
            local_view_angles: Vector3::ZERO,
            attackangles: helper.angles,
            buttons,
            impulse: 0,
            weaponselect: 0,
            meleetarget: 0,
            camera_pos: Vector3::ZERO,
            camera_angles: Vector3::ZERO,
            tick_something: helper.globals.tickCount as i32,
            dword90: helper.globals.tickCount + 4,
            ..CUserCmd::init_default(helper.sv_funcs)
        }
    }

    pub fn new_empty(helper: &CUserCmdHelper) -> Self {
        CUserCmd {
            tick_count: helper.globals.tickCount,
            frame_time: helper.globals.absoluteFrameTime,
            command_time: helper.globals.curTime,
            command_number: helper.cmd_num,
            world_view_angles: helper.angles,
            local_view_angles: Vector3::ZERO,
            attackangles: helper.angles,
            impulse: 0,
            weaponselect: 0,
            meleetarget: 0,
            camera_pos: Vector3::ZERO,
            camera_angles: helper.angles,
            tick_something: helper.globals.tickCount as i32,
            dword90: helper.globals.tickCount + 4,
            ..CUserCmd::init_default(helper.sv_funcs)
        }
    }
}
