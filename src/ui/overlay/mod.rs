use crate::{
    input_poll::InputSnapshot,
    net::{
        is_in_valid_online_game, is_valid_online_mode, latency_slider::LatencySliderManager,
        pia::StationExt,
    },
    render::profile::RenderProfileManager,
    ui::{check_overlay_toggle_buttons_pressed, OVERLAY_POLLER},
    utils::is_emulator,
};
use imgui_api::bindings::*;
use ssbu_pia_interface::{NetworkStability, StationConnectionManager};
use std::fmt::Display;
use std::sync::atomic::{AtomicUsize, Ordering};
use ultelier::sync_guest::{self, BufferMode, IndexMode, ResolutionLevel};

static IMGUI_EMPTY_CELL_STR: &str = "---\0";
static IMGUI_INTERACT_TABLE_ROW_NAME_STRS: [&str; 4] =
    ["Name\0", "Ping\0", "NetLatency\0", "NetProfile\0"];
static IMGUI_WINDOW_TITLE: &str = "Player Info\0";
static IMGUI_HIDDEN_WINDOW_TITLE: &str = "##overlay_hidden\0";
static IMGUI_PLAYER_NAME: &str = "You\0";
static IMGUI_TABLE_ID: &str = "conn_player_info\0";
static IMGUI_FPS_TABLE_ID: &str = "fps_info\0";
static IMGUI_DEBUG_TABLE_ID: &str = "render_profile_debug_info\0";
static IMGUI_FPS_TABLE_ROW_NAME_STRS: [&str; 4] =
    ["Ping\0", "Resolution\0", "FPS\0", "FrameTime\0"];
const FRAME_GRAPH_CAPACITY: usize = 180;

static SELECTED_TABLE_ROW: AtomicUsize = AtomicUsize::new(ROW_NET_LATENCY);

const ROW_NAME: usize = 0;
const ROW_PING: usize = 1;
const ROW_NET_LATENCY: usize = 2;
const ROW_RENDER_PROFILE: usize = 3;

const WINDOW_STATE_HIDDEN: usize = 0;
const WINDOW_STATE_INTERACT: usize = 1;
const WINDOW_STATE_PERFORMANCE: usize = 2;
const WINDOW_STATE_DEBUG: usize = 3;
const WINDOW_HEIGHT_INTERACT: f32 = 250.0;
const WINDOW_HEIGHT_PERFORMANCE: f32 = 180.0;
const WINDOW_HEIGHT_DEBUG: f32 = 300.0;
static WINDOW_STATE: AtomicUsize = AtomicUsize::new(WINDOW_STATE_HIDDEN);

const SOURCE_DISP_WIDTH: f32 = 1920.0;
const SOURCE_DISP_HEIGHT: f32 = 1080.0;

#[link(name = "imgui_smash")]
extern "C" {}

unsafe extern "C" fn setup_imgui_context(imgui_ctx: *mut u64) {
    igSetCurrentContext(imgui_ctx as _);
}

fn ping_color_for_stability(stability: NetworkStability) -> ImVec4 {
    match stability {
        NetworkStability::Stable => ImVec4 {
            x: 0.45,
            y: 0.90,
            z: 0.45,
            w: 1.0,
        },
        NetworkStability::Inconsistent => ImVec4 {
            x: 0.95,
            y: 0.85,
            z: 0.25,
            w: 1.0,
        },
        NetworkStability::Unstable => ImVec4 {
            x: 0.95,
            y: 0.35,
            z: 0.35,
            w: 1.0,
        },
    }
}

fn load_color_for_frametime_ms(smooth_ms: f64) -> ImVec4 {
    let load_ratio = smooth_ms / 16.6667;
    if load_ratio <= 1.02 {
        ImVec4 {
            x: 0.45,
            y: 0.90,
            z: 0.45,
            w: 1.0,
        }
    } else if load_ratio <= 1.15 {
        ImVec4 {
            x: 0.95,
            y: 0.85,
            z: 0.25,
            w: 1.0,
        }
    } else {
        ImVec4 {
            x: 0.95,
            y: 0.35,
            z: 0.35,
            w: 1.0,
        }
    }
}

