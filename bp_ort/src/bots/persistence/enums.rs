macro_rules! pdata_enum {
    (enum $name:ident for [$($property:expr),*] { $($fname:ident = $fvalue:expr),*, }) => {
        #[allow(non_snake_case, non_camel_case_types, dead_code, clippy::upper_case_acronyms)]
        pub enum $name {
            $($fname = $fvalue),*
        }

        impl $name {
            pub fn check(property: &str, value: &str) -> Option<i32> {
                if ![$($property),*].contains(&property) {
                    return None;
                }

                match value {
                    $(stringify!($fname) => Some($fvalue),)*
                    _ => None
                }
            }
        }
    };
    (enum $name:ident for $property:literal; { $($fname:ident = $fvalue:expr),*, }) => {
        #[allow(non_snake_case, non_camel_case_types, dead_code, clippy::upper_case_acronyms)]
        pub enum $name {
            $($fname = $fvalue),*
        }

        impl $name {
            pub fn check(property: &str, value: &str) -> Option<i32> {
                if property != $property {
                    return None;
                }

                match value {
                    $(stringify!($fname) => Some($fvalue),)*
                    _ => None
                }
            }
        }
    };
}

pdata_enum! {
    enum TitanExecution for "titanExecution"; {
        execution_ion             = 0,
        execution_ion_prime       = 1,
        execution_tone            = 2,
        execution_tone_prime      = 3,
        execution_ronin           = 4,
        execution_ronin_prime     = 5,
        execution_northstar       = 6,
        execution_northstar_prime = 7,
        execution_legion          = 8,
        execution_legion_prime    = 9,
        execution_vanguard        = 10,
        execution_scorch          = 11,
        execution_scorch_prime    = 12,
        execution_random_0        = 13,
        execution_random_1        = 14,
        execution_random_2        = 15,
        execution_random_3        = 16,
        execution_random_4        = 17,
        execution_random_5        = 18,
        execution_random_6        = 19,
    }
}

pdata_enum! {
    enum TitanClasses for "titanClass"; {
        ion = 0,
        scorch = 1,
        ronin = 2,
        tone = 3,
        northstar = 4,
        legion = 5,
        vanguard = 6,
    }
}

pdata_enum! {
    enum TitanIsPrimeTitan for "isPrime"; {
           titan_is_not_prime = 0,
           titan_is_prime     = 1,
    }
}

pdata_enum! {
    enum TitanMod for "primaryMod"; {
        NULL                            = 0,
        accelerator                     = 1,
        afterburners                    = 2,
        arc_triple_threat               = 3,
        burn_mod_titan_40mm             = 4,
        burn_mod_titan_arc_cannon       = 5,
        burn_mod_titan_sniper           = 6,
        burn_mod_titan_triple_threat    = 7,
        burn_mod_titan_xo16             = 8,
        burn_mod_titan_dumbfire_rockets = 9,
        burn_mod_titan_homing_rockets   = 10,
        burn_mod_titan_salvo_rockets    = 11,
        burn_mod_titan_shoulder_rockets = 12,
        burn_mod_titan_vortex_shield    = 13,
        burn_mod_titan_smoke            = 14,
        burn_mod_titan_particle_wall    = 15,
        burst                           = 16,
        capacitor                       = 17,
        extended_ammo                   = 18,
        fast_lock                       = 19,
        fast_reload                     = 20,
        instant_shot                    = 21,
        overcharge                      = 22,
        quick_shot                      = 23,
        rapid_fire_missiles             = 24,
        stryder_sniper                  = 25,
    }
}

