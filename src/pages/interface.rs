use std::fs;
use std::io::Write;
use crate::app::{AppAction, PageContent};
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{
    ColorSelector, Spinbox, Element, Dropdown, TextBox, FontSelector, Toggle, MultiControl,
    Slider
};

const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.json";
const LINKS_PATH: &str = "/home/lsgalante/.config/cce/cce-system-settings/links.json";

thread_local! {
    pub(crate) static TEST_CONFIG_PATH: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    static TEST_LINKS_PATH: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

fn get_links_path_val() -> String {
    #[cfg(test)]
    {
        TEST_LINKS_PATH.with(|p| {
            if let Some(path) = p.borrow().as_ref() {
                return path.clone();
            }
            LINKS_PATH.to_string()
        })
    }
    #[cfg(not(test))]
    {
        LINKS_PATH.to_string()
    }
}

pub(crate) fn get_config_path() -> String {
    #[cfg(test)]
    {
        TEST_CONFIG_PATH.with(|p| {
            if let Some(path) = p.borrow().as_ref() {
                return path.clone();
            }
            CONFIG_PATH.to_string()
        })
    }
    #[cfg(not(test))]
    {
        CONFIG_PATH.to_string()
    }
}


fn perform_rolling_backup(path: &str) {
    if path != CONFIG_PATH {
        return;
    }
    if !std::path::Path::new(path).exists() {
        return;
    }
    let backup_dir = "/home/lsgalante/.config/cce/backups";
    if let Err(_) = fs::create_dir_all(backup_dir) {
        return;
    }
    for i in (1..=4).rev() {
        let src = format!("{}/config.json.{}.bak", backup_dir, i);
        let dst = format!("{}/config.json.{}.bak", backup_dir, i + 1);
        if std::path::Path::new(&src).exists() {
            let _ = fs::rename(src, dst);
        }
    }
    let dst = format!("{}/config.json.1.bak", backup_dir);
    let _ = fs::copy(path, dst);
}

fn safe_write(path: &str, content: &str) -> bool {
    perform_rolling_backup(path);
    let temp_path = format!("{}.tmp", path);
    if fs::write(&temp_path, content).is_ok() {
        if fs::rename(&temp_path, path).is_ok() {
            return true;
        }
        let _ = fs::remove_file(&temp_path);
    }
    false
}

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/cce-{}.sock", display),
        Err(_) => "/tmp/cce.sock".to_string(),
    }
}



#[derive(Debug, Clone)]
pub struct WindowsState {
    pub cascade_offset: u16,
    pub edge_gap: u16,
    pub top_gap: u16,
    pub status_height: u16,
    pub transition_duration: u16,
    pub cascade_offset_spinbox: Spinbox,
    pub edge_gap_spinbox: Spinbox,
    pub top_gap_spinbox: Spinbox,
    pub status_height_spinbox: Spinbox,
    pub transition_duration_spinbox: Spinbox,
    pub side_panel_behavior_menu: Dropdown,
    pub side_panel_position_menu: Dropdown,
    pub side_panel_width: u16,
    pub side_panel_width_spinbox: Spinbox,
    pub side_panel_border_gap: u16,
    pub side_panel_border_gap_spinbox: Spinbox,
    pub blur_enabled: bool,
    pub blur_toggle: Toggle,
    pub fullscreen_opacity: f32,
    pub fullscreen_opacity_spinbox: Spinbox,
    pub cascade_opacity: f32,
    pub cascade_opacity_spinbox: Spinbox,
    pub grid_opacity: f32,
    pub grid_opacity_spinbox: Spinbox,
    pub floating_opacity: f32,
    pub floating_opacity_spinbox: Spinbox,
    pub pinned_opacity: f32,
    pub pinned_opacity_spinbox: Spinbox,
    pub popup_opacity: f32,
    pub popup_opacity_spinbox: Spinbox,
}