fn as_imgui_text(value: impl Display) -> String {
    format!("{value}\0")
}

unsafe fn draw_text_cell(text: &str) {
    igText(text.as_ptr() as _);
}

unsafe fn draw_empty_cell() {
    draw_text_cell(IMGUI_EMPTY_CELL_STR);
}

unsafe fn draw_hidden_workaround_window() {
    igPushStyleVar_Float(ImGuiStyleVar_WindowBorderSize as i32, 0.0);
    igPushStyleVar_Vec2(
        ImGuiStyleVar_WindowPadding as i32,
        ImVec2 { x: 0.0, y: 0.0 },
    );
    igPushStyleVar_Vec2(
        ImGuiStyleVar_WindowMinSize as i32,
        ImVec2 { x: 0.0, y: 0.0 },
    );
    igSetNextWindowPos(
        ImVec2 { x: 0.0, y: 0.0 },
        ImGuiCond_Always as i32,
        ImVec2 { x: 0.0, y: 0.0 },
    );
    igSetNextWindowSize(ImVec2 { x: 1.0, y: 1.0 }, ImGuiCond_Always as i32);
    igSetNextWindowBgAlpha(0.0);
    let began = igBegin(
        IMGUI_HIDDEN_WINDOW_TITLE.as_ptr() as _,
        std::ptr::null_mut() as _,
        (ImGuiWindowFlags_NoInputs
            | ImGuiWindowFlags_NoNav
            | ImGuiWindowFlags_NoBackground
            | ImGuiWindowFlags_NoDecoration
            | ImGuiWindowFlags_NoSavedSettings
            | ImGuiWindowFlags_NoBringToFrontOnFocus
            | ImGuiWindowFlags_NoFocusOnAppearing) as i32,
    );
    if began {
        let draw_list = igGetWindowDrawList();
        if !draw_list.is_null() {
            let p = igGetWindowPos();
            ImDrawList_AddRectFilled(
                draw_list,
                ImVec2 { x: p.x, y: p.y },
                ImVec2 {
                    x: p.x + 1.0,
                    y: p.y + 1.0,
                },
                0x01000000,
                0.0,
                0,
            );
        }
    }
    igEnd();
    igPopStyleVar(3);
}

unsafe fn begin_table_row(row_idx: usize, label: &str) {
    igTableNextRow(0, 0.0);
    igTableNextColumn();
    let row_label = row_label_with_cursor(row_idx, label);
    draw_text_cell(&row_label);
    igTableNextColumn();
}

fn row_label_with_cursor(row_idx: usize, label: &str) -> String {
    let label = label.trim_end_matches('\0');
    if SELECTED_TABLE_ROW.load(Ordering::SeqCst) == row_idx {
        format!("> {label}\0")
    } else {
        format!("{label}\0")
    }
}

fn move_row_cursor(delta: isize) {
    if delta == 0 {
        return;
    }

    let step = if delta > 0 { 1 } else { -1 };
    let mut next = SELECTED_TABLE_ROW.load(Ordering::SeqCst) as isize;
    let max = (IMGUI_INTERACT_TABLE_ROW_NAME_STRS.len() - 1) as isize;

    loop {
        next = (next + step).clamp(0, max);
        if next == 0 || next == max || is_row_configurable(next as usize) {
            break;
        }
    }

    if is_row_configurable(next as usize) {
        SELECTED_TABLE_ROW.store(next as usize, Ordering::SeqCst);
    }
}

fn is_row_configurable(row: usize) -> bool {
    matches!(row, ROW_NET_LATENCY | ROW_RENDER_PROFILE)
}

fn poll_selected_setting(input_snapshot: &InputSnapshot) {
    let allow_interact = is_valid_online_mode() && !is_in_valid_online_game();
    if !allow_interact {
        return;
    }
    match SELECTED_TABLE_ROW.load(Ordering::SeqCst) {
        ROW_NET_LATENCY => {
            LatencySliderManager::instance().poll(
                input_snapshot,
                ninput::Buttons::LEFT,
                ninput::Buttons::RIGHT,
            );
        }
        ROW_RENDER_PROFILE => {
            RenderProfileManager::instance().poll(
                input_snapshot,
                ninput::Buttons::LEFT,
                ninput::Buttons::RIGHT,
            );
        }
        _ => {}
    }
}