pdata_enum! {
    enum TitanPassive for ["passive1", "passive2", "passive3", "passive4", "passive5", "passive6"] {
            NULL                       = 0,
            pas_enhanced_titan_ai      = 1,
            pas_auto_eject             = 2,
            pas_dash_recharge          = 3,
            pas_defensive_core         = 4,
            pas_shield_regen           = 5,
            pas_assault_reactor        = 6,
            pas_hyper_core             = 7,
            pas_anti_rodeo             = 8,
            pas_build_up_nuclear_core  = 9,
            pas_offensive_autoload     = 10,
            pas_offensive_hitnrun      = 11,
            pas_offensive_regen        = 12,
            pas_defensive_tacload      = 13,
            pas_defensive_quickdash    = 14,
            pas_defensive_domeshield   = 15,
            pas_mobility_dash_capacity = 16,
            pas_warpfall               = 17,
            pas_bubbleshield           = 18,
            pas_ronin_weapon           = 19,
            pas_northstar_weapon       = 20,
            pas_ion_weapon             = 21,
            pas_tone_weapon            = 22,
            pas_scorch_weapon          = 23,
            pas_legion_weapon          = 24,
            pas_ion_tripwire           = 25,
            pas_ion_vortex             = 26,
            pas_ion_lasercannon        = 27,
            pas_tone_rockets           = 28,
            pas_tone_sonar             = 29,
            pas_tone_wall              = 30,
            pas_ronin_arcwave          = 31,
            pas_ronin_phase            = 32,
            pas_ronin_swordcore        = 33,
            pas_northstar_cluster      = 34,
            pas_northstar_trap         = 35,
            pas_northstar_flightcore   = 36,
            pas_scorch_firewall        = 37,
            pas_scorch_shield          = 38,
            pas_scorch_selfdmg         = 39,
            pas_legion_spinup          = 40,
            pas_legion_gunshield       = 41,
            pas_legion_smartcore       = 42,
            pas_ion_weapon_ads         = 43,
            pas_tone_burst             = 44,
            pas_legion_chargeshot      = 45,
            pas_ronin_autoshift        = 46,
            pas_northstar_optics       = 47,
            pas_scorch_flamecore       = 48,
            pas_vanguard_coremeter     = 49,
            pas_vanguard_shield        = 50,
            pas_vanguard_rearm         = 51,
            pas_vanguard_doom          = 52,
            pas_vanguard_core1         = 53,
            pas_vanguard_core2         = 54,
            pas_vanguard_core3         = 55,
            pas_vanguard_core4         = 56,
            pas_vanguard_core5         = 57,
            pas_vanguard_core6         = 58,
            pas_vanguard_core7         = 59,
            pas_vanguard_core8         = 60,
            pas_vanguard_core9         = 61,
    }
}