impl Default for WindowsState {
    fn default() -> Self {
        Self {
            cascade_offset: 20,
            edge_gap: 48,
            top_gap: 48,
            status_height: 24,
            transition_duration: 300,
            cascade_offset_spinbox: Spinbox::new(20, 0, 200, 1).with_config(CONFIG_PATH, "cascade_offset"),
            edge_gap_spinbox: Spinbox::new(48, 0, 200, 1).with_config(CONFIG_PATH, "edge_gap"),
            top_gap_spinbox: Spinbox::new(48, 0, 200, 1).with_config(CONFIG_PATH, "gap_top"),
            status_height_spinbox: Spinbox::new(24, 0, 100, 1).with_config(CONFIG_PATH, "bar_height"),
            transition_duration_spinbox: Spinbox::new(300, 0, 2000, 50).with_config(CONFIG_PATH, "transition_duration"),
            side_panel_behavior_menu: Dropdown::new(
                vec!["Above".to_string(), "Inline".to_string()],
                1,
            ).with_label("Behavior")
            .with_config(CONFIG_PATH, "side_panel_behavior"),
            side_panel_position_menu: Dropdown::new(
                vec!["Left".to_string(), "Right".to_string()],
                0,
            ).with_label("Position")
            .with_config(CONFIG_PATH, "side_panel_position"),
            side_panel_width: 360,
            side_panel_width_spinbox: Spinbox::new(360, 0, 2000, 10).with_config(CONFIG_PATH, "side_panel_width"),
            side_panel_border_gap: 0,
            side_panel_border_gap_spinbox: Spinbox::new(0, 0, 500, 1).with_config(CONFIG_PATH, "side_panel_border_gap"),
            blur_enabled: true,
            blur_toggle: Toggle::new().with_label("Blur").with_config(CONFIG_PATH, "window_blur"),
            fullscreen_opacity: 0.95,
            fullscreen_opacity_spinbox: Spinbox::new(95, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "fullscreen_backplate_opacity"),
            cascade_opacity: 0.05,
            cascade_opacity_spinbox: Spinbox::new(5, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "cascade_backplate_opacity"),
            grid_opacity: 0.05,
            grid_opacity_spinbox: Spinbox::new(5, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "grid_backplate_opacity"),
            floating_opacity: 0.9,
            floating_opacity_spinbox: Spinbox::new(90, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "floating_backplate_opacity"),
            pinned_opacity: 0.05,
            pinned_opacity_spinbox: Spinbox::new(5, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "pinned_backplate_opacity"),
            popup_opacity: 0.20,
            popup_opacity_spinbox: Spinbox::new(20, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "popup_backplate_opacity"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum WindowsMessage {
    SetCascadeOffset(u16),
    SetEdgeGap(u16),
    SetTopGap(u16),
    SetTransitionDuration(u16),
    SetStatusHeight(u16),
    SetSidePanelBehavior(usize),
    SetSidePanelPosition(usize),
    SetSidePanelWidth(u16),
    SetSidePanelBorderGap(u16),
    SetFullscreenOpacity(u16),
    SetCascadeOpacity(u16),
    SetGridOpacity(u16),
    SetFloatingOpacity(u16),
    SetPinnedOpacity(u16),
    SetPopupOpacity(u16),
    ToggleBlur,
    Refreshed(WindowsState),
}

#[derive(Debug, Clone)]
pub struct StatusInterfaceControls {
    pub box_opacity: f32,
    pub box_blur: f32,
    pub box_opacity_slider: Slider,
    pub box_blur_slider: Slider,
    pub opacity_dragging: bool,
    pub blur_dragging: bool,
}

impl Default for StatusInterfaceControls {
    fn default() -> Self {
        let opacity = read_status_box_opacity().unwrap_or(1.0);
        let blur = read_status_box_blur().unwrap_or(0.0);
        Self {
            box_opacity: opacity,
            box_blur: blur,
            box_opacity_slider: Slider::new().with_range(0.0, 100.0).with_scroll(true).with_value(opacity).with_label("Background Opacity").with_config(CONFIG_PATH, "status_box_opacity"),
            box_blur_slider: Slider::new().with_range(0.0, 100.0).with_scroll(true).with_value(blur).with_label("Background Blur").with_config(CONFIG_PATH, "status_box_blur"),
            opacity_dragging: false,
            blur_dragging: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceState {
    pub windows: WindowsState,
    pub desktop_background_color: [u8; 3],
    pub desktop_grid_color: [u8; 4],
    pub desktop_grid_scale: u16,
    pub desktop_grid_scale_spinbox: Spinbox,
    pub desktop_line_width: u16,
    pub desktop_line_width_spinbox: Spinbox,
    pub high_color: [u8; 3],
    pub disabled_color: [u8; 3],
    pub visual_guides_color: [u8; 3],
    pub slider_track_color: [u8; 3],
    pub page_low_color: [u8; 3],
    pub color_borders_color: [u8; 3],
    pub normal_color: [u8; 3],
    pub paginator_sidebar_color: [u8; 3],
    pub primary_highlight_color: [u8; 3],
    pub menubar_tab_label_color: [u8; 3],
    pub toggle_enabled_color: [u8; 3],
    pub toggle_disabled_color: [u8; 3],
    pub toggle_bg_color: [u8; 3],
    pub scrollinglist_bg_color: [u8; 3],
    pub scrollinglist_entry_bg_color: [u8; 4],
    pub scrollinglist_entry_highlight_color: [u8; 4],
    pub breadcrumb_bg_color: [u8; 3],
    pub popover_bg_color: [u8; 3],
    pub color_selectors: Vec<ColorSelector>,
    pub menubar_opacity: f32,
    pub menubar_opacity_spinbox: Spinbox,
    pub notification_bg_color: [u8; 3],
    pub notification_opacity: f32,
    pub notification_opacity_spinbox: Spinbox,
    pub backplate_color: [u8; 3],
    pub backplate_corner_radius: u16,
    pub backplate_corner_radius_spinbox: Spinbox,
    pub paginator_tab_padding_x: u16,
    pub paginator_tab_padding_y: u16,
    pub button_padding: u16,
    pub button_padding_spinbox: Spinbox,
    pub button_strip_spacing: u16,
    pub button_strip_spacing_spinbox: Spinbox,
    pub section_padding: u16,
    pub section_padding_spinbox: Spinbox,
    pub plate_padding: u16,
    pub plate_padding_spinbox: Spinbox,
    pub plate_opacity: f32,
    pub plate_opacity_spinbox: Spinbox,
    pub plate_corner_radius: u16,
    pub plate_corner_radius_spinbox: Spinbox,
    pub page_color: [u8; 3],
    pub page_opacity: f32,
    pub page_opacity_spinbox: Spinbox,
    pub layer_color: [u8; 3],
    pub layer_opacity: f32,
    pub layer_opacity_spinbox: Spinbox,

    pub page_margin: u16,
    pub page_margin_spinbox: Spinbox,
    pub grid_min_col_width: u16,
    pub grid_min_col_width_spinbox: Spinbox,
    pub layout_grid_gap: u16,
    pub layout_grid_gap_spinbox: Spinbox,
    pub spinbox_height: u16,
    pub spinbox_height_spinbox: Spinbox,
    pub spinbox_corner_radius: u16,
    pub spinbox_corner_radius_spinbox: Spinbox,
    pub toggle_height: u16,
    pub toggle_height_spinbox: Spinbox,
    pub toggle_corner_radius: u16,
    pub toggle_corner_radius_spinbox: Spinbox,
    pub toggle_border_width: u16,
    pub toggle_border_width_spinbox: Spinbox,

    pub color_selector_height: u16,
    pub color_selector_height_spinbox: Spinbox,
    pub color_selector_corner_radius: u16,
    pub color_selector_corner_radius_spinbox: Spinbox,
    pub color_selector_preview_corner_radius: u16,
    pub color_selector_preview_corner_radius_spinbox: Spinbox,
    pub color_selector_preview_margin: u16,
    pub color_selector_preview_margin_spinbox: Spinbox,
    pub textbox_height: u16,
    pub textbox_height_spinbox: Spinbox,
    pub textbox_corner_radius: u16,
    pub textbox_corner_radius_spinbox: Spinbox,
    pub slider_height: u16,
    pub slider_height_spinbox: Spinbox,
    pub slider_corner_radius: u16,
    pub slider_corner_radius_spinbox: Spinbox,
    pub font_selector_height: u16,
    pub font_selector_height_spinbox: Spinbox,
    pub font_selector_corner_radius: u16,
    pub font_selector_corner_radius_spinbox: Spinbox,
    pub dropdown_height: u16,
    pub dropdown_height_spinbox: Spinbox,
    pub dropdown_corner_radius: u16,
    pub dropdown_corner_radius_spinbox: Spinbox,
    pub button_corner_radius: u16,
    pub button_corner_radius_spinbox: Spinbox,
    pub status_box_background_color: [u8; 3],
    pub status_box_corner_radius: u16,
    pub status_box_corner_radius_spinbox: Spinbox,
    pub status_padding: u16,
    pub status_padding_spinbox: Spinbox,
    pub status_module_spacing: u16,
    pub status_module_spacing_spinbox: Spinbox,
    pub status_controls: StatusInterfaceControls,
    pub nested_section_label_alignment: u8,
    pub label_alignment_menu: Dropdown,
    pub nested_section_label_offset: i16,
    pub label_offset_spinbox: Spinbox,
    pub label_margin: u16,
    pub label_margin_spinbox: Spinbox,
    // Typeface state fields
    pub typeface_loaded: bool,
    pub sans_serif: String,
    pub serif: String,
    pub monospace: String,
    pub window_borders: String,
    pub status_interface: String,
    pub fuzzel: String,
    pub terminal: String,
    pub all_fonts: Vec<String>,
    pub mono_fonts: Vec<String>,
    pub sans_box: TextBox,
    pub serif_box: TextBox,
    pub mono_box: TextBox,
    pub borders_box: TextBox,
    pub status_box: TextBox,
    pub fuzzel_box: TextBox,
    pub terminal_box: TextBox,
    pub borders_menu: Dropdown,
    pub status_menu: Dropdown,
    pub fuzzel_menu: Dropdown,
    pub terminal_menu: Dropdown,
    pub borders_size_box: Spinbox,
    pub status_size_box: Spinbox,
    pub fuzzel_size_box: Spinbox,
    pub terminal_size_box: Spinbox,
    pub color_selector_font: String,
    pub color_selector_font_selector: FontSelector,
    pub menubar_font: String,
    pub menubar_font_selector: FontSelector,
    pub section_label_font: String,
    pub section_label_font_selector: FontSelector,
    pub nested_section_label_font: String,
    pub nested_section_label_font_selector: FontSelector,
    pub breadcrumb_font: String,
    pub breadcrumb_font_selector: FontSelector,
    pub toggle_font: String,
    pub toggle_font_selector: FontSelector,
    pub font_selector_font: String,
    pub font_selector_font_selector: FontSelector,
    pub button_strip_font: String,
    pub button_strip_font_selector: FontSelector,
    pub button_font: String,
    pub button_font_selector: FontSelector,
    pub label_font: String,
    pub label_font_selector: FontSelector,
    pub dropdown_font: String,
    pub dropdown_font_selector: FontSelector,
    pub textbox_font: String,
    pub textbox_font_selector: FontSelector,
    pub spinbox_font: String,
    pub spinbox_font_selector: FontSelector,
    pub slider_font: String,
    pub slider_font_selector: FontSelector,
    // Graph configuration fields
    pub graph_show_grid: bool,
    pub graph_show_grid_toggle: Toggle,
    pub graph_snap_enabled: bool,
    pub graph_snap_enabled_toggle: Toggle,
    pub graph_uniform_background: bool,
    pub graph_uniform_background_toggle: Toggle,
    pub graph_cell_opacity: f32,
    pub graph_cell_opacity_spinbox: Spinbox,
    pub graph_gap_opacity: f32,
    pub graph_gap_opacity_spinbox: Spinbox,
    pub graph_gap_width: u16,
    pub graph_gap_width_spinbox: Spinbox,
    pub custom_multicontrol: MultiControl,
}

impl Default for InterfaceState {
    fn default() -> Self {
        Self {
            windows: WindowsState::default(),
            desktop_background_color: [0, 0, 0],
            desktop_grid_color: [255, 255, 255, 13],
            desktop_grid_scale: 100,
            desktop_grid_scale_spinbox: Spinbox::new(100, 5, 1000, 5).with_label("Grid Cell Size").with_config(CONFIG_PATH, "desktop_grid_scale"),
            desktop_line_width: 1,
            desktop_line_width_spinbox: Spinbox::new(1, 0, 100, 1).with_label("Grid Gap").with_unit("px").with_config(CONFIG_PATH, "desktop_line_width"),
            high_color: [0x3e, 0x3e, 0x3e],
            disabled_color: [0x55, 0x55, 0x55],
            visual_guides_color: [0xff, 0x8c, 0x00],
            slider_track_color: [116, 116, 128],
            page_low_color: [71, 71, 81],
            color_borders_color: [124, 124, 137],
            normal_color: [0xcc, 0xcc, 0xd8],
            paginator_sidebar_color: [90, 90, 101],
            primary_highlight_color: [255, 255, 255],
            menubar_tab_label_color: [230, 230, 242],
            toggle_enabled_color: [104, 217, 165],
            toggle_disabled_color: [135, 135, 148],
            toggle_bg_color: [116, 116, 128],
            scrollinglist_bg_color: [81, 81, 97],
            scrollinglist_entry_bg_color: [255, 255, 255, 10],
            scrollinglist_entry_highlight_color: [255, 255, 255, 204],
            breadcrumb_bg_color: [81, 81, 97],
            popover_bg_color: [81, 81, 97],
            color_selectors: vec![
                ColorSelector::new([71, 71, 81]).with_label("Low Color").with_config(CONFIG_PATH, "page_low_color"), // 0: Plate - Low Color
                ColorSelector::new([0x3e, 0x3e, 0x3e]).with_label("High Color").with_config(CONFIG_PATH, "high_color"), // 1: Layout - High Color
                ColorSelector::new([0xff, 0x8c, 0x00]).with_label("Visual Guides").with_config(CONFIG_PATH, "visual_guides_color"), // 2: Layout - Visual Guides
                ColorSelector::new([0x55, 0x55, 0x55]).with_label("Disabled").with_config(CONFIG_PATH, "disabled_color"), // 3: Status - Disabled
                ColorSelector::new([116, 116, 128]).with_label("Slider Track").with_config(CONFIG_PATH, "slider_track_color"), // 4: Controls - Slider Track
                ColorSelector::new([124, 124, 137]).with_label("Borders").with_config(CONFIG_PATH, "color_borders_color"), // 6: Controls - Borders
                ColorSelector::new([0, 0, 0]).with_label("Background Color").with_config(CONFIG_PATH, "desktop_background_color"), // 7: Surfaces - Desktop Background Color
                ColorSelector::new([0xcc, 0xcc, 0xd8]).with_label("Normal").with_config(CONFIG_PATH, "status_normal_color"), // 8: Status - Normal
                ColorSelector::new([90, 90, 101]).with_label("Background").with_config(CONFIG_PATH, "paginator_sidebar_color"), // 9: Controls - Paginator Sidebar (now Background)
                ColorSelector::new([255, 255, 255]).with_label("Primary Highlight").with_config(CONFIG_PATH, "primary_highlight_color"), // 10: Controls - Primary Highlight
                ColorSelector::new([230, 230, 242]).with_label("Tab Label").with_config(CONFIG_PATH, "menubar_tab_label_color"), // 11: Controls - Tab Label
                ColorSelector::new([104, 217, 165]).with_label("Enabled").with_config(CONFIG_PATH, "toggle_enabled_color"), // 12: Toggles - Enabled
                ColorSelector::new([135, 135, 148]).with_label("Disabled").with_config(CONFIG_PATH, "toggle_disabled_color"), // 13: Toggles - Disabled
                ColorSelector::new([81, 81, 97]).with_label("Background").with_config(CONFIG_PATH, "scrollinglist_bg_color"), // 14: ScrollingList - Background
                ColorSelector::new([81, 81, 97]).with_label("Background").with_config(CONFIG_PATH, "breadcrumb_bg_color"), // 15: Breadcrumb - Background
                ColorSelector::new([81, 81, 97]).with_label("Background").with_config(CONFIG_PATH, "popover_bg_color"), // 16: Popover - Background
                ColorSelector::new([0x08, 0x08, 0x0c]).with_label("Background").with_config(CONFIG_PATH, "notification_bg_color"), // 17: Notification - Background
                ColorSelector::new([0x0a, 0x1a, 0x0e]).with_label("Color").with_config(CONFIG_PATH, "backplate_color"), // 18: Surfaces - Backplate Color
                ColorSelector::new([0, 0, 0]).with_label("Page Color").with_config(CONFIG_PATH, "page_color"), // 19: Containers - Page Color
                ColorSelector::new([0, 0, 0]).with_label("Layer Color").with_config(CONFIG_PATH, "layer_color"), // 20: Containers - Layer Color
                ColorSelector::new_rgba([255, 255, 255, 10]).with_label("Entry Background").with_config(CONFIG_PATH, "scrollinglist_entry_bg_color"), // 21: ScrollingList - Entry Background
                ColorSelector::new_rgba([255, 255, 255, 204]).with_label("Entry Highlight").with_config(CONFIG_PATH, "scrollinglist_entry_highlight_color"), // 22: ScrollingList - Entry Highlight
                ColorSelector::new([0x15, 0x15, 0x20]).with_label("Background Color").with_config(CONFIG_PATH, "status_box_background_color"), // 23: Status - Background Color
                ColorSelector::new_rgba([255, 255, 255, 13]).with_label("Grid Color").with_config(CONFIG_PATH, "desktop_grid_color"), // 24: Surfaces - Desktop Grid Color
                ColorSelector::new([116, 116, 128]).with_label("Background").with_config(CONFIG_PATH, "toggle_bg_color"), // 24: Toggles - Background
            ],
            paginator_tab_padding_x: 10,
            paginator_tab_padding_y: 14,
            button_padding: 14,
            button_padding_spinbox: Spinbox::new(14, 0, 100, 1).with_label("Button Padding").with_unit("px").with_config(CONFIG_PATH, "button_padding"),
            button_strip_spacing: 8,
            button_strip_spacing_spinbox: Spinbox::new(8, 0, 100, 1).with_label("Spacing").with_unit("px").with_config(CONFIG_PATH, "button_strip_spacing"),
            section_padding: 8,
            section_padding_spinbox: Spinbox::new(8, 0, 100, 1).with_label("Padding").with_unit("px").with_config(CONFIG_PATH, "section_padding"),
            plate_padding: 20,
            plate_padding_spinbox: Spinbox::new(20, 0, 100, 1).with_label("Padding").with_unit("px").with_config(CONFIG_PATH, "plate_padding"),
            plate_opacity: 1.0,
            plate_opacity_spinbox: Spinbox::new(100, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "plate_opacity"),
            plate_corner_radius: 12,
            plate_corner_radius_spinbox: Spinbox::new(12, 0, 50, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "plate_corner_radius"),
            page_color: [0, 0, 0],
            page_opacity: 1.0,
            page_opacity_spinbox: Spinbox::new(100, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "page_opacity"),
            layer_color: [0, 0, 0],
            layer_opacity: 1.0,
            layer_opacity_spinbox: Spinbox::new(100, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "layer_opacity"),

            page_margin: 20,
            page_margin_spinbox: Spinbox::new(20, 0, 100, 1).with_label("Page Margin").with_unit("px").with_config(CONFIG_PATH, "page_margin"),
            grid_min_col_width: 260,
            grid_min_col_width_spinbox: Spinbox::new(260, 100, 1000, 10).with_label("Minimum Width").with_unit("px").with_config(CONFIG_PATH, "grid_min_col_width"),
            layout_grid_gap: 8,
            layout_grid_gap_spinbox: Spinbox::new(8, 0, 100, 1).with_label("Gap").with_unit("px").with_config(CONFIG_PATH, "grid_gap"),
            spinbox_height: 26,
            spinbox_height_spinbox: Spinbox::new(26, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "spinbox_height"),
            spinbox_corner_radius: 4,
            spinbox_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Border Radius").with_unit("px").with_config(CONFIG_PATH, "spinbox_corner_radius"),
            toggle_height: 44,
            toggle_height_spinbox: Spinbox::new(44, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "toggle_height"),
            toggle_corner_radius: 4,
            toggle_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "toggle_corner_radius"),
            toggle_border_width: 1,
            toggle_border_width_spinbox: Spinbox::new(1, 0, 10, 1).with_label("Border Width").with_unit("px").with_config(CONFIG_PATH, "toggle_border_width"),

            color_selector_height: 22,
            color_selector_height_spinbox: Spinbox::new(22, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "color_selector_height"),
            color_selector_corner_radius: 4,
            color_selector_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Border Radius").with_unit("px").with_config(CONFIG_PATH, "color_selector_corner_radius"),
            color_selector_preview_corner_radius: 4,
            color_selector_preview_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Preview Corner Radius").with_unit("px").with_config(CONFIG_PATH, "color_selector_preview_corner_radius"),
            color_selector_preview_margin: 0,
            color_selector_preview_margin_spinbox: Spinbox::new(0, 0, 20, 1).with_label("Preview Margin").with_unit("px").with_config(CONFIG_PATH, "color_selector_preview_margin"),
            textbox_height: 44,
            textbox_height_spinbox: Spinbox::new(44, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "textbox_height"),
            textbox_corner_radius: 4,
            textbox_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Border Radius").with_unit("px").with_config(CONFIG_PATH, "textbox_corner_radius"),
            slider_height: 28,
            slider_height_spinbox: Spinbox::new(28, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "slider_height"),
            slider_corner_radius: 4,
            slider_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "slider_corner_radius"),
            font_selector_height: 44,
            font_selector_height_spinbox: Spinbox::new(44, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "font_selector_height"),
            font_selector_corner_radius: 4,
            font_selector_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Border Radius").with_unit("px").with_config(CONFIG_PATH, "font_selector_corner_radius"),
            dropdown_height: 44,
            dropdown_height_spinbox: Spinbox::new(44, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "dropdown_height"),
            dropdown_corner_radius: 4,
            dropdown_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Radius").with_unit("px").with_config(CONFIG_PATH, "dropdown_corner_radius"),
            button_corner_radius: 4,
            button_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Radius").with_unit("px").with_config(CONFIG_PATH, "button_corner_radius"),
            nested_section_label_alignment: 0,
            label_alignment_menu: Dropdown::new(
                vec!["Left".to_string(), "Center".to_string(), "Right".to_string()],
                0,
            ).with_label("Label Alignment")
            .with_config(CONFIG_PATH, "nested_section_label_alignment"),
            nested_section_label_offset: 0,
            label_offset_spinbox: Spinbox::new(0, -100, 100, 1).with_label("Label Offset").with_unit("px").with_config(CONFIG_PATH, "nested_section_label_offset"),
            label_margin: 6,
            label_margin_spinbox: Spinbox::new(6, 0, 100, 1).with_label("Label Margin").with_unit("px").with_config(CONFIG_PATH, "label_margin"),
            typeface_loaded: false,
            sans_serif: String::new(),
            serif: String::new(),
            monospace: String::new(),
            window_borders: String::new(),
            status_interface: String::new(),
            fuzzel: String::new(),
            terminal: String::new(),
            all_fonts: Vec::new(),
            mono_fonts: Vec::new(),
            sans_box: TextBox::new(String::new()).with_label("Sans-Serif").with_config(FONTS_CONF_PATH, "sans-serif"),
            serif_box: TextBox::new(String::new()).with_label("Serif").with_config(FONTS_CONF_PATH, "serif"),
            mono_box: TextBox::new(String::new()).with_label("Monospace").with_config(FONTS_CONF_PATH, "monospace"),
            borders_box: TextBox::new(String::new()).with_label("Active Font").with_config(FONTS_CONF_PATH, "window_borders"),
            status_box: TextBox::new(String::new()).with_label("Active Font").with_config(FONTS_CONF_PATH, "status_interface"),
            fuzzel_box: TextBox::new(String::new()).with_label("Active Font").with_config(FONTS_CONF_PATH, "fuzzel"),
            terminal_box: TextBox::new(String::new()).with_label("Active Font").with_config(FONTS_CONF_PATH, "terminal"),
            borders_menu: Dropdown::new(Vec::new(), 0).with_config(FONTS_CONF_PATH, "window_borders"),
            status_menu: Dropdown::new(Vec::new(), 0).with_config(FONTS_CONF_PATH, "status_interface"),
            fuzzel_menu: Dropdown::new(Vec::new(), 0).with_config(FONTS_CONF_PATH, "fuzzel"),
            terminal_menu: Dropdown::new(Vec::new(), 0).with_config(FONTS_CONF_PATH, "terminal"),
            borders_size_box: Spinbox::new(14, 6, 72, 1).with_config(CONFIG_PATH, "border_font_size"),
            status_size_box: Spinbox::new(14, 6, 72, 1).with_config(CONFIG_PATH, "status_font_size"),
            fuzzel_size_box: Spinbox::new(14, 6, 72, 1).with_config("/home/lsgalante/.config/fuzzel/fuzzel.ini", "size"),
            terminal_size_box: Spinbox::new(14, 6, 72, 1).with_config("/home/lsgalante/.config/foot/foot.ini", "size"),
            color_selector_font: "monospace".to_string(),
            color_selector_font_selector: FontSelector::new("monospace".to_string()).with_label("Color Label").with_config(CONFIG_PATH, "color_selector_font"),
            menubar_font: "Outfit".to_string(),
            menubar_font_selector: FontSelector::new("Outfit".to_string()).with_label("Menu Label").with_config(CONFIG_PATH, "menubar_font"),
            section_label_font: "Outfit".to_string(),
            section_label_font_selector: FontSelector::new("Outfit".to_string()).with_label("Label").with_config(CONFIG_PATH, "section_label_font"),
            nested_section_label_font: "Outfit".to_string(),
            nested_section_label_font_selector: FontSelector::new("Outfit".to_string()).with_label("Label").with_config(CONFIG_PATH, "nested_section_label_font"),
            breadcrumb_font: "Outfit".to_string(),
            breadcrumb_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font").with_config(CONFIG_PATH, "breadcrumb_font"),
            toggle_font: "Outfit".to_string(),
            toggle_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font").with_config(CONFIG_PATH, "toggle_font"),
            font_selector_font: "Outfit".to_string(),
            font_selector_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font").with_config(CONFIG_PATH, "font_selector_font"),
            button_strip_font: "Outfit".to_string(),
            button_strip_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font").with_config(CONFIG_PATH, "button_strip_font"),
            button_font: "Outfit".to_string(),
            button_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font").with_config(CONFIG_PATH, "button_font"),
            label_font: "Outfit".to_string(),
            label_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font").with_config(CONFIG_PATH, "label_font"),
            dropdown_font: "Outfit".to_string(),
            dropdown_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font").with_config(CONFIG_PATH, "dropdown_font"),
            textbox_font: "Outfit".to_string(),
            textbox_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font").with_config(CONFIG_PATH, "textbox_font"),
            spinbox_font: "monospace".to_string(),
            spinbox_font_selector: FontSelector::new("monospace".to_string()).with_label("Font").with_config(CONFIG_PATH, "spinbox_font"),
            slider_font: "Outfit".to_string(),
            slider_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font").with_config(CONFIG_PATH, "slider_font"),
            graph_show_grid: true,
            graph_show_grid_toggle: Toggle::new().with_label("Show Grid").with_config(CONFIG_PATH, "graph_show_grid"),
            graph_snap_enabled: true,
            graph_snap_enabled_toggle: Toggle::new().with_label("Grid Snapping").with_config(CONFIG_PATH, "graph_snap_enabled"),
            graph_uniform_background: false,
            graph_uniform_background_toggle: Toggle::new().with_label("Uniform Background").with_config(CONFIG_PATH, "graph_uniform_background"),
            graph_cell_opacity: 0.95,
            graph_cell_opacity_spinbox: Spinbox::new(95, 0, 100, 5).with_label("Cell Opacity").with_unit("%").with_config(CONFIG_PATH, "graph_cell_opacity"),
            graph_gap_opacity: 0.95,
            graph_gap_opacity_spinbox: Spinbox::new(95, 0, 100, 5).with_label("Gap Opacity").with_unit("%").with_config(CONFIG_PATH, "graph_gap_opacity"),
            graph_gap_width: 35,
            graph_gap_width_spinbox: Spinbox::new(35, 0, 100, 1).with_label("Gap Width").with_unit("px").with_config(CONFIG_PATH, "graph_gap_width"),
            menubar_opacity: 0.9,
            menubar_opacity_spinbox: Spinbox::new(90, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "menubar_opacity"),
            notification_bg_color: [0x08, 0x08, 0x0c],
            notification_opacity: 0.9,
            notification_opacity_spinbox: Spinbox::new(90, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "notification_opacity"),
            backplate_color: [0x0a, 0x1a, 0x0e],
            backplate_corner_radius: 12,
            backplate_corner_radius_spinbox: Spinbox::new(12, 0, 100, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "backplate_corner_radius"),
            status_box_background_color: [0x15, 0x15, 0x20],
            status_box_corner_radius: 4,
            status_box_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "status_box_corner_radius"),
            status_padding: 8,
            status_padding_spinbox: Spinbox::new(8, 0, 32, 1).with_label("Padding").with_unit("px").with_config(CONFIG_PATH, "status_padding"),
            status_module_spacing: 8,
            status_module_spacing_spinbox: Spinbox::new(8, 0, 100, 1).with_label("Spacing").with_unit("px").with_config(CONFIG_PATH, "status_module_spacing"),
            status_controls: StatusInterfaceControls::default(),
            custom_multicontrol: MultiControl::new("custom_parameters".to_string()).with_label("custom_parameters"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InterfaceMessage {
    Windows(WindowsMessage),
    SetDesktopBackground([u8; 3]),
    SetDesktopGridColor([u8; 4]),
    SetDesktopGridScale(u16),
    SetDesktopLineWidth(u16),
    SetHighColor([u8; 3]),
    SetDisabledColor([u8; 3]),
    SetVisualGuidesColor([u8; 3]),
    SetSliderTrackColor([u8; 3]),
    SetPageLowColor([u8; 3]),
    SetColorBordersColor([u8; 3]),
    SetNormalColor([u8; 3]),
    SetPaginatorSidebarColor([u8; 3]),
    SetPrimaryHighlightColor([u8; 3]),
    SetMenubarTabLabelColor([u8; 3]),
    SetToggleEnabledColor([u8; 3]),
    SetToggleDisabledColor([u8; 3]),
    SetScrollingListBgColor([u8; 3]),
    SetScrollingListEntryBgColor([u8; 4]),
    SetScrollingListEntryHighlightColor([u8; 4]),
    SetBreadcrumbBgColor([u8; 3]),
    SetPopoverBgColor([u8; 3]),
    SetNotificationBgColor([u8; 3]),
    SetNotificationOpacity(f32),
    SetBackplateColor([u8; 3]),
    SetBackplateCornerRadius(u16),
    SetButtonPadding(u16),
    SetButtonStripSpacing(u16),
    SetSectionPadding(u16),
    SetPlatePadding(u16),
    SetPlateOpacity(f32),
    SetPlateCornerRadius(u16),
    SetPageColor([u8; 3]),
    SetPageOpacity(f32),
    SetLayerColor([u8; 3]),
    SetLayerOpacity(f32),
    SetStatusBoxBackgroundColor([u8; 3]),
    SetStatusBoxCornerRadius(u16),
    SetStatusPadding(u16),
    SetStatusModuleSpacing(u16),

    SetPageMargin(u16),
    SetGridMinColWidth(u16),
    SetLayoutGridGap(u16),
    SetSpinboxHeight(u16),
    SetSpinboxCornerRadius(u16),
    SetToggleHeight(u16),
    SetToggleCornerRadius(u16),
    SetToggleBgColor([u8; 3]),
    SetToggleBorderWidth(u16),
    SetToggleFont(String),

    SetColorSelectorHeight(u16),
    SetColorSelectorCornerRadius(u16),
    SetColorSelectorPreviewCornerRadius(u16),
    SetColorSelectorPreviewMargin(u16),
    SetTextboxHeight(u16),
    SetTextboxCornerRadius(u16),
    SetSliderHeight(u16),
    SetSliderCornerRadius(u16),
    SetFontSelectorHeight(u16),
    SetFontSelectorCornerRadius(u16),
    SetFontSelectorFont(String),
    SetButtonStripFont(String),
    SetButtonFont(String),
    SetLabelFont(String),
    SetDropdownFont(String),
    SetTextboxFont(String),
    SetSpinboxFont(String),
    SetSliderFont(String),
    SetDropdownHeight(u16),
    SetDropdownCornerRadius(u16),
    SetButtonCornerRadius(u16),
    SetColorSelectorFont(String),
    SetMenubarFont(String),
    SetBreadcrumbFont(String),
    SetSectionLabelFont(String),
    SetNestedSectionLabelFont(String),
    SetNestedSectionLabelAlignment(usize),
    SetNestedSectionLabelOffset(i16),
    SetLabelMargin(u16),
    SetGraphShowGrid(bool),
    SetGraphSnapEnabled(bool),
    SetGraphUniformBackground(bool),
    SetGraphCellOpacity(f32),
    SetGraphGapOpacity(f32),
    SetGraphGapWidth(u16),
    SetMenubarOpacity(f32),
    PickLowColor,
    PickHighColor,
    PickDisabledColor,
    PickVisualGuides,
    PickSliderTrackColor,
    PickPageLowColor,
    PickColorBordersColor,
    PickNormalColor,
    PickPaginatorSidebarColor,
    PickPrimaryHighlightColor,
    PickMenubarTabLabelColor,
    PickToggleEnabledColor,
    PickToggleDisabledColor,
    PickScrollingListBgColor,
    PickScrollingListEntryBgColor,
    PickScrollingListEntryHighlightColor,
    PickBreadcrumbBgColor,
    PickPopoverBgColor,
    PickNotificationBgColor,
    PickWindowColor,
    PickPageColor,
    PickLayerColor,
    Refreshed(InterfaceState),
    TypefaceRefreshed(InterfaceState),
    StatusSetPadding(u16),
    StatusSetBoxOpacity(f32),
    StatusSetBoxBlur(f32),
    SetSans(String),
    SetSerif(String),
    SetMono(String),
    SetBorders(String),
    SetStatus(String),
    SetFuzzel(String),
    SetTerminal(String),
    SetBordersMenu(usize),
    SetStatusMenu(usize),
    SetFuzzelMenu(usize),
    SetTerminalMenu(usize),
    SetBordersSize(i32),
    SetStatusSize(i32),
    SetFuzzelSize(i32),
    SetTerminalSize(i32),
}

pub fn read_interface_config() -> InterfaceState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let bg = parse_surfaces_color(&content, "desktop_background", parse_color_from_key(&content, "low_color", [0, 0, 0]));
    
    let has_high = content.lines().any(|l| l.trim().starts_with("high_color"));
    let border = if has_high {
        parse_color_from_key(&content, "high_color", [0x3e, 0x3e, 0x3e])
    } else {
        parse_color_from_key(&content, "border_color", [0x3e, 0x3e, 0x3e])
    };
    
    let disabled = parse_color_from_key(&content, "disabled_color", [0x55, 0x55, 0x55]);
    
    let visual_guides = parse_color_from_key(&content, "visual_guides_color", [0xff, 0x8c, 0x00]);
    
    let slider_track = parse_color_from_key(&content, "slider_track_color", [116, 116, 128]);

    let page_low = parse_color_from_key(&content, "page_low_color", [71, 71, 81]);

    let color_borders = parse_color_from_key(&content, "color_borders_color", [124, 124, 137]);

    let normal = parse_color_from_key(&content, "status_normal_color", [0xcc, 0xcc, 0xd8]);

    let paginator_sidebar = parse_color_from_key(&content, "paginator_sidebar_color", [90, 90, 101]);

    let primary_highlight = parse_color_from_key(&content, "primary_highlight_color", [255, 255, 255]);

    let menubar_tab_label = parse_color_from_key(&content, "menubar_tab_label_color",
        parse_color_from_key(&content, "paginator_tab_label_color", [230, 230, 242])
    );

    let toggle_enabled = parse_color_from_key(&content, "toggle_enabled_color", [104, 217, 165]);

    let toggle_disabled = parse_color_from_key(&content, "toggle_disabled_color", [135, 135, 148]);

    let toggle_bg = parse_color_from_key(&content, "toggle_bg_color", [116, 116, 128]);

    let scrollinglist_bg = parse_color_from_key(&content, "scrollinglist_bg_color", [81, 81, 97]);
    let scrollinglist_entry_bg = parse_rgba_color_from_key(&content, "scrollinglist_entry_bg_color", [255, 255, 255, 10]);
    let scrollinglist_entry_highlight = parse_rgba_color_from_key(&content, "scrollinglist_entry_highlight_color", [255, 255, 255, 204]);

    let breadcrumb_bg = parse_color_from_key(&content, "breadcrumb_bg_color", scrollinglist_bg);

    let popover_bg = parse_color_from_key(&content, "popover_bg_color", scrollinglist_bg);
    
    let paginator_tab_padding_x = parse_u16_from(&content, "paginator_tab_padding_x", 10);
    let paginator_tab_padding_y = parse_u16_from(&content, "paginator_tab_padding_y", 14);
    let button_padding = parse_u16_from(&content, "button_padding", paginator_tab_padding_y);
    let button_strip_spacing = parse_u16_from(&content, "button_strip_spacing", 8);
    let section_padding = parse_u16_from(&content, "section_padding", 8);
    let plate_padding = parse_u16_from(&content, "plate_padding", 20);
    let plate_opacity = parse_f32_from(&content, "plate_opacity", 1.0);
    let plate_corner_radius = parse_u16_from(&content, "plate_corner_radius", 12);
    let page_color = parse_color_from_key(&content, "page_color", [0, 0, 0]);
    let page_opacity = parse_f32_from(&content, "page_opacity", 1.0);
    let layer_color = parse_color_from_key(&content, "layer_color", [0, 0, 0]);
    let layer_opacity = parse_f32_from(&content, "layer_opacity", 1.0);

    let page_margin = parse_u16_from(&content, "page_margin", 20);
    let grid_min_col_width = parse_u16_from(&content, "grid_min_col_width", 260);
    let layout_grid_gap = parse_u16_from(&content, "grid_gap", 8);
    let spinbox_height = parse_u16_from(&content, "spinbox_height", 26);
    let spinbox_corner_radius = parse_u16_from(&content, "spinbox_corner_radius", 4);
    let toggle_height = parse_u16_from(&content, "toggle_height", 44);
    let toggle_corner_radius = parse_u16_from(&content, "toggle_corner_radius", 4);
    let toggle_border_width = parse_u16_from(&content, "toggle_border_width", 1);

    let color_selector_height = parse_u16_from(&content, "color_selector_height", 22);
    let color_selector_corner_radius = parse_u16_from(&content, "color_selector_corner_radius", 4);
    let color_selector_preview_corner_radius = parse_u16_from(&content, "color_selector_preview_corner_radius", 4);
    let color_selector_preview_margin = parse_u16_from(&content, "color_selector_preview_margin", 0);
    let textbox_height = parse_u16_from(&content, "textbox_height", 44);
    let textbox_corner_radius = parse_u16_from(&content, "textbox_corner_radius", 4);
    let slider_height = parse_u16_from(&content, "slider_height", 28);
    let slider_corner_radius = parse_u16_from(&content, "slider_corner_radius", 4);
    let font_selector_height = parse_u16_from(&content, "font_selector_height", 44);
    let font_selector_corner_radius = parse_u16_from(&content, "font_selector_corner_radius", 4);
    let dropdown_height = parse_u16_from(&content, "dropdown_height", 44);
    let dropdown_corner_radius = parse_u16_from(&content, "dropdown_corner_radius", 4);
    let button_corner_radius = parse_u16_from(&content, "button_corner_radius", 4);
    let color_selector_font = parse_string_from(&content, "color_selector_font", "monospace");
    let menubar_font = parse_string_from(&content, "menubar_font", "Outfit");
    let section_label_font = parse_string_from(&content, "section_label_font", "Outfit");
    let nested_section_label_font = parse_string_from(&content, "nested_section_label_font", "Outfit");
    let breadcrumb_font = parse_string_from(&content, "breadcrumb_font", "Outfit");
    let toggle_font = parse_string_from(&content, "toggle_font", "Outfit");
    let font_selector_font = parse_string_from(&content, "font_selector_font", "Outfit");
    cce_ui::layout::set_font_selector_font(&font_selector_font);
    let button_strip_font = parse_string_from(&content, "button_strip_font", "Outfit");
    cce_ui::layout::set_button_strip_font(&button_strip_font);
    let button_font = parse_string_from(&content, "button_font", "Outfit");
    cce_ui::layout::set_button_font(&button_font);
    let label_font = parse_string_from(&content, "label_font", "Outfit");
    cce_ui::layout::set_label_font(&label_font);
    let dropdown_font = parse_string_from(&content, "dropdown_font", "Outfit");
    cce_ui::layout::set_dropdown_font(&dropdown_font);
    let textbox_font = parse_string_from(&content, "textbox_font", "Outfit");
    cce_ui::layout::set_textbox_font(&textbox_font);
    let spinbox_font = parse_string_from(&content, "spinbox_font", "monospace");
    cce_ui::layout::set_spinbox_font(&spinbox_font);
    let slider_font = parse_string_from(&content, "slider_font", "Outfit");
    cce_ui::layout::set_slider_font(&slider_font);

    cce_ui::layout::set_toggle_border_width(toggle_border_width as f32);
    cce_ui::layout::set_toggle_font(&toggle_font);
    cce_ui::color::set_toggle_bg_color(cce_ui::color::to_linear([
        toggle_bg[0] as f32 / 255.0,
        toggle_bg[1] as f32 / 255.0,
        toggle_bg[2] as f32 / 255.0,
        1.0,
    ]));
    let nested_section_label_alignment = parse_u16_from(&content, "nested_section_label_alignment", 0) as u8;
    let nested_section_label_offset = parse_i16_from(&content, "nested_section_label_offset", 0);
    let label_margin = parse_u16_from(&content, "label_margin", 6);
    let graph_show_grid = parse_bool_from(&content, "graph_show_grid", true);
    let graph_snap_enabled = parse_bool_from(&content, "graph_snap_enabled", true);
    let graph_uniform_background = parse_bool_from(&content, "graph_uniform_background", false);
    let legacy_opacity = parse_f32_from(&content, "graph_network_opacity", 0.95);
    let graph_cell_opacity = parse_f32_from(&content, "graph_cell_opacity", legacy_opacity);
    let graph_gap_opacity = parse_f32_from(&content, "graph_gap_opacity", legacy_opacity);
    let graph_gap_width = parse_u16_from(&content, "graph_gap_width", 35);
    let menubar_opacity = parse_f32_from(&content, "menubar_opacity", 0.90);
    let notification_bg_color = parse_notifications_color(&content, "bg_color", [0x08, 0x08, 0x0c]);
    let notification_opacity = parse_notifications_opacity(&content);
    let backplate_color = parse_surfaces_color(&content, "backplate_color", [0x0a, 0x1a, 0x0e]);
    let backplate_corner_radius = parse_surfaces_u16(&content, "backplate_corner_radius", 12);
    let status_box_background_color = parse_color_from_key(&content, "status_box_background_color", [0x15, 0x15, 0x20]);
    let status_box_corner_radius = parse_u16_from(&content, "status_box_corner_radius", 4);
    let status_padding = parse_u16_from(&content, "status_padding", 8);
    let status_module_spacing = parse_u16_from(&content, "status_module_spacing", 8);
    
    let desktop_grid_color = parse_surfaces_rgba_color(&content, "desktop_grid_color", [255, 255, 255, 13]);
    let desktop_grid_scale = parse_surfaces_u16(&content, "desktop_grid_scale", 100);
    let desktop_line_width = parse_surfaces_u16(&content, "desktop_line_width", 1);
    
    InterfaceState {
        windows: read_windows_config(),
        desktop_background_color: bg,
        desktop_grid_color,
        desktop_grid_scale,
        desktop_grid_scale_spinbox: Spinbox::new(desktop_grid_scale as i32, 5, 1000, 5).with_label("Grid Cell Size").with_config(CONFIG_PATH, "desktop_grid_scale"),
        desktop_line_width,
        desktop_line_width_spinbox: Spinbox::new(desktop_line_width as i32, 0, 100, 1).with_label("Grid Gap").with_unit("px").with_config(CONFIG_PATH, "desktop_line_width"),
        high_color: border,
        disabled_color: disabled,
        visual_guides_color: visual_guides,
        slider_track_color: slider_track,
        page_low_color: page_low,
        color_borders_color: color_borders,
        normal_color: normal,
        paginator_sidebar_color: paginator_sidebar,
        primary_highlight_color: primary_highlight,
        menubar_tab_label_color: menubar_tab_label,
        toggle_enabled_color: toggle_enabled,
        toggle_disabled_color: toggle_disabled,
        toggle_bg_color: toggle_bg,
        scrollinglist_bg_color: scrollinglist_bg,
        scrollinglist_entry_bg_color: scrollinglist_entry_bg,
        scrollinglist_entry_highlight_color: scrollinglist_entry_highlight,
        breadcrumb_bg_color: breadcrumb_bg,
        popover_bg_color: popover_bg,
        color_selectors: vec![
            ColorSelector::new(page_low).with_label("Low Color").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "page_low_color"), // 0: Plate - Low Color
            ColorSelector::new(border).with_label("High Color").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "high_color"), // 1: Layout - High Color
            ColorSelector::new(visual_guides).with_label("Visual Guides").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "visual_guides_color"), // 2: Layout - Visual Guides
            ColorSelector::new(disabled).with_label("Disabled").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "disabled_color"), // 3: Status - Disabled
            ColorSelector::new(slider_track).with_label("Slider Track").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "slider_track_color"), // 4: Controls - Slider Track
            ColorSelector::new(color_borders).with_label("Borders").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "color_borders_color"), // 6: Controls - Borders
            ColorSelector::new(bg).with_label("Background Color").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "desktop_background_color"), // 7: Layout - Low Color
            ColorSelector::new(normal).with_label("Normal").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "status_normal_color"), // 8: Status - Normal
            ColorSelector::new(paginator_sidebar).with_label("Background").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "paginator_sidebar_color"), // 9: Controls - Paginator Sidebar (now Background)
            ColorSelector::new(primary_highlight).with_label("Primary Highlight").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "primary_highlight_color"), // 10: Controls - Primary Highlight
            ColorSelector::new(menubar_tab_label).with_label("Tab Label").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "menubar_tab_label_color"), // 11: Controls - Tab Label
            ColorSelector::new(toggle_enabled).with_label("Enabled").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "toggle_enabled_color"), // 12: Toggles - Enabled
            ColorSelector::new(toggle_disabled).with_label("Disabled").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "toggle_disabled_color"), // 13: Toggles - Disabled
            ColorSelector::new(scrollinglist_bg).with_label("Background").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "scrollinglist_bg_color"), // 14: ScrollingList - Background
            ColorSelector::new(breadcrumb_bg).with_label("Background").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "breadcrumb_bg_color"), // 15: Breadcrumb - Background
            ColorSelector::new(popover_bg).with_label("Background").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "popover_bg_color"), // 16: Popover - Background
            ColorSelector::new(notification_bg_color).with_label("Background").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "notification_bg_color"), // 17: Notification - Background
            ColorSelector::new(backplate_color).with_label("Color").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "backplate_color"), // 18: Surfaces - Backplate Color
            ColorSelector::new(page_color).with_label("Page Color").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "page_color"), // 19: Containers - Page Color
            ColorSelector::new(layer_color).with_label("Layer Color").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "layer_color"), // 20: Containers - Layer Color
            ColorSelector::new_rgba(scrollinglist_entry_bg).with_label("Entry Background").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "scrollinglist_entry_bg_color"), // 21: ScrollingList - Entry Background
            ColorSelector::new_rgba(scrollinglist_entry_highlight).with_label("Entry Highlight").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "scrollinglist_entry_highlight_color"), // 22: ScrollingList - Entry Highlight
            ColorSelector::new(status_box_background_color).with_label("Background Color").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "status_box_background_color"), // 23: Status - Background Color
            ColorSelector::new_rgba(desktop_grid_color).with_label("Grid Color").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "desktop_grid_color"), // 24: Surfaces - Desktop Grid Color
            ColorSelector::new(toggle_bg).with_label("Background").with_font_family(&color_selector_font).with_config(CONFIG_PATH, "toggle_bg_color"), // 24: Toggles - Background
        ],
        paginator_tab_padding_x,
        paginator_tab_padding_y,
        button_padding,
        button_padding_spinbox: Spinbox::new(button_padding as i32, 0, 100, 1).with_label("Button Padding").with_unit("px").with_config(CONFIG_PATH, "button_padding"),
        button_strip_spacing,
        button_strip_spacing_spinbox: Spinbox::new(button_strip_spacing as i32, 0, 100, 1).with_label("Spacing").with_unit("px").with_config(CONFIG_PATH, "button_strip_spacing"),
        section_padding,
        section_padding_spinbox: Spinbox::new(section_padding as i32, 0, 100, 1).with_label("Padding").with_unit("px").with_config(CONFIG_PATH, "section_padding"),
        plate_padding,
        plate_padding_spinbox: Spinbox::new(plate_padding as i32, 0, 100, 1).with_label("Padding").with_unit("px").with_config(CONFIG_PATH, "plate_padding"),
        plate_opacity,
        plate_opacity_spinbox: Spinbox::new((plate_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "plate_opacity"),
        plate_corner_radius,
        plate_corner_radius_spinbox: Spinbox::new(plate_corner_radius as i32, 0, 50, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "plate_corner_radius"),
        page_color,
        page_opacity,
        page_opacity_spinbox: Spinbox::new((page_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "page_opacity"),
        layer_color,
        layer_opacity,
        layer_opacity_spinbox: Spinbox::new((layer_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "layer_opacity"),

        page_margin,
        page_margin_spinbox: Spinbox::new(page_margin as i32, 0, 100, 1).with_label("Page Margin").with_unit("px").with_config(CONFIG_PATH, "page_margin"),
        grid_min_col_width,
        grid_min_col_width_spinbox: Spinbox::new(grid_min_col_width as i32, 100, 1000, 10).with_label("Minimum Width").with_unit("px").with_config(CONFIG_PATH, "grid_min_col_width"),
        layout_grid_gap,
        layout_grid_gap_spinbox: Spinbox::new(layout_grid_gap as i32, 0, 100, 1).with_label("Gap").with_unit("px").with_config(CONFIG_PATH, "grid_gap"),
        spinbox_height,
        spinbox_height_spinbox: Spinbox::new(spinbox_height as i32, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "spinbox_height"),
        spinbox_corner_radius,
        spinbox_corner_radius_spinbox: Spinbox::new(spinbox_corner_radius as i32, 0, 50, 1).with_label("Border Radius").with_unit("px").with_config(CONFIG_PATH, "spinbox_corner_radius"),
        toggle_height,
        toggle_height_spinbox: Spinbox::new(toggle_height as i32, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "toggle_height"),
        toggle_corner_radius,
        toggle_corner_radius_spinbox: Spinbox::new(toggle_corner_radius as i32, 0, 50, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "toggle_corner_radius"),
        toggle_border_width,
        toggle_border_width_spinbox: Spinbox::new(toggle_border_width as i32, 0, 10, 1).with_label("Border Width").with_unit("px").with_config(CONFIG_PATH, "toggle_border_width"),

        color_selector_height,
        color_selector_height_spinbox: Spinbox::new(color_selector_height as i32, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "color_selector_height"),
        color_selector_corner_radius,
        color_selector_corner_radius_spinbox: Spinbox::new(color_selector_corner_radius as i32, 0, 50, 1).with_label("Border Radius").with_unit("px").with_config(CONFIG_PATH, "color_selector_corner_radius"),
        color_selector_preview_corner_radius,
        color_selector_preview_corner_radius_spinbox: Spinbox::new(color_selector_preview_corner_radius as i32, 0, 50, 1).with_label("Preview Corner Radius").with_unit("px").with_config(CONFIG_PATH, "color_selector_preview_corner_radius"),
        color_selector_preview_margin,
        color_selector_preview_margin_spinbox: Spinbox::new(color_selector_preview_margin as i32, 0, 20, 1).with_label("Preview Margin").with_unit("px").with_config(CONFIG_PATH, "color_selector_preview_margin"),
        textbox_height,
        textbox_height_spinbox: Spinbox::new(textbox_height as i32, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "textbox_height"),
        textbox_corner_radius,
        textbox_corner_radius_spinbox: Spinbox::new(textbox_corner_radius as i32, 0, 50, 1).with_label("Border Radius").with_unit("px").with_config(CONFIG_PATH, "textbox_corner_radius"),
        slider_height,
        slider_height_spinbox: Spinbox::new(slider_height as i32, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "slider_height"),
        slider_corner_radius,
        slider_corner_radius_spinbox: Spinbox::new(slider_corner_radius as i32, 0, 50, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "slider_corner_radius"),
        font_selector_height,
        font_selector_height_spinbox: Spinbox::new(font_selector_height as i32, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "font_selector_height"),
        font_selector_corner_radius,
        font_selector_corner_radius_spinbox: Spinbox::new(font_selector_corner_radius as i32, 0, 50, 1).with_label("Border Radius").with_unit("px").with_config(CONFIG_PATH, "font_selector_corner_radius"),
        dropdown_height,
        dropdown_height_spinbox: Spinbox::new(dropdown_height as i32, 10, 100, 1).with_label("Height").with_unit("px").with_config(CONFIG_PATH, "dropdown_height"),
        dropdown_corner_radius,
        dropdown_corner_radius_spinbox: Spinbox::new(dropdown_corner_radius as i32, 0, 50, 1).with_label("Radius").with_unit("px").with_config(CONFIG_PATH, "dropdown_corner_radius"),
        button_corner_radius,
        button_corner_radius_spinbox: Spinbox::new(button_corner_radius as i32, 0, 50, 1).with_label("Radius").with_unit("px").with_config(CONFIG_PATH, "button_corner_radius"),
        status_box_background_color,
        status_box_corner_radius,
        status_box_corner_radius_spinbox: Spinbox::new(status_box_corner_radius as i32, 0, 50, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "status_box_corner_radius"),
        status_padding,
        status_padding_spinbox: Spinbox::new(status_padding as i32, 0, 32, 1).with_label("Padding").with_unit("px").with_config(CONFIG_PATH, "status_padding"),
        status_module_spacing,
        status_module_spacing_spinbox: Spinbox::new(status_module_spacing as i32, 0, 100, 1).with_label("Spacing").with_unit("px").with_config(CONFIG_PATH, "status_module_spacing"),
        status_controls: StatusInterfaceControls::default(),
        nested_section_label_alignment,
        label_alignment_menu: Dropdown::new(
            vec!["Left".to_string(), "Center".to_string(), "Right".to_string()],
            nested_section_label_alignment as usize,
        ).with_label("Label Alignment").with_config(CONFIG_PATH, "nested_section_label_alignment"),
        nested_section_label_offset,
        label_offset_spinbox: Spinbox::new(nested_section_label_offset as i32, -100, 100, 1).with_label("Label Offset").with_unit("px").with_config(CONFIG_PATH, "nested_section_label_offset"),
        label_margin,
        label_margin_spinbox: Spinbox::new(label_margin as i32, 0, 100, 1).with_label("Label Margin").with_unit("px").with_config(CONFIG_PATH, "label_margin"),
        typeface_loaded: false,
        sans_serif: String::new(),
        serif: String::new(),
        monospace: String::new(),
        window_borders: String::new(),
        status_interface: String::new(),
        fuzzel: String::new(),
        terminal: String::new(),
        all_fonts: Vec::new(),
        mono_fonts: Vec::new(),
        sans_box: TextBox::default(),
        serif_box: TextBox::default(),
        mono_box: TextBox::default(),
        borders_box: TextBox::default(),
        status_box: TextBox::default(),
        fuzzel_box: TextBox::default(),
        terminal_box: TextBox::default(),
        borders_menu: Dropdown::default(),
        status_menu: Dropdown::default(),
        fuzzel_menu: Dropdown::default(),
        terminal_menu: Dropdown::default(),
        borders_size_box: Spinbox::new(11, 6, 72, 1).with_config(CONFIG_PATH, "border_font_size"),
        status_size_box: Spinbox::new(11, 6, 72, 1).with_config(CONFIG_PATH, "status_font_size"),
        fuzzel_size_box: Spinbox::new(14, 6, 72, 1).with_config("/home/lsgalante/.config/fuzzel/fuzzel.ini", "size"),
        terminal_size_box: Spinbox::new(12, 6, 72, 1).with_config("/home/lsgalante/.config/foot/foot.ini", "size"),
        color_selector_font: color_selector_font.clone(),
        color_selector_font_selector: FontSelector::new(color_selector_font.clone()).with_label("Value").with_config(CONFIG_PATH, "color_selector_font"),
        menubar_font: menubar_font.clone(),
        menubar_font_selector: FontSelector::new(menubar_font.clone()).with_label("Font").with_config(CONFIG_PATH, "menubar_font"),
        section_label_font: section_label_font.clone(),
        section_label_font_selector: FontSelector::new(section_label_font.clone()).with_label("Label").with_config(CONFIG_PATH, "section_label_font"),
        nested_section_label_font: nested_section_label_font.clone(),
        nested_section_label_font_selector: FontSelector::new(nested_section_label_font.clone()).with_label("Label").with_config(CONFIG_PATH, "nested_section_label_font"),
        breadcrumb_font: breadcrumb_font.clone(),
        breadcrumb_font_selector: FontSelector::new(breadcrumb_font.clone()).with_label("Font").with_config(CONFIG_PATH, "breadcrumb_font"),
        toggle_font: toggle_font.clone(),
        toggle_font_selector: FontSelector::new(toggle_font.clone()).with_label("Font").with_config(CONFIG_PATH, "toggle_font"),
        font_selector_font: font_selector_font.clone(),
        font_selector_font_selector: FontSelector::new(font_selector_font.clone()).with_label("Font").with_config(CONFIG_PATH, "font_selector_font"),
        button_strip_font: button_strip_font.clone(),
        button_strip_font_selector: FontSelector::new(button_strip_font.clone()).with_label("Font").with_config(CONFIG_PATH, "button_strip_font"),
        button_font: button_font.clone(),
        button_font_selector: FontSelector::new(button_font.clone()).with_label("Font").with_config(CONFIG_PATH, "button_font"),
        label_font: label_font.clone(),
        label_font_selector: FontSelector::new(label_font.clone()).with_label("Font").with_config(CONFIG_PATH, "label_font"),
        dropdown_font: dropdown_font.clone(),
        dropdown_font_selector: FontSelector::new(dropdown_font.clone()).with_label("Font").with_config(CONFIG_PATH, "dropdown_font"),
        textbox_font: textbox_font.clone(),
        textbox_font_selector: FontSelector::new(textbox_font.clone()).with_label("Font").with_config(CONFIG_PATH, "textbox_font"),
        spinbox_font: spinbox_font.clone(),
        spinbox_font_selector: FontSelector::new(spinbox_font.clone()).with_label("Font").with_config(CONFIG_PATH, "spinbox_font"),
        slider_font: slider_font.clone(),
        slider_font_selector: FontSelector::new(slider_font.clone()).with_label("Font").with_config(CONFIG_PATH, "slider_font"),
        graph_show_grid,
        graph_show_grid_toggle: Toggle::new().with_label("Show Grid").with_config(CONFIG_PATH, "graph_show_grid"),
        graph_snap_enabled,
        graph_snap_enabled_toggle: Toggle::new().with_label("Grid Snapping").with_config(CONFIG_PATH, "graph_snap_enabled"),
        graph_uniform_background,
        graph_uniform_background_toggle: Toggle::new().with_label("Uniform Background").with_config(CONFIG_PATH, "graph_uniform_background"),
        graph_cell_opacity,
        graph_cell_opacity_spinbox: Spinbox::new((graph_cell_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Cell Opacity").with_unit("%").with_config(CONFIG_PATH, "graph_cell_opacity"),
        graph_gap_opacity,
        graph_gap_opacity_spinbox: Spinbox::new((graph_gap_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Gap Opacity").with_unit("%").with_config(CONFIG_PATH, "graph_gap_opacity"),
        graph_gap_width,
        graph_gap_width_spinbox: Spinbox::new(graph_gap_width as i32, 0, 100, 1).with_label("Gap Width").with_unit("px").with_config(CONFIG_PATH, "graph_gap_width"),
        menubar_opacity,
        menubar_opacity_spinbox: Spinbox::new((menubar_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "menubar_opacity"),
        notification_bg_color,
        notification_opacity,
        notification_opacity_spinbox: Spinbox::new((notification_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%").with_config(CONFIG_PATH, "notification_opacity"),
        backplate_color,
        backplate_corner_radius,
        backplate_corner_radius_spinbox: Spinbox::new(backplate_corner_radius as i32, 0, 100, 1).with_label("Corner Radius").with_unit("px").with_config(CONFIG_PATH, "backplate_corner_radius"),
        custom_multicontrol: MultiControl::new("custom_parameters".to_string()).with_label("custom_parameters"),
    }
}

pub fn read_windows_config() -> WindowsState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let co = parse_u16_from(&content, "cascade_offset", 20);
    let gl = parse_u16_from(&content, "gap_left", 48);
    let gt = parse_u16_from(&content, "gap_top", 48);
    let sh = parse_u16_from(&content, "bar_height", 24);
    let td = parse_u16_from(&content, "transition_duration", 300);

    let mut side_panel_behavior = parse_string_from(&content, "pinned_behavior", "");
    if side_panel_behavior.is_empty() {
        side_panel_behavior = parse_string_from(&content, "side_panel_behavior", "inline");
    }
    let side_panel_behavior_idx = if side_panel_behavior == "above" { 0 } else { 1 };
    let side_panel_behavior_menu = Dropdown::new(
        vec!["Above".to_string(), "Inline".to_string()],
        side_panel_behavior_idx,
    ).with_label("Behavior")
    .with_config(CONFIG_PATH, "pinned_behavior");

    let mut side_panel_position = parse_string_from(&content, "pinned_position", "");
    if side_panel_position.is_empty() {
        side_panel_position = parse_string_from(&content, "side_panel_position", "left");
    }
    let side_panel_position_idx = if side_panel_position == "right" { 1 } else { 0 };
    let side_panel_position_menu = Dropdown::new(
        vec!["Left".to_string(), "Right".to_string()],
        side_panel_position_idx,
    ).with_label("Position")
    .with_config(CONFIG_PATH, "pinned_position");

    let mut spw = parse_u16_from(&content, "pinned_width", 0);
    if spw == 0 {
        spw = parse_u16_from(&content, "side_panel_width", 360);
    }
    let mut spbg = parse_u16_from(&content, "pinned_border_gap", 9999);
    if spbg == 9999 {
        spbg = parse_u16_from(&content, "side_panel_border_gap", 0);
    }

    let window_blur = parse_bool_from(&content, "window_blur", false);
    let border_blur = parse_bool_from(&content, "border_blur", false);
    let blur_enabled = window_blur || border_blur;

    let mut fullscreen_op = parse_f32_from(&content, "fullscreen_backplate_opacity", -1.0);
    if fullscreen_op < 0.0 {
        fullscreen_op = parse_f32_from(&content, "fullscreen_opacity", 0.95);
    }
    let mut cascade_op = parse_f32_from(&content, "cascade_backplate_opacity", -1.0);
    if cascade_op < 0.0 {
        cascade_op = parse_f32_from(&content, "cascade_opacity", 0.05);
    }
    let mut grid_op = parse_f32_from(&content, "grid_backplate_opacity", -1.0);
    if grid_op < 0.0 {
        grid_op = parse_f32_from(&content, "grid_opacity", 0.05);
    }
    let mut floating_op = parse_f32_from(&content, "floating_backplate_opacity", -1.0);
    if floating_op < 0.0 {
        floating_op = parse_f32_from(&content, "floating_opacity", 0.9);
    }
    let mut pinned_op = parse_f32_from(&content, "pinned_backplate_opacity", -1.0);
    if pinned_op < 0.0 {
        pinned_op = parse_f32_from(&content, "pinned_opacity", 0.05);
    }
    let mut popup_op = parse_f32_from(&content, "popup_backplate_opacity", -1.0);
    if popup_op < 0.0 {
        popup_op = parse_f32_from(&content, "popup_opacity", 0.20);
    }

    WindowsState {
        cascade_offset: co,
        edge_gap: gl,
        top_gap: gt,
        status_height: sh,
        transition_duration: td,
        cascade_offset_spinbox: Spinbox::new(co as i32, 0, 200, 1).with_config(CONFIG_PATH, "cascade_offset"),
        edge_gap_spinbox: Spinbox::new(gl as i32, 0, 200, 1).with_config(CONFIG_PATH, "edge_gap"),
        top_gap_spinbox: Spinbox::new(gt as i32, 0, 200, 1).with_config(CONFIG_PATH, "gap_top"),
        status_height_spinbox: Spinbox::new(sh as i32, 0, 100, 1).with_config(CONFIG_PATH, "bar_height"),
        transition_duration_spinbox: Spinbox::new(td as i32, 0, 2000, 50).with_config(CONFIG_PATH, "transition_duration"),
        side_panel_behavior_menu,
        side_panel_position_menu,
        side_panel_width: spw,
        side_panel_width_spinbox: Spinbox::new(spw as i32, 0, 2000, 10).with_config(CONFIG_PATH, "pinned_width"),
        side_panel_border_gap: spbg,
        side_panel_border_gap_spinbox: Spinbox::new(spbg as i32, 0, 500, 1).with_config(CONFIG_PATH, "pinned_border_gap"),
        blur_enabled,
        blur_toggle: Toggle::new().with_label("Blur").with_config(CONFIG_PATH, "window_blur"),
        fullscreen_opacity: fullscreen_op,
        fullscreen_opacity_spinbox: Spinbox::new((fullscreen_op * 100.0).round() as i32, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "fullscreen_backplate_opacity"),
        cascade_opacity: cascade_op,
        cascade_opacity_spinbox: Spinbox::new((cascade_op * 100.0).round() as i32, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "cascade_backplate_opacity"),
        grid_opacity: grid_op,
        grid_opacity_spinbox: Spinbox::new((grid_op * 100.0).round() as i32, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "grid_backplate_opacity"),
        floating_opacity: floating_op,
        floating_opacity_spinbox: Spinbox::new((floating_op * 100.0).round() as i32, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "floating_backplate_opacity"),
        pinned_opacity: pinned_op,
        pinned_opacity_spinbox: Spinbox::new((pinned_op * 100.0).round() as i32, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "pinned_backplate_opacity"),
        popup_opacity: popup_op,
        popup_opacity_spinbox: Spinbox::new((popup_op * 100.0).round() as i32, 0, 100, 5).with_label("Backplate Opacity").with_unit("%").with_config(CONFIG_PATH, "popup_backplate_opacity"),
    }
}


fn apply_single_layout_param(key: &str, val: u16) {
    write_config_value(key, &val.to_string());
    send_ipc_command(&format!("layout {} {}", key, val));
}

fn apply_edge_gap(val: u16) {
    let val_str = val.to_string();
    write_config_value("gap_left", &val_str);
    write_config_value("gap_right", &val_str);
    write_config_value("gap_bottom", &val_str);
    send_ipc_command(&format!("layout gap_left {}", val));
    send_ipc_command(&format!("layout gap_right {}", val));
    send_ipc_command(&format!("layout gap_bottom {}", val));
}





fn parse_json(content: &str) -> serde_json::Value {
    serde_json::from_str(content).unwrap_or_default()
}

fn json_find_key<'a>(val: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    if let Some(obj) = val.as_object() {
        for (_, sec_val) in obj.iter() {
            if let Some(sec_obj) = sec_val.as_object() {
                if let Some(v) = sec_obj.get(key) {
                    return Some(v);
                }
            }
        }
    }
    None
}

pub fn parse_string_from(content: &str, key: &str, default: &str) -> String {
    let val = parse_json(content);
    if let Some(v) = json_find_key(&val, key) {
        if let Some(s) = v.as_str() {
            return s.to_string();
        }
    }
    default.to_string()
}

fn parse_color_from_key(content: &str, key: &str, default: [u8; 3]) -> [u8; 3] {
    let val = parse_json(content);
    if let Some(v) = json_find_key(&val, key) {
        if let Some(s) = v.as_str() {
            return parse_hex(s);
        }
    }
    default
}

fn parse_hex(s: &str) -> [u8; 3] {
    let s = s.trim_matches(|c| c == '"' || c == '\'' || c == ' ');
    let s = s.trim_start_matches('#');
    if s.len() >= 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0x0a);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0x1a);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0x0e);
        [r, g, b]
    } else { [0x0a, 0x1a, 0x0e] }
}

fn parse_rgba_color_from_key(content: &str, key: &str, default: [u8; 4]) -> [u8; 4] {
    let val = parse_json(content);
    if let Some(v) = json_find_key(&val, key) {
        if let Some(s) = v.as_str() {
            return parse_hex_rgba(s);
        }
    }
    default
}

fn parse_hex_rgba(s: &str) -> [u8; 4] {
    let s = s.trim_matches(|c| c == '"' || c == '\'' || c == ' ');
    let s = s.trim_start_matches('#');
    if s.len() >= 8 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
        let a = u8::from_str_radix(&s[6..8], 16).unwrap_or(255);
        [r, g, b, a]
    } else if s.len() >= 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
        [r, g, b, 255]
    } else { [255, 255, 255, 255] }
}