fn get_apparent_game_resolution() -> (u32, u32) {
    match sync_guest::apparent_game_resolution() {
        Some(res) => (res.width, res.height),
        None => (1920, 1080),
    }
}

pub fn get_fixed_width(pos: f32, cur_disp_width: f32) -> f32 {
    if cur_disp_width == SOURCE_DISP_WIDTH {
        pos
    } else {
        let multi = pos / SOURCE_DISP_WIDTH;
        multi * cur_disp_width
    }
}

pub fn get_fixed_height(pos: f32, cur_disp_height: f32) -> f32 {
    if cur_disp_height == SOURCE_DISP_HEIGHT {
        pos
    } else {
        let multi = pos / SOURCE_DISP_HEIGHT;
        multi * cur_disp_height
    }
}

unsafe fn draw_interact_table(first_col_width: f32) {
    let is_valid_online_mode = is_valid_online_mode();
    let stations = StationConnectionManager::get_connected_stations();
    if !igBeginTable(
        IMGUI_TABLE_ID.as_ptr() as _,
        (stations.len() + 2) as i32,
        (ImGuiTableFlags_Borders | ImGuiTableFlags_NoClip) as i32,
        ImVec2 { x: 0.0, y: 0.0 },
        0.0,
    ) {
        return;
    }

    igTableSetupColumn(
        "##label_column\0".as_ptr() as _,
        ImGuiTableColumnFlags_WidthFixed as i32,
        first_col_width,
        0,
    );
    for idx in 1..(stations.len() + 2) {
        let column_name = format!("##value_col_{idx}\0");
        igTableSetupColumn(
            column_name.as_ptr() as _,
            ImGuiTableColumnFlags_WidthStretch as i32,
            1.0,
            0,
        );
    }

    begin_table_row(ROW_NAME, IMGUI_INTERACT_TABLE_ROW_NAME_STRS[ROW_NAME]);
    draw_text_cell(IMGUI_PLAYER_NAME);
    for station in stations.iter() {
        igTableNextColumn();
        if let Some(account_name) = station.get_main_account_name() {
            let account_name = as_imgui_text(account_name);
            draw_text_cell(&account_name);
        }
    }

    begin_table_row(ROW_PING, IMGUI_INTERACT_TABLE_ROW_NAME_STRS[ROW_PING]);
    draw_empty_cell();
    for station in stations.iter() {
        igTableNextColumn();
        if let Some(rtt) = station.get_rtt() {
            let rtt = format!("{rtt} ms\0");
            igPushStyleColor_Vec4(
                ImGuiCol_Text as i32,
                ping_color_for_stability(station.get_network_stability()),
            );
            draw_text_cell(&rtt);
            igPopStyleColor(1);
        } else {
            draw_empty_cell();
        }
    }

    begin_table_row(
        ROW_NET_LATENCY,
        IMGUI_INTERACT_TABLE_ROW_NAME_STRS[ROW_NET_LATENCY],
    );
    if is_valid_online_mode {
        let latency = LatencySliderManager::instance()
            .active_latency()
            .unwrap_or_else(|| LatencySliderManager::instance().selected_latency());
        let slider_val = as_imgui_text(latency.to_string());
        draw_text_cell(&slider_val);
    } else {
        draw_empty_cell();
    }
    for station in stations.iter() {
        igTableNextColumn();
        if let Some(latency) = station.get_latency() {
            let slider_val = as_imgui_text(latency.to_string());
            draw_text_cell(&slider_val);
        } else {
            draw_empty_cell();
        }
    }

    begin_table_row(
        ROW_RENDER_PROFILE,
        IMGUI_INTERACT_TABLE_ROW_NAME_STRS[ROW_RENDER_PROFILE],
    );
    if is_valid_online_mode {
        let rp = match is_in_valid_online_game() {
            true => RenderProfileManager::active_render_profile(),
            false => RenderProfileManager::instance().selected_render_profile(),
        };
        let rp_str = match RenderProfileManager::instance().is_auto_mode() {
            true => format!("Auto({})", rp),
            false => rp.to_string(),
        };
        let rp_disp = as_imgui_text(rp_str);
        draw_text_cell(&rp_disp);
    } else {
        draw_empty_cell();
    }
    for station in stations.iter() {
        igTableNextColumn();
        if let Some(rp) = station.get_render_profile() {
            let rp_disp = as_imgui_text(rp);
            draw_text_cell(&rp_disp);
        } else {
            draw_empty_cell();
        }
    }

    igEndTable();
}

