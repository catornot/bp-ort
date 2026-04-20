use rrplug::{
    bindings::{
        cvar::convar::FCVAR_GAMEDLL,
        server::{
            cai_base_npc::CAI_BaseNPC, cbaseentity::CBaseEntity, cnpc_melee_only::CNPC_MeleeOnly,
        },
    },
    prelude::*,
};
use shared::{
    bindings::{SERVER_FUNCTIONS, ServerFunctions},
    utils::{self, get_entity_handle, lookup_ent, nudge_type},
};
use std::sync::OnceLock;

pub struct TickPlus {
    enabled: OnceLock<ConVarStruct>,
}

impl Plugin for TickPlus {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        c"holoplus",
        c"holoplus",
        c"holoplus",
        PluginContext::DEDICATED,
    );

    fn new(_: bool) -> Self {
        Self {
            enabled: OnceLock::new(),
        }
    }

    fn on_dll_load(&self, _engine: Option<&EngineData>, dll_ptr: &DLLPointer, _token: EngineToken) {
        if let WhichDll::Engine = dll_ptr.which_dll()
            && self
                .enabled
                .set(
                    ConVarStruct::try_new(
                        &ConVarRegister::new(
                            "tickplus_enabled",
                            "0",
                            FCVAR_GAMEDLL as i32,
                            "toggles tickplus",
                        ),
                        _token,
                    )
                    .expect("couldn't create convar holoplus_enabled"),
                )
                .is_err()
        {
            panic!("couldn't set convar holoplus_enabled");
        }
    }

    fn runframe(&self, _engine_token: EngineToken) {
        if !self.enabled.wait().get_value_bool() {
            return;
        }

        let server_funcs = SERVER_FUNCTIONS.wait();
        for frag_drone in utils::ClassNameIter::new(c"npc_frag_drone", server_funcs)
            .filter_map(|ent| unsafe { ent.cast::<CNPC_MeleeOnly>().as_mut() })
        {
            if let Some((target, _)) = get_closest_player(frag_drone, server_funcs)
                && !frag_drone.m_AssaultBehavior.m_bAssaultActive
            {
                if let Some(target) = lookup_ent(target, server_funcs) {
                    unsafe {
                        (server_funcs.set_enemy)(
                            nudge_type::<&mut CAI_BaseNPC>(frag_drone),
                            target,
                        );
                    }
                }
                frag_drone.m_AssaultBehavior.m_assaultMovingGroundEnt = target;
                frag_drone.m_AssaultBehavior.m_fOverrode = true;
                frag_drone.m_AssaultBehavior.m_flGoalRadius = 20.;
                frag_drone.m_AssaultBehavior.m_bAssaultActive = true;
                if let Some(navigator) = unsafe { frag_drone.m_pNavigator.as_mut() } {
                    navigator.m_moveFlags = i32::MAX;
                }
            }
        }
    }
}

fn get_closest_player(ent: &CBaseEntity, server_funcs: &ServerFunctions) -> Option<(i32, Vector3)> {
    let mut v = Vector3::ZERO;
    let pos = unsafe { *ent.get_origin(&mut v) };
    (0..32)
        .filter_map(|i| unsafe {
            (server_funcs.get_player_by_index)(i)
                .as_ref()
                .filter(|player| player.m_iTeamNum != ent.m_iTeamNum)
        })
        .map(|player| {
            (get_entity_handle(player), unsafe {
                *player.get_origin(&mut v)
            })
        })
        .reduce(|player1, player2| {
            if crate::bots::cmds_utils::distance3(player1.1, pos)
                < crate::bots::cmds_utils::distance3(player2.1, pos)
            {
                player1
            } else {
                player2
            }
        })
}