pub fn write_config_value(key: &str, value: &str) -> bool {
    write_config_value_path(&get_config_path(), key, value)
}

pub fn get_links() -> Vec<(String, String)> {
    get_links_path(&get_links_path_val())
}

pub fn get_links_path(path: &str) -> Vec<(String, String)> {
    let content = fs::read_to_string(path).unwrap_or_default();
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&content) {
        let mut links = Vec::new();
        for (k, v) in map {
            if let Some(v_str) = v.as_str() {
                links.push((k, v_str.to_string()));
            }
        }
        links
    } else {
        Vec::new()
    }
}

pub fn write_config_value_path(path: &str, key: &str, value: &str) -> bool {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut val = parse_json(&content);
    
    let mut keys_to_update = vec![key.to_string()];
    let links = get_links();
    let mut i = 0;
    while i < keys_to_update.len() {
        let cur = keys_to_update[i].clone();
        for (k, v) in &links {
            if k == &cur && !keys_to_update.contains(v) {
                keys_to_update.push(v.clone());
            }
            if v == &cur && !keys_to_update.contains(k) {
                keys_to_update.push(k.clone());
            }
        }
        i += 1;
    }

    let mut updated_any = false;
    for k in &keys_to_update {
        let mapped_k = if k == key {
            match key {
                // Removed low_color
                "high_color" => "border_color",
                _ => k,
            }
        } else {
            k
        };
        if cce_ui::config::update_json_in_memory(&mut val, mapped_k, value, "layout") {
            updated_any = true;
        }
    }

    if updated_any {
        if let Ok(updated_str) = serde_json::to_string_pretty(&val) {
            if safe_write(path, &updated_str) {
                return true;
            }
        }
    }
    false
}

pub fn propagate_links(state: &mut InterfaceState, key: &str, val_str: &str) {
    let links = get_links();
    
    let mut keys_to_update = Vec::new();
    let mut visited = vec![key.to_string()];
    let mut queue = vec![key.to_string()];
    
    while let Some(cur) = queue.pop() {
        for (k, v) in &links {
            if k == &cur && !visited.contains(v) {
                visited.push(v.clone());
                queue.push(v.clone());
                keys_to_update.push(v.clone());
            }
            if v == &cur && !visited.contains(k) {
                visited.push(k.clone());
                queue.push(k.clone());
                keys_to_update.push(k.clone());
            }
        }
    }

    for k in keys_to_update {
        match k.as_str() {

            "button_padding" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.button_padding = val;
                    state.button_padding_spinbox.value = val as i32;
                    apply_button_padding(val);
                }
            }
            "button_strip_spacing" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.button_strip_spacing = val;
                    state.button_strip_spacing_spinbox.value = val as i32;
                    apply_button_strip_spacing(val);
                }
            }
            "section_padding" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.section_padding = val;
                    state.section_padding_spinbox.value = val as i32;
                    apply_section_padding(val);
                }
            }
            "plate_padding" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.plate_padding = val;
                    state.plate_padding_spinbox.value = val as i32;
                    apply_plate_padding(val);
                }
            }
            "plate_opacity" => {
                if let Ok(val) = val_str.parse::<f32>() {
                    state.plate_opacity = val;
                    state.plate_opacity_spinbox.value = (val * 100.0).round() as i32;
                    apply_plate_opacity(val);
                }
            }
            "plate_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.plate_corner_radius = val;
                    state.plate_corner_radius_spinbox.value = val as i32;
                    apply_plate_corner_radius(val);
                }
            }

            "page_margin" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.page_margin = val;
                    state.page_margin_spinbox.value = val as i32;
                    apply_page_margin(val);
                }
            }
            "grid_min_col_width" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.grid_min_col_width = val;
                    state.grid_min_col_width_spinbox.value = val as i32;
                    apply_grid_min_col_width(val);
                }
            }
            "grid_gap" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.layout_grid_gap = val;
                    state.layout_grid_gap_spinbox.value = val as i32;
                    apply_layout_grid_gap(val);
                }
            }
            "spinbox_height" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.spinbox_height = val;
                    state.spinbox_height_spinbox.value = val as i32;
                    apply_spinbox_height(val);
                }
            }
            "spinbox_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.spinbox_corner_radius = val;
                    state.spinbox_corner_radius_spinbox.value = val as i32;
                    apply_spinbox_corner_radius(val);
                }
            }
            "toggle_height" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.toggle_height = val;
                    state.toggle_height_spinbox.value = val as i32;
                    apply_toggle_height(val);
                }
            }
            "toggle_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.toggle_corner_radius = val;
                    state.toggle_corner_radius_spinbox.value = val as i32;
                    apply_toggle_corner_radius(val);
                }
            }
            "toggle_bg_color" => {
                let trimmed = val_str.trim().trim_matches('"');
                let rgb = parse_hex(trimmed);
                state.toggle_bg_color = rgb;
                state.color_selectors[24].color = rgb;
                apply_toggle_bg_color(rgb);
            }
            "toggle_border_width" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.toggle_border_width = val;
                    state.toggle_border_width_spinbox.value = val as i32;
                    apply_toggle_border_width(val);
                }
            }
            "toggle_font" => {
                let font = val_str.trim_matches('"').to_string();
                state.toggle_font = font.clone();
                state.toggle_font_selector.font_family = font.clone();
                apply_toggle_font(&font);
            }
            "font_selector_font" => {
                let font = val_str.trim_matches('"').to_string();
                state.font_selector_font = font.clone();
                state.font_selector_font_selector.font_family = font.clone();
                apply_font_selector_font(&font);
            }
            "button_strip_font" => {
                let font = val_str.trim_matches('"').to_string();
                state.button_strip_font = font.clone();
                state.button_strip_font_selector.font_family = font.clone();
                apply_button_strip_font(&font);
            }
            "button_font" => {
                let font = val_str.trim_matches('"').to_string();
                state.button_font = font.clone();
                state.button_font_selector.font_family = font.clone();
                apply_button_font(&font);
            }
            "label_font" => {
                let font = val_str.trim_matches('"').to_string();
                state.label_font = font.clone();
                state.label_font_selector.font_family = font.clone();
                apply_label_font(&font);
            }
            "dropdown_font" => {
                let font = val_str.trim_matches('"').to_string();
                state.dropdown_font = font.clone();
                state.dropdown_font_selector.font_family = font.clone();
                apply_dropdown_font(&font);
            }
            "textbox_font" => {
                let font = val_str.trim_matches('"').to_string();
                state.textbox_font = font.clone();
                state.textbox_font_selector.font_family = font.clone();
                apply_textbox_font(&font);
            }
            "spinbox_font" => {
                let font = val_str.trim_matches('"').to_string();
                state.spinbox_font = font.clone();
                state.spinbox_font_selector.font_family = font.clone();
                apply_spinbox_font(&font);
            }
            "slider_font" => {
                let font = val_str.trim_matches('"').to_string();
                state.slider_font = font.clone();
                state.slider_font_selector.font_family = font.clone();
                apply_slider_font(&font);
            }

            "color_selector_height" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.color_selector_height = val;
                    state.color_selector_height_spinbox.value = val as i32;
                    apply_color_selector_height(val);
                }
            }
            "color_selector_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.color_selector_corner_radius = val;
                    state.color_selector_corner_radius_spinbox.value = val as i32;
                    apply_color_selector_corner_radius(val);
                }
            }
            "color_selector_preview_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.color_selector_preview_corner_radius = val;
                    state.color_selector_preview_corner_radius_spinbox.value = val as i32;
                    apply_color_selector_preview_corner_radius(val);
                }
            }
            "color_selector_preview_margin" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.color_selector_preview_margin = val;
                    state.color_selector_preview_margin_spinbox.value = val as i32;
                    apply_color_selector_preview_margin(val);
                }
            }
            "textbox_height" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.textbox_height = val;
                    state.textbox_height_spinbox.value = val as i32;
                    apply_textbox_height(val);
                }
            }
            "textbox_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.textbox_corner_radius = val;
                    state.textbox_corner_radius_spinbox.value = val as i32;
                    apply_textbox_corner_radius(val);
                }
            }
            "slider_height" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.slider_height = val;
                    state.slider_height_spinbox.value = val as i32;
                    apply_slider_height(val);
                }
            }
            "slider_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.slider_corner_radius = val;
                    state.slider_corner_radius_spinbox.value = val as i32;
                    apply_slider_corner_radius(val);
                }
            }
            "font_selector_height" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.font_selector_height = val;
                    state.font_selector_height_spinbox.value = val as i32;
                    apply_font_selector_height(val);
                }
            }
            "font_selector_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.font_selector_corner_radius = val;
                    state.font_selector_corner_radius_spinbox.value = val as i32;
                    apply_font_selector_corner_radius(val);
                }
            }
            "dropdown_height" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.dropdown_height = val;
                    state.dropdown_height_spinbox.value = val as i32;
                    apply_dropdown_height(val);
                }
            }
            "dropdown_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.dropdown_corner_radius = val;
                    state.dropdown_corner_radius_spinbox.value = val as i32;
                    apply_dropdown_corner_radius(val);
                }
            }
            "button_corner_radius" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.button_corner_radius = val;
                    state.button_corner_radius_spinbox.value = val as i32;
                    apply_button_corner_radius(val);
                }
            }
            "label_margin" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.label_margin = val;
                    state.label_margin_spinbox.value = val as i32;
                    apply_label_margin(val);
                }
            }
            "nested_section_label_offset" => {
                if let Ok(val) = val_str.parse::<i16>() {
                    state.nested_section_label_offset = val;
                    state.label_offset_spinbox.value = val as i32;
                    apply_nested_section_label_offset(val);
                }
            }
            "graph_gap_width" => {
                if let Ok(val) = val_str.parse::<u16>() {
                    state.graph_gap_width = val;
                    state.graph_gap_width_spinbox.value = val as i32;
                    cce_graph_reload();
                }
            }
            "graph_cell_opacity" => {
                if let Ok(val) = val_str.parse::<f32>() {
                    state.graph_cell_opacity = val;
                    state.graph_cell_opacity_spinbox.value = (val * 100.0).round() as i32;
                    cce_graph_reload();
                }
            }
            "graph_gap_opacity" => {
                if let Ok(val) = val_str.parse::<f32>() {
                    state.graph_gap_opacity = val;
                    state.graph_gap_opacity_spinbox.value = (val * 100.0).round() as i32;
                    cce_graph_reload();
                }
            }
            "menubar_opacity" => {
                if let Ok(val) = val_str.parse::<f32>() {
                    state.menubar_opacity = val;
                    state.menubar_opacity_spinbox.value = (val * 100.0).round() as i32;
                    send_ipc_command("reload");
                    status_interface_reload();
                }
            }
            "page_opacity" => {
                if let Ok(val) = val_str.parse::<f32>() {
                    state.page_opacity = val;
                    state.page_opacity_spinbox.value = (val * 100.0).round() as i32;
                    apply_page_opacity(val);
                }
            }
            "layer_opacity" => {
                if let Ok(val) = val_str.parse::<f32>() {
                    state.layer_opacity = val;
                    state.layer_opacity_spinbox.value = (val * 100.0).round() as i32;
                    apply_layer_opacity(val);
                }
            }
            "page_color" => {
                let trimmed = val_str.trim().trim_matches('"');
                let rgb = parse_hex(trimmed);
                state.page_color = rgb;
                state.color_selectors[19].color = rgb;
                apply_page_color(rgb);
            }
            "layer_color" => {
                let trimmed = val_str.trim().trim_matches('"');
                let rgb = parse_hex(trimmed);
                state.layer_color = rgb;
                state.color_selectors[20].color = rgb;
                apply_layer_color(rgb);
            }
            _ => {}
        }
    }
}

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(get_socket_path()) {
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn cce_graph_reload() {
    std::thread::spawn(|| {
        let is_running = std::process::Command::new("pgrep")
            .args(["-f", "cce-graph"])
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);
        if is_running {
            let _ = std::process::Command::new("pkill")
                .args(["-f", "cce-graph"])
                .status();
            std::thread::sleep(std::time::Duration::from_millis(150));
            send_ipc_command("spawn cce-graph");
        }
    });
}

fn apply_desktop_background(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_surfaces_config_value("desktop_background", &hex);
    send_ipc_command(&format!("layout desktop_background #{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
}

fn apply_desktop_grid_color(rgba: [u8; 4]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}{:02x}\"", rgba[0], rgba[1], rgba[2], rgba[3]);
    write_surfaces_config_value("desktop_grid_color", &hex);
    send_ipc_command(&format!("layout desktop_grid_color #{:02x}{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2], rgba[3]));
}

fn apply_desktop_grid_scale(val: u16) {
    write_surfaces_config_value("desktop_grid_scale", &val.to_string());
    send_ipc_command(&format!("layout desktop_grid_scale {}", val));
}

fn apply_desktop_line_width(val: u16) {
    write_surfaces_config_value("desktop_line_width", &val.to_string());
    send_ipc_command(&format!("layout desktop_line_width {}", val));
}

fn apply_border_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("high_color", &hex);
    send_ipc_command(&format!("layout high_color #{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
}

fn apply_disabled_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("disabled_color", &hex);
    send_ipc_command(&format!("layout disabled_color #{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
}

pub fn status_interface_reload() {
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};

    struct StatusDebouncer {
        last_trigger: Instant,
        active: bool,
    }

    impl StatusDebouncer {
        fn new() -> Self {
            Self {
                last_trigger: Instant::now(),
                active: false,
            }
        }
    }

    static DEBOUNCER: OnceLock<Mutex<StatusDebouncer>> = OnceLock::new();
    let debouncer = DEBOUNCER.get_or_init(|| Mutex::new(StatusDebouncer::new()));
    
    let mut guard = debouncer.lock().unwrap();
    guard.last_trigger = Instant::now();
    if !guard.active {
        guard.active = true;
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(Duration::from_millis(100));
                let mut guard = DEBOUNCER.get().unwrap().lock().unwrap();
                if guard.last_trigger.elapsed() >= Duration::from_millis(250) {
                    guard.active = false;
                    drop(guard);

                    // Perform the actual reload
                    let _ = std::process::Command::new("pkill")
                        .args(["-f", "cce-status-interface"])
                        .status();
                    std::thread::sleep(Duration::from_millis(150));
                    send_ipc_command("spawn cce-status-interface");
                    break;
                }
            }
        });
    }
}

fn write_status_value(key: &str, value: &str) {
    write_config_value_path(&get_config_path(), key, value);
}

#[allow(dead_code)]
fn read_status_padding() -> Option<u16> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    Some(parse_u16_from(&content, "status_padding", 8))
}

fn write_status_padding(padding: u16) {
    write_status_value("status_padding", &padding.to_string());
}

fn read_status_box_opacity() -> Option<f32> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    Some(parse_f32_from(&content, "status_box_opacity", 1.0))
}

fn write_status_box_opacity(val: f32) {
    write_status_value("status_box_opacity", &val.to_string());
}

fn read_status_box_blur() -> Option<f32> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    Some(parse_f32_from(&content, "status_box_blur", 0.0))
}

fn write_status_box_blur(val: f32) {
    write_status_value("status_box_blur", &val.to_string());
}


fn apply_visual_guides_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("visual_guides_color", &hex);
}

fn apply_slider_track_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("slider_track_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_slider_track([r, g, b, 1.0]);
}

fn apply_page_low_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("page_low_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_page_low_color([r, g, b, 1.0]);
}

fn apply_color_borders_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("color_borders_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_color_borders_color([r, g, b, 1.0]);
}

fn apply_normal_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("status_normal_color", &hex);
    status_interface_reload();
}

fn apply_paginator_sidebar_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("paginator_sidebar_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_sidebar_bg_color([r, g, b, 1.0]);
}

fn apply_primary_highlight_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("primary_highlight_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_highlight_primary_color([r, g, b, 0.12]);
}

fn apply_menubar_tab_label_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("menubar_tab_label_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_menubar_tab_label_color([r, g, b, 1.0]);
}

fn apply_toggle_enabled_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("toggle_enabled_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_toggle_on_color([r, g, b, 1.0]);
}

fn apply_toggle_disabled_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("toggle_disabled_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_toggle_off_color([r, g, b, 1.0]);
}

fn apply_scrollinglist_bg_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("scrollinglist_bg_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_scrollinglist_bg_color([r, g, b, 0.3]);
}

fn apply_scrollinglist_entry_bg_color(rgba: [u8; 4]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}{:02x}\"", rgba[0], rgba[1], rgba[2], rgba[3]);
    write_config_value("scrollinglist_entry_bg_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgba[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgba[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgba[2] as f32 / 255.0);
    let a = rgba[3] as f32 / 255.0;
    cce_ui::color::set_scrollinglist_entry_bg_color([r, g, b, a]);
}

fn apply_scrollinglist_entry_highlight_color(rgba: [u8; 4]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}{:02x}\"", rgba[0], rgba[1], rgba[2], rgba[3]);
    write_config_value("scrollinglist_entry_highlight_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgba[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgba[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgba[2] as f32 / 255.0);
    let a = rgba[3] as f32 / 255.0;
    cce_ui::color::set_scrollinglist_entry_highlight_color([r, g, b, a]);
}

fn apply_breadcrumb_bg_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("breadcrumb_bg_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_breadcrumb_bg_color([r, g, b, 1.0]);
}

fn apply_popover_bg_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("popover_bg_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_popover_bg_color([r, g, b, 1.0]);
}

fn apply_notifications_bg_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_notifications_config_value("bg_color", &hex);
    send_ipc_command("reload");
}

fn apply_notifications_opacity(opacity: f32) {
    write_notifications_config_value("opacity", &format!("{:.2}", opacity));
    send_ipc_command("reload");
}

fn apply_backplate_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_surfaces_config_value("backplate_color", &hex);
    send_ipc_command("reload");
}

fn apply_status_box_background_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("status_box_background_color", &hex);
    status_interface_reload();
}

fn apply_status_box_corner_radius(radius: u16) {
    write_config_value("status_box_corner_radius", &radius.to_string());
    status_interface_reload();
}

fn apply_status_padding(padding: u16) {
    write_config_value("status_padding", &padding.to_string());
    status_interface_reload();
}

fn apply_status_module_spacing(spacing: u16) {
    write_config_value("status_module_spacing", &spacing.to_string());
    status_interface_reload();
}



fn apply_backplate_corner_radius(radius: u16) {
    write_surfaces_config_value("backplate_corner_radius", &radius.to_string());
    send_ipc_command("reload");
    cce_ui::color::set_backplate_corner_radius(radius as f32);
}


fn apply_button_padding(padding: u16) {
    write_config_value("button_padding", &padding.to_string());
    send_ipc_command(&format!("layout button_padding {}", padding));
    cce_ui::layout::set_button_padding(padding as f32);
}

fn apply_button_strip_spacing(spacing: u16) {
    write_config_value("button_strip_spacing", &spacing.to_string());
    send_ipc_command(&format!("layout button_strip_spacing {}", spacing));
    cce_ui::layout::set_button_strip_spacing(spacing as f32);
}

fn apply_plate_padding(padding: u16) {
    write_config_value("plate_padding", &padding.to_string());
    cce_ui::layout::set_plate_padding(padding as f32);
}

fn apply_plate_opacity(opacity: f32) {
    write_config_value("plate_opacity", &opacity.to_string());
    cce_ui::layout::set_plate_opacity(opacity);
}

fn apply_plate_corner_radius(radius: u16) {
    write_config_value("plate_corner_radius", &radius.to_string());
    cce_ui::layout::set_plate_corner_radius(radius as f32);
}

fn apply_page_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("page_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_page_color([r, g, b, 1.0]);
}

fn apply_page_opacity(opacity: f32) {
    write_config_value("page_opacity", &opacity.to_string());
    cce_ui::layout::set_page_opacity(opacity);
}

fn apply_layer_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("layer_color", &hex);
    let r = cce_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = cce_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = cce_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    cce_ui::color::set_layer_color([r, g, b, 1.0]);
}

fn apply_layer_opacity(opacity: f32) {
    write_config_value("layer_opacity", &opacity.to_string());
    cce_ui::layout::set_layer_opacity(opacity);
}


fn apply_page_margin(margin: u16) {
    write_config_value("page_margin", &margin.to_string());
    cce_ui::layout::set_page_margin(margin as f32);
}

fn apply_grid_min_col_width(width: u16) {
    write_config_value("grid_min_col_width", &width.to_string());
    cce_ui::layout::set_grid_min_col_width(width as f32);
}

fn apply_layout_grid_gap(gap: u16) {
    write_config_value("grid_gap", &gap.to_string());
    cce_ui::layout::set_grid_gap(gap as f32);
    send_ipc_command(&format!("layout grid_gap {}", gap));
}

fn apply_section_padding(padding: u16) {
    write_config_value("section_padding", &padding.to_string());
    cce_ui::layout::set_section_padding(padding as f32);
}