unsafe fn draw_performance_table(
    first_col_width: f32,
    res_w: u32,
    res_h: u32,
    show_avg_overall_ping: bool,
    avg_overall_ping_ms: Option<u64>,
) {
    if igBeginTable(
        IMGUI_FPS_TABLE_ID.as_ptr() as _,
        2,
        ImGuiTableFlags_Borders as i32,
        ImVec2 { x: 0.0, y: 0.0 },
        0.0,
    ) {
        igTableSetupColumn(
            "##fps_label_column\0".as_ptr() as _,
            ImGuiTableColumnFlags_WidthFixed as i32,
            first_col_width,
            0,
        );
        igTableSetupColumn(
            "##fps_value_column\0".as_ptr() as _,
            ImGuiTableColumnFlags_WidthStretch as i32,
            1.0,
            0,
        );

        let smooth_ms = sync_guest::smooth_frametime_ms().unwrap_or(0.0);
        let load_color = load_color_for_frametime_ms(smooth_ms);

        if show_avg_overall_ping {
            igTableNextRow(0, 0.0);
            igTableNextColumn();
            igText(IMGUI_FPS_TABLE_ROW_NAME_STRS[0].as_ptr() as _);
            igTableNextColumn();
            let network_stability = StationConnectionManager::get_network_stability();
            let avg_ping = match avg_overall_ping_ms {
                Some(ms) => format!("{} ms\0", ms),
                None => IMGUI_EMPTY_CELL_STR.to_string(),
            };
            if avg_overall_ping_ms.is_some() {
                igPushStyleColor_Vec4(
                    ImGuiCol_Text as i32,
                    ping_color_for_stability(network_stability),
                );
            }
            igText(avg_ping.as_ptr() as _);
            if avg_overall_ping_ms.is_some() {
                igPopStyleColor(1);
            }
        }

        igTableNextRow(0, 0.0);
        igTableNextColumn();
        igText(IMGUI_FPS_TABLE_ROW_NAME_STRS[1].as_ptr() as _);
        igTableNextColumn();
        let resolution = format!("{}x{}\0", res_w, res_h);
        igText(resolution.as_ptr() as _);

        igTableNextRow(0, 0.0);
        igTableNextColumn();
        igText(IMGUI_FPS_TABLE_ROW_NAME_STRS[2].as_ptr() as _);
        igTableNextColumn();
        let smooth_fps = format!("{}\0", sync_guest::smooth_fps().unwrap_or(0));
        igPushStyleColor_Vec4(ImGuiCol_Text as i32, load_color);
        igText(smooth_fps.as_ptr() as _);
        igPopStyleColor(1);

        igTableNextRow(0, 50.0);
        igTableNextColumn();
        igText(IMGUI_FPS_TABLE_ROW_NAME_STRS[3].as_ptr() as _);
        igTableNextColumn();

        static mut FRAME_GRAPH_VALUES: [f32; FRAME_GRAPH_CAPACITY] = [0.0; FRAME_GRAPH_CAPACITY];
        #[allow(static_mut_refs)]
        let values_count = sync_guest::copy_frametime_history(unsafe { &mut FRAME_GRAPH_VALUES })
            .unwrap_or(0)
            .min(FRAME_GRAPH_CAPACITY);
        let graph_overlay = format!("{smooth_ms:.2} ms\0");
        let graph_size = igGetContentRegionAvail();

        igPushStyleColor_Vec4(ImGuiCol_PlotLines as i32, load_color);
        igPushStyleColor_Vec4(ImGuiCol_PlotLinesHovered as i32, load_color);
        igPlotLines_FloatPtr(
            "##frametime_graph\0".as_ptr() as _,
            unsafe { FRAME_GRAPH_VALUES.as_ptr() },
            values_count as i32,
            0,
            graph_overlay.as_ptr() as _,
            0.0,
            35.0,
            graph_size,
            std::mem::size_of::<f32>() as i32,
        );
        igPopStyleColor(2);

        igEndTable();
    }
}