pdata_enum! {
    enum LoadoutWeaponsAndAbilities for ["primary", "secondary", "weapon3", "ordnance", "special", "antirodeo"] {
        NULL                                  = 0,
        melee_pilot_emptyhanded               = 1,
        melee_pilot_sword                     = 2,
        melee_titan_sword                     = 3,
        melee_titan_sword_aoe                 = 4,
        mp_ability_cloak                      = 5,
        mp_ability_grapple                    = 6,
        mp_ability_heal                       = 7,
        mp_ability_holopilot                  = 8,
        mp_ability_phase_rewind               = 9,
        mp_ability_shifter                    = 10,
        mp_titanability_ammo_swap             = 11,
        mp_titanability_basic_block           = 12,
        mp_titanability_gun_shield            = 13,
        mp_titanability_hover                 = 14,
        mp_titanability_laser_trip            = 15,
        mp_titanability_particle_wall         = 16,
        mp_titanability_phase_dash            = 17,
        mp_titanability_power_shot            = 18,
        mp_titanability_slow_trap             = 19,
        mp_titanability_smoke                 = 20,
        mp_titanability_sonar_pulse           = 21,
        mp_titanability_tether_trap           = 22,
        mp_titanability_rearm                 = 23,
        mp_titancore_flame_wave               = 24,
        mp_titancore_flight_core              = 25,
        mp_titancore_laser_cannon             = 26,
        mp_titancore_salvo_core               = 27,
        mp_titancore_shift_core               = 28,
        mp_titancore_siege_mode               = 29,
        mp_titancore_upgrade                  = 30,
        mp_titanweapon_40mm                   = 31,
        mp_titanweapon_arc_wave               = 32,
        mp_titanweapon_flame_wall             = 33,
        mp_titanweapon_heat_shield            = 34,
        mp_titanweapon_homing_rockets         = 35,
        mp_titanweapon_dumbfire_rockets       = 36,
        mp_titanweapon_laser_lite             = 37,
        mp_titanweapon_leadwall               = 38,
        mp_titanweapon_meteor                 = 39,
        mp_titanweapon_particle_accelerator   = 40,
        mp_titanweapon_predator_cannon        = 41,
        mp_titanweapon_rocket_launcher        = 42,
        mp_titanweapon_rocketeer_rocketstream = 43,
        mp_titanweapon_salvo_rockets          = 44,
        mp_titanweapon_sniper                 = 45,
        mp_titanweapon_sticky_40mm            = 46,
        mp_titanweapon_stun_laser             = 47,
        mp_titanweapon_tracker_rockets        = 48,
        mp_titanweapon_vortex_shield          = 49,
        mp_titanweapon_vortex_shield_ion      = 50,
        mp_titanweapon_xo16                   = 51,
        mp_titanweapon_xo16_shorty            = 52,
        mp_titanweapon_xo16_vanguard          = 53,
        mp_weapon_alternator_smg              = 54,
        mp_weapon_arc_launcher                = 55,
        mp_weapon_autopistol                  = 56,
        mp_weapon_car                         = 57,
        mp_weapon_defender                    = 58,
        mp_weapon_deployable_cover            = 59,
        mp_weapon_dmr                         = 60,
        mp_weapon_doubletake                  = 61,
        mp_weapon_epg                         = 62,
        mp_weapon_esaw                        = 63,
        mp_weapon_frag_drone                  = 64,
        mp_weapon_frag_grenade                = 65,
        mp_weapon_g2                          = 66,
        mp_weapon_grenade_electric_smoke      = 67,
        mp_weapon_grenade_emp                 = 68,
        mp_weapon_grenade_gravity             = 69,
        mp_weapon_grenade_sonar               = 70,
        mp_weapon_hemlok                      = 71,
        mp_weapon_hemlok_smg                  = 72,
        mp_weapon_lmg                         = 73,
        mp_weapon_lstar                       = 74,
        mp_weapon_mastiff                     = 75,
        mp_weapon_mgl                         = 76,
        mp_weapon_pulse_lmg                   = 77,
        mp_weapon_r97                         = 78,
        mp_weapon_rocket_launcher             = 79,
        mp_weapon_rspn101                     = 80,
        mp_weapon_rspn101_og                  = 81,
        mp_weapon_satchel                     = 82,
        mp_weapon_semipistol                  = 83,
        mp_weapon_shotgun                     = 84,
        mp_weapon_shotgun_pistol              = 85,
        mp_weapon_smart_pistol                = 86,
        mp_weapon_smr                         = 87,
        mp_weapon_sniper                      = 88,
        mp_weapon_softball                    = 89,
        mp_weapon_thermite_grenade            = 90,
        mp_weapon_vinson                      = 91,
        mp_weapon_wingman                     = 92,
        mp_weapon_wingman_n                   = 93,
        melee_titan_punch_ion                 = 94,
        melee_titan_punch_legion              = 95,
        melee_titan_punch_northstar           = 96,
        melee_titan_punch_scorch              = 97,
        melee_titan_punch_tone                = 98,
        melee_titan_punch_vanguard            = 99,
    }
}

pdata_enum! {
    enum PilotSuit for "suit"; {
        medium  = 0,
        geist   = 1,
        stalker = 2,
        light   = 3,
        heavy   = 4,
        grapple = 5,
        nomad   = 6,
    }
}

pdata_enum! {
    enum PilotRace for "race"; {
           race_human_male   = 0,
           race_human_female = 1,
    }
}

pdata_enum! {
    enum PilotPassive for ["passive1", "passive2"] {
        NULL                  = 0,
        pas_stealth_movement  = 1,
        pas_ordnance_pack     = 2,
        pas_power_cell        = 3,
        pas_wallhang          = 4,
        pas_fast_health_regen = 5,
        pas_minimap_ai        = 6,
        pas_longer_bubble     = 7,
        pas_run_and_gun       = 8,
        pas_dead_mans_trigger = 9,
        pas_wall_runner       = 10,
        pas_fast_hack         = 11,
        pas_cloaked_wallrun   = 12,
        pas_cloaked_wallhang  = 13,
        pas_smoke_sight       = 14,
        pas_fast_embark       = 15,
        pas_cdr_on_kill       = 16,
        pas_at_hunter         = 17,
        pas_ordnance_beam     = 18,
        pas_fast_rodeo        = 19,
        pas_phase_eject       = 20,
        pas_ads_hover         = 21,
        pas_enemy_death_icons = 22,
        pas_off_the_grid      = 23,
    }
}