fn apply_spinbox_height(height: u16) {
    write_config_value("spinbox_height", &height.to_string());
    cce_ui::layout::set_spinbox_height(height as f32);
}

fn apply_spinbox_corner_radius(radius: u16) {
    write_config_value("spinbox_corner_radius", &radius.to_string());
    cce_ui::layout::set_spinbox_corner_radius(radius as f32);
}

fn apply_toggle_height(height: u16) {
    write_config_value("toggle_height", &height.to_string());
    cce_ui::layout::set_toggle_height(height as f32);
}

fn apply_toggle_corner_radius(radius: u16) {
    write_config_value("toggle_corner_radius", &radius.to_string());
    cce_ui::layout::set_toggle_corner_radius(radius as f32);
}

fn apply_toggle_border_width(width: u16) {
    write_config_value("toggle_border_width", &width.to_string());
    cce_ui::layout::set_toggle_border_width(width as f32);
}

fn apply_toggle_bg_color(rgb: [u8; 3]) {
    let hex = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
    write_config_value("toggle_bg_color", &hex);
    cce_ui::color::set_toggle_bg_color(cce_ui::color::to_linear([
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        1.0,
    ]));
}

fn apply_toggle_font(font: &str) {
    write_config_value("toggle_font", &format!("\"{}\"", font));
    cce_ui::layout::set_toggle_font(font);
}

fn apply_font_selector_font(font: &str) {
    write_config_value("font_selector_font", &format!("\"{}\"", font));
    cce_ui::layout::set_font_selector_font(font);
}

fn apply_button_strip_font(font: &str) {
    write_config_value("button_strip_font", &format!("\"{}\"", font));
    cce_ui::layout::set_button_strip_font(font);
}

fn apply_button_font(font: &str) {
    write_config_value("button_font", &format!("\"{}\"", font));
    cce_ui::layout::set_button_font(font);
}

fn apply_label_font(font: &str) {
    write_config_value("label_font", &format!("\"{}\"", font));
    cce_ui::layout::set_label_font(font);
}

fn apply_dropdown_font(font: &str) {
    write_config_value("dropdown_font", &format!("\"{}\"", font));
    cce_ui::layout::set_dropdown_font(font);
}

fn apply_textbox_font(font: &str) {
    write_config_value("textbox_font", &format!("\"{}\"", font));
    cce_ui::layout::set_textbox_font(font);
}

fn apply_spinbox_font(font: &str) {
    write_config_value("spinbox_font", &format!("\"{}\"", font));
    cce_ui::layout::set_spinbox_font(font);
}

fn apply_slider_font(font: &str) {
    write_config_value("slider_font", &format!("\"{}\"", font));
    cce_ui::layout::set_slider_font(font);
}


fn apply_color_selector_height(height: u16) {
    write_config_value("color_selector_height", &height.to_string());
    cce_ui::layout::set_color_selector_height(height as f32);
}

fn apply_color_selector_corner_radius(radius: u16) {
    write_config_value("color_selector_corner_radius", &radius.to_string());
    cce_ui::layout::set_color_selector_corner_radius(radius as f32);
}

fn apply_color_selector_preview_corner_radius(radius: u16) {
    write_config_value("color_selector_preview_corner_radius", &radius.to_string());
    cce_ui::layout::set_color_selector_preview_corner_radius(radius as f32);
}

fn apply_color_selector_preview_margin(margin: u16) {
    write_config_value("color_selector_preview_margin", &margin.to_string());
    cce_ui::layout::set_color_selector_preview_margin(margin as f32);
}

fn apply_textbox_height(height: u16) {
    write_config_value("textbox_height", &height.to_string());
    cce_ui::layout::set_textbox_height(height as f32);
}

fn apply_textbox_corner_radius(radius: u16) {
    write_config_value("textbox_corner_radius", &radius.to_string());
    cce_ui::layout::set_textbox_corner_radius(radius as f32);
}

fn apply_slider_height(height: u16) {
    write_config_value("slider_height", &height.to_string());
    cce_ui::layout::set_slider_height(height as f32);
}

fn apply_slider_corner_radius(radius: u16) {
    write_config_value("slider_corner_radius", &radius.to_string());
    cce_ui::layout::set_slider_corner_radius(radius as f32);
}


fn apply_font_selector_height(height: u16) {
    write_config_value("font_selector_height", &height.to_string());
    cce_ui::layout::set_font_selector_height(height as f32);
}

fn apply_font_selector_corner_radius(radius: u16) {
    write_config_value("font_selector_corner_radius", &radius.to_string());
    cce_ui::layout::set_font_selector_corner_radius(radius as f32);
}

fn apply_dropdown_height(height: u16) {
    write_config_value("dropdown_height", &height.to_string());
    cce_ui::layout::set_dropdown_height(height as f32);
}

fn apply_dropdown_corner_radius(radius: u16) {
    write_config_value("dropdown_corner_radius", &radius.to_string());
    cce_ui::layout::set_dropdown_corner_radius(radius as f32);
}

fn apply_button_corner_radius(radius: u16) {
    write_config_value("button_corner_radius", &radius.to_string());
    cce_ui::layout::set_button_corner_radius(radius as f32);
}

fn apply_color_selector_font(font: &str) {
    write_config_value("color_selector_font", &format!("\"{}\"", font));
    cce_ui::layout::set_color_selector_font(font);
}

fn apply_menubar_font(font: &str) {
    write_config_value("menubar_font", &format!("\"{}\"", font));
    cce_ui::layout::set_menubar_font(font);
}

fn apply_breadcrumb_font(font: &str) {
    write_config_value("breadcrumb_font", &format!("\"{}\"", font));
    cce_ui::layout::set_breadcrumb_font(font);
}

fn apply_section_label_font(font: &str) {
    write_config_value("section_label_font", &format!("\"{}\"", font));
    cce_ui::layout::set_section_label_font(font);
}

fn apply_nested_section_label_font(font: &str) {
    write_config_value("nested_section_label_font", &format!("\"{}\"", font));
    cce_ui::layout::set_nested_section_label_font(font);
}

fn apply_nested_section_label_alignment(align: u8) {
    write_config_value("nested_section_label_alignment", &align.to_string());
    cce_ui::layout::set_nested_section_label_alignment(align);
}

fn apply_nested_section_label_offset(offset: i16) {
    write_config_value("nested_section_label_offset", &offset.to_string());
    cce_ui::layout::set_nested_section_label_offset(offset as f32);
}

fn apply_label_margin(margin: u16) {
    write_config_value("label_margin", &margin.to_string());
    cce_ui::layout::set_label_margin(margin as f32);
}

const FONTS_CONF_PATH: &str = "/home/lsgalante/.config/fontconfig/fonts.conf";

