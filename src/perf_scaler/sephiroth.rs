use smashline::{
    skyline_smash::{
        app::lua_bind::{StatusModule, WorkModule},
        lib::lua_const::{
            FIGHTER_EDGE_STATUS_KIND_SPECIAL_N_SHOOT,
            FIGHTER_EDGE_STATUS_SPECIAL_N_WORK_INT_CHARGE_KIND,
            FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
        },
    },
    *,
};

use crate::perf_scaler::{
    pop_dynamic_res_report, push_dynamic_res_report, utils::is_valid_fighter_entry_id,
};

static mut GIGAFLARE_ACTIVE: [bool; 8] = [false; 8];

unsafe extern "C" fn sephiroth_init(fighter: &mut L2CFighterCommon) {
    let entry_id = WorkModule::get_int(
        fighter.module_accessor,
        *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    );
    if !is_valid_fighter_entry_id(entry_id) {
        return;
    }
    GIGAFLARE_ACTIVE[entry_id as usize] = false;
}

unsafe extern "C" fn sephiroth_fighter_frame(fighter: &mut L2CFighterCommon) {
    let entry_id = WorkModule::get_int(
        fighter.module_accessor,
        *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    );
    if !is_valid_fighter_entry_id(entry_id) {
        return;
    }

    let status = StatusModule::status_kind(fighter.module_accessor);
    let charge_kind = WorkModule::get_int(
        fighter.module_accessor,
        *FIGHTER_EDGE_STATUS_SPECIAL_N_WORK_INT_CHARGE_KIND,
    );
    let fully_charged_shoot =
        status == *FIGHTER_EDGE_STATUS_KIND_SPECIAL_N_SHOOT && charge_kind >= 3;

    let gigaflare_active = GIGAFLARE_ACTIVE[entry_id as usize];

    if fully_charged_shoot {
        if !gigaflare_active {
            GIGAFLARE_ACTIVE[entry_id as usize] = true;
            println!("[SEPHIROTH_DRS] intensive_frame_start");
            push_dynamic_res_report();
        }
    } else if gigaflare_active {
        GIGAFLARE_ACTIVE[entry_id as usize] = false;
        println!("[SEPHIROTH_DRS] intensive_frame_end");
        pop_dynamic_res_report();
    }
}

pub fn install() {
    Agent::new("edge")
        .on_start(sephiroth_init)
        .on_line(Main, sephiroth_fighter_frame)
        .install();
}