pdata_enum! {
    enum PilotExecution for "execution"; {
           execution_neck_snap  = 0,
           execution_face_stab  = 1,
           execution_backshot   = 2,
           execution_combo      = 3,
           execution_knockout   = 4,
           execution_telefrag   = 5,
           execution_stim       = 6,
           execution_grapple    = 7,
           execution_pulseblade = 8,
           execution_random     = 9,
           execution_cloak      = 10,
           execution_holopilot  = 11,
           execution_ampedwall  = 12,
    }
}

pdata_enum! {
    enum PilotMod for ["primaryAttachment", "primaryMod1", "primaryMod2", "primaryMod3", "secondaryMod1", "secondaryMod2", "secondaryMod3", "weapon3Mod1", "weapon3Mod2", "weapon3Mod3"] {
        NULL                            = 0,
        aog                             = 1,
        automatic_fire                  = 2,
        burn_mod_rspn101                = 3,
        burn_mod_g2                     = 4,
        burn_mod_hemlok                 = 5,
        burn_mod_vinson                 = 6,
        burn_mod_lstar                  = 7,
        burn_mod_car                    = 8,
        burn_mod_r97                    = 9,
        burn_mod_alternator_smg         = 10,
        burn_mod_lmg                    = 11,
        burn_mod_esaw                   = 12,
        burn_mod_pulse_lmg              = 13,
        burn_mod_sniper                 = 14,
        burn_mod_dmr                    = 15,
        burn_mod_doubletake             = 16,
        burn_mod_mastiff                = 17,
        burn_mod_shotgun                = 18,
        burn_mod_softball               = 19,
        burn_mod_shotgun_pistol         = 20,
        burn_mod_autopistol             = 21,
        burn_mod_wingman                = 22,
        burn_mod_semipistol             = 23,
        burn_mod_smart_pistol           = 24,
        burn_mod_emp_grenade            = 25,
        burn_mod_frag_grenade           = 26,
        burn_mod_satchel                = 27,
        burn_mod_proximity_mine         = 28,
        burn_mod_grenade_electric_smoke = 29,
        burn_mod_grenade_gravity        = 30,
        burn_mod_thermite_grenade       = 31,
        burn_mod_defender               = 32,
        burn_mod_rocket_launcher        = 33,
        burn_mod_arc_launcher           = 34,
        burn_mod_smr                    = 35,
        burn_mod_mgl                    = 36,
        burst                           = 37,
        enhanced_targeting              = 38,
        extended_ammo                   = 39,
        fast_lock                       = 40,
        fast_reload                     = 41,
        guided_missile                  = 42,
        hcog                            = 43,
        high_density                    = 44,
        holosight                       = 45,
        iron_sights                     = 46,
        long_fuse                       = 47,
        powered_magnets                 = 48,
        scope_4x                        = 49,
        scope_6x                        = 50,
        scope_8x                        = 51,
        scope_10x                       = 52,
        scope_12x                       = 53,
        silencer                        = 54,
        sniper_assist                   = 55,
        stabilizer                      = 56,
        single_shot                     = 57,
        slammer                         = 58,
        stabilized_warhead              = 59,
        tank_buster                     = 60,
        amped_wall                      = 61,
        short_shift                     = 62,
        burn_mod_epg                    = 63,
        ricochet                        = 64,
        ar_trajectory                   = 65,
        redline_sight                   = 66,
        threat_scope                    = 67,
        smart_lock                      = 68,
        pro_screen                      = 69,
        delayed_shot                    = 70,
        pas_run_and_gun                 = 71,
        tactical_cdr_on_kill            = 72,
        pas_fast_ads                    = 73,
        pas_fast_swap                   = 74,
        pas_fast_reload                 = 75,
        jump_kit                        = 76,
        quick_charge                    = 77,
        rocket_arena                    = 78,
    }
}

pub fn string_to_enum(property: &str, value: &str) -> Option<i32> {
    [
        TitanClasses::check,
        TitanExecution::check,
        TitanIsPrimeTitan::check,
        TitanMod::check,
        TitanPassive::check,
        LoadoutWeaponsAndAbilities::check,
        PilotSuit::check,
        PilotRace::check,
        PilotPassive::check,
        PilotExecution::check,
        PilotMod::check,
    ]
    .into_iter()
    .find_map(|checker| checker(property, value))
}