fn parse_font_for_alias(content: &str, alias: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    for i in 0..lines.len() {
        let line = lines[i].trim();
        if line.contains("<test") && line.contains("name=\"family\"") && line.contains(&format!("<string>{}</string>", alias)) {
            for j in (i + 1)..(i + 6).min(lines.len()) {
                let next_line = lines[j].trim();
                if next_line.contains("<edit") {
                    for k in (j + 1)..(j + 6).min(lines.len()) {
                        let str_line = lines[k].trim();
                        if str_line.contains("<string>") && str_line.contains("</string>") {
                            if let Some(start) = str_line.find("<string>") {
                                if let Some(end) = str_line.find("</string>") {
                                    let font = &str_line[start + 8..end];
                                    return Some(font.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn read_preferred_fonts() -> (String, String, String, String, String, String, String) {
    let content = fs::read_to_string(FONTS_CONF_PATH).unwrap_or_default();
    
    let sans = parse_font_for_alias(&content, "sans-serif").unwrap_or_else(|| "Noto Sans".to_string());
    let serif = parse_font_for_alias(&content, "serif").unwrap_or_else(|| "Noto Serif".to_string());
    let mono = parse_font_for_alias(&content, "monospace").unwrap_or_else(|| "Noto Sans Mono".to_string());
    let borders = parse_font_for_alias(&content, "window-borders").unwrap_or_else(|| "Noto Sans".to_string());
    let status = parse_font_for_alias(&content, "status-interface").unwrap_or_else(|| "Noto Sans".to_string());
    let fuzzel_font = parse_font_for_alias(&content, "fuzzel").unwrap_or_else(|| "Noto Sans".to_string());
    let term = parse_font_for_alias(&content, "terminal").unwrap_or_else(|| "Noto Sans Mono".to_string());
    
    (sans, serif, mono, borders, status, fuzzel_font, term)
}

pub fn save_preferred_fonts(
    sans: &str,
    serif: &str,
    mono: &str,
    borders: &str,
    status: &str,
    fuzzel: &str,
    terminal: &str,
) {
    let content = fs::read_to_string(FONTS_CONF_PATH).unwrap_or_default();
    
    let mut dirs = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<dir>") && trimmed.ends_with("</dir>") {
            dirs.push(trimmed.to_string());
        }
    }
    if dirs.is_empty() {
        dirs.push("<dir>~/Dropbox/Fonts</dir>".to_string());
    }
    
    let mut new_content = String::new();
    new_content.push_str("<?xml version=\"1.0\"?>\n");
    new_content.push_str("<!DOCTYPE fontconfig SYSTEM \"fonts.dtd\">\n");
    new_content.push_str("<fontconfig>\n");
    
    for dir in dirs {
        new_content.push_str(&format!("    {}\n", dir));
    }
    
    // Sans-Serif
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>sans-serif</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", sans));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Serif
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>serif</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", serif));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Monospace
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>monospace</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", mono));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Window Borders
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>window-borders</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", borders));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Status Interface
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>status-interface</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", status));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Fuzzel
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>fuzzel</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", fuzzel));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Terminal
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>terminal</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", terminal));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    new_content.push_str("</fontconfig>\n");
    
    let _ = fs::write(FONTS_CONF_PATH, new_content);
}

pub fn parse_i16_from(content: &str, key: &str, default: i16) -> i16 {
    let val = parse_json(content);
    if let Some(v) = json_find_key(&val, key) {
        if let Some(n) = v.as_i64() {
            return n as i16;
        }
    }
    default
}

pub fn parse_u16_from(content: &str, key: &str, default: u16) -> u16 {
    let val = parse_json(content);
    if let Some(v) = json_find_key(&val, key) {
        if let Some(n) = v.as_u64() {
            return n as u16;
        }
    }
    default
}

pub fn parse_bool_from(content: &str, key: &str, default: bool) -> bool {
    let val = parse_json(content);
    if let Some(v) = json_find_key(&val, key) {
        if let Some(b) = v.as_bool() {
            return b;
        }
    }
    default
}

pub fn parse_f32_from(content: &str, key: &str, default: f32) -> f32 {
    let val = parse_json(content);
    if let Some(v) = json_find_key(&val, key) {
        if let Some(n) = v.as_f64() {
            return n as f32;
        }
    }
    default
}

fn read_border_font_size() -> Option<u16> {
    let content = fs::read_to_string(CONFIG_PATH).ok()?;
    Some(parse_u16_from(&content, "border_font_size", 11))
}

fn read_status_size() -> Option<u16> {
    let content = fs::read_to_string(CONFIG_PATH).ok()?;
    Some(parse_u16_from(&content, "status_font_size", 11))
}

fn write_status_size(size: u16) {
    write_config_value("status_font_size", &size.to_string());
    let mut cmd = std::process::Command::new("pkill");
    cmd.args(["-f", "cce-status-interface"]);
    let _ = cce_ui::process::spawn_detached(cmd);
}

fn read_fuzzel_size() -> Option<u16> {
    let ini = fs::read_to_string("/home/lsgalante/.config/fuzzel/fuzzel.ini").ok()?;
    for line in ini.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font") {
            if let Some(pos) = trimmed.find("size=") {
                let size_str = &trimmed[pos + 5..];
                let end_pos = size_str.find(|c: char| !c.is_ascii_digit()).unwrap_or(size_str.len());
                return size_str[..end_pos].parse::<u16>().ok();
            }
        }
    }
    None
}

fn write_fuzzel_size(size: u16) {
    let path = "/home/lsgalante/.config/fuzzel/fuzzel.ini";
    let ini = fs::read_to_string(path).unwrap_or_default();
    let mut new_lines = Vec::new();
    for line in ini.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font") {
            if let Some(pos) = line.find("size=") {
                let mut new_line = line[..pos + 5].to_string();
                new_line.push_str(&size.to_string());
                let size_str = &line[pos + 5..];
                let skip = size_str.find(|c: char| !c.is_ascii_digit()).unwrap_or(size_str.len());
                new_line.push_str(&size_str[skip..]);
                new_lines.push(new_line);
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }
    let _ = fs::write(path, new_lines.join("\n"));
}

fn read_terminal_size() -> Option<u16> {
    let ini = fs::read_to_string("/home/lsgalante/.config/foot/foot.ini").ok()?;
    for line in ini.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font") {
            if let Some(pos) = trimmed.find("terminal:size=") {
                let size_str = &trimmed[pos + 14..];
                let end_pos = size_str.find(|c: char| !c.is_ascii_digit()).unwrap_or(size_str.len());
                return size_str[..end_pos].parse::<u16>().ok();
            }
        }
    }
    None
}

fn write_terminal_size(size: u16) {
    let path = "/home/lsgalante/.config/foot/foot.ini";
    let ini = fs::read_to_string(path).unwrap_or_default();
    let mut new_lines = Vec::new();
    for line in ini.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font") {
            if let Some(pos) = line.find("terminal:size=") {
                let mut new_line = line[..pos + 14].to_string();
                new_line.push_str(&size.to_string());
                let size_str = &line[pos + 14..];
                let skip = size_str.find(|c: char| !c.is_ascii_digit()).unwrap_or(size_str.len());
                new_line.push_str(&size_str[skip..]);
                new_lines.push(new_line);
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }
    let _ = fs::write(path, new_lines.join("\n"));
}



fn parse_families(output: Option<std::process::Output>) -> Vec<String> {
    let mut families = Vec::new();
    if let Some(o) = output {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let family = trimmed.split(',').next().unwrap_or(trimmed).to_string();
                if !family.is_empty() && !families.contains(&family) {
                    families.push(family);
                }
            }
        }
    }
    families.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    families
}

pub async fn fetch_typeface_state() -> InterfaceState {
    let (sans, serif, mono, borders, status, fuzzel_font, term) = read_preferred_fonts();
    
    let all_output = tokio::process::Command::new("fc-list")
        .args([":", "family"])
        .output().await.ok();
    let all_fonts = parse_families(all_output);
    
    let mono_output = tokio::process::Command::new("fc-list")
        .args([":spacing=100", "family"])
        .output().await.ok();
    let mono_fonts = parse_families(mono_output);
    


    let determine_dropdown_index = |font: &str, sans: &str, serif: &str, mono: &str| -> usize {
        if font == sans {
            0
        } else if font == serif {
            1
        } else if font == mono {
            2
        } else {
            3
        }
    };

    let borders_idx = determine_dropdown_index(&borders, &sans, &serif, &mono);
    let status_idx = determine_dropdown_index(&status, &sans, &serif, &mono);
    let fuzzel_idx = determine_dropdown_index(&fuzzel_font, &sans, &serif, &mono);
    let terminal_idx = determine_dropdown_index(&term, &sans, &serif, &mono);
    let menu_options = vec![
        "Sans-Serif".to_string(),
        "Serif".to_string(),
        "Monospace".to_string(),
        "Other".to_string(),
    ];

    let mut borders_box = TextBox::new(borders.clone()).with_label("Window Borders").with_config(FONTS_CONF_PATH, "window_borders");
    borders_box.disabled = borders_idx != 3;

    let mut status_box = TextBox::new(status.clone()).with_label("Status Interface").with_config(FONTS_CONF_PATH, "status_interface");
    status_box.disabled = status_idx != 3;

    let mut fuzzel_box = TextBox::new(fuzzel_font.clone()).with_label("Fuzzel").with_config(FONTS_CONF_PATH, "fuzzel");
    fuzzel_box.disabled = fuzzel_idx != 3;

    let mut terminal_box = TextBox::new(term.clone()).with_label("Terminal").with_config(FONTS_CONF_PATH, "terminal");
    terminal_box.disabled = terminal_idx != 3;


    let borders_size = read_border_font_size().unwrap_or(11);
    let status_size = read_status_size().unwrap_or(11);
    let fuzzel_size = read_fuzzel_size().unwrap_or(14);
    let terminal_size = read_terminal_size().unwrap_or(12);

    let mut state = InterfaceState::default();
    state.typeface_loaded = true;
    state.sans_serif = sans.clone();
    state.serif = serif.clone();
    state.monospace = mono.clone();
    state.window_borders = borders;
    state.status_interface = status;
    state.fuzzel = fuzzel_font;
    state.terminal = term;
    state.all_fonts = all_fonts;
    state.mono_fonts = mono_fonts;
    state.sans_box = TextBox::new(sans).with_label("Sans-Serif").with_config(FONTS_CONF_PATH, "sans-serif");
    state.serif_box = TextBox::new(serif).with_label("Serif").with_config(FONTS_CONF_PATH, "serif");
    state.mono_box = TextBox::new(mono).with_label("Monospace").with_config(FONTS_CONF_PATH, "monospace");
    state.borders_box = borders_box;
    state.status_box = status_box;
    state.fuzzel_box = fuzzel_box;
    state.terminal_box = terminal_box;
    state.borders_menu = Dropdown::new(menu_options.clone(), borders_idx).with_config(FONTS_CONF_PATH, "window_borders");
    state.status_menu = Dropdown::new(menu_options.clone(), status_idx).with_config(FONTS_CONF_PATH, "status_interface");
    state.fuzzel_menu = Dropdown::new(menu_options.clone(), fuzzel_idx).with_config(FONTS_CONF_PATH, "fuzzel");
    state.terminal_menu = Dropdown::new(menu_options, terminal_idx).with_config(FONTS_CONF_PATH, "terminal");
    state.borders_size_box = Spinbox::new(borders_size as i32, 6, 72, 1).with_config(CONFIG_PATH, "border_font_size");
    state.status_size_box = Spinbox::new(status_size as i32, 6, 72, 1).with_config(CONFIG_PATH, "status_font_size");
    state.fuzzel_size_box = Spinbox::new(fuzzel_size as i32, 6, 72, 1).with_config("/home/lsgalante/.config/fuzzel/fuzzel.ini", "size");
    state.terminal_size_box = Spinbox::new(terminal_size as i32, 6, 72, 1).with_config("/home/lsgalante/.config/foot/foot.ini", "size");
    state
}

pub fn view(state: &mut InterfaceState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 260.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(10);


    // 1. Custom Parameters Section
    builder.add_section_with_width(&mut final_pc, cw, "Custom Parameters", false, |sec| {
        let h = state.custom_multicontrol.preferred_height().unwrap_or(100.0);
        sec.widget_full(&mut state.custom_multicontrol, h, ctx);
    });

    // 2. Layout Section
    builder.add_section_with_width(&mut final_pc, cw, "Layout", false, |sec| {
        state.color_selectors[1].color = state.high_color;
        sec.widget_full(&mut state.color_selectors[1], 40.0, ctx);
        state.color_selectors[2].color = state.visual_guides_color;
        sec.widget_full(&mut state.color_selectors[2], 40.0, ctx);

        // Grid Layout child section
        sec.add_section("Adaptive Grid", false, |subsec| {
            state.grid_min_col_width_spinbox.value = state.grid_min_col_width as i32;
            subsec.widget_full(&mut state.grid_min_col_width_spinbox, 44.0, ctx);
            state.layout_grid_gap_spinbox.value = state.layout_grid_gap as i32;
            subsec.widget_full(&mut state.layout_grid_gap_spinbox, 44.0, ctx);
        });
    });


    // 3. Status Section
    builder.add_section_with_width(&mut final_pc, cw, "Status", false, |sec| {
        state.color_selectors[7].color = state.normal_color;
        sec.widget_full(&mut state.color_selectors[7], 40.0, ctx);
        state.color_selectors[3].color = state.disabled_color;
        sec.widget_full(&mut state.color_selectors[3], 40.0, ctx);
        state.color_selectors[22].color = state.status_box_background_color;
        sec.widget_full(&mut state.color_selectors[22], 40.0, ctx);
        state.status_box_corner_radius_spinbox.value = state.status_box_corner_radius as i32;
        sec.widget_full(&mut state.status_box_corner_radius_spinbox, 44.0, ctx);
        state.status_padding_spinbox.value = state.status_padding as i32;
        sec.widget_full(&mut state.status_padding_spinbox, 44.0, ctx);
        state.status_module_spacing_spinbox.value = state.status_module_spacing as i32;
        sec.widget_full(&mut state.status_module_spacing_spinbox, 44.0, ctx);

        // Opacity slider
        state.status_controls.box_opacity_slider.set_label(&format!("Background Opacity: {}%", (state.status_controls.box_opacity * 100.0).round() as i32));
        state.status_controls.box_opacity_slider.set_value(state.status_controls.box_opacity);
        let label_h_op = cce_ui::widget::label_offset(&state.status_controls.box_opacity_slider);
        let slider_h_op = cce_ui::layout::slider_height() + label_h_op;
        sec.widget_full(&mut state.status_controls.box_opacity_slider, slider_h_op, ctx);

        // Blur slider
        state.status_controls.box_blur_slider.set_label(&format!("Background Blur: {}%", (state.status_controls.box_blur * 100.0).round() as i32));
        state.status_controls.box_blur_slider.set_value(state.status_controls.box_blur);
        let label_h_bl = cce_ui::widget::label_offset(&state.status_controls.box_blur_slider);
        let slider_h_bl = cce_ui::layout::slider_height() + label_h_bl;
        sec.widget_full(&mut state.status_controls.box_blur_slider, slider_h_bl, ctx);
    });

    // 4. Controls Section
    builder.add_section_with_width(&mut final_pc, cw, "Controls", false, |sec| {
        // Slider Section
        sec.add_section("Slider", false, |subsec| {
            state.color_selectors[4].color = state.slider_track_color;
            subsec.widget_full(&mut state.color_selectors[4], 40.0, ctx);
            state.slider_height_spinbox.value = state.slider_height as i32;
            subsec.widget_full(&mut state.slider_height_spinbox, 44.0, ctx);
            state.slider_corner_radius_spinbox.value = state.slider_corner_radius as i32;
            subsec.widget_full(&mut state.slider_corner_radius_spinbox, 44.0, ctx);
            state.slider_font_selector.font_family = state.slider_font.clone();
            subsec.widget_full(&mut state.slider_font_selector, 44.0, ctx);
        });


        // MenuBar Section
        sec.add_section("MenuBar", false, |subsec| {
            state.color_selectors[8].color = state.paginator_sidebar_color;
            subsec.widget_full(&mut state.color_selectors[8], 40.0, ctx);
            state.color_selectors[10].color = state.menubar_tab_label_color;
            subsec.widget_full(&mut state.color_selectors[10], 40.0, ctx);
            state.menubar_font_selector.font_family = state.menubar_font.clone();
            subsec.widget_full(&mut state.menubar_font_selector, 44.0, ctx);
            state.menubar_opacity_spinbox.value = (state.menubar_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.menubar_opacity_spinbox, 44.0, ctx);
        });

        // Toggle Section
        sec.add_section("Toggle", false, |subsec| {
            state.color_selectors[11].color = state.toggle_enabled_color;
            subsec.widget_full(&mut state.color_selectors[11], 40.0, ctx);
            state.color_selectors[12].color = state.toggle_disabled_color;
            subsec.widget_full(&mut state.color_selectors[12], 40.0, ctx);
            state.color_selectors[24].color = state.toggle_bg_color;
            subsec.widget_full(&mut state.color_selectors[24], 40.0, ctx);
            state.toggle_height_spinbox.value = state.toggle_height as i32;
            subsec.widget_full(&mut state.toggle_height_spinbox, 44.0, ctx);
            state.toggle_corner_radius_spinbox.value = state.toggle_corner_radius as i32;
            subsec.widget_full(&mut state.toggle_corner_radius_spinbox, 44.0, ctx);
            state.toggle_border_width_spinbox.value = state.toggle_border_width as i32;
            subsec.widget_full(&mut state.toggle_border_width_spinbox, 44.0, ctx);
            state.toggle_font_selector.font_family = state.toggle_font.clone();
            subsec.widget_full(&mut state.toggle_font_selector, 44.0, ctx);
        });

        // Breadcrumb Section
        sec.add_section("Breadcrumb", false, |subsec| {
            state.color_selectors[14].color = state.breadcrumb_bg_color;
            subsec.widget_full(&mut state.color_selectors[14], 40.0, ctx);
            state.breadcrumb_font_selector.font_family = state.breadcrumb_font.clone();
            subsec.widget_full(&mut state.breadcrumb_font_selector, 44.0, ctx);
        });


        // Spinbox Section
        sec.add_section("Spinbox", false, |subsec| {
            state.spinbox_height_spinbox.value = state.spinbox_height as i32;
            subsec.widget_full(&mut state.spinbox_height_spinbox, 44.0, ctx);

            state.spinbox_corner_radius_spinbox.value = state.spinbox_corner_radius as i32;
            subsec.widget_full(&mut state.spinbox_corner_radius_spinbox, 44.0, ctx);
            state.spinbox_font_selector.font_family = state.spinbox_font.clone();
            subsec.widget_full(&mut state.spinbox_font_selector, 44.0, ctx);
        });

        // ColorSelector Section
        sec.add_section("ColorSelector", false, |subsec| {
            state.color_selector_height_spinbox.value = state.color_selector_height as i32;
            subsec.widget_full(&mut state.color_selector_height_spinbox, 44.0, ctx);

            state.color_selector_preview_corner_radius_spinbox.value = state.color_selector_preview_corner_radius as i32;
            subsec.widget_full(&mut state.color_selector_preview_corner_radius_spinbox, 44.0, ctx);

            state.color_selector_preview_margin_spinbox.value = state.color_selector_preview_margin as i32;
            subsec.widget_full(&mut state.color_selector_preview_margin_spinbox, 44.0, ctx);

            state.color_selector_corner_radius_spinbox.value = state.color_selector_corner_radius as i32;
            subsec.widget_full(&mut state.color_selector_corner_radius_spinbox, 44.0, ctx);

            state.color_selector_font_selector.font_family = state.color_selector_font.clone();
            subsec.widget_full(&mut state.color_selector_font_selector, 44.0, ctx);
        });

        // Textbox Section
        sec.add_section("Textbox", false, |subsec| {
            state.textbox_height_spinbox.value = state.textbox_height as i32;
            subsec.widget_full(&mut state.textbox_height_spinbox, 44.0, ctx);
            state.textbox_corner_radius_spinbox.value = state.textbox_corner_radius as i32;
            subsec.widget_full(&mut state.textbox_corner_radius_spinbox, 44.0, ctx);
            state.textbox_font_selector.font_family = state.textbox_font.clone();
            subsec.widget_full(&mut state.textbox_font_selector, 44.0, ctx);
        });

        // FontSelector Section
        sec.add_section("FontSelector", false, |subsec| {
            state.font_selector_height_spinbox.value = state.font_selector_height as i32;
            subsec.widget_full(&mut state.font_selector_height_spinbox, 44.0, ctx);
            state.font_selector_corner_radius_spinbox.value = state.font_selector_corner_radius as i32;
            subsec.widget_full(&mut state.font_selector_corner_radius_spinbox, 44.0, ctx);
            state.font_selector_font_selector.font_family = state.font_selector_font.clone();
            subsec.widget_full(&mut state.font_selector_font_selector, 44.0, ctx);
        });

        // Dropdown Section
        sec.add_section("Dropdown", false, |subsec| {
            state.dropdown_height_spinbox.value = state.dropdown_height as i32;
            subsec.widget_full(&mut state.dropdown_height_spinbox, 44.0, ctx);
            state.dropdown_corner_radius_spinbox.value = state.dropdown_corner_radius as i32;
            subsec.widget_full(&mut state.dropdown_corner_radius_spinbox, 44.0, ctx);
            state.dropdown_font_selector.font_family = state.dropdown_font.clone();
            subsec.widget_full(&mut state.dropdown_font_selector, 44.0, ctx);
        });

        // Button Section
        sec.add_section("Button", false, |subsec| {
            state.button_corner_radius_spinbox.value = state.button_corner_radius as i32;
            subsec.widget_full(&mut state.button_corner_radius_spinbox, 44.0, ctx);
            state.button_font_selector.font_family = state.button_font.clone();
            subsec.widget_full(&mut state.button_font_selector, 44.0, ctx);
        });

        // ButtonStrip Section
        sec.add_section("ButtonStrip", false, |subsec| {
            state.button_padding_spinbox.value = state.button_padding as i32;
            subsec.widget_full(&mut state.button_padding_spinbox, 44.0, ctx);
            state.button_strip_spacing_spinbox.value = state.button_strip_spacing as i32;
            subsec.widget_full(&mut state.button_strip_spacing_spinbox, 44.0, ctx);
            state.button_strip_font_selector.font_family = state.button_strip_font.clone();
            subsec.widget_full(&mut state.button_strip_font_selector, 44.0, ctx);
        });

        // Labels Section
        sec.add_section("Labels", false, |subsec| {
            state.label_margin_spinbox.value = state.label_margin as i32;
            subsec.widget_full(&mut state.label_margin_spinbox, 44.0, ctx);
            state.label_font_selector.font_family = state.label_font.clone();
            subsec.widget_full(&mut state.label_font_selector, 44.0, ctx);
        });
    });

    // 5. Indicators Section
    builder.add_section_with_width(&mut final_pc, cw, "Indicators", false, |sec| {
        sec.add_section("Primary Highlight", false, |subsec| {
            state.color_selectors[9].color = state.primary_highlight_color;
            subsec.widget_full(&mut state.color_selectors[9], 40.0, ctx);
        });
    });

    // 5. Notification Section
    builder.add_section_with_width(&mut final_pc, cw, "Notification", false, |sec| {
        state.color_selectors[16].color = state.notification_bg_color;
        sec.widget_full(&mut state.color_selectors[16], 40.0, ctx);
        state.notification_opacity_spinbox.value = (state.notification_opacity * 100.0).round() as i32;
        sec.widget_full(&mut state.notification_opacity_spinbox, 44.0, ctx);
    });

    // 6. Surfaces Section
    builder.add_section_with_width(&mut final_pc, cw, "Surfaces", sec_focused.get(6).copied().unwrap_or(false), |sec| {
        sec.add_section("Backplate", false, |subsec| {
            state.color_selectors[17].color = state.backplate_color;
            subsec.widget_full(&mut state.color_selectors[17], 40.0, ctx);
            state.backplate_corner_radius_spinbox.value = state.backplate_corner_radius as i32;
            subsec.widget_full(&mut state.backplate_corner_radius_spinbox, 44.0, ctx);
        });

        // Plate child section
        sec.add_section("Plate", false, |subsec| {
            state.color_selectors[0].color = state.page_low_color;
            subsec.widget_full(&mut state.color_selectors[0], 40.0, ctx);
            state.plate_padding_spinbox.value = state.plate_padding as i32;
            subsec.widget_full(&mut state.plate_padding_spinbox, 44.0, ctx);
            state.plate_opacity_spinbox.value = (state.plate_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.plate_opacity_spinbox, 44.0, ctx);
            state.plate_corner_radius_spinbox.value = state.plate_corner_radius as i32;
            subsec.widget_full(&mut state.plate_corner_radius_spinbox, 44.0, ctx);
        });

        // Popover Section
        sec.add_section("Popover", false, |subsec| {
            state.color_selectors[15].color = state.popover_bg_color;
            subsec.widget_full(&mut state.color_selectors[15], 40.0, ctx);
        });

        // Desktop Section
        sec.add_section("Desktop", false, |subsec| {
            state.color_selectors[6].color = state.desktop_background_color;
            subsec.widget_full(&mut state.color_selectors[6], 40.0, ctx);

            let grid_color_rgba = state.desktop_grid_color;
            state.color_selectors[23].color = [grid_color_rgba[0], grid_color_rgba[1], grid_color_rgba[2]];
            state.color_selectors[23].alpha = grid_color_rgba[3];
            subsec.widget_full(&mut state.color_selectors[23], 40.0, ctx);

            state.desktop_grid_scale_spinbox.value = state.desktop_grid_scale as i32;
            subsec.widget_full(&mut state.desktop_grid_scale_spinbox, 44.0, ctx);

            state.desktop_line_width_spinbox.value = state.desktop_line_width as i32;
            subsec.widget_full(&mut state.desktop_line_width_spinbox, 44.0, ctx);
        });
    });


    let widget_h = 26.0;
    const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

    // 7. Fonts Section
    builder.add_section_with_width(&mut final_pc, cw, "Fonts", sec_focused.get(7).copied().unwrap_or(false), |sec| {

        // System Fonts Section
        sec.add_section("System Fonts", false, |subsec| {

            if !state.typeface_loaded {
                subsec.text("Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
            } else {
                // Sans-Serif
                subsec.widget_full(&mut state.sans_box, 44.0, ctx);

                // Serif
                subsec.widget_full(&mut state.serif_box, 44.0, ctx);

                // Monospace
                subsec.widget_full(&mut state.mono_box, 44.0, ctx);
            }
        });

        // Program Fonts Section
        sec.add_section("Program Fonts", false, |subsec| {

            if !state.typeface_loaded {
                subsec.text("Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
            } else {
                // Window Borders
                let start_y = subsec.ay();
                let cols = subsec.row_layout(2, 10.0);
                if cols.len() == 2 {
                    state.borders_menu.set_row_rect(cols[0].0, cols[0].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.borders_menu, cols[0].0, start_y, cols[0].1, widget_h, ctx);
                    state.borders_size_box.set_row_rect(cols[1].0, cols[1].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.borders_size_box, cols[1].0, start_y, cols[1].1, widget_h, ctx);
                }
                subsec.widget_full(&mut state.borders_box, 44.0, ctx);

                // Status Interface
                let start_y = subsec.ay();
                let cols = subsec.row_layout(2, 10.0);
                if cols.len() == 2 {
                    state.status_menu.set_row_rect(cols[0].0, cols[0].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.status_menu, cols[0].0, start_y, cols[0].1, widget_h, ctx);
                    state.status_size_box.set_row_rect(cols[1].0, cols[1].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.status_size_box, cols[1].0, start_y, cols[1].1, widget_h, ctx);
                }
                subsec.widget_full(&mut state.status_box, 44.0, ctx);

                // Fuzzel
                let start_y = subsec.ay();
                let cols = subsec.row_layout(2, 10.0);
                if cols.len() == 2 {
                    state.fuzzel_menu.set_row_rect(cols[0].0, cols[0].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.fuzzel_menu, cols[0].0, start_y, cols[0].1, widget_h, ctx);
                    state.fuzzel_size_box.set_row_rect(cols[1].0, cols[1].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.fuzzel_size_box, cols[1].0, start_y, cols[1].1, widget_h, ctx);
                }
                subsec.widget_full(&mut state.fuzzel_box, 44.0, ctx);

                // Terminal
                let start_y = subsec.ay();
                let cols = subsec.row_layout(2, 10.0);
                if cols.len() == 2 {
                    state.terminal_menu.set_row_rect(cols[0].0, cols[0].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.terminal_menu, cols[0].0, start_y, cols[0].1, widget_h, ctx);
                    state.terminal_size_box.set_row_rect(cols[1].0, cols[1].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.terminal_size_box, cols[1].0, start_y, cols[1].1, widget_h, ctx);
                }
                subsec.widget_full(&mut state.terminal_box, 44.0, ctx);
            }
        });
    });

    builder.add_section_with_width(&mut final_pc, cw, "Containers", sec_focused.get(8).copied().unwrap_or(false), |sec| {

        // Page child section
        sec.add_section("Page", false, |subsec| {
            state.color_selectors[18].color = state.page_color;
            subsec.widget_full(&mut state.color_selectors[18], 40.0, ctx);
            state.page_opacity_spinbox.value = (state.page_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.page_opacity_spinbox, 44.0, ctx);
        });

        // Layer child section
        sec.add_section("Layer", false, |subsec| {
            state.color_selectors[19].color = state.layer_color;
            subsec.widget_full(&mut state.color_selectors[19], 40.0, ctx);
            state.layer_opacity_spinbox.value = (state.layer_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.layer_opacity_spinbox, 44.0, ctx);
        });

        // Section child section
        sec.add_section("Section", false, |subsec| {
            state.section_padding_spinbox.value = state.section_padding as i32;
            subsec.widget_full(&mut state.section_padding_spinbox, 44.0, ctx);
            state.page_margin_spinbox.value = state.page_margin as i32;
            subsec.widget_full(&mut state.page_margin_spinbox, 44.0, ctx);
            state.section_label_font_selector.font_family = state.section_label_font.clone();
            subsec.widget_full(&mut state.section_label_font_selector, 44.0, ctx);

            // Nested Section child section
            subsec.add_section("Nested Section", false, |subsubsec| {
                subsubsec.widget_full(&mut state.label_alignment_menu, 44.0, ctx);
                state.label_offset_spinbox.value = state.nested_section_label_offset as i32;
                subsubsec.widget_full(&mut state.label_offset_spinbox, 44.0, ctx);
                state.nested_section_label_font_selector.font_family = state.nested_section_label_font.clone();
                subsubsec.widget_full(&mut state.nested_section_label_font_selector, 44.0, ctx);
            });
        });

        // ScrollingList Section
        sec.add_section("ScrollingList", false, |subsec| {
            state.color_selectors[13].color = state.scrollinglist_bg_color;
            subsec.widget_full(&mut state.color_selectors[13], 40.0, ctx);

            let bg_rgba = state.scrollinglist_entry_bg_color;
            state.color_selectors[20].color = [bg_rgba[0], bg_rgba[1], bg_rgba[2]];
            state.color_selectors[20].alpha = bg_rgba[3];
            subsec.widget_full(&mut state.color_selectors[20], 40.0, ctx);

            let highlight_rgba = state.scrollinglist_entry_highlight_color;
            state.color_selectors[21].color = [highlight_rgba[0], highlight_rgba[1], highlight_rgba[2]];
            state.color_selectors[21].alpha = highlight_rgba[3];
            subsec.widget_full(&mut state.color_selectors[21], 40.0, ctx);
        });

        // Graph child section
        sec.add_section("Graph", false, |subsec| {
            state.graph_show_grid_toggle.set_toggled(state.graph_show_grid);
            subsec.widget_full(&mut state.graph_show_grid_toggle, state.toggle_height as f32, ctx);
            state.graph_snap_enabled_toggle.set_toggled(state.graph_snap_enabled);
            subsec.widget_full(&mut state.graph_snap_enabled_toggle, state.toggle_height as f32, ctx);
            state.graph_uniform_background_toggle.set_toggled(state.graph_uniform_background);
            subsec.widget_full(&mut state.graph_uniform_background_toggle, state.toggle_height as f32, ctx);
            state.graph_cell_opacity_spinbox.value = (state.graph_cell_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.graph_cell_opacity_spinbox, 44.0, ctx);
            state.graph_gap_opacity_spinbox.value = (state.graph_gap_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.graph_gap_opacity_spinbox, 44.0, ctx);
            state.graph_gap_width_spinbox.value = state.graph_gap_width as i32;
            subsec.widget_full(&mut state.graph_gap_width_spinbox, 44.0, ctx);
        });
    });

    // 10. Windows Section
    builder.add_section_with_width(&mut final_pc, cw, "Windows", sec_focused.get(9).copied().unwrap_or(false), |sec| {


        // 2. Fullscreen Section
        sec.add_section("Fullscreen", false, |subsec| {
            state.windows.fullscreen_opacity_spinbox.set_label("Backplate Opacity");
            subsec.widget_full(&mut state.windows.fullscreen_opacity_spinbox, 44.0, ctx);
        });

        // 3. Cascade Section
        sec.add_section("Cascade", false, |subsec| {
            state.windows.cascade_offset_spinbox.set_label("Offset");
            subsec.widget_full(&mut state.windows.cascade_offset_spinbox, 44.0, ctx);
            state.windows.edge_gap_spinbox.set_label("Edge Gap");
            subsec.widget_full(&mut state.windows.edge_gap_spinbox, 44.0, ctx);
            state.windows.top_gap_spinbox.set_label("Top Gap");
            subsec.widget_full(&mut state.windows.top_gap_spinbox, 44.0, ctx);
            state.windows.cascade_opacity_spinbox.set_label("Backplate Opacity");
            subsec.widget_full(&mut state.windows.cascade_opacity_spinbox, 44.0, ctx);
        });

        // 4. Grid Section
        sec.add_section("Grid", false, |subsec| {
            state.windows.grid_opacity_spinbox.set_label("Backplate Opacity");
            subsec.widget_full(&mut state.windows.grid_opacity_spinbox, 44.0, ctx);
        });

        // 5. Floating Section
        sec.add_section("Floating", false, |subsec| {
            state.windows.floating_opacity_spinbox.set_label("Backplate Opacity");
            subsec.widget_full(&mut state.windows.floating_opacity_spinbox, 44.0, ctx);
        });

        // 6. Movement Section
        sec.add_section("Movement", false, |subsec| {
            state.windows.transition_duration_spinbox.set_label("Duration (ms)");
            subsec.widget_full(&mut state.windows.transition_duration_spinbox, 44.0, ctx);
        });

        // 8. Pinned Section
        sec.add_section("Pinned", false, |subsec| {
            subsec.widget_full(&mut state.windows.side_panel_behavior_menu, 44.0, ctx);
            subsec.widget_full(&mut state.windows.side_panel_position_menu, 44.0, ctx);
            state.windows.side_panel_width_spinbox.set_label("Default Width");
            subsec.widget_full(&mut state.windows.side_panel_width_spinbox, 44.0, ctx);
            state.windows.side_panel_border_gap_spinbox.set_label("Border Gap");
            subsec.widget_full(&mut state.windows.side_panel_border_gap_spinbox, 44.0, ctx);
            state.windows.pinned_opacity_spinbox.set_label("Backplate Opacity");
            subsec.widget_full(&mut state.windows.pinned_opacity_spinbox, 44.0, ctx);
        });

        // 8b. Popup Section
        sec.add_section("Popup", false, |subsec| {
            state.windows.popup_opacity_spinbox.set_label("Backplate Opacity");
            subsec.widget_full(&mut state.windows.popup_opacity_spinbox, 44.0, ctx);
        });

        // 9. Effects Section
        sec.add_section("Effects", false, |subsec| {
            state.windows.blur_toggle.set_toggled(state.windows.blur_enabled);
            subsec.widget_full(&mut state.windows.blur_toggle, cce_ui::layout::toggle_height(), ctx);
        });

    });

    final_pc
}


pub fn update(state: &mut InterfaceState, msg: InterfaceMessage) {
    match msg {
        InterfaceMessage::SetDesktopBackground(rgb) => {
            state.desktop_background_color = rgb;
            apply_desktop_background(rgb);
        }
        InterfaceMessage::SetDesktopGridColor(rgba) => {
            state.desktop_grid_color = rgba;
            apply_desktop_grid_color(rgba);
        }
        InterfaceMessage::SetDesktopGridScale(val) => {
            state.desktop_grid_scale = val;
            apply_desktop_grid_scale(val);
        }
        InterfaceMessage::SetDesktopLineWidth(val) => {
            state.desktop_line_width = val;
            apply_desktop_line_width(val);
        }
        InterfaceMessage::SetPageLowColor(rgb) => {
            state.page_low_color = rgb;
            apply_page_low_color(rgb);
        }
        InterfaceMessage::SetHighColor(rgb) => {
            state.high_color = rgb;
            apply_border_color(rgb);
        }
        InterfaceMessage::SetDisabledColor(rgb) => {
            state.disabled_color = rgb;
            apply_disabled_color(rgb);
        }

        InterfaceMessage::SetVisualGuidesColor(rgb) => {
            state.visual_guides_color = rgb;
            apply_visual_guides_color(rgb);
        }
        InterfaceMessage::SetSliderTrackColor(rgb) => {
            state.slider_track_color = rgb;
            apply_slider_track_color(rgb);
        }
        InterfaceMessage::SetColorBordersColor(rgb) => {
            state.color_borders_color = rgb;
            apply_color_borders_color(rgb);
        }
        InterfaceMessage::SetNormalColor(rgb) => {
            state.normal_color = rgb;
            apply_normal_color(rgb);
        }
        InterfaceMessage::SetPaginatorSidebarColor(rgb) => {
            state.paginator_sidebar_color = rgb;
            apply_paginator_sidebar_color(rgb);
        }
        InterfaceMessage::SetPrimaryHighlightColor(rgb) => {
            state.primary_highlight_color = rgb;
            apply_primary_highlight_color(rgb);
        }
        InterfaceMessage::SetMenubarTabLabelColor(rgb) => {
            state.menubar_tab_label_color = rgb;
            apply_menubar_tab_label_color(rgb);
        }
        InterfaceMessage::SetToggleEnabledColor(rgb) => {
            state.toggle_enabled_color = rgb;
            apply_toggle_enabled_color(rgb);
        }
        InterfaceMessage::SetToggleDisabledColor(rgb) => {
            state.toggle_disabled_color = rgb;
            apply_toggle_disabled_color(rgb);
        }
        InterfaceMessage::SetScrollingListBgColor(rgb) => {
            state.scrollinglist_bg_color = rgb;
            apply_scrollinglist_bg_color(rgb);
        }
        InterfaceMessage::SetScrollingListEntryBgColor(rgba) => {
            state.scrollinglist_entry_bg_color = rgba;
            apply_scrollinglist_entry_bg_color(rgba);
        }
        InterfaceMessage::SetScrollingListEntryHighlightColor(rgba) => {
            state.scrollinglist_entry_highlight_color = rgba;
            apply_scrollinglist_entry_highlight_color(rgba);
        }
        InterfaceMessage::SetBreadcrumbBgColor(rgb) => {
            state.breadcrumb_bg_color = rgb;
            apply_breadcrumb_bg_color(rgb);
        }
        InterfaceMessage::SetPopoverBgColor(rgb) => {
            state.popover_bg_color = rgb;
            apply_popover_bg_color(rgb);
        }
        InterfaceMessage::SetNotificationBgColor(rgb) => {
            state.notification_bg_color = rgb;
            apply_notifications_bg_color(rgb);
        }
        InterfaceMessage::SetNotificationOpacity(opacity) => {
            state.notification_opacity = opacity;
            state.notification_opacity_spinbox.value = (opacity * 100.0).round() as i32;
            apply_notifications_opacity(opacity);
        }
        InterfaceMessage::SetBackplateColor(rgb) => {
            state.backplate_color = rgb;
            apply_backplate_color(rgb);
        }

        InterfaceMessage::SetBackplateCornerRadius(radius) => {
            state.backplate_corner_radius = radius;
            state.backplate_corner_radius_spinbox.value = radius as i32;
            apply_backplate_corner_radius(radius);
        }
        InterfaceMessage::SetStatusBoxBackgroundColor(rgb) => {
            state.status_box_background_color = rgb;
            apply_status_box_background_color(rgb);
        }
        InterfaceMessage::SetStatusBoxCornerRadius(radius) => {
            state.status_box_corner_radius = radius;
            state.status_box_corner_radius_spinbox.value = radius as i32;
            apply_status_box_corner_radius(radius);
        }
        InterfaceMessage::SetStatusPadding(padding) => {
            state.status_padding = padding;
            state.status_padding_spinbox.value = padding as i32;
            apply_status_padding(padding);
        }
        InterfaceMessage::StatusSetPadding(val) => {
            state.status_padding = val;
            write_status_padding(val);
            status_interface_reload();
        }
        InterfaceMessage::StatusSetBoxOpacity(val) => {
            state.status_controls.box_opacity = val;
            write_status_box_opacity(val);
            status_interface_reload();
        }
        InterfaceMessage::StatusSetBoxBlur(val) => {
            state.status_controls.box_blur = val;
            write_status_box_blur(val);
            status_interface_reload();
        }
        InterfaceMessage::SetStatusModuleSpacing(spacing) => {
            state.status_module_spacing = spacing;
            state.status_module_spacing_spinbox.value = spacing as i32;
            apply_status_module_spacing(spacing);
        }
        InterfaceMessage::SetButtonPadding(padding) => {
            state.button_padding = padding;
            apply_button_padding(padding);
            propagate_links(state, "button_padding", &padding.to_string());
        }
        InterfaceMessage::SetButtonStripSpacing(spacing) => {
            state.button_strip_spacing = spacing;
            apply_button_strip_spacing(spacing);
            propagate_links(state, "button_strip_spacing", &spacing.to_string());
        }
        InterfaceMessage::SetSectionPadding(padding) => {
            state.section_padding = padding;
            apply_section_padding(padding);
            propagate_links(state, "section_padding", &padding.to_string());
        }
        InterfaceMessage::SetPlatePadding(padding) => {
            state.plate_padding = padding;
            apply_plate_padding(padding);
            propagate_links(state, "plate_padding", &padding.to_string());
        }
        InterfaceMessage::SetPlateOpacity(opacity) => {
            state.plate_opacity = opacity;
            apply_plate_opacity(opacity);
            propagate_links(state, "plate_opacity", &opacity.to_string());
        }
        InterfaceMessage::SetPlateCornerRadius(radius) => {
            state.plate_corner_radius = radius;
            apply_plate_corner_radius(radius);
            propagate_links(state, "plate_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetPageColor(rgb) => {
            state.page_color = rgb;
            apply_page_color(rgb);
            let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
            propagate_links(state, "page_color", &hex);
        }
        InterfaceMessage::SetPageOpacity(opacity) => {
            state.page_opacity = opacity;
            state.page_opacity_spinbox.value = (opacity * 100.0).round() as i32;
            apply_page_opacity(opacity);
            propagate_links(state, "page_opacity", &opacity.to_string());
        }
        InterfaceMessage::SetLayerColor(rgb) => {
            state.layer_color = rgb;
            apply_layer_color(rgb);
            let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
            propagate_links(state, "layer_color", &hex);
        }
        InterfaceMessage::SetLayerOpacity(opacity) => {
            state.layer_opacity = opacity;
            state.layer_opacity_spinbox.value = (opacity * 100.0).round() as i32;
            apply_layer_opacity(opacity);
            propagate_links(state, "layer_opacity", &opacity.to_string());
        }

        InterfaceMessage::SetPageMargin(margin) => {
            state.page_margin = margin;
            apply_page_margin(margin);
            propagate_links(state, "page_margin", &margin.to_string());
        }
        InterfaceMessage::SetGridMinColWidth(width) => {
            state.grid_min_col_width = width;
            apply_grid_min_col_width(width);
            propagate_links(state, "grid_min_col_width", &width.to_string());
        }
        InterfaceMessage::SetLayoutGridGap(gap) => {
            let gap = gap.min(200);
            state.layout_grid_gap = gap;
            state.layout_grid_gap_spinbox.value = gap as i32;
            apply_layout_grid_gap(gap);
            propagate_links(state, "grid_gap", &gap.to_string());
        }
        InterfaceMessage::SetSpinboxHeight(height) => {
            state.spinbox_height = height;
            apply_spinbox_height(height);
            propagate_links(state, "spinbox_height", &height.to_string());
        }
        InterfaceMessage::SetSpinboxCornerRadius(radius) => {
            state.spinbox_corner_radius = radius;
            apply_spinbox_corner_radius(radius);
            propagate_links(state, "spinbox_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetToggleHeight(height) => {
            state.toggle_height = height;
            apply_toggle_height(height);
            propagate_links(state, "toggle_height", &height.to_string());
        }
        InterfaceMessage::SetToggleCornerRadius(radius) => {
            state.toggle_corner_radius = radius;
            apply_toggle_corner_radius(radius);
            propagate_links(state, "toggle_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetToggleBgColor(rgb) => {
            state.toggle_bg_color = rgb;
            apply_toggle_bg_color(rgb);
            let hex = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
            propagate_links(state, "toggle_bg_color", &hex);
        }
        InterfaceMessage::SetToggleBorderWidth(width) => {
            state.toggle_border_width = width;
            apply_toggle_border_width(width);
            propagate_links(state, "toggle_border_width", &width.to_string());
        }
        InterfaceMessage::SetToggleFont(font) => {
            state.toggle_font = font.clone();
            apply_toggle_font(&font);
            propagate_links(state, "toggle_font", &format!("\"{}\"", font));
        }

        InterfaceMessage::SetColorSelectorHeight(height) => {
            state.color_selector_height = height;
            apply_color_selector_height(height);
            propagate_links(state, "color_selector_height", &height.to_string());
        }
        InterfaceMessage::SetColorSelectorCornerRadius(radius) => {
            state.color_selector_corner_radius = radius;
            apply_color_selector_corner_radius(radius);
            propagate_links(state, "color_selector_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetColorSelectorPreviewCornerRadius(radius) => {
            state.color_selector_preview_corner_radius = radius;
            apply_color_selector_preview_corner_radius(radius);
            propagate_links(state, "color_selector_preview_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetColorSelectorPreviewMargin(margin) => {
            state.color_selector_preview_margin = margin;
            apply_color_selector_preview_margin(margin);
            propagate_links(state, "color_selector_preview_margin", &margin.to_string());
        }
        InterfaceMessage::SetTextboxHeight(height) => {
            state.textbox_height = height;
            apply_textbox_height(height);
            propagate_links(state, "textbox_height", &height.to_string());
        }
        InterfaceMessage::SetTextboxCornerRadius(radius) => {
            state.textbox_corner_radius = radius;
            apply_textbox_corner_radius(radius);
            propagate_links(state, "textbox_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetSliderHeight(height) => {
            state.slider_height = height;
            apply_slider_height(height);
            propagate_links(state, "slider_height", &height.to_string());
        }
        InterfaceMessage::SetSliderCornerRadius(radius) => {
            state.slider_corner_radius = radius;
            apply_slider_corner_radius(radius);
            propagate_links(state, "slider_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetFontSelectorHeight(height) => {
            state.font_selector_height = height;
            apply_font_selector_height(height);
            propagate_links(state, "font_selector_height", &height.to_string());
        }
        InterfaceMessage::SetFontSelectorCornerRadius(radius) => {
            state.font_selector_corner_radius = radius;
            apply_font_selector_corner_radius(radius);
            propagate_links(state, "font_selector_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetFontSelectorFont(font) => {
            state.font_selector_font = font.clone();
            apply_font_selector_font(&font);
            propagate_links(state, "font_selector_font", &format!("\"{}\"", font));
        }
        InterfaceMessage::SetButtonStripFont(font) => {
            state.button_strip_font = font.clone();
            apply_button_strip_font(&font);
            propagate_links(state, "button_strip_font", &format!("\"{}\"", font));
        }
        InterfaceMessage::SetButtonFont(font) => {
            state.button_font = font.clone();
            apply_button_font(&font);
            propagate_links(state, "button_font", &format!("\"{}\"", font));
        }
        InterfaceMessage::SetLabelFont(font) => {
            state.label_font = font.clone();
            apply_label_font(&font);
            propagate_links(state, "label_font", &format!("\"{}\"", font));
        }
        InterfaceMessage::SetDropdownFont(font) => {
            state.dropdown_font = font.clone();
            apply_dropdown_font(&font);
            propagate_links(state, "dropdown_font", &format!("\"{}\"", font));
        }
        InterfaceMessage::SetTextboxFont(font) => {
            state.textbox_font = font.clone();
            apply_textbox_font(&font);
            propagate_links(state, "textbox_font", &format!("\"{}\"", font));
        }
        InterfaceMessage::SetSpinboxFont(font) => {
            state.spinbox_font = font.clone();
            apply_spinbox_font(&font);
            propagate_links(state, "spinbox_font", &format!("\"{}\"", font));
        }
        InterfaceMessage::SetSliderFont(font) => {
            state.slider_font = font.clone();
            apply_slider_font(&font);
            propagate_links(state, "slider_font", &format!("\"{}\"", font));
        }
        InterfaceMessage::SetDropdownHeight(height) => {
            state.dropdown_height = height;
            apply_dropdown_height(height);
            propagate_links(state, "dropdown_height", &height.to_string());
        }
        InterfaceMessage::SetDropdownCornerRadius(radius) => {
            state.dropdown_corner_radius = radius;
            apply_dropdown_corner_radius(radius);
            propagate_links(state, "dropdown_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetButtonCornerRadius(radius) => {
            state.button_corner_radius = radius;
            apply_button_corner_radius(radius);
            propagate_links(state, "button_corner_radius", &radius.to_string());
        }
        InterfaceMessage::SetColorSelectorFont(font) => {
            state.color_selector_font = font.clone();
            state.color_selector_font_selector.font_family = font.clone();
            for cs in &mut state.color_selectors {
                cs.font_family = font.clone();
            }
            apply_color_selector_font(&font);
        }
        InterfaceMessage::SetMenubarFont(font) => {
            state.menubar_font = font.clone();
            state.menubar_font_selector.font_family = font.clone();
            apply_menubar_font(&font);
        }
        InterfaceMessage::SetBreadcrumbFont(font) => {
            state.breadcrumb_font = font.clone();
            state.breadcrumb_font_selector.font_family = font.clone();
            apply_breadcrumb_font(&font);
        }
        InterfaceMessage::SetSectionLabelFont(font) => {
            state.section_label_font = font.clone();
            state.section_label_font_selector.font_family = font.clone();
            apply_section_label_font(&font);
        }
        InterfaceMessage::SetNestedSectionLabelFont(font) => {
            state.nested_section_label_font = font.clone();
            state.nested_section_label_font_selector.font_family = font.clone();
            apply_nested_section_label_font(&font);
        }
        InterfaceMessage::SetNestedSectionLabelAlignment(idx) => {
            state.nested_section_label_alignment = idx as u8;
            state.label_alignment_menu.selected = idx;
            apply_nested_section_label_alignment(idx as u8);
        }
        InterfaceMessage::SetNestedSectionLabelOffset(offset) => {
            state.nested_section_label_offset = offset;
            state.label_offset_spinbox.value = offset as i32;
            apply_nested_section_label_offset(offset);
            propagate_links(state, "nested_section_label_offset", &offset.to_string());
        }
        InterfaceMessage::SetLabelMargin(margin) => {
            state.label_margin = margin;
            state.label_margin_spinbox.value = margin as i32;
            apply_label_margin(margin);
            propagate_links(state, "label_margin", &margin.to_string());
        }
        InterfaceMessage::SetGraphShowGrid(show) => {
            state.graph_show_grid = show;
            state.graph_show_grid_toggle.set_toggled(show);
            write_config_value("graph_show_grid", &show.to_string());
            cce_graph_reload();
        }
        InterfaceMessage::SetGraphSnapEnabled(snap) => {
            state.graph_snap_enabled = snap;
            state.graph_snap_enabled_toggle.set_toggled(snap);
            write_config_value("graph_snap_enabled", &snap.to_string());
            cce_graph_reload();
        }
        InterfaceMessage::SetGraphUniformBackground(uniform) => {
            state.graph_uniform_background = uniform;
            state.graph_uniform_background_toggle.set_toggled(uniform);
            write_config_value("graph_uniform_background", &uniform.to_string());
            cce_graph_reload();
        }
        InterfaceMessage::SetGraphCellOpacity(opacity) => {
            state.graph_cell_opacity = opacity;
            state.graph_cell_opacity_spinbox.value = (opacity * 100.0).round() as i32;
            write_config_value("graph_cell_opacity", &format!("{:.2}", opacity));
            cce_graph_reload();
            propagate_links(state, "graph_cell_opacity", &opacity.to_string());
        }
        InterfaceMessage::SetGraphGapOpacity(opacity) => {
            state.graph_gap_opacity = opacity;
            state.graph_gap_opacity_spinbox.value = (opacity * 100.0).round() as i32;
            write_config_value("graph_gap_opacity", &format!("{:.2}", opacity));
            cce_graph_reload();
            propagate_links(state, "graph_gap_opacity", &opacity.to_string());
        }
        InterfaceMessage::SetGraphGapWidth(gap) => {
            state.graph_gap_width = gap;
            state.graph_gap_width_spinbox.value = gap as i32;
            write_config_value("graph_gap_width", &gap.to_string());
            cce_graph_reload();
            propagate_links(state, "graph_gap_width", &gap.to_string());
        }
        InterfaceMessage::SetMenubarOpacity(opacity) => {
            state.menubar_opacity = opacity;
            state.menubar_opacity_spinbox.value = (opacity * 100.0).round() as i32;
            write_config_value("menubar_opacity", &format!("{:.2}", opacity));
            send_ipc_command("reload");
            status_interface_reload();
            propagate_links(state, "menubar_opacity", &opacity.to_string());
        }
        InterfaceMessage::PickLowColor | InterfaceMessage::PickHighColor | InterfaceMessage::PickDisabledColor | InterfaceMessage::PickVisualGuides | InterfaceMessage::PickSliderTrackColor | InterfaceMessage::PickPageLowColor | InterfaceMessage::PickColorBordersColor | InterfaceMessage::PickNormalColor | InterfaceMessage::PickPaginatorSidebarColor | InterfaceMessage::PickPrimaryHighlightColor | InterfaceMessage::PickMenubarTabLabelColor | InterfaceMessage::PickToggleEnabledColor | InterfaceMessage::PickToggleDisabledColor | InterfaceMessage::PickScrollingListBgColor | InterfaceMessage::PickScrollingListEntryBgColor | InterfaceMessage::PickScrollingListEntryHighlightColor | InterfaceMessage::PickBreadcrumbBgColor | InterfaceMessage::PickPopoverBgColor | InterfaceMessage::PickNotificationBgColor | InterfaceMessage::PickWindowColor | InterfaceMessage::PickPageColor | InterfaceMessage::PickLayerColor => {}
        InterfaceMessage::Refreshed(new) => {
            let was_bp_hovered = state.button_padding_spinbox.hovered();
            let was_bss_hovered = state.button_strip_spacing_spinbox.hovered();
            let was_sp_hovered = state.section_padding_spinbox.hovered();
            let was_pp_hovered = state.plate_padding_spinbox.hovered();
            let was_pl_op_hovered = state.plate_opacity_spinbox.hovered();
            let was_pl_cr_hovered = state.plate_corner_radius_spinbox.hovered();
            let was_page_op_hovered = state.page_opacity_spinbox.hovered();
            let was_layer_op_hovered = state.layer_opacity_spinbox.hovered();
            let was_sp_pad_hovered = state.status_padding_spinbox.hovered();
            let was_sp_spc_hovered = state.status_module_spacing_spinbox.hovered();

            let was_pm_hovered = state.page_margin_spinbox.hovered();
            let was_gm_hovered = state.grid_min_col_width_spinbox.hovered();
            let was_sh_hovered = state.spinbox_height_spinbox.hovered();
            let was_scr_hovered = state.spinbox_corner_radius_spinbox.hovered();
            let was_th_hovered = state.toggle_height_spinbox.hovered();
            let was_tgcr_hovered = state.toggle_corner_radius_spinbox.hovered();

            let was_gsg_hovered = state.graph_show_grid_toggle.hovered();
            let was_gse_hovered = state.graph_snap_enabled_toggle.hovered();
            let was_gub_hovered = state.graph_uniform_background_toggle.hovered();
            let was_gco_hovered = state.graph_cell_opacity_spinbox.hovered();
            let was_ggo_hovered = state.graph_gap_opacity_spinbox.hovered();
            let was_ggw_hovered = state.graph_gap_width_spinbox.hovered();
            let was_csh_hovered = state.color_selector_height_spinbox.hovered();
            let was_cspcr_hovered = state.color_selector_preview_corner_radius_spinbox.hovered();
            let was_cspm_hovered = state.color_selector_preview_margin_spinbox.hovered();
            let was_cscr_hovered = state.color_selector_corner_radius_spinbox.hovered();
            let was_tbh_hovered = state.textbox_height_spinbox.hovered();
            let was_tcr_hovered = state.textbox_corner_radius_spinbox.hovered();
            let was_fsh_hovered = state.font_selector_height_spinbox.hovered();
            let was_fscr_hovered = state.font_selector_corner_radius_spinbox.hovered();
            let was_bfs_hovered = state.breadcrumb_font_selector.hovered();
            let was_fsfs_hovered = state.font_selector_font_selector.hovered();
            let was_bsfs_hovered = state.button_strip_font_selector.hovered();
            let was_bfsn_hovered = state.button_font_selector.hovered();
            let was_lfs_hovered = state.label_font_selector.hovered();
            let was_dfs_hovered = state.dropdown_font_selector.hovered();
            let was_tbfs_hovered = state.textbox_font_selector.hovered();
            let was_sfs_hovered = state.spinbox_font_selector.hovered();
            let was_slfs_hovered = state.slider_font_selector.hovered();

            let button_strip_font = state.button_strip_font.clone();
            let button_font = state.button_font.clone();
            let label_font = state.label_font.clone();
            let dropdown_font = state.dropdown_font.clone();
            let textbox_font = state.textbox_font.clone();
            let spinbox_font = state.spinbox_font.clone();
            let slider_font = state.slider_font.clone();

            let was_lm_hovered = state.label_margin_spinbox.hovered();
            let was_mo_hovered = state.menubar_opacity_spinbox.hovered();
            let was_no_hovered = state.notification_opacity_spinbox.hovered();
            let was_wcr_hovered = state.backplate_corner_radius_spinbox.hovered();
            // Preserve typeface fields
            let typeface_loaded = state.typeface_loaded;
            let sans_serif = state.sans_serif.clone();
            let serif = state.serif.clone();
            let monospace = state.monospace.clone();
            let window_borders = state.window_borders.clone();
            let status_interface = state.status_interface.clone();
            let fuzzel = state.fuzzel.clone();
            let terminal = state.terminal.clone();
            let all_fonts = state.all_fonts.clone();
            let mono_fonts = state.mono_fonts.clone();
            let sans_box = state.sans_box.clone();
            let serif_box = state.serif_box.clone();
            let mono_box = state.mono_box.clone();
            let borders_box = state.borders_box.clone();
            let status_box = state.status_box.clone();
            let fuzzel_box = state.fuzzel_box.clone();
            let terminal_box = state.terminal_box.clone();
            let borders_menu = state.borders_menu.clone();
            let status_menu = state.status_menu.clone();
            let fuzzel_menu = state.fuzzel_menu.clone();
            let terminal_menu = state.terminal_menu.clone();
            let borders_size_box = state.borders_size_box.clone();
            let status_size_box = state.status_size_box.clone();
            let fuzzel_size_box = state.fuzzel_size_box.clone();
            let terminal_size_box = state.terminal_size_box.clone();

            *state = new;

            state.button_padding_spinbox.set_hovered(was_bp_hovered);
            state.button_strip_spacing_spinbox.set_hovered(was_bss_hovered);
            state.section_padding_spinbox.set_hovered(was_sp_hovered);
            state.plate_padding_spinbox.set_hovered(was_pp_hovered);
            state.plate_opacity_spinbox.set_hovered(was_pl_op_hovered);
            state.plate_corner_radius_spinbox.set_hovered(was_pl_cr_hovered);
            state.page_opacity_spinbox.set_hovered(was_page_op_hovered);
            state.layer_opacity_spinbox.set_hovered(was_layer_op_hovered);

            state.page_margin_spinbox.set_hovered(was_pm_hovered);
            state.grid_min_col_width_spinbox.set_hovered(was_gm_hovered);
            state.spinbox_height_spinbox.set_hovered(was_sh_hovered);
            state.spinbox_corner_radius_spinbox.set_hovered(was_scr_hovered);
            state.toggle_height_spinbox.set_hovered(was_th_hovered);
            state.toggle_corner_radius_spinbox.set_hovered(was_tgcr_hovered);

            state.graph_show_grid_toggle.set_hovered(was_gsg_hovered);
            state.graph_snap_enabled_toggle.set_hovered(was_gse_hovered);
            state.graph_uniform_background_toggle.set_hovered(was_gub_hovered);
            state.graph_cell_opacity_spinbox.set_hovered(was_gco_hovered);
            state.graph_gap_opacity_spinbox.set_hovered(was_ggo_hovered);
            state.graph_gap_width_spinbox.set_hovered(was_ggw_hovered);
            state.color_selector_height_spinbox.set_hovered(was_csh_hovered);
            state.color_selector_preview_corner_radius_spinbox.set_hovered(was_cspcr_hovered);
            state.color_selector_preview_margin_spinbox.set_hovered(was_cspm_hovered);
            state.color_selector_corner_radius_spinbox.set_hovered(was_cscr_hovered);
            state.textbox_height_spinbox.set_hovered(was_tbh_hovered);
            state.textbox_corner_radius_spinbox.set_hovered(was_tcr_hovered);
            state.font_selector_height_spinbox.set_hovered(was_fsh_hovered);
            state.font_selector_corner_radius_spinbox.set_hovered(was_fscr_hovered);
            state.breadcrumb_font_selector.set_hovered(was_bfs_hovered);
            state.font_selector_font_selector.set_hovered(was_fsfs_hovered);
            state.button_strip_font_selector.set_hovered(was_bsfs_hovered);
            state.button_font_selector.set_hovered(was_bfsn_hovered);
            state.label_font_selector.set_hovered(was_lfs_hovered);
            state.dropdown_font_selector.set_hovered(was_dfs_hovered);
            state.textbox_font_selector.set_hovered(was_tbfs_hovered);
            state.spinbox_font_selector.set_hovered(was_sfs_hovered);
            state.slider_font_selector.set_hovered(was_slfs_hovered);

            state.button_strip_font = button_strip_font;
            state.button_font = button_font;
            state.label_font = label_font;
            state.dropdown_font = dropdown_font;
            state.textbox_font = textbox_font;
            state.spinbox_font = spinbox_font;
            state.slider_font = slider_font;

            state.label_margin_spinbox.set_hovered(was_lm_hovered);
            state.menubar_opacity_spinbox.set_hovered(was_mo_hovered);
            state.notification_opacity_spinbox.set_hovered(was_no_hovered);
            state.backplate_corner_radius_spinbox.set_hovered(was_wcr_hovered);
            state.status_padding_spinbox.set_hovered(was_sp_pad_hovered);
            state.status_module_spacing_spinbox.set_hovered(was_sp_spc_hovered);

            if typeface_loaded {
                state.typeface_loaded = typeface_loaded;
                state.sans_serif = sans_serif;
                state.serif = serif;
                state.monospace = monospace;
                state.window_borders = window_borders;
                state.status_interface = status_interface;
                state.fuzzel = fuzzel;
                state.terminal = terminal;
                state.all_fonts = all_fonts;
                state.mono_fonts = mono_fonts;
                state.sans_box = sans_box;
                state.serif_box = serif_box;
                state.mono_box = mono_box;
                state.borders_box = borders_box;
                state.status_box = status_box;
                state.fuzzel_box = fuzzel_box;
                state.terminal_box = terminal_box;
                state.borders_menu = borders_menu;
                state.status_menu = status_menu;
                state.fuzzel_menu = fuzzel_menu;
                state.terminal_menu = terminal_menu;
                state.borders_size_box = borders_size_box;
                state.status_size_box = status_size_box;
                state.fuzzel_size_box = fuzzel_size_box;
                state.terminal_size_box = terminal_size_box;
            }
        }
        InterfaceMessage::TypefaceRefreshed(new) => {
            state.typeface_loaded = new.typeface_loaded;
            state.all_fonts = new.all_fonts;
            state.mono_fonts = new.mono_fonts;
            if !state.sans_box.editing {
                state.sans_serif = new.sans_serif.clone();
                state.sans_box = new.sans_box;
            }
            if !state.serif_box.editing {
                state.serif = new.serif.clone();
                state.serif_box = new.serif_box;
            }
            if !state.mono_box.editing {
                state.monospace = new.monospace.clone();
                state.mono_box = new.mono_box;
            }
            if !state.borders_box.editing {
                state.window_borders = new.window_borders.clone();
                state.borders_box = new.borders_box;
                state.borders_menu = new.borders_menu;
            }
            if !state.status_box.editing {
                state.status_interface = new.status_interface.clone();
                state.status_box = new.status_box;
                state.status_menu = new.status_menu;
            }
            if !state.fuzzel_box.editing {
                state.fuzzel = new.fuzzel.clone();
                state.fuzzel_box = new.fuzzel_box;
                state.fuzzel_menu = new.fuzzel_menu;
            }
            if !state.terminal_box.editing {
                state.terminal = new.terminal.clone();
                state.terminal_box = new.terminal_box;
                state.terminal_menu = new.terminal_menu;
            }
            state.borders_size_box = new.borders_size_box;
            state.status_size_box = new.status_size_box;
            state.fuzzel_size_box = new.fuzzel_size_box;
            state.terminal_size_box = new.terminal_size_box;
        }

        InterfaceMessage::SetSans(sans) => {
            state.sans_serif = sans.clone();
            state.sans_box.text = sans;
            if state.borders_menu.selected == 0 {
                state.window_borders = state.sans_serif.clone();
                state.borders_box.text = state.sans_serif.clone();
            }
            if state.status_menu.selected == 0 {
                state.status_interface = state.sans_serif.clone();
                state.status_box.text = state.sans_serif.clone();
            }
            if state.fuzzel_menu.selected == 0 {
                state.fuzzel = state.sans_serif.clone();
                state.fuzzel_box.text = state.sans_serif.clone();
            }
            if state.terminal_menu.selected == 0 {
                state.terminal = state.sans_serif.clone();
                state.terminal_box.text = state.sans_serif.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetSerif(serif) => {
            state.serif = serif.clone();
            state.serif_box.text = serif;
            if state.borders_menu.selected == 1 {
                state.window_borders = state.serif.clone();
                state.borders_box.text = state.serif.clone();
            }
            if state.status_menu.selected == 1 {
                state.status_interface = state.serif.clone();
                state.status_box.text = state.serif.clone();
            }
            if state.fuzzel_menu.selected == 1 {
                state.fuzzel = state.serif.clone();
                state.fuzzel_box.text = state.serif.clone();
            }
            if state.terminal_menu.selected == 1 {
                state.terminal = state.serif.clone();
                state.terminal_box.text = state.serif.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetMono(mono) => {
            state.monospace = mono.clone();
            state.mono_box.text = mono;
            if state.borders_menu.selected == 2 {
                state.window_borders = state.monospace.clone();
                state.borders_box.text = state.monospace.clone();
            }
            if state.status_menu.selected == 2 {
                state.status_interface = state.monospace.clone();
                state.status_box.text = state.monospace.clone();
            }
            if state.fuzzel_menu.selected == 2 {
                state.fuzzel = state.monospace.clone();
                state.fuzzel_box.text = state.monospace.clone();
            }
            if state.terminal_menu.selected == 2 {
                state.terminal = state.monospace.clone();
                state.terminal_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetBorders(borders) => {
            state.window_borders = borders.clone();
            state.borders_box.text = borders;
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetStatus(status) => {
            state.status_interface = status.clone();
            state.status_box.text = status;
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetFuzzel(fuzzel) => {
            state.fuzzel = fuzzel.clone();
            state.fuzzel_box.text = fuzzel;
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetTerminal(term) => {
            state.terminal = term.clone();
            state.terminal_box.text = term;
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }

        InterfaceMessage::SetBordersMenu(idx) => {
            state.borders_menu.selected = idx;
            state.borders_box.disabled = idx != 3;
            if idx == 0 {
                state.window_borders = state.sans_serif.clone();
                state.borders_box.text = state.sans_serif.clone();
            } else if idx == 1 {
                state.window_borders = state.serif.clone();
                state.borders_box.text = state.serif.clone();
            } else if idx == 2 {
                state.window_borders = state.monospace.clone();
                state.borders_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetStatusMenu(idx) => {
            state.status_menu.selected = idx;
            state.status_box.disabled = idx != 3;
            if idx == 0 {
                state.status_interface = state.sans_serif.clone();
                state.status_box.text = state.sans_serif.clone();
            } else if idx == 1 {
                state.status_interface = state.serif.clone();
                state.status_box.text = state.serif.clone();
            } else if idx == 2 {
                state.status_interface = state.monospace.clone();
                state.status_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetFuzzelMenu(idx) => {
            state.fuzzel_menu.selected = idx;
            state.fuzzel_box.disabled = idx != 3;
            if idx == 0 {
                state.fuzzel = state.sans_serif.clone();
                state.fuzzel_box.text = state.sans_serif.clone();
            } else if idx == 1 {
                state.fuzzel = state.serif.clone();
                state.fuzzel_box.text = state.serif.clone();
            } else if idx == 2 {
                state.fuzzel = state.monospace.clone();
                state.fuzzel_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetTerminalMenu(idx) => {
            state.terminal_menu.selected = idx;
            state.terminal_box.disabled = idx != 3;
            if idx == 0 {
                state.terminal = state.sans_serif.clone();
                state.terminal_box.text = state.sans_serif.clone();
            } else if idx == 1 {
                state.terminal = state.serif.clone();
                state.terminal_box.text = state.serif.clone();
            } else if idx == 2 {
                state.terminal = state.monospace.clone();
                state.terminal_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
            );
        }
        InterfaceMessage::SetBordersSize(val) => {
            state.borders_size_box.value = val;
            write_config_value("border_font_size", &val.to_string());
            send_ipc_command(&format!("layout border_font_size {}", val));
        }
        InterfaceMessage::SetStatusSize(val) => {
            state.status_size_box.value = val;
            write_status_size(val as u16);
        }
        InterfaceMessage::SetFuzzelSize(val) => {
            state.fuzzel_size_box.value = val;
            write_fuzzel_size(val as u16);
        }
        InterfaceMessage::SetTerminalSize(val) => {
            state.terminal_size_box.value = val;
            write_terminal_size(val as u16);
        }
        InterfaceMessage::Windows(sub_msg) => {
            update_windows(&mut state.windows, sub_msg);
        }
    }
}

pub fn update_windows(state: &mut WindowsState, msg: WindowsMessage) {
    match msg {
        WindowsMessage::SetCascadeOffset(v) => {
            let val = v.min(200);
            state.cascade_offset = val;
            state.cascade_offset_spinbox.value = val as i32;
            apply_single_layout_param("cascade_offset", val);
        }
        WindowsMessage::SetEdgeGap(v) => {
            let val = v.min(200);
            state.edge_gap = val;
            state.edge_gap_spinbox.value = val as i32;
            apply_edge_gap(val);
        }
        WindowsMessage::SetTopGap(v) => {
            let val = v.min(200);
            state.top_gap = val;
            state.top_gap_spinbox.value = val as i32;
            apply_single_layout_param("gap_top", val);
        }
        WindowsMessage::SetTransitionDuration(v) => {
            let val = v.min(2000);
            state.transition_duration = val;
            state.transition_duration_spinbox.value = val as i32;
            apply_single_layout_param("transition_duration", val);
        }
        WindowsMessage::SetStatusHeight(v) => {
            let val = v.min(100);
            state.status_height = val;
            state.status_height_spinbox.value = val as i32;
            apply_single_layout_param("bar_height", val);
        }
        WindowsMessage::SetSidePanelBehavior(idx) => {
            if idx < 2 {
                state.side_panel_behavior_menu.selected = idx;
                let val = if idx == 0 { "above" } else { "inline" };
                write_config_value("pinned_behavior", &format!("\"{}\"", val));
                send_ipc_command(&format!("layout pinned_behavior {}", val));
            }
        }
        WindowsMessage::SetSidePanelPosition(idx) => {
            if idx < 2 {
                state.side_panel_position_menu.selected = idx;
                let val = if idx == 1 { "right" } else { "left" };
                write_config_value("pinned_position", &format!("\"{}\"", val));
                send_ipc_command(&format!("layout pinned_position {}", val));
            }
        }
        WindowsMessage::SetSidePanelWidth(v) => {
            let val = v.min(2000);
            state.side_panel_width = val;
            state.side_panel_width_spinbox.value = val as i32;
            write_config_value("pinned_width", &val.to_string());
            send_ipc_command(&format!("layout pinned_width {}", val));
        }
        WindowsMessage::SetSidePanelBorderGap(v) => {
            let val = v.min(500);
            state.side_panel_border_gap = val;
            state.side_panel_border_gap_spinbox.value = val as i32;
            write_config_value("pinned_border_gap", &val.to_string());
            send_ipc_command(&format!("layout pinned_border_gap {}", val));
        }

        WindowsMessage::SetFullscreenOpacity(v) => {
            let val = v.min(100);
            state.fullscreen_opacity = val as f32 / 100.0;
            state.fullscreen_opacity_spinbox.value = val as i32;
            let val_str = format!("{:.2}", val as f32 / 100.0);
            write_config_value("fullscreen_backplate_opacity", &val_str);
            send_ipc_command("reload");
        }
        WindowsMessage::SetCascadeOpacity(v) => {
            let val = v.min(100);
            state.cascade_opacity = val as f32 / 100.0;
            state.cascade_opacity_spinbox.value = val as i32;
            let val_str = format!("{:.2}", val as f32 / 100.0);
            write_config_value("cascade_backplate_opacity", &val_str);
            send_ipc_command("reload");
        }
        WindowsMessage::SetGridOpacity(v) => {
            let val = v.min(100);
            state.grid_opacity = val as f32 / 100.0;
            state.grid_opacity_spinbox.value = val as i32;
            let val_str = format!("{:.2}", val as f32 / 100.0);
            write_config_value("grid_backplate_opacity", &val_str);
            send_ipc_command("reload");
        }
        WindowsMessage::SetFloatingOpacity(v) => {
            let val = v.min(100);
            state.floating_opacity = val as f32 / 100.0;
            state.floating_opacity_spinbox.value = val as i32;
            let val_str = format!("{:.2}", val as f32 / 100.0);
            write_config_value("floating_backplate_opacity", &val_str);
            send_ipc_command("reload");
        }
        WindowsMessage::SetPinnedOpacity(v) => {
            let val = v.min(100);
            state.pinned_opacity = val as f32 / 100.0;
            state.pinned_opacity_spinbox.value = val as i32;
            let val_str = format!("{:.2}", val as f32 / 100.0);
            write_config_value("pinned_backplate_opacity", &val_str);
            send_ipc_command("reload");
        }
        WindowsMessage::SetPopupOpacity(v) => {
            let val = v.min(100);
            state.popup_opacity = val as f32 / 100.0;
            state.popup_opacity_spinbox.value = val as i32;
            let val_str = format!("{:.2}", val as f32 / 100.0);
            write_config_value("popup_backplate_opacity", &val_str);
            send_ipc_command("reload");
        }

        WindowsMessage::ToggleBlur => {
            state.blur_enabled = !state.blur_enabled;
            let val = state.blur_enabled.to_string();
            write_config_value("window_blur", &val);
            write_config_value("border_blur", &val);
            send_ipc_command("reload");
        }
        WindowsMessage::Refreshed(new) => {
            // Restore hover states
            let blur_hover = state.blur_toggle.hovered();
            let side_panel_behavior_hover = state.side_panel_behavior_menu.hovered();
            let side_panel_position_hover = state.side_panel_position_menu.hovered();
            let cascade_offset_hover = state.cascade_offset_spinbox.hovered();
            let edge_gap_hover = state.edge_gap_spinbox.hovered();
            let top_gap_hover = state.top_gap_spinbox.hovered();
            let status_height_hover = state.status_height_spinbox.hovered();
            let transition_duration_hover = state.transition_duration_spinbox.hovered();
            let side_panel_width_hover = state.side_panel_width_spinbox.hovered();
            let side_panel_border_gap_hover = state.side_panel_border_gap_spinbox.hovered();
            let fullscreen_opacity_hover = state.fullscreen_opacity_spinbox.hovered();
            let cascade_opacity_hover = state.cascade_opacity_spinbox.hovered();
            let grid_opacity_hover = state.grid_opacity_spinbox.hovered();
            let floating_opacity_hover = state.floating_opacity_spinbox.hovered();
            let pinned_opacity_hover = state.pinned_opacity_spinbox.hovered();
            let popup_opacity_hover = state.popup_opacity_spinbox.hovered();

            *state = new;

            state.blur_toggle.set_hovered(blur_hover);
            state.side_panel_behavior_menu.set_hovered(side_panel_behavior_hover);
            state.side_panel_position_menu.set_hovered(side_panel_position_hover);
            state.cascade_offset_spinbox.set_hovered(cascade_offset_hover);
            state.edge_gap_spinbox.set_hovered(edge_gap_hover);
            state.top_gap_spinbox.set_hovered(top_gap_hover);
            state.status_height_spinbox.set_hovered(status_height_hover);
            state.transition_duration_spinbox.set_hovered(transition_duration_hover);
            state.side_panel_width_spinbox.set_hovered(side_panel_width_hover);
            state.side_panel_border_gap_spinbox.set_hovered(side_panel_border_gap_hover);
            state.fullscreen_opacity_spinbox.set_hovered(fullscreen_opacity_hover);
            state.cascade_opacity_spinbox.set_hovered(cascade_opacity_hover);
            state.grid_opacity_spinbox.set_hovered(grid_opacity_hover);
            state.floating_opacity_spinbox.set_hovered(floating_opacity_hover);
            state.pinned_opacity_spinbox.set_hovered(pinned_opacity_hover);
            state.popup_opacity_spinbox.set_hovered(popup_opacity_hover);
        }
    }
}

pub fn parse_transparency_opacity(content: &str) -> f32 {
    let val = parse_json(content);
    val["transparency"]["opacity"].as_f64().map(|v| v as f32).unwrap_or(0.9)
}

pub fn write_transparency_config_value(key: &str, value: &str) {
    let path = get_config_path();
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut val = parse_json(&content);
    let j_val = if let Ok(parsed_val) = serde_json::from_str::<serde_json::Value>(value) {
        parsed_val
    } else {
        serde_json::json!(value)
    };
    if val.get("transparency").is_none() {
        if let Some(obj) = val.as_object_mut() {
            obj.insert("transparency".to_string(), serde_json::Value::Object(serde_json::Map::new()));
        }
    }
    if let Some(transparency) = val.get_mut("transparency").and_then(|t| t.as_object_mut()) {
        transparency.insert(key.to_string(), j_val);
    }
    if let Ok(updated_str) = serde_json::to_string_pretty(&val) {
        let _ = safe_write(&path, &updated_str);
    }
}

fn write_surfaces_config_value(key: &str, value: &str) {
    write_surfaces_config_value_path(&get_config_path(), key, value);
}

fn write_surfaces_config_value_path(path: &str, key: &str, value: &str) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut val = parse_json(&content);
    let j_val = if let Ok(parsed_val) = serde_json::from_str::<serde_json::Value>(value) {
        parsed_val
    } else {
        serde_json::json!(value)
    };
    if val.get("surfaces").is_none() {
        if let Some(obj) = val.as_object_mut() {
            obj.insert("surfaces".to_string(), serde_json::Value::Object(serde_json::Map::new()));
        }
    }
    if let Some(surfaces) = val.get_mut("surfaces").and_then(|s| s.as_object_mut()) {
        surfaces.insert(key.to_string(), j_val);
    }
    if let Ok(updated_str) = serde_json::to_string_pretty(&val) {
        let _ = safe_write(path, &updated_str);
    }
}

fn parse_surfaces_color(content: &str, key: &str, default: [u8; 3]) -> [u8; 3] {
    let val = parse_json(content);
    if let Some(s) = val["surfaces"].get(key).and_then(|v| v.as_str()) {
        return parse_hex(s);
    }
    default
}

fn parse_surfaces_rgba_color(content: &str, key: &str, default: [u8; 4]) -> [u8; 4] {
    let val = parse_json(content);
    if let Some(s) = val["surfaces"].get(key).and_then(|v| v.as_str()) {
        return parse_hex_rgba(s);
    }
    default
}



fn parse_surfaces_u16(content: &str, key: &str, default: u16) -> u16 {
    let val = parse_json(content);
    if let Some(n) = val["surfaces"].get(key).and_then(|v| v.as_u64()) {
        return n as u16;
    }
    default
}

fn parse_notifications_color(content: &str, key: &str, default: [u8; 3]) -> [u8; 3] {
    let val = parse_json(content);
    if let Some(s) = val["notifications"].get(key).and_then(|v| v.as_str()) {
        return parse_hex(s);
    }
    default
}

fn parse_notifications_opacity(content: &str) -> f32 {
    let val = parse_json(content);
    val["notifications"]["opacity"].as_f64().map(|v| v as f32).unwrap_or(0.9)
}

fn write_notifications_config_value(key: &str, value: &str) {
    write_notifications_config_value_path(&get_config_path(), key, value);
}

fn write_notifications_config_value_path(path: &str, key: &str, value: &str) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut val = parse_json(&content);
    let j_val = if let Ok(parsed_val) = serde_json::from_str::<serde_json::Value>(value) {
        parsed_val
    } else {
        serde_json::json!(value)
    };
    if val.get("notifications").is_none() {
        if let Some(obj) = val.as_object_mut() {
            obj.insert("notifications".to_string(), serde_json::Value::Object(serde_json::Map::new()));
        }
    }
    if let Some(notifications) = val.get_mut("notifications").and_then(|n| n.as_object_mut()) {
        notifications.insert(key.to_string(), j_val);
    }
    if let Ok(updated_str) = serde_json::to_string_pretty(&val) {
        let _ = safe_write(path, &updated_str);
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex("#ffffff"), [255, 255, 255]);
        assert_eq!(parse_hex("000000"), [0, 0, 0]);
        assert_eq!(parse_hex("#123456"), [18, 52, 86]);
        assert_eq!(parse_hex("invalid"), [0x0a, 0x1a, 0x0e]);
    }

    #[test]
    fn test_interface_spinbox_right_click_crash() {
        use cce_ui::widget::Element;
        let mut state = InterfaceState::default();
        let mut ctx = cce_ui::context::UiContext::new();
        let sb = &mut state.backplate_corner_radius_spinbox;
        sb.set_rect(0.0, 0.0, 100.0, 44.0);
        let res = sb.mouse_input(
            cce_ui::widget::MouseButton::Right,
            cce_ui::widget::ElementState::Pressed,
            50.0,
            20.0,
            &mut ctx,
        );
        assert!(res);
        assert!(cce_ui::widget::context_menu::is_visible());

        // Now simulate update(InterfaceMessage::Refreshed)
        let new_state = InterfaceState::default();
        update(&mut state, InterfaceMessage::Refreshed(new_state));

        // Assert that the context menu is hidden (cleared)
        assert!(!cce_ui::widget::context_menu::is_visible());
    }

    #[test]
    fn test_context_menu_null_target_safety() {
        let mut ctx = cce_ui::context::UiContext::new();
        // Hide context menu initially
        ctx.hide_context_menu();
        assert!(!ctx.is_context_menu_visible());

        // This should return early and not panic/crash on a null target
        ctx.handle_right_click(std::ptr::null_mut::<cce_ui::widget::Spinbox>() as *mut (dyn Element + 'static), 10.0, 10.0);
        assert!(!ctx.is_context_menu_visible());
    }


    #[test]
    fn test_parse_color_from_key() {
        let content = r##"{
            "layout": {
                "low_color": "#112233",
                "high_color": "#445566",
                "disabled_color": "#778899",
                "status_separator_color": "#aabbcc",
                "visual_guides_color": "#ddeeff",
                "slider_track_color": "#123456",
                "page_low_color": "#474751",
                "color_borders_color": "#abcdef",
                "status_normal_color": "#ccccd8",
                "paginator_sidebar_color": "#5a5a65",
                "primary_highlight_color": "#ffffff",
                "menubar_tab_label_color": "#e6e6f2",
                "toggle_enabled_color": "#68d8a5",
                "toggle_disabled_color": "#878794",
                "scrollinglist_bg_color": "#515161",
                "breadcrumb_bg_color": "#515161",
                "page_color": "#0a1a0e",
                "layer_color": "#123456"
            }
        }"##;
        assert_eq!(parse_color_from_key(content, "low_color", [0, 0, 0]), [17, 34, 51]);
        assert_eq!(parse_color_from_key(content, "high_color", [0, 0, 0]), [68, 85, 102]);
        assert_eq!(parse_color_from_key(content, "disabled_color", [0, 0, 0]), [119, 136, 153]);
        assert_eq!(parse_color_from_key(content, "status_separator_color", [0, 0, 0]), [170, 187, 204]);
        assert_eq!(parse_color_from_key(content, "visual_guides_color", [0, 0, 0]), [221, 238, 255]);
        assert_eq!(parse_color_from_key(content, "slider_track_color", [0, 0, 0]), [18, 52, 86]);
        assert_eq!(parse_color_from_key(content, "page_low_color", [0, 0, 0]), [71, 71, 81]);
        assert_eq!(parse_color_from_key(content, "color_borders_color", [0, 0, 0]), [171, 205, 239]);
        assert_eq!(parse_color_from_key(content, "status_normal_color", [0, 0, 0]), [204, 204, 216]);
        assert_eq!(parse_color_from_key(content, "paginator_sidebar_color", [0, 0, 0]), [90, 90, 101]);
        assert_eq!(parse_color_from_key(content, "primary_highlight_color", [0, 0, 0]), [255, 255, 255]);
        assert_eq!(parse_color_from_key(content, "menubar_tab_label_color", [0, 0, 0]), [230, 230, 242]);
        assert_eq!(parse_color_from_key(content, "toggle_enabled_color", [0, 0, 0]), [104, 216, 165]);
        assert_eq!(parse_color_from_key(content, "toggle_disabled_color", [0, 0, 0]), [135, 135, 148]);
        assert_eq!(parse_color_from_key(content, "scrollinglist_bg_color", [0, 0, 0]), [81, 81, 97]);
        assert_eq!(parse_color_from_key(content, "breadcrumb_bg_color", [0, 0, 0]), [81, 81, 97]);
        assert_eq!(parse_color_from_key(content, "page_color", [0, 0, 0]), [10, 26, 14]);
        assert_eq!(parse_color_from_key(content, "layer_color", [0, 0, 0]), [18, 52, 86]);
        assert_eq!(parse_color_from_key(content, "non_existent", [1, 2, 3]), [1, 2, 3]);
    }

    #[test]
    fn test_parse_rgba_color_from_key() {
        let content = r##"{
            "layout": {
                "scrollinglist_entry_bg_color": "#ffffff0a",
                "scrollinglist_entry_highlight_color": "#ffffffcc"
            }
        }"##;
        assert_eq!(parse_rgba_color_from_key(content, "scrollinglist_entry_bg_color", [0, 0, 0, 0]), [255, 255, 255, 10]);
        assert_eq!(parse_rgba_color_from_key(content, "scrollinglist_entry_highlight_color", [0, 0, 0, 0]), [255, 255, 255, 204]);
    }

    #[test]
    fn test_write_config_value_path() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Write initial file content with [layout] and other keys
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}, \"output\": {\"scale\": 2}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Write disabled_color which does not exist yet (key not found case)
        assert!(write_config_value_path(path_str, "disabled_color", "\"#555555\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"disabled_color\": \"#555555\""));
        // Check it was inserted before [output]
        assert!(updated.find("\"disabled_color\": \"#555555\"").unwrap() < updated.find("\"output\"").unwrap());

        // 3. Update disabled_color (key found case)
        assert!(write_config_value_path(path_str, "disabled_color", "\"#666666\""));
        let updated2 = fs::read_to_string(path_str).unwrap();
        assert!(updated2.contains("\"disabled_color\": \"#666666\""));
        assert!(!updated2.contains("\"disabled_color\": \"#555555\""));

        // 4. Write visual_guides_color which does not exist yet
        assert!(write_config_value_path(path_str, "visual_guides_color", "\"#ff8c00\""));
        let updated3 = fs::read_to_string(path_str).unwrap();
        assert!(updated3.contains("\"visual_guides_color\": \"#ff8c00\""));

        // 5. Write slider_track_color which does not exist yet
        assert!(write_config_value_path(path_str, "slider_track_color", "\"#123456\""));
        let updated4 = fs::read_to_string(path_str).unwrap();
        assert!(updated4.contains("\"slider_track_color\": \"#123456\""));

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_parse_font_for_alias() {
        let content = r#"<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
    <dir>~/Dropbox/Fonts</dir>
    <match target="pattern">
        <test qual="any" name="family"><string>sans-serif</string></test>
        <edit name="family" mode="assign" binding="same">
            <string>Adwaita Sans</string>
        </edit>
    </match>
    <match target="pattern">
        <test qual="any" name="family"><string>monospace</string></test>
        <edit name="family" mode="assign" binding="same">
            <string>Berkeley Mono</string>
        </edit>
    </match>
</fontconfig>
"#;

        assert_eq!(parse_font_for_alias(content, "sans-serif"), Some("Adwaita Sans".to_string()));
        assert_eq!(parse_font_for_alias(content, "monospace"), Some("Berkeley Mono".to_string()));
        assert_eq!(parse_font_for_alias(content, "serif"), None);
    }

    #[test]
    fn test_read_write_section_padding() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_section_padding_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse section_padding when missing (should return default 8)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "section_padding", 8);
        assert_eq!(val, 8);

        // 3. Write section_padding config
        assert!(write_config_value_path(path_str, "section_padding", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"section_padding\": 12"));

        // 4. Parse section_padding when present (should return written value 12)
        let val2 = parse_u16_from(&updated, "section_padding", 8);
        assert_eq!(val2, 12);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_plate_padding() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_plate_padding_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse plate_padding when missing (should return default 20)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "plate_padding", 20);
        assert_eq!(val, 20);

        // 3. Write plate_padding config
        assert!(write_config_value_path(path_str, "plate_padding", "15"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"plate_padding\": 15"));

        // 4. Parse plate_padding when present (should return written value 15)
        let val2 = parse_u16_from(&updated, "plate_padding", 20);
        assert_eq!(val2, 15);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_page_margin() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_page_margin_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse page_margin when missing (should return default 20)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "page_margin", 20);
        assert_eq!(val, 20);

        // 3. Write page_margin config
        assert!(write_config_value_path(path_str, "page_margin", "15"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"page_margin\": 15"));

        // 4. Parse page_margin when present (should return written value 15)
        let val2 = parse_u16_from(&updated, "page_margin", 20);
        assert_eq!(val2, 15);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_spinbox_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_spinbox_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse spinbox_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "spinbox_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write spinbox_corner_radius config
        assert!(write_config_value_path(path_str, "spinbox_corner_radius", "8"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"spinbox_corner_radius\": 8"));

        // 4. Parse spinbox_corner_radius when present (should return written value 8)
        let val2 = parse_u16_from(&updated, "spinbox_corner_radius", 4);
        assert_eq!(val2, 8);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_spinbox_height() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_spinbox_height_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse spinbox_height when missing (should return default 26)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "spinbox_height", 26);
        assert_eq!(val, 26);

        // 3. Write spinbox_height config
        assert!(write_config_value_path(path_str, "spinbox_height", "30"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"spinbox_height\": 30"));

        // 4. Parse spinbox_height when present (should return written value 30)
        let val2 = parse_u16_from(&updated, "spinbox_height", 26);
        assert_eq!(val2, 30);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_toggle_height() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_toggle_height_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse toggle_height when missing (should return default 44)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "toggle_height", 44);
        assert_eq!(val, 44);

        // 3. Write toggle_height config
        assert!(write_config_value_path(path_str, "toggle_height", "52"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"toggle_height\": 52"));

        // 4. Parse toggle_height when present (should return written value 52)
        let val2 = parse_u16_from(&updated, "toggle_height", 44);
        assert_eq!(val2, 52);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_toggle_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_toggle_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse toggle_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "toggle_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write toggle_corner_radius config
        assert!(write_config_value_path(path_str, "toggle_corner_radius", "8"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"toggle_corner_radius\": 8"));

        // 4. Parse toggle_corner_radius when present (should return written value 8)
        let val2 = parse_u16_from(&updated, "toggle_corner_radius", 4);
        assert_eq!(val2, 8);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_toggle_bg_color() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_toggle_bg_color_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_color_from_key(&content, "toggle_bg_color", [116, 116, 128]);
        assert_eq!(val, [116, 116, 128]);

        assert!(write_config_value_path(path_str, "toggle_bg_color", "\"#123456\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"toggle_bg_color\": \"#123456\""));

        let val2 = parse_color_from_key(&updated, "toggle_bg_color", [116, 116, 128]);
        assert_eq!(val2, [18, 52, 86]);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_toggle_border_width() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_toggle_border_width_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "toggle_border_width", 1);
        assert_eq!(val, 1);

        assert!(write_config_value_path(path_str, "toggle_border_width", "3"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"toggle_border_width\": 3"));

        let val2 = parse_u16_from(&updated, "toggle_border_width", 1);
        assert_eq!(val2, 3);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_toggle_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_toggle_font_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "toggle_font", "Outfit");
        assert_eq!(val, "Outfit");

        assert!(write_config_value_path(path_str, "toggle_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"toggle_font\": \"Inter\""));

        let val2 = parse_string_from(&updated, "toggle_font", "Outfit");
        assert_eq!(val2, "Inter");

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_font_selector_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_font_selector_font_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "font_selector_font", "Outfit");
        assert_eq!(val, "Outfit");

        assert!(write_config_value_path(path_str, "font_selector_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"font_selector_font\": \"Inter\""));

        let val2 = parse_string_from(&updated, "font_selector_font", "Outfit");
        assert_eq!(val2, "Inter");

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_button_strip_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_button_strip_font_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "button_strip_font", "Outfit");
        assert_eq!(val, "Outfit");

        assert!(write_config_value_path(path_str, "button_strip_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"button_strip_font\": \"Inter\""));

        let val2 = parse_string_from(&updated, "button_strip_font", "Outfit");
        assert_eq!(val2, "Inter");

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_button_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_button_font_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "button_font", "Outfit");
        assert_eq!(val, "Outfit");

        assert!(write_config_value_path(path_str, "button_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"button_font\": \"Inter\""));

        let val2 = parse_string_from(&updated, "button_font", "Outfit");
        assert_eq!(val2, "Inter");

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_label_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_label_font_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "label_font", "Outfit");
        assert_eq!(val, "Outfit");

        assert!(write_config_value_path(path_str, "label_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"label_font\": \"Inter\""));

        let val2 = parse_string_from(&updated, "label_font", "Outfit");
        assert_eq!(val2, "Inter");

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_dropdown_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_dropdown_font_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "dropdown_font", "Outfit");
        assert_eq!(val, "Outfit");

        assert!(write_config_value_path(path_str, "dropdown_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"dropdown_font\": \"Inter\""));

        let val2 = parse_string_from(&updated, "dropdown_font", "Outfit");
        assert_eq!(val2, "Inter");

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_textbox_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_textbox_font_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "textbox_font", "Outfit");
        assert_eq!(val, "Outfit");

        assert!(write_config_value_path(path_str, "textbox_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"textbox_font\": \"Inter\""));

        let val2 = parse_string_from(&updated, "textbox_font", "Outfit");
        assert_eq!(val2, "Inter");

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_spinbox_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_spinbox_font_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "spinbox_font", "monospace");
        assert_eq!(val, "monospace");

        assert!(write_config_value_path(path_str, "spinbox_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"spinbox_font\": \"Inter\""));

        let val2 = parse_string_from(&updated, "spinbox_font", "monospace");
        assert_eq!(val2, "Inter");

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_slider_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_slider_font_config.json");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "slider_font", "Outfit");
        assert_eq!(val, "Outfit");

        assert!(write_config_value_path(path_str, "slider_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"slider_font\": \"Inter\""));

        let val2 = parse_string_from(&updated, "slider_font", "Outfit");
        assert_eq!(val2, "Inter");

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_slider_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_slider_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse slider_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "slider_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write slider_corner_radius config
        assert!(write_config_value_path(path_str, "slider_corner_radius", "6"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"slider_corner_radius\": 6"));

        // 4. Parse slider_corner_radius when present (should return written value 6)
        let val2 = parse_u16_from(&updated, "slider_corner_radius", 4);
        assert_eq!(val2, 6);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_plate_opacity() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_plate_opacity_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse plate_opacity when missing (should return default 1.0)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_f32_from(&content, "plate_opacity", 1.0);
        assert_eq!(val, 1.0);

        // 3. Write plate_opacity config
        assert!(write_config_value_path(path_str, "plate_opacity", "0.85"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"plate_opacity\": 0.85"));

        // 4. Parse plate_opacity when present (should return written value 0.85)
        let val2 = parse_f32_from(&updated, "plate_opacity", 1.0);
        assert_eq!(val2, 0.85);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_plate_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_plate_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse plate_corner_radius when missing (should return default 12)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "plate_corner_radius", 12);
        assert_eq!(val, 12);

        // 3. Write plate_corner_radius config
        assert!(write_config_value_path(path_str, "plate_corner_radius", "16"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"plate_corner_radius\": 16"));

        // 4. Parse plate_corner_radius when present (should return written value 16)
        let val2 = parse_u16_from(&updated, "plate_corner_radius", 12);
        assert_eq!(val2, 16);

        // Clean up
        let _ = fs::remove_file(path_str);
    }



    #[test]
    fn test_read_write_color_selector_height() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_color_selector_height_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_height when missing (should return default 22)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "color_selector_height", 22);
        assert_eq!(val, 22);

        // 3. Write color_selector_height config
        assert!(write_config_value_path(path_str, "color_selector_height", "28"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"color_selector_height\": 28"));

        // 4. Parse color_selector_height when present (should return written value 28)
        let val2 = parse_u16_from(&updated, "color_selector_height", 22);
        assert_eq!(val2, 28);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_font_selector_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_font_selector_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse font_selector_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "font_selector_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write font_selector_corner_radius config
        assert!(write_config_value_path(path_str, "font_selector_corner_radius", "8"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"font_selector_corner_radius\": 8"));

        // 4. Parse font_selector_corner_radius when present (should return written value 8)
        let val2 = parse_u16_from(&updated, "font_selector_corner_radius", 4);
        assert_eq!(val2, 8);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_textbox_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_textbox_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse textbox_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "textbox_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write textbox_corner_radius config
        assert!(write_config_value_path(path_str, "textbox_corner_radius", "8"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"textbox_corner_radius\": 8"));

        // 4. Parse textbox_corner_radius when present (should return written value 8)
        let val2 = parse_u16_from(&updated, "textbox_corner_radius", 4);
        assert_eq!(val2, 8);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_textbox_height() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_textbox_height_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse textbox_height when missing (should return default 44)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "textbox_height", 44);
        assert_eq!(val, 44);

        // 3. Write textbox_height config
        assert!(write_config_value_path(path_str, "textbox_height", "48"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"textbox_height\": 48"));

        // 4. Parse textbox_height when present (should return written value 48)
        let val2 = parse_u16_from(&updated, "textbox_height", 44);
        assert_eq!(val2, 48);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_color_selector_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_color_selector_font_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_font when missing (should return default "monospace")
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "color_selector_font", "monospace");
        assert_eq!(val, "monospace");

        // 3. Write color_selector_font config
        assert!(write_config_value_path(path_str, "color_selector_font", "\"Berkeley Mono\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains(r#""color_selector_font": "Berkeley Mono""#));

        // 4. Parse color_selector_font when present (should return written value)
        let val2 = parse_string_from(&updated, "color_selector_font", "monospace");
        assert_eq!(val2, "Berkeley Mono");

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_menubar_font() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_menubar_font_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse menubar_font when missing (should return default "Outfit")
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "menubar_font", "Outfit");
        assert_eq!(val, "Outfit");

        // 3. Write menubar_font config
        assert!(write_config_value_path(path_str, "menubar_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains(r#""menubar_font": "Inter""#));

        // 4. Parse menubar_font when present (should return written value)
        let val2 = parse_string_from(&updated, "menubar_font", "Outfit");
        assert_eq!(val2, "Inter");

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_font_selector_height() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_font_selector_height_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse font_selector_height when missing (should return default 44)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "font_selector_height", 44);
        assert_eq!(val, 44);

        // 3. Write font_selector_height config
        assert!(write_config_value_path(path_str, "font_selector_height", "48"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"font_selector_height\": 48"));

        // 4. Parse font_selector_height when present (should return written value 48)
        let val2 = parse_u16_from(&updated, "font_selector_height", 44);
        assert_eq!(val2, 48);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_grid_min_col_width() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_grid_min_col_width_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse grid_min_col_width when missing (should return default 260)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "grid_min_col_width", 260);
        assert_eq!(val, 260);

        // 3. Write grid_min_col_width config
        assert!(write_config_value_path(path_str, "grid_min_col_width", "280"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"grid_min_col_width\": 280"));

        // 4. Parse grid_min_col_width when present (should return written value 280)
        let val2 = parse_u16_from(&updated, "grid_min_col_width", 260);
        assert_eq!(val2, 280);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_layout_grid_gap() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_layout_grid_gap_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse grid_gap when missing (should return default 8)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "grid_gap", 8);
        assert_eq!(val, 8);

        // 3. Write grid_gap config
        assert!(write_config_value_path(path_str, "grid_gap", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"grid_gap\": 12"));

        // 4. Parse grid_gap when present (should return written value 12)
        let val2 = parse_u16_from(&updated, "grid_gap", 8);
        assert_eq!(val2, 12);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_color_selector_preview_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_color_selector_preview_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_preview_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "color_selector_preview_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write color_selector_preview_corner_radius config
        assert!(write_config_value_path(path_str, "color_selector_preview_corner_radius", "8"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"color_selector_preview_corner_radius\": 8"));

        // 4. Parse color_selector_preview_corner_radius when present (should return written value 8)
        let val2 = parse_u16_from(&updated, "color_selector_preview_corner_radius", 4);
        assert_eq!(val2, 8);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_color_selector_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_color_selector_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "color_selector_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write color_selector_corner_radius config
        assert!(write_config_value_path(path_str, "color_selector_corner_radius", "6"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"color_selector_corner_radius\": 6"));

        // 4. Parse color_selector_corner_radius when present (should return written value 6)
        let val2 = parse_u16_from(&updated, "color_selector_corner_radius", 4);
        assert_eq!(val2, 6);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_color_selector_preview_margin() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_color_selector_preview_margin_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_preview_margin when missing (should return default 0)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "color_selector_preview_margin", 0);
        assert_eq!(val, 0);

        // 3. Write color_selector_preview_margin config
        assert!(write_config_value_path(path_str, "color_selector_preview_margin", "3"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"color_selector_preview_margin\": 3"));

        // 4. Parse color_selector_preview_margin when present (should return written value 3)
        let val2 = parse_u16_from(&updated, "color_selector_preview_margin", 0);
        assert_eq!(val2, 3);

        // Clean up
        let _ = fs::remove_file(path_str);
    }



    #[test]
    fn test_read_write_button_padding() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_button_padding_config.toml");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "button_padding", 14);
        assert_eq!(val, 14);

        assert!(write_config_value_path(path_str, "button_padding", "20"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"button_padding\": 20"));

        let val2 = parse_u16_from(&updated, "button_padding", 14);
        assert_eq!(val2, 20);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_button_strip_spacing() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_button_strip_spacing_config.toml");
        let path_str = path.to_str().unwrap();
 
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();
 
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "button_strip_spacing", 8);
        assert_eq!(val, 8);
 
        assert!(write_config_value_path(path_str, "button_strip_spacing", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"button_strip_spacing\": 12"));
 
        let val2 = parse_u16_from(&updated, "button_strip_spacing", 8);
        assert_eq!(val2, 12);
 
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_slider_height() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_slider_height_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse slider_height when missing (should return default 28)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "slider_height", 28);
        assert_eq!(val, 28);

        // 3. Write slider_height config
        assert!(write_config_value_path(path_str, "slider_height", "32"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"slider_height\": 32"));

        // 4. Parse slider_height when present (should return written value 32)
        let val2 = parse_u16_from(&updated, "slider_height", 28);
        assert_eq!(val2, 32);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_nested_section_label_alignment() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_nested_section_label_alignment_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when missing (should return default 0)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "nested_section_label_alignment", 0);
        assert_eq!(val, 0);

        // 3. Write alignment config
        assert!(write_config_value_path(path_str, "nested_section_label_alignment", "2"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"nested_section_label_alignment\": 2"));

        // 4. Parse when present (should return written value 2)
        let val2 = parse_u16_from(&updated, "nested_section_label_alignment", 0);
        assert_eq!(val2, 2);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_nested_section_label_offset() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_nested_section_label_offset_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when missing (should return default 0)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_i16_from(&content, "nested_section_label_offset", 0);
        assert_eq!(val, 0);

        // 3. Write alignment config
        assert!(write_config_value_path(path_str, "nested_section_label_offset", "-15"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"nested_section_label_offset\": -15"));

        // 4. Parse when present (should return written value -15)
        let val2 = parse_i16_from(&updated, "nested_section_label_offset", 0);
        assert_eq!(val2, -15);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_dropdown_height() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_dropdown_height_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse dropdown_height when missing (should return default 44)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "dropdown_height", 44);
        assert_eq!(val, 44);

        // 3. Write dropdown_height config
        assert!(write_config_value_path(path_str, "dropdown_height", "48"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"dropdown_height\": 48"));

        // 4. Parse dropdown_height when present (should return written value 48)
        let val2 = parse_u16_from(&updated, "dropdown_height", 44);
        assert_eq!(val2, 48);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_dropdown_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_dropdown_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse dropdown_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "dropdown_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write dropdown_corner_radius config
        assert!(write_config_value_path(path_str, "dropdown_corner_radius", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"dropdown_corner_radius\": 12"));

        // 4. Parse dropdown_corner_radius when present (should return written value 12)
        let val2 = parse_u16_from(&updated, "dropdown_corner_radius", 4);
        assert_eq!(val2, 12);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_label_margin() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_label_margin_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18, \"border_color\": \"#374673\"}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when missing (should return default 6)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "label_margin", 6);
        assert_eq!(val, 6);

        // 3. Write label_margin config
        assert!(write_config_value_path(path_str, "label_margin", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"label_margin\": 12"));

        // 4. Parse when present (should return written value 12)
        let val2 = parse_u16_from(&updated, "label_margin", 6);
        assert_eq!(val2, 12);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_menubar_opacity() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_menubar_opacity_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"menubar_opacity\": 0.85}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when present (should return 0.85)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_f32_from(&content, "menubar_opacity", 0.9);
        assert_eq!(val, 0.85);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_button_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_button_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "button_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write button_corner_radius config
        assert!(write_config_value_path(path_str, "button_corner_radius", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"button_corner_radius\": 12"));

        // 4. Parse when present (should return written value 12)
        let val2 = parse_u16_from(&updated, "button_corner_radius", 4);
        assert_eq!(val2, 12);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_notifications_config() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_notifications_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"notifications\": {\"enable\": true}, \"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse opacity when missing (should return default 0.9)
        let content = fs::read_to_string(path_str).unwrap();
        let opacity = parse_notifications_opacity(&content);
        assert_eq!(opacity, 0.9);

        // 3. Write opacity config
        write_notifications_config_value_path(path_str, "opacity", "0.85");
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"opacity\": 0.85"));

        // 4. Parse opacity when present (should return 0.85)
        let opacity2 = parse_notifications_opacity(&updated);
        assert_eq!(opacity2, 0.85);

        // 5. Write bg_color config
        write_notifications_config_value_path(path_str, "bg_color", "\"#112233\"");
        let updated2 = fs::read_to_string(path_str).unwrap();
        assert!(updated2.contains("\"bg_color\": \"#112233\""));

        // 6. Parse bg_color when present
        let bg_color = parse_notifications_color(&updated2, "bg_color", [0, 0, 0]);
        assert_eq!(bg_color, [17, 34, 51]);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_custom_multicontrol_initialization() {
        let state = InterfaceState::default();
        assert_eq!(state.custom_multicontrol.name, "custom_parameters");
        assert_eq!(state.custom_multicontrol.base.label, Some("custom_parameters".to_string()));
    }

    #[test]
    fn test_widget_value_linking() {
        let dir = std::env::temp_dir();
        let config_path = dir.join("test_linking_config.json");
        let links_path = dir.join("test_linking_links.json");

        let config_path_str = config_path.to_str().unwrap().to_string();
        let links_path_str = links_path.to_str().unwrap().to_string();

        let test_links = r#"{
            "spinbox_height": "textbox_height",
            "textbox_height": "dropdown_height"
        }"#;
        fs::write(&links_path, test_links).unwrap();

        let initial_content = "{\"layout\": {\"spinbox_height\": 28, \"textbox_height\": 28, \"dropdown_height\": 28, \"button_corner_radius\": 4}}";
        fs::write(&config_path, initial_content).unwrap();

        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(config_path_str));
        TEST_LINKS_PATH.with(|p| *p.borrow_mut() = Some(links_path_str));

        assert!(write_config_value("spinbox_height", "32"));

        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = None);
        TEST_LINKS_PATH.with(|p| *p.borrow_mut() = None);

        let updated = fs::read_to_string(&config_path).unwrap();
        assert!(updated.contains("\"spinbox_height\": 32"));
        assert!(updated.contains("\"textbox_height\": 32"));
        assert!(updated.contains("\"dropdown_height\": 32"));
        assert!(updated.contains("\"button_corner_radius\": 4"));

        let _ = fs::remove_file(config_path);
        let _ = fs::remove_file(links_path);
    }

    #[test]
    fn test_propagate_links() {
        let dir = std::env::temp_dir();
        let config_path = dir.join("test_propagate_links_config.json");
        let links_path = dir.join("test_propagate_links_links.json");

        let config_path_str = config_path.to_str().unwrap().to_string();
        let links_path_str = links_path.to_str().unwrap().to_string();

        let test_config = "{\"layout\": {\"spinbox_height\": 28, \"textbox_height\": 28, \"dropdown_height\": 28}}";
        fs::write(&config_path, test_config).unwrap();

        let test_links = r#"{
            "spinbox_height": "textbox_height",
            "textbox_height": "dropdown_height"
        }"#;
        fs::write(&links_path, test_links).unwrap();

        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(config_path_str));
        TEST_LINKS_PATH.with(|p| *p.borrow_mut() = Some(links_path_str));

        let mut state = InterfaceState::default();
        state.spinbox_height = 36;
        state.textbox_height = 28;
        state.dropdown_height = 28;
        state.spinbox_height_spinbox.value = 36;
        state.textbox_height_spinbox.value = 28;
        state.dropdown_height_spinbox.value = 28;

        propagate_links(&mut state, "spinbox_height", "36");

        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = None);
        TEST_LINKS_PATH.with(|p| *p.borrow_mut() = None);

        let _ = fs::remove_file(config_path);
        let _ = fs::remove_file(links_path);

        assert_eq!(state.spinbox_height, 36);
        assert_eq!(state.textbox_height, 36);
        assert_eq!(state.dropdown_height, 36);
        assert_eq!(state.spinbox_height_spinbox.value, 36);
        assert_eq!(state.textbox_height_spinbox.value, 36);
        assert_eq!(state.dropdown_height_spinbox.value, 36);
    }

    #[test]
    fn test_read_write_surfaces_config() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_surfaces_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "{\"notifications\": {\"enable\": true}, \"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse opacity/color when missing (should return defaults)
        let content = fs::read_to_string(path_str).unwrap();
        let color = parse_surfaces_color(&content, "backplate_color", [0x0a, 0x1a, 0x0e]);
        assert_eq!(color, [0x0a, 0x1a, 0x0e]);
        let radius = parse_surfaces_u16(&content, "backplate_corner_radius", 12);
        assert_eq!(radius, 12);

        // 5. Write surfaces backplate_color config
        write_surfaces_config_value_path(path_str, "backplate_color", "\"#112233\"");
        let updated2 = fs::read_to_string(path_str).unwrap();
        assert!(updated2.contains("\"backplate_color\": \"#112233\""));

        // 6. Parse surfaces backplate_color when present
        let color2 = parse_surfaces_color(&updated2, "backplate_color", [0, 0, 0]);
        assert_eq!(color2, [17, 34, 51]);

        // 7. Write surfaces backplate_corner_radius config
        write_surfaces_config_value_path(path_str, "backplate_corner_radius", "16");
        let updated3 = fs::read_to_string(path_str).unwrap();
        assert!(updated3.contains("\"backplate_corner_radius\": 16"));

        // 8. Parse surfaces backplate_corner_radius when present
        let radius2 = parse_surfaces_u16(&updated3, "backplate_corner_radius", 12);
        assert_eq!(radius2, 16);

        // 9. Write surfaces desktop_background config
        write_surfaces_config_value_path(path_str, "desktop_background", "\"#445566\"");
        let updated4 = fs::read_to_string(path_str).unwrap();
        assert!(updated4.contains("\"desktop_background\": \"#445566\""));

        // 10. Parse surfaces desktop_background when present
        let color3 = parse_surfaces_color(&updated4, "desktop_background", [0, 0, 0]);
        assert_eq!(color3, [68, 85, 102]);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_page_opacity() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_page_opacity_config.toml");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_f32_from(&content, "page_opacity", 1.0);
        assert_eq!(val, 1.0);

        assert!(write_config_value_path(path_str, "page_opacity", "0.75"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"page_opacity\": 0.75"));

        let val2 = parse_f32_from(&updated, "page_opacity", 1.0);
        assert_eq!(val2, 0.75);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_layer_opacity() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_layer_opacity_config.toml");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_f32_from(&content, "layer_opacity", 1.0);
        assert_eq!(val, 1.0);

        assert!(write_config_value_path(path_str, "layer_opacity", "0.60"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"layer_opacity\": 0.6"));

        let val2 = parse_f32_from(&updated, "layer_opacity", 1.0);
        assert_eq!(val2, 0.60);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_page_color() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_page_color_config.toml");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let color = parse_color_from_key(&content, "page_color", [0, 0, 0]);
        assert_eq!(color, [0, 0, 0]);

        assert!(write_config_value_path(path_str, "page_color", "\"#112233\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"page_color\": \"#112233\""));

        let color2 = parse_color_from_key(&updated, "page_color", [0, 0, 0]);
        assert_eq!(color2, [17, 34, 51]);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_layer_color() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_layer_color_config.toml");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"gap\": 18}}";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let color = parse_color_from_key(&content, "layer_color", [0, 0, 0]);
        assert_eq!(color, [0, 0, 0]);

        assert!(write_config_value_path(path_str, "layer_color", "\"#445566\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"layer_color\": \"#445566\""));

        let color2 = parse_color_from_key(&updated, "layer_color", [0, 0, 0]);
        assert_eq!(color2, [68, 85, 102]);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_parse_side_panel_width_default() {
        let content = "";
        let width = parse_u16_from(content, "side_panel_width", 360);
        assert_eq!(width, 360);
    }

    #[test]
    fn test_parse_side_panel_width_explicit() {
        let content = "{\"layout\": {\"side_panel_width\": 450}}";
        let width = parse_u16_from(content, "side_panel_width", 360);
        assert_eq!(width, 450);
    }

    #[test]
    fn test_parse_side_panel_border_opacity_default() {
        let content = "";
        let opacity = parse_u16_from(content, "side_panel_border_opacity", 100);
        assert_eq!(opacity, 100);
    }

    #[test]
    fn test_parse_side_panel_border_opacity_explicit() {
        let content = "{\"layout\": {\"side_panel_border_opacity\": 75}}";
        let opacity = parse_u16_from(content, "side_panel_border_opacity", 100);
        assert_eq!(opacity, 75);
    }

    #[test]
    fn test_view_layout_grid() {
        let mut state = InterfaceState::default();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false, false, false, false, false, false, false, false, false, false], &mut layout, &mut cce_ui::context::UiContext::new());
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }

    #[test]
    fn test_spinbox_right_click() {
        use cce_ui::widget::Element;
        let mut state = InterfaceState::default();
        let mut ctx = cce_ui::context::UiContext::new();
        let sb = &mut state.windows.cascade_offset_spinbox;
        sb.set_rect(0.0, 0.0, 100.0, 44.0);
        let res = sb.mouse_input(
            cce_ui::widget::MouseButton::Right,
            cce_ui::widget::ElementState::Pressed,
            50.0,
            20.0,
            &mut ctx,
        );
        assert!(res);
    }

    #[test]
    #[allow(unused_assignments)]
    fn test_spinbox_right_click_crash() {
        use cce_ui::widget::Element;
        let mut state = InterfaceState::default();
        let mut ctx = cce_ui::context::UiContext::new();
        let sb = &mut state.windows.cascade_offset_spinbox;
        sb.set_rect(0.0, 0.0, 100.0, 44.0);
        let res = sb.mouse_input(
            cce_ui::widget::MouseButton::Right,
            cce_ui::widget::ElementState::Pressed,
            50.0,
            20.0,
            &mut ctx,
        );
        assert!(res);
        assert!(cce_ui::widget::context_menu::is_visible());

        // Now replace the state simulating config reload/refresh
        let new_state = InterfaceState::default();
        state = new_state;

        // Assert that the context menu is hidden (cleared)
        assert!(!cce_ui::widget::context_menu::is_visible());
    }

    #[test]
    fn test_read_write_padding() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_padding.json");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "{\"layout\": {\"status_separators\": true, \"status_padding\": 8}}");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_padding().unwrap_or(8);
        write_status_padding(12);
        assert_eq!(read_status_padding(), Some(12));
        write_status_padding(original);
        assert_eq!(read_status_padding(), Some(original));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_write_status_box_opacity() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_box_opacity.json");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "{\"layout\": {\"status_box_opacity\": 1.0}}");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_box_opacity().unwrap_or(1.0);
        write_status_box_opacity(0.75);
        assert_eq!(read_status_box_opacity(), Some(0.75));
        write_status_box_opacity(original);
        assert_eq!(read_status_box_opacity(), Some(original));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_write_status_box_blur() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_box_blur.json");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "{\"layout\": {\"status_box_blur\": 0.0}}");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_box_blur().unwrap_or(0.0);
        write_status_box_blur(0.5);
        assert_eq!(read_status_box_blur(), Some(0.5));
        write_status_box_blur(original);
        assert_eq!(read_status_box_blur(), Some(original));

        let _ = fs::remove_file(path);
    }
}