unsafe fn draw_debug_table(first_col_width: f32) {
    if !igBeginTable(
        IMGUI_DEBUG_TABLE_ID.as_ptr() as _,
        2,
        ImGuiTableFlags_Borders as i32,
        ImVec2 { x: 0.0, y: 0.0 },
        0.0,
    ) {
        return;
    }

    igTableSetupColumn(
        "##debug_label_column\0".as_ptr() as _,
        ImGuiTableColumnFlags_WidthFixed as i32,
        first_col_width,
        0,
    );
    igTableSetupColumn(
        "##debug_value_column\0".as_ptr() as _,
        ImGuiTableColumnFlags_WidthStretch as i32,
        1.0,
        0,
    );

    let draw_row_str = |label: &str, value: Option<String>| {
        igTableNextRow(0, 0.0);
        igTableNextColumn();
        igText(label.as_ptr() as _);
        igTableNextColumn();
        match value {
            None => draw_empty_cell(),
            Some(v) => draw_text_cell(format!("{}\0", v).as_str()),
        };
    };

    let draw_row_bool = |label: &str, value: bool| {
        let value = if value { "Enabled" } else { "Disabled" };
        draw_row_str(label, Some(value.to_string()));
    };

    let platform = match is_emulator() {
        true => String::from("Emulator"),
        false => String::from("Console"),
    };
    draw_row_str("Platform\0", Some(platform));

    draw_row_str(
        "Active Profile\0",
        Some(RenderProfileManager::active_render_profile().to_string()),
    );
    draw_row_str(
        "Active Latency\0",
        LatencySliderManager::instance()
            .active_latency()
            .and_then(|v| Some(v.to_string())),
    );
    draw_row_str(
        "BufferMode\0",
        Some(format!(
            "{:?}",
            sync_guest::buffer_mode()
                .flatten()
                .unwrap_or(BufferMode::Triple)
        )),
    );
    draw_row_str(
        "IndexMode\0",
        Some(format!(
            "{:?}",
            sync_guest::index_mode()
                .flatten()
                .unwrap_or(IndexMode::TwoBehind)
        )),
    );
    let default_res = sync_guest::default_game_resolution_level()
        .flatten()
        .unwrap_or(ResolutionLevel::Res1920x1080);
    let (w, h) = default_res.to_values();
    draw_row_bool("Vsync\0", sync_guest::vsync_enabled().unwrap_or(false));
    draw_row_bool(
        "RenderOpts\0",
        sync_guest::render_opts_enabled().unwrap_or(false),
    );
    draw_row_bool(
        "DynamicRes\0",
        sync_guest::dynamic_resolution_enabled().unwrap_or(false),
    );
    draw_row_str("DefaultRes\0", Some(format!("{}x{}", w, h)));
    igEndTable();
}