impl crate::pages::AppPage for InterfaceState {
    fn clear_children(&mut self, ctx: &mut cce_ui::context::UiContext) {
        self.windows.cascade_offset_spinbox.clear_children(ctx);
        self.windows.cascade_offset_spinbox.set_parent(None, ctx);
        self.windows.edge_gap_spinbox.clear_children(ctx);
        self.windows.edge_gap_spinbox.set_parent(None, ctx);
        self.windows.top_gap_spinbox.clear_children(ctx);
        self.windows.top_gap_spinbox.set_parent(None, ctx);
        self.windows.transition_duration_spinbox.clear_children(ctx);
        self.windows.transition_duration_spinbox.set_parent(None, ctx);
        self.windows.status_height_spinbox.clear_children(ctx);
        self.windows.status_height_spinbox.set_parent(None, ctx);

        for cs in &mut self.color_selectors {
            cs.clear_children(ctx);
            cs.set_parent(None, ctx);
        }
        self.menubar_opacity_spinbox.clear_children(ctx);
        self.menubar_opacity_spinbox.set_parent(None, ctx);
        self.button_padding_spinbox.clear_children(ctx);
        self.button_padding_spinbox.set_parent(None, ctx);
        self.section_padding_spinbox.clear_children(ctx);
        self.section_padding_spinbox.set_parent(None, ctx);
        self.label_alignment_menu.clear_children(ctx);
        self.label_alignment_menu.set_parent(None, ctx);
        self.label_offset_spinbox.clear_children(ctx);
        self.label_offset_spinbox.set_parent(None, ctx);
        self.label_margin_spinbox.clear_children(ctx);
        self.label_margin_spinbox.set_parent(None, ctx);
        self.plate_padding_spinbox.clear_children(ctx);
        self.plate_padding_spinbox.set_parent(None, ctx);
        self.graph_show_grid_toggle.clear_children(ctx);
        self.graph_show_grid_toggle.set_parent(None, ctx);
        self.graph_snap_enabled_toggle.clear_children(ctx);
        self.graph_snap_enabled_toggle.set_parent(None, ctx);
        self.graph_uniform_background_toggle.clear_children(ctx);
        self.graph_uniform_background_toggle.set_parent(None, ctx);
        self.graph_cell_opacity_spinbox.clear_children(ctx);
        self.graph_cell_opacity_spinbox.set_parent(None, ctx);
        self.graph_gap_opacity_spinbox.clear_children(ctx);
        self.graph_gap_opacity_spinbox.set_parent(None, ctx);
        self.graph_gap_width_spinbox.clear_children(ctx);
        self.graph_gap_width_spinbox.set_parent(None, ctx);
        self.page_margin_spinbox.clear_children(ctx);
        self.page_margin_spinbox.set_parent(None, ctx);
        self.desktop_grid_scale_spinbox.clear_children(ctx);
        self.desktop_grid_scale_spinbox.set_parent(None, ctx);
        self.desktop_line_width_spinbox.clear_children(ctx);
        self.desktop_line_width_spinbox.set_parent(None, ctx);
        self.grid_min_col_width_spinbox.clear_children(ctx);
        self.grid_min_col_width_spinbox.set_parent(None, ctx);
        self.layout_grid_gap_spinbox.clear_children(ctx);
        self.layout_grid_gap_spinbox.set_parent(None, ctx);
        self.spinbox_height_spinbox.clear_children(ctx);
        self.spinbox_height_spinbox.set_parent(None, ctx);
        self.spinbox_corner_radius_spinbox.clear_children(ctx);
        self.spinbox_corner_radius_spinbox.set_parent(None, ctx);
        self.toggle_height_spinbox.clear_children(ctx);
        self.toggle_height_spinbox.set_parent(None, ctx);
        self.toggle_corner_radius_spinbox.clear_children(ctx);
        self.toggle_corner_radius_spinbox.set_parent(None, ctx);
        self.toggle_border_width_spinbox.clear_children(ctx);
        self.toggle_border_width_spinbox.set_parent(None, ctx);

        self.color_selector_height_spinbox.clear_children(ctx);
        self.color_selector_height_spinbox.set_parent(None, ctx);
        self.color_selector_corner_radius_spinbox.clear_children(ctx);
        self.color_selector_corner_radius_spinbox.set_parent(None, ctx);
        self.plate_opacity_spinbox.clear_children(ctx);
        self.plate_opacity_spinbox.set_parent(None, ctx);
        self.plate_corner_radius_spinbox.clear_children(ctx);
        self.plate_corner_radius_spinbox.set_parent(None, ctx);
        self.page_opacity_spinbox.clear_children(ctx);
        self.page_opacity_spinbox.set_parent(None, ctx);
        self.layer_opacity_spinbox.clear_children(ctx);
        self.layer_opacity_spinbox.set_parent(None, ctx);

        self.color_selector_preview_corner_radius_spinbox.clear_children(ctx);
        self.color_selector_preview_corner_radius_spinbox.set_parent(None, ctx);
        self.color_selector_preview_margin_spinbox.clear_children(ctx);
        self.color_selector_preview_margin_spinbox.set_parent(None, ctx);
        self.textbox_height_spinbox.clear_children(ctx);
        self.textbox_height_spinbox.set_parent(None, ctx);
        self.textbox_corner_radius_spinbox.clear_children(ctx);
        self.textbox_corner_radius_spinbox.set_parent(None, ctx);
        self.slider_height_spinbox.clear_children(ctx);
        self.slider_height_spinbox.set_parent(None, ctx);
        self.slider_corner_radius_spinbox.clear_children(ctx);
        self.slider_corner_radius_spinbox.set_parent(None, ctx);
        self.color_selector_font_selector.clear_children(ctx);
        self.color_selector_font_selector.set_parent(None, ctx);
        self.menubar_font_selector.clear_children(ctx);
        self.menubar_font_selector.set_parent(None, ctx);
        self.breadcrumb_font_selector.clear_children(ctx);
        self.breadcrumb_font_selector.set_parent(None, ctx);
        self.toggle_font_selector.clear_children(ctx);
        self.toggle_font_selector.set_parent(None, ctx);
        self.font_selector_font_selector.clear_children(ctx);
        self.font_selector_font_selector.set_parent(None, ctx);
        self.button_strip_font_selector.clear_children(ctx);
        self.button_strip_font_selector.set_parent(None, ctx);
        self.button_font_selector.clear_children(ctx);
        self.button_font_selector.set_parent(None, ctx);
        self.label_font_selector.clear_children(ctx);
        self.label_font_selector.set_parent(None, ctx);
        self.dropdown_font_selector.clear_children(ctx);
        self.dropdown_font_selector.set_parent(None, ctx);
        self.textbox_font_selector.clear_children(ctx);
        self.textbox_font_selector.set_parent(None, ctx);
        self.spinbox_font_selector.clear_children(ctx);
        self.spinbox_font_selector.set_parent(None, ctx);
        self.slider_font_selector.clear_children(ctx);
        self.slider_font_selector.set_parent(None, ctx);
        self.section_label_font_selector.clear_children(ctx);
        self.section_label_font_selector.set_parent(None, ctx);
        self.nested_section_label_font_selector.clear_children(ctx);
        self.nested_section_label_font_selector.set_parent(None, ctx);
        self.font_selector_height_spinbox.clear_children(ctx);
        self.font_selector_height_spinbox.set_parent(None, ctx);
        self.font_selector_corner_radius_spinbox.clear_children(ctx);
        self.font_selector_corner_radius_spinbox.set_parent(None, ctx);
        self.dropdown_height_spinbox.clear_children(ctx);
        self.dropdown_height_spinbox.set_parent(None, ctx);
        self.dropdown_corner_radius_spinbox.clear_children(ctx);
        self.dropdown_corner_radius_spinbox.set_parent(None, ctx);
        self.button_corner_radius_spinbox.clear_children(ctx);
        self.button_corner_radius_spinbox.set_parent(None, ctx);
        self.notification_opacity_spinbox.clear_children(ctx);
        self.notification_opacity_spinbox.set_parent(None, ctx);
        self.backplate_corner_radius_spinbox.clear_children(ctx);
        self.backplate_corner_radius_spinbox.set_parent(None, ctx);
        self.custom_multicontrol.clear_children(ctx);
        self.custom_multicontrol.set_parent(None, ctx);
        self.status_box_corner_radius_spinbox.clear_children(ctx);
        self.status_box_corner_radius_spinbox.set_parent(None, ctx);
        self.status_padding_spinbox.clear_children(ctx);
        self.status_padding_spinbox.set_parent(None, ctx);
        self.status_module_spacing_spinbox.clear_children(ctx);
        self.status_module_spacing_spinbox.set_parent(None, ctx);
        self.status_controls.box_opacity_slider.clear_children(ctx);
        self.status_controls.box_opacity_slider.set_parent(None, ctx);
        self.status_controls.box_blur_slider.clear_children(ctx);
        self.status_controls.box_blur_slider.set_parent(None, ctx);

        self.sans_box.clear_children(ctx);
        self.sans_box.set_parent(None, ctx);
        self.serif_box.clear_children(ctx);
        self.serif_box.set_parent(None, ctx);
        self.mono_box.clear_children(ctx);
        self.mono_box.set_parent(None, ctx);
        self.borders_box.clear_children(ctx);
        self.borders_box.set_parent(None, ctx);
        self.status_box.clear_children(ctx);
        self.status_box.set_parent(None, ctx);
        self.fuzzel_box.clear_children(ctx);
        self.fuzzel_box.set_parent(None, ctx);
        self.terminal_box.clear_children(ctx);
        self.terminal_box.set_parent(None, ctx);
        self.borders_menu.clear_children(ctx);
        self.borders_menu.set_parent(None, ctx);
        self.status_menu.clear_children(ctx);
        self.status_menu.set_parent(None, ctx);
        self.fuzzel_menu.clear_children(ctx);
        self.fuzzel_menu.set_parent(None, ctx);
        self.terminal_menu.clear_children(ctx);
        self.terminal_menu.set_parent(None, ctx);
        self.borders_size_box.clear_children(ctx);
        self.borders_size_box.set_parent(None, ctx);
        self.status_size_box.clear_children(ctx);
        self.status_size_box.set_parent(None, ctx);
        self.fuzzel_size_box.clear_children(ctx);
        self.fuzzel_size_box.set_parent(None, ctx);
        self.terminal_size_box.clear_children(ctx);
        self.terminal_size_box.set_parent(None, ctx);

        self.windows.side_panel_behavior_menu.clear_children(ctx);
        self.windows.side_panel_behavior_menu.set_parent(None, ctx);
        self.windows.side_panel_position_menu.clear_children(ctx);
        self.windows.side_panel_position_menu.set_parent(None, ctx);
        self.windows.side_panel_width_spinbox.clear_children(ctx);
        self.windows.side_panel_width_spinbox.set_parent(None, ctx);
        self.windows.side_panel_border_gap_spinbox.clear_children(ctx);
        self.windows.side_panel_border_gap_spinbox.set_parent(None, ctx);

        self.windows.fullscreen_opacity_spinbox.clear_children(ctx);
        self.windows.fullscreen_opacity_spinbox.set_parent(None, ctx);
        self.windows.cascade_opacity_spinbox.clear_children(ctx);
        self.windows.cascade_opacity_spinbox.set_parent(None, ctx);
        self.windows.grid_opacity_spinbox.clear_children(ctx);
        self.windows.grid_opacity_spinbox.set_parent(None, ctx);
        self.windows.floating_opacity_spinbox.clear_children(ctx);
        self.windows.floating_opacity_spinbox.set_parent(None, ctx);
        self.windows.pinned_opacity_spinbox.clear_children(ctx);
        self.windows.pinned_opacity_spinbox.set_parent(None, ctx);
        self.windows.popup_opacity_spinbox.clear_children(ctx);
        self.windows.popup_opacity_spinbox.set_parent(None, ctx);
        self.windows.blur_toggle.clear_children(ctx);
        self.windows.blur_toggle.set_parent(None, ctx);
    }

    fn get_section_containers(&self) -> Vec<cce_ui::widget::SectionContainer> {
        vec![
            cce_ui::widget::SectionContainer::new("Custom Parameters").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
            cce_ui::widget::SectionContainer::new("Layout").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
            cce_ui::widget::SectionContainer::new("Status").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
            cce_ui::widget::SectionContainer::new("Controls").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
            cce_ui::widget::SectionContainer::new("Indicators").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
            cce_ui::widget::SectionContainer::new("Notification").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
            cce_ui::widget::SectionContainer::new("Surfaces").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
            cce_ui::widget::SectionContainer::new("Fonts").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
            cce_ui::widget::SectionContainer::new("Containers").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
            cce_ui::widget::SectionContainer::new("Windows").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }),
        ]
    }

    fn link_children(
        &mut self,
        page_root: &mut dyn cce_ui::widget::Element,
        sec_containers: &mut [cce_ui::widget::SectionContainer],
        ctx: &mut cce_ui::context::UiContext,
    ) {
        for i in 0..10 {
            cce_ui::widget::link_parent_child(page_root, &mut sec_containers[i], ctx);
        }

        // Section 0: Custom Parameters
        cce_ui::widget::link_parent_child(&mut sec_containers[0], &mut self.custom_multicontrol, ctx);

        // Section 1: Layout
        cce_ui::widget::link_parent_child(&mut sec_containers[1], &mut self.color_selectors[1], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[1], &mut self.color_selectors[2], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[1], &mut self.grid_min_col_width_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[1], &mut self.layout_grid_gap_spinbox, ctx);

        // Section 2: Status
        cce_ui::widget::link_parent_child(&mut sec_containers[2], &mut self.color_selectors[7], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[2], &mut self.color_selectors[3], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[2], &mut self.color_selectors[22], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[2], &mut self.status_box_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[2], &mut self.status_padding_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[2], &mut self.status_module_spacing_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[2], &mut self.status_controls.box_opacity_slider, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[2], &mut self.status_controls.box_blur_slider, ctx);

        // Section 3: Controls
        // Slider
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selectors[4], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.slider_height_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.slider_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.slider_font_selector, ctx);
        // MenuBar
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selectors[8], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selectors[10], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.menubar_font_selector, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.menubar_opacity_spinbox, ctx);
        // Toggle
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selectors[11], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selectors[12], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.toggle_height_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.toggle_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selectors[24], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.toggle_border_width_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.toggle_font_selector, ctx);
        // Breadcrumb
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selectors[14], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.breadcrumb_font_selector, ctx);
        // Spinbox
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.spinbox_height_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.spinbox_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.spinbox_font_selector, ctx);
        // ColorSelector
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selector_height_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selector_preview_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selector_preview_margin_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selector_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.color_selector_font_selector, ctx);
        // Textbox
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.textbox_height_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.textbox_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.textbox_font_selector, ctx);
        // FontSelector
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.font_selector_height_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.font_selector_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.font_selector_font_selector, ctx);
        // Dropdown
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.dropdown_height_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.dropdown_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.dropdown_font_selector, ctx);
        // Button
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.button_corner_radius_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.button_font_selector, ctx);
        // ButtonStrip
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.button_padding_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.button_strip_spacing_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.button_strip_font_selector, ctx);
        // Labels
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.label_margin_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[3], &mut self.label_font_selector, ctx);

        // Section 4: Indicators
        cce_ui::widget::link_parent_child(&mut sec_containers[4], &mut self.color_selectors[9], ctx);

        // Section 5: Notification
        cce_ui::widget::link_parent_child(&mut sec_containers[5], &mut self.color_selectors[16], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[5], &mut self.notification_opacity_spinbox, ctx);

        // Section 6: Surfaces
        // Backplate
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.color_selectors[17], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.backplate_corner_radius_spinbox, ctx);
        // Plate
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.color_selectors[0], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.plate_padding_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.plate_opacity_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.plate_corner_radius_spinbox, ctx);
        // Popover
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.color_selectors[15], ctx);
        // Desktop
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.color_selectors[6], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.color_selectors[23], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.desktop_grid_scale_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[6], &mut self.desktop_line_width_spinbox, ctx);

        // Section 7: Fonts
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.sans_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.serif_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.mono_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.borders_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.status_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.fuzzel_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.terminal_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.borders_menu, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.status_menu, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.fuzzel_menu, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.terminal_menu, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.borders_size_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.status_size_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.fuzzel_size_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[7], &mut self.terminal_size_box, ctx);

        // Section 8: Containers
        // Page
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.color_selectors[18], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.page_opacity_spinbox, ctx);
        // Layer
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.color_selectors[19], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.layer_opacity_spinbox, ctx);
        // Section
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.section_padding_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.page_margin_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.section_label_font_selector, ctx);
        // Nested Section
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.label_alignment_menu, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.label_offset_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.nested_section_label_font_selector, ctx);
        // ScrollingList
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.color_selectors[13], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.color_selectors[20], ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.color_selectors[21], ctx);
        // Graph
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.graph_show_grid_toggle, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.graph_snap_enabled_toggle, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.graph_uniform_background_toggle, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.graph_cell_opacity_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.graph_gap_opacity_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[8], &mut self.graph_gap_width_spinbox, ctx);

        // Section 9: Windows
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.side_panel_behavior_menu, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.side_panel_position_menu, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.side_panel_width_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.side_panel_border_gap_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.pinned_opacity_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.popup_opacity_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.fullscreen_opacity_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.cascade_opacity_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.grid_opacity_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.floating_opacity_spinbox, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[9], &mut self.windows.blur_toggle, ctx);
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        _root_focused: bool,
        sec_focused: &[bool],
        layout: &mut dyn cce_ui::layout::LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>) {
        for (i, cp) in self.color_selectors.iter_mut().enumerate() {
            if cp.take_change() {
                actions.push(AppAction::Interface(match i {
                    0 => InterfaceMessage::SetPageLowColor(cp.color),
                    1 => InterfaceMessage::SetHighColor(cp.color),
                    2 => InterfaceMessage::SetVisualGuidesColor(cp.color),
                    3 => InterfaceMessage::SetDisabledColor(cp.color),
                    4 => InterfaceMessage::SetSliderTrackColor(cp.color),
                    5 => InterfaceMessage::SetColorBordersColor(cp.color),
                    6 => InterfaceMessage::SetDesktopBackground(cp.color),
                    7 => InterfaceMessage::SetNormalColor(cp.color),
                    8 => InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                    9 => InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                    10 => InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                    11 => InterfaceMessage::SetToggleEnabledColor(cp.color),
                    12 => InterfaceMessage::SetToggleDisabledColor(cp.color),
                    13 => InterfaceMessage::SetScrollingListBgColor(cp.color),
                    14 => InterfaceMessage::SetScrollingListEntryBgColor([cp.color[0], cp.color[1], cp.color[2], 255]),
                    15 => InterfaceMessage::SetScrollingListEntryHighlightColor([cp.color[0], cp.color[1], cp.color[2], 255]),
                    16 => InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                    17 => InterfaceMessage::SetPopoverBgColor(cp.color),
                    18 => InterfaceMessage::SetNotificationBgColor(cp.color),
                    19 => InterfaceMessage::SetBackplateColor(cp.color),
                    20 => InterfaceMessage::SetPageColor(cp.color),
                    21 => InterfaceMessage::SetLayerColor(cp.color),
                    22 => InterfaceMessage::SetStatusBoxBackgroundColor(cp.color),
                    23 => InterfaceMessage::SetDesktopGridColor([cp.color[0], cp.color[1], cp.color[2], 255]),
                    24 => InterfaceMessage::SetToggleBgColor(cp.color),
                    _ => return,
                }));
            }
        }

        if self.status_box_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetStatusBoxCornerRadius(self.status_box_corner_radius_spinbox.value as u16)));
        }
        if self.status_padding_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetStatusPadding(self.status_padding_spinbox.value as u16)));
        }
        if self.status_module_spacing_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetStatusModuleSpacing(self.status_module_spacing_spinbox.value as u16)));
        }
        if self.status_controls.box_opacity_slider.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::StatusSetBoxOpacity(self.status_controls.box_opacity_slider.value())));
        }
        if self.status_controls.box_blur_slider.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::StatusSetBoxBlur(self.status_controls.box_blur_slider.value())));
        }

        if self.menubar_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetMenubarOpacity(self.menubar_opacity_spinbox.value as f32 / 100.0)));
        }
        if self.button_padding_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetButtonPadding(self.button_padding_spinbox.value as u16)));
        }
        if self.button_strip_spacing_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetButtonStripSpacing(self.button_strip_spacing_spinbox.value as u16)));
        }
        if self.section_padding_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSectionPadding(self.section_padding_spinbox.value as u16)));
        }
        if self.plate_padding_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetPlatePadding(self.plate_padding_spinbox.value as u16)));
        }
        if self.plate_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetPlateOpacity(self.plate_opacity_spinbox.value as f32 / 100.0)));
        }
        if self.plate_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetPlateCornerRadius(self.plate_corner_radius_spinbox.value as u16)));
        }
        if self.page_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetPageOpacity(self.page_opacity_spinbox.value as f32 / 100.0)));
        }
        if self.layer_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetLayerOpacity(self.layer_opacity_spinbox.value as f32 / 100.0)));
        }
        if self.page_margin_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetPageMargin(self.page_margin_spinbox.value as u16)));
        }
        if self.desktop_grid_scale_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetDesktopGridScale(self.desktop_grid_scale_spinbox.value as u16)));
        }
        if self.desktop_line_width_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetDesktopLineWidth(self.desktop_line_width_spinbox.value as u16)));
        }
        if self.grid_min_col_width_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetGridMinColWidth(self.grid_min_col_width_spinbox.value as u16)));
        }
        if self.layout_grid_gap_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetLayoutGridGap(self.layout_grid_gap_spinbox.value as u16)));
        }
        if self.spinbox_height_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSpinboxHeight(self.spinbox_height_spinbox.value as u16)));
        }
        if self.spinbox_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSpinboxCornerRadius(self.spinbox_corner_radius_spinbox.value as u16)));
        }
        if self.toggle_height_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetToggleHeight(self.toggle_height_spinbox.value as u16)));
        }
        if self.toggle_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetToggleCornerRadius(self.toggle_corner_radius_spinbox.value as u16)));
        }
        if self.toggle_border_width_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetToggleBorderWidth(self.toggle_border_width_spinbox.value as u16)));
        }
        if self.color_selector_height_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetColorSelectorHeight(self.color_selector_height_spinbox.value as u16)));
        }
        if self.color_selector_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetColorSelectorCornerRadius(self.color_selector_corner_radius_spinbox.value as u16)));
        }
        if self.color_selector_preview_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetColorSelectorPreviewCornerRadius(self.color_selector_preview_corner_radius_spinbox.value as u16)));
        }
        if self.color_selector_preview_margin_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetColorSelectorPreviewMargin(self.color_selector_preview_margin_spinbox.value as u16)));
        }
        if self.textbox_height_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetTextboxHeight(self.textbox_height_spinbox.value as u16)));
        }
        if self.textbox_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetTextboxCornerRadius(self.textbox_corner_radius_spinbox.value as u16)));
        }
        if self.slider_height_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSliderHeight(self.slider_height_spinbox.value as u16)));
        }
        if self.slider_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSliderCornerRadius(self.slider_corner_radius_spinbox.value as u16)));
        }

        if self.label_alignment_menu.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetNestedSectionLabelAlignment(self.label_alignment_menu.selected)));
        }
        if self.label_offset_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetNestedSectionLabelOffset(self.label_offset_spinbox.value as i16)));
        }
        if self.label_margin_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetLabelMargin(self.label_margin_spinbox.value as u16)));
        }

        // Fonts
        if self.sans_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSans(self.sans_box.text.clone())));
        }
        if self.serif_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSerif(self.serif_box.text.clone())));
        }
        if self.mono_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetMono(self.mono_box.text.clone())));
        }
        if self.borders_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetBorders(self.borders_box.text.clone())));
        }
        if self.status_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetStatus(self.status_box.text.clone())));
        }
        if self.fuzzel_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetFuzzel(self.fuzzel_box.text.clone())));
        }
        if self.terminal_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetTerminal(self.terminal_box.text.clone())));
        }
        if self.borders_menu.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetBordersMenu(self.borders_menu.selected)));
        }
        if self.status_menu.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetStatusMenu(self.status_menu.selected)));
        }
        if self.fuzzel_menu.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetFuzzelMenu(self.fuzzel_menu.selected)));
        }
        if self.terminal_menu.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetTerminalMenu(self.terminal_menu.selected)));
        }
        if self.borders_size_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetBordersSize(self.borders_size_box.value)));
        }
        if self.status_size_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetStatusSize(self.status_size_box.value)));
        }
        if self.fuzzel_size_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetFuzzelSize(self.fuzzel_size_box.value)));
        }
        if self.terminal_size_box.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetTerminalSize(self.terminal_size_box.value)));
        }

        // Windows Page Settings
        if self.windows.side_panel_behavior_menu.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetSidePanelBehavior(self.windows.side_panel_behavior_menu.selected))));
        }
        if self.windows.side_panel_position_menu.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetSidePanelPosition(self.windows.side_panel_position_menu.selected))));
        }
        if self.windows.side_panel_width_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetSidePanelWidth(self.windows.side_panel_width_spinbox.value as u16))));
        }
        if self.windows.side_panel_border_gap_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetSidePanelBorderGap(self.windows.side_panel_border_gap_spinbox.value as u16))));
        }

        if self.windows.fullscreen_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetFullscreenOpacity(self.windows.fullscreen_opacity_spinbox.value as u16))));
        }
        if self.windows.cascade_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetCascadeOpacity(self.windows.cascade_opacity_spinbox.value as u16))));
        }
        if self.windows.grid_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetGridOpacity(self.windows.grid_opacity_spinbox.value as u16))));
        }
        if self.windows.floating_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetFloatingOpacity(self.windows.floating_opacity_spinbox.value as u16))));
        }
        if self.windows.pinned_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetPinnedOpacity(self.windows.pinned_opacity_spinbox.value as u16))));
        }
        if self.windows.popup_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetPopupOpacity(self.windows.popup_opacity_spinbox.value as u16))));
        }
        if self.windows.blur_toggle.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::ToggleBlur)));
        }

        // Toggles in font_selector & graph
        if self.graph_show_grid_toggle.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetGraphShowGrid(self.graph_show_grid_toggle.toggled())));
        }
        if self.graph_snap_enabled_toggle.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetGraphSnapEnabled(self.graph_snap_enabled_toggle.toggled())));
        }
        if self.graph_uniform_background_toggle.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetGraphUniformBackground(self.graph_uniform_background_toggle.toggled())));
        }
        if self.graph_cell_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetGraphCellOpacity(self.graph_cell_opacity_spinbox.value as f32 / 100.0)));
        }
        if self.graph_gap_opacity_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetGraphGapOpacity(self.graph_gap_opacity_spinbox.value as f32 / 100.0)));
        }
        if self.graph_gap_width_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetGraphGapWidth(self.graph_gap_width_spinbox.value as u16)));
        }
        if self.color_selector_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetColorSelectorFont(self.color_selector_font_selector.font_family.clone())));
        }
        if self.menubar_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetMenubarFont(self.menubar_font_selector.font_family.clone())));
        }
        if self.breadcrumb_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetBreadcrumbFont(self.breadcrumb_font_selector.font_family.clone())));
        }
        if self.toggle_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetToggleFont(self.toggle_font_selector.font_family.clone())));
        }
        if self.font_selector_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetFontSelectorFont(self.font_selector_font_selector.font_family.clone())));
        }
        if self.button_strip_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetButtonStripFont(self.button_strip_font_selector.font_family.clone())));
        }
        if self.button_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetButtonFont(self.button_font_selector.font_family.clone())));
        }
        if self.label_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetLabelFont(self.label_font_selector.font_family.clone())));
        }
        if self.dropdown_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetDropdownFont(self.dropdown_font_selector.font_family.clone())));
        }
        if self.textbox_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetTextboxFont(self.textbox_font_selector.font_family.clone())));
        }
        if self.spinbox_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSpinboxFont(self.spinbox_font_selector.font_family.clone())));
        }
        if self.slider_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSliderFont(self.slider_font_selector.font_family.clone())));
        }
        if self.section_label_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetSectionLabelFont(self.section_label_font_selector.font_family.clone())));
        }
        if self.nested_section_label_font_selector.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetNestedSectionLabelFont(self.nested_section_label_font_selector.font_family.clone())));
        }
        if self.font_selector_height_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetFontSelectorHeight(self.font_selector_height_spinbox.value as u16)));
        }
        if self.font_selector_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetFontSelectorCornerRadius(self.font_selector_corner_radius_spinbox.value as u16)));
        }
        if self.dropdown_height_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetDropdownHeight(self.dropdown_height_spinbox.value as u16)));
        }
        if self.dropdown_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetDropdownCornerRadius(self.dropdown_corner_radius_spinbox.value as u16)));
        }
        if self.button_corner_radius_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::SetButtonCornerRadius(self.button_corner_radius_spinbox.value as u16)));
        }

        // Windows Page Settings (rest)
        if self.windows.status_height_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetStatusHeight(self.windows.status_height_spinbox.value as u16))));
        }
        if self.windows.cascade_offset_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetCascadeOffset(self.windows.cascade_offset_spinbox.value as u16))));
        }
        if self.windows.edge_gap_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetEdgeGap(self.windows.edge_gap_spinbox.value as u16))));
        }
        if self.windows.top_gap_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetTopGap(self.windows.top_gap_spinbox.value as u16))));
        }
        if self.windows.transition_duration_spinbox.take_change() {
            actions.push(AppAction::Interface(InterfaceMessage::Windows(WindowsMessage::SetTransitionDuration(self.windows.transition_duration_spinbox.value as u16))));
        }
    }

    fn handle_pointer_move(
        &mut self,
        lx: f32,
        ly: f32,
        actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) -> bool {
        if self.status_controls.opacity_dragging {
            if self.status_controls.box_opacity_slider.drag_update(lx, ly) {
                let val = self.status_controls.box_opacity_slider.value();
                actions.push(AppAction::Interface(InterfaceMessage::StatusSetBoxOpacity(val)));
                return true;
            }
        } else if self.status_controls.blur_dragging {
            if self.status_controls.box_blur_slider.drag_update(lx, ly) {
                let val = self.status_controls.box_blur_slider.value();
                actions.push(AppAction::Interface(InterfaceMessage::StatusSetBoxBlur(val)));
                return true;
            }
        }
        false
    }

    fn handle_pointer_down(&mut self, _lx: f32, _ly: f32, _ctx: &mut cce_ui::context::UiContext) -> bool {
        if self.status_controls.box_opacity_slider.is_dragging() {
            self.status_controls.opacity_dragging = true;
            return true;
        }
        if self.status_controls.box_blur_slider.is_dragging() {
            self.status_controls.blur_dragging = true;
            return true;
        }
        false
    }

    fn handle_pointer_up(&mut self, _ctx: &mut cce_ui::context::UiContext) -> bool {
        let mut any = false;
        if self.status_controls.opacity_dragging {
            self.status_controls.box_opacity_slider.drag_end();
            self.status_controls.opacity_dragging = false;
            any = true;
        }
        if self.status_controls.blur_dragging {
            self.status_controls.box_blur_slider.drag_end();
            self.status_controls.blur_dragging = false;
            any = true;
        }
        any
    }
}