unsafe extern "C" fn draw() {
    OVERLAY_POLLER.poll();
    let input_snapshot = OVERLAY_POLLER.snapshot();
    let toggle_window_buttons = check_overlay_toggle_buttons_pressed(&input_snapshot);

    let magic_pressed =
        input_snapshot.is_pressed_any(ninput::Buttons::MINUS | ninput::Buttons::PLUS);

    if !toggle_window_buttons.is_empty()
        && input_snapshot.is_pressed_with_cooldown(toggle_window_buttons)
    {
        toggle_window(magic_pressed);
    }

    let window_state = WINDOW_STATE.load(Ordering::SeqCst);
    if window_state == WINDOW_STATE_HIDDEN {
        // Workaround: submit a invisible window to avoid draw hook spam when no draw calls submitted.
        draw_hidden_workaround_window();
        return;
    }

    let performance_view = window_state == WINDOW_STATE_PERFORMANCE;
    let interact_view = window_state == WINDOW_STATE_INTERACT;
    let debug_view = window_state == WINDOW_STATE_DEBUG;

    let connected_stations = StationConnectionManager::num_connected_stations();
    let width_station_count = if performance_view || debug_view {
        0
    } else {
        connected_stations
    };
    let window_width = ((width_station_count + 2) as f32 * 100.0).max(300.0);
    let window_height = if performance_view {
        WINDOW_HEIGHT_PERFORMANCE
    } else if debug_view {
        WINDOW_HEIGHT_DEBUG
    } else {
        WINDOW_HEIGHT_INTERACT
    };
    let (res_w, res_h) = get_apparent_game_resolution();
    let scaled_window_width = get_fixed_width(window_width, res_w as f32);
    let scaled_window_height = get_fixed_height(window_height, res_h as f32);
    let scaled_font_size = get_fixed_height(16.0, res_h as f32);
    let first_col_width = get_fixed_width(100.0, res_w as f32);
    let show_avg_overall_ping = performance_view && connected_stations > 0;
    let avg_overall_ping_ms = if show_avg_overall_ping {
        StationConnectionManager::get_rtt()
    } else {
        None
    };

    igSetNextWindowPos(
        ImVec2 { x: 0.0, y: 0.0 },
        ImGuiCond_Always as i32,
        ImVec2 { x: 0.0, y: 0.0 },
    );

    igSetNextWindowSize(
        ImVec2 {
            x: scaled_window_width,
            y: scaled_window_height,
        },
        ImGuiCond_Always as i32,
    );

    let flags = ImGuiWindowFlags_NoMouseInputs
        | ImGuiWindowFlags_NoNavInputs
        | ImGuiWindowFlags_NoNavFocus
        | ImGuiWindowFlags_NoCollapse
        | ImGuiWindowFlags_NoMove;

    igPushFont(std::ptr::null_mut(), scaled_font_size);

    if !igBegin(
        IMGUI_WINDOW_TITLE.as_ptr() as _,
        std::ptr::null_mut() as _,
        flags as i32,
    ) {
        igPopFont();
        igEnd();
        return;
    }

    if interact_view {
        let menu_btns_pressed = input_snapshot
            .check_buttons_pressed_with_cooldown(&[ninput::Buttons::UP, ninput::Buttons::DOWN]);
        match menu_btns_pressed {
            ninput::Buttons::UP => move_row_cursor(-1),
            ninput::Buttons::DOWN => move_row_cursor(1),
            _ => {}
        }
        poll_selected_setting(&input_snapshot);
        draw_interact_table(first_col_width);
    } else if debug_view {
        draw_debug_table(first_col_width);
    }

    draw_performance_table(
        first_col_width,
        res_w,
        res_h,
        show_avg_overall_ping,
        avg_overall_ping_ms,
    );

    igEnd();
    igPopFont();
}

pub fn toggle_window(magic_pressed: bool) {
    let mut current = WINDOW_STATE.load(Ordering::SeqCst);
    loop {
        let next = match (current, magic_pressed) {
            (WINDOW_STATE_HIDDEN, _) => WINDOW_STATE_INTERACT,
            (WINDOW_STATE_INTERACT, false) => WINDOW_STATE_PERFORMANCE,
            (WINDOW_STATE_INTERACT, true) => WINDOW_STATE_DEBUG,
            (WINDOW_STATE_DEBUG, _) => WINDOW_STATE_PERFORMANCE,
            (WINDOW_STATE_PERFORMANCE, _) => WINDOW_STATE_HIDDEN,
            _ => WINDOW_STATE_HIDDEN,
        };
        match WINDOW_STATE.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}

pub fn set_window_open(show: bool) {
    let next_state = if show {
        WINDOW_STATE_INTERACT
    } else {
        WINDOW_STATE_HIDDEN
    };
    WINDOW_STATE.store(next_state, Ordering::SeqCst);
}

pub fn is_window_open() -> bool {
    WINDOW_STATE.load(Ordering::SeqCst) != WINDOW_STATE_HIDDEN
}

pub fn is_window_interactable() -> bool {
    WINDOW_STATE.load(Ordering::SeqCst) == WINDOW_STATE_INTERACT
}

pub(super) fn install() {
    imgui_api::imgui_setup_context(setup_imgui_context);
    imgui_api::imgui_smash_add_on_draw_frame(draw as _);
}
