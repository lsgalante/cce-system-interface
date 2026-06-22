use std::fs;
use std::io::Write;
use crate::app::PageContent;
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{
    ColorSelector, Spinbox, Element, Dropdown, TextBox, FontSelector, Toggle, MultiControl,
    LayoutPreview, PreviewLayoutMode
};

const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.json";
const LINKS_PATH: &str = "/home/lsgalante/.config/cce/cce-system-interface/links.json";

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/cce-{}.sock", display),
        Err(_) => "/tmp/cce.sock".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthParam {
    Fullscreen, Cascade, Grid, Floating,
}

impl WidthParam {
    pub const ALL: [WidthParam; 4] = [
        WidthParam::Fullscreen, WidthParam::Cascade, WidthParam::Grid,
        WidthParam::Floating,
    ];
    pub fn key(self) -> &'static str {
        match self {
            WidthParam::Fullscreen => "fullscreen_border_width",
            WidthParam::Cascade => "cascade_border_width",
            WidthParam::Grid => "grid_border_width",
            WidthParam::Floating => "floating_border_width",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            WidthParam::Fullscreen => "Fullscreen",
            WidthParam::Cascade => "Cascade",
            WidthParam::Grid => "Grid",
            WidthParam::Floating => "Floating",
        }
    }
}

fn make_spinboxes(fs: u16, ca: u16, g: u16, fl: u16) -> Vec<Spinbox> {
    vec![
        Spinbox::new(fs as i32, 0, 100, 1),
        Spinbox::new(ca as i32, 0, 100, 1),
        Spinbox::new(g as i32, 0, 100, 1),
        Spinbox::new(fl as i32, 0, 100, 1),
    ]
}

#[derive(Debug, Clone)]
pub struct WindowsState {
    pub fullscreen_border_width: u16,
    pub cascade_border_width: u16,
    pub grid_border_width: u16,
    pub floating_border_width: u16,
    pub cascade_offset: u16,
    pub edge_gap: u16,
    pub top_gap: u16,
    pub grid_gap: u16,
    pub status_height: u16,
    pub transition_duration: u16,
    pub spinboxes: Vec<Spinbox>,
    pub cascade_offset_spinbox: Spinbox,
    pub edge_gap_spinbox: Spinbox,
    pub top_gap_spinbox: Spinbox,
    pub grid_gap_spinbox: Spinbox,
    pub status_height_spinbox: Spinbox,
    pub transition_duration_spinbox: Spinbox,
    pub tag_layout_menus: Vec<Dropdown>,
    pub side_panel_behavior_menu: Dropdown,
    pub side_panel_position_menu: Dropdown,
    pub side_panel_width: u16,
    pub side_panel_width_spinbox: Spinbox,
    pub side_panel_border_gap: u16,
    pub side_panel_border_gap_spinbox: Spinbox,
    pub side_panel_border_opacity: u16,
    pub side_panel_border_opacity_spinbox: Spinbox,
    pub transparency_enabled: bool,
    pub transparency_toggle: Toggle,
    pub blur_enabled: bool,
    pub blur_toggle: Toggle,
}

impl Default for WindowsState {
    fn default() -> Self {
        Self {
            fullscreen_border_width: 0,
            cascade_border_width: 6,
            grid_border_width: 6,
            floating_border_width: 6,
            cascade_offset: 20,
            edge_gap: 48,
            top_gap: 48,
            grid_gap: 6,
            status_height: 24,
            transition_duration: 300,
            spinboxes: make_spinboxes(0, 6, 6, 6),
            cascade_offset_spinbox: Spinbox::new(20, 0, 200, 1),
            edge_gap_spinbox: Spinbox::new(48, 0, 200, 1),
            top_gap_spinbox: Spinbox::new(48, 0, 200, 1),
            grid_gap_spinbox: Spinbox::new(6, 0, 200, 1),
            status_height_spinbox: Spinbox::new(24, 0, 100, 1),
            transition_duration_spinbox: Spinbox::new(300, 0, 2000, 50),
            tag_layout_menus: (1..=4).map(|i| {
                Dropdown::new(
                    vec![
                        "Cascade".to_string(),
                        "Grid".to_string(),
                        "Fullscreen".to_string(),
                        "Floating".to_string(),
                        "Popup".to_string(),
                    ],
                    0
                ).with_label(&format!("Tag {}", i))
            }).collect(),
            side_panel_behavior_menu: Dropdown::new(
                vec!["Above".to_string(), "Inline".to_string()],
                1,
            ).with_label("Behavior"),
            side_panel_position_menu: Dropdown::new(
                vec!["Left".to_string(), "Right".to_string()],
                0,
            ).with_label("Position"),
            side_panel_width: 360,
            side_panel_width_spinbox: Spinbox::new(360, 0, 2000, 10),
            side_panel_border_gap: 0,
            side_panel_border_gap_spinbox: Spinbox::new(0, 0, 500, 1),
            side_panel_border_opacity: 100,
            side_panel_border_opacity_spinbox: Spinbox::new(100, 0, 100, 5),
            transparency_enabled: true,
            transparency_toggle: Toggle::new().with_label("Transparency"),
            blur_enabled: true,
            blur_toggle: Toggle::new().with_label("Blur"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum WindowsMessage {
    SetWidth(WidthParam, u16),
    SetCascadeOffset(u16),
    SetEdgeGap(u16),
    SetTopGap(u16),
    SetGridGap(u16),
    SetTransitionDuration(u16),
    SetStatusHeight(u16),
    SetTagLayout(usize, usize),
    SetSidePanelBehavior(usize),
    SetSidePanelPosition(usize),
    SetSidePanelWidth(u16),
    SetSidePanelBorderGap(u16),
    SetSidePanelBorderOpacity(u16),
    ToggleTransparency,
    ToggleBlur,
    Refreshed(WindowsState),
}

#[derive(Debug, Clone)]
pub struct InterfaceState {
    pub windows: WindowsState,
    pub desktop_background_color: [u8; 3],
    pub high_color: [u8; 3],
    pub disabled_color: [u8; 3],
    pub separator_color: [u8; 3],
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
    pub window_color: [u8; 3],
    pub window_opacity: f32,
    pub window_opacity_spinbox: Spinbox,
    pub window_corner_radius: u16,
    pub window_corner_radius_spinbox: Spinbox,
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
    pub spinbox_height: u16,
    pub spinbox_height_spinbox: Spinbox,
    pub spinbox_corner_radius: u16,
    pub spinbox_corner_radius_spinbox: Spinbox,
    pub toggle_height: u16,
    pub toggle_height_spinbox: Spinbox,
    pub toggle_corner_radius: u16,
    pub toggle_corner_radius_spinbox: Spinbox,

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
            high_color: [0x3e, 0x3e, 0x3e],
            disabled_color: [0x55, 0x55, 0x55],
            separator_color: [124, 124, 137],
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
            scrollinglist_bg_color: [81, 81, 97],
            scrollinglist_entry_bg_color: [255, 255, 255, 10],
            scrollinglist_entry_highlight_color: [255, 255, 255, 204],
            breadcrumb_bg_color: [81, 81, 97],
            popover_bg_color: [81, 81, 97],
            color_selectors: vec![
                ColorSelector::new([71, 71, 81]).with_label("Low Color"), // 0: Plate - Low Color
                ColorSelector::new([0x3e, 0x3e, 0x3e]).with_label("High Color"), // 1: Layout - High Color
                ColorSelector::new([0xff, 0x8c, 0x00]).with_label("Visual Guides"), // 2: Layout - Visual Guides
                ColorSelector::new([0x55, 0x55, 0x55]).with_label("Disabled"), // 3: Status - Disabled
                ColorSelector::new([124, 124, 137]).with_label("Separators"), // 4: Status - Separators
                ColorSelector::new([116, 116, 128]).with_label("Slider Track"), // 5: Controls - Slider Track
                ColorSelector::new([124, 124, 137]).with_label("Borders"), // 6: Controls - Borders
                ColorSelector::new([0, 0, 0]).with_label("Color"), // 7: Surfaces - Desktop Background Color
                ColorSelector::new([0xcc, 0xcc, 0xd8]).with_label("Normal"), // 8: Status - Normal
                ColorSelector::new([90, 90, 101]).with_label("Background"), // 9: Controls - Paginator Sidebar (now Background)
                ColorSelector::new([255, 255, 255]).with_label("Primary Highlight"), // 10: Controls - Primary Highlight
                ColorSelector::new([230, 230, 242]).with_label("Tab Label"), // 11: Controls - Tab Label
                ColorSelector::new([104, 217, 165]).with_label("Enabled"), // 12: Toggles - Enabled
                ColorSelector::new([135, 135, 148]).with_label("Disabled"), // 13: Toggles - Disabled
                ColorSelector::new([81, 81, 97]).with_label("Background"), // 14: ScrollingList - Background
                ColorSelector::new([81, 81, 97]).with_label("Background"), // 15: Breadcrumb - Background
                ColorSelector::new([81, 81, 97]).with_label("Background"), // 16: Popover - Background
                ColorSelector::new([0x08, 0x08, 0x0c]).with_label("Background"), // 17: Notification - Background
                ColorSelector::new([0x0a, 0x1a, 0x0e]).with_label("Color"), // 18: Surfaces - Window Color
                ColorSelector::new([0, 0, 0]).with_label("Page Color"), // 19: Containers - Page Color
                ColorSelector::new([0, 0, 0]).with_label("Layer Color"), // 20: Containers - Layer Color
                ColorSelector::new_rgba([255, 255, 255, 10]).with_label("Entry Background"), // 21: ScrollingList - Entry Background
                ColorSelector::new_rgba([255, 255, 255, 204]).with_label("Entry Highlight"), // 22: ScrollingList - Entry Highlight
            ],
            paginator_tab_padding_x: 10,
            paginator_tab_padding_y: 14,
            button_padding: 14,
            button_padding_spinbox: Spinbox::new(14, 0, 100, 1).with_label("Button Padding").with_unit("px"),
            button_strip_spacing: 8,
            button_strip_spacing_spinbox: Spinbox::new(8, 0, 100, 1).with_label("Spacing").with_unit("px"),
            section_padding: 8,
            section_padding_spinbox: Spinbox::new(8, 0, 100, 1).with_label("Padding").with_unit("px"),
            plate_padding: 20,
            plate_padding_spinbox: Spinbox::new(20, 0, 100, 1).with_label("Padding").with_unit("px"),
            plate_opacity: 1.0,
            plate_opacity_spinbox: Spinbox::new(100, 0, 100, 5).with_label("Opacity").with_unit("%"),
            plate_corner_radius: 12,
            plate_corner_radius_spinbox: Spinbox::new(12, 0, 50, 1).with_label("Corner Radius").with_unit("px"),
            page_color: [0, 0, 0],
            page_opacity: 1.0,
            page_opacity_spinbox: Spinbox::new(100, 0, 100, 5).with_label("Opacity").with_unit("%"),
            layer_color: [0, 0, 0],
            layer_opacity: 1.0,
            layer_opacity_spinbox: Spinbox::new(100, 0, 100, 5).with_label("Opacity").with_unit("%"),

            page_margin: 20,
            page_margin_spinbox: Spinbox::new(20, 0, 100, 1).with_label("Page Margin").with_unit("px"),
            grid_min_col_width: 260,
            grid_min_col_width_spinbox: Spinbox::new(260, 100, 1000, 10).with_label("Minimum Width").with_unit("px"),
            spinbox_height: 26,
            spinbox_height_spinbox: Spinbox::new(26, 10, 100, 1).with_label("Height").with_unit("px"),
            spinbox_corner_radius: 4,
            spinbox_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Border Radius").with_unit("px"),
            toggle_height: 44,
            toggle_height_spinbox: Spinbox::new(44, 10, 100, 1).with_label("Height").with_unit("px"),
            toggle_corner_radius: 4,
            toggle_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Corner Radius").with_unit("px"),

            color_selector_height: 22,
            color_selector_height_spinbox: Spinbox::new(22, 10, 100, 1).with_label("Height").with_unit("px"),
            color_selector_corner_radius: 4,
            color_selector_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Border Radius").with_unit("px"),
            color_selector_preview_corner_radius: 4,
            color_selector_preview_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Preview Corner Radius").with_unit("px"),
            color_selector_preview_margin: 0,
            color_selector_preview_margin_spinbox: Spinbox::new(0, 0, 20, 1).with_label("Preview Margin").with_unit("px"),
            textbox_height: 44,
            textbox_height_spinbox: Spinbox::new(44, 10, 100, 1).with_label("Height").with_unit("px"),
            textbox_corner_radius: 4,
            textbox_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Border Radius").with_unit("px"),
            slider_height: 28,
            slider_height_spinbox: Spinbox::new(28, 10, 100, 1).with_label("Height").with_unit("px"),
            font_selector_height: 44,
            font_selector_height_spinbox: Spinbox::new(44, 10, 100, 1).with_label("Height").with_unit("px"),
            font_selector_corner_radius: 4,
            font_selector_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Border Radius").with_unit("px"),
            dropdown_height: 44,
            dropdown_height_spinbox: Spinbox::new(44, 10, 100, 1).with_label("Height").with_unit("px"),
            dropdown_corner_radius: 4,
            dropdown_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Radius").with_unit("px"),
            button_corner_radius: 4,
            button_corner_radius_spinbox: Spinbox::new(4, 0, 50, 1).with_label("Radius").with_unit("px"),
            nested_section_label_alignment: 0,
            label_alignment_menu: Dropdown::new(
                vec!["Left".to_string(), "Center".to_string(), "Right".to_string()],
                0,
            ).with_label("Label Alignment"),
            nested_section_label_offset: 0,
            label_offset_spinbox: Spinbox::new(0, -100, 100, 1).with_label("Label Offset").with_unit("px"),
            label_margin: 6,
            label_margin_spinbox: Spinbox::new(6, 0, 100, 1).with_label("Label Margin").with_unit("px"),
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
            sans_box: TextBox::new(String::new()).with_label("Sans-Serif"),
            serif_box: TextBox::new(String::new()).with_label("Serif"),
            mono_box: TextBox::new(String::new()).with_label("Monospace"),
            borders_box: TextBox::new(String::new()).with_label("Active Font"),
            status_box: TextBox::new(String::new()).with_label("Active Font"),
            fuzzel_box: TextBox::new(String::new()).with_label("Active Font"),
            terminal_box: TextBox::new(String::new()).with_label("Active Font"),
            borders_menu: Dropdown::new(Vec::new(), 0),
            status_menu: Dropdown::new(Vec::new(), 0),
            fuzzel_menu: Dropdown::new(Vec::new(), 0),
            terminal_menu: Dropdown::new(Vec::new(), 0),
            borders_size_box: Spinbox::new(14, 6, 72, 1),
            status_size_box: Spinbox::new(14, 6, 72, 1),
            fuzzel_size_box: Spinbox::new(14, 6, 72, 1),
            terminal_size_box: Spinbox::new(14, 6, 72, 1),
            color_selector_font: "monospace".to_string(),
            color_selector_font_selector: FontSelector::new("monospace".to_string()).with_label("Color Label"),
            menubar_font: "Outfit".to_string(),
            menubar_font_selector: FontSelector::new("Outfit".to_string()).with_label("Menu Label"),
            section_label_font: "Outfit".to_string(),
            section_label_font_selector: FontSelector::new("Outfit".to_string()).with_label("Label"),
            nested_section_label_font: "Outfit".to_string(),
            nested_section_label_font_selector: FontSelector::new("Outfit".to_string()).with_label("Label"),
            breadcrumb_font: "Outfit".to_string(),
            breadcrumb_font_selector: FontSelector::new("Outfit".to_string()).with_label("Font"),
            graph_show_grid: true,
            graph_show_grid_toggle: Toggle::new().with_label("Show Grid"),
            graph_snap_enabled: true,
            graph_snap_enabled_toggle: Toggle::new().with_label("Grid Snapping"),
            graph_uniform_background: false,
            graph_uniform_background_toggle: Toggle::new().with_label("Uniform Background"),
            graph_cell_opacity: 0.95,
            graph_cell_opacity_spinbox: Spinbox::new(95, 0, 100, 5).with_label("Cell Opacity").with_unit("%"),
            graph_gap_opacity: 0.95,
            graph_gap_opacity_spinbox: Spinbox::new(95, 0, 100, 5).with_label("Gap Opacity").with_unit("%"),
            graph_gap_width: 35,
            graph_gap_width_spinbox: Spinbox::new(35, 0, 100, 1).with_label("Gap Width").with_unit("px"),
            menubar_opacity: 0.9,
            menubar_opacity_spinbox: Spinbox::new(90, 0, 100, 5).with_label("Opacity").with_unit("%"),
            notification_bg_color: [0x08, 0x08, 0x0c],
            notification_opacity: 0.9,
            notification_opacity_spinbox: Spinbox::new(90, 0, 100, 5).with_label("Opacity").with_unit("%"),
            window_color: [0x0a, 0x1a, 0x0e],
            window_opacity: 0.9,
            window_opacity_spinbox: Spinbox::new(90, 0, 100, 5).with_label("Opacity").with_unit("%"),
            window_corner_radius: 12,
            window_corner_radius_spinbox: Spinbox::new(12, 0, 100, 1).with_label("Corner Radius").with_unit("px"),
            custom_multicontrol: MultiControl::new("custom_parameters".to_string()).with_label("custom_parameters"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InterfaceMessage {
    Windows(WindowsMessage),
    SetDesktopBackground([u8; 3]),
    SetHighColor([u8; 3]),
    SetDisabledColor([u8; 3]),
    SetSeparatorColor([u8; 3]),
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
    SetWindowColor([u8; 3]),
    SetWindowOpacity(f32),
    SetWindowCornerRadius(u16),
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

    SetPageMargin(u16),
    SetGridMinColWidth(u16),
    SetSpinboxHeight(u16),
    SetSpinboxCornerRadius(u16),
    SetToggleHeight(u16),
    SetToggleCornerRadius(u16),

    SetColorSelectorHeight(u16),
    SetColorSelectorCornerRadius(u16),
    SetColorSelectorPreviewCornerRadius(u16),
    SetColorSelectorPreviewMargin(u16),
    SetTextboxHeight(u16),
    SetTextboxCornerRadius(u16),
    SetSliderHeight(u16),
    SetFontSelectorHeight(u16),
    SetFontSelectorCornerRadius(u16),
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
    PickSeparatorColor,
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
    
    let separator = parse_color_from_key(&content, "status_separator_color", [124, 124, 137]);
    
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
    let spinbox_height = parse_u16_from(&content, "spinbox_height", 26);
    let spinbox_corner_radius = parse_u16_from(&content, "spinbox_corner_radius", 4);
    let toggle_height = parse_u16_from(&content, "toggle_height", 44);
    let toggle_corner_radius = parse_u16_from(&content, "toggle_corner_radius", 4);

    let color_selector_height = parse_u16_from(&content, "color_selector_height", 22);
    let color_selector_corner_radius = parse_u16_from(&content, "color_selector_corner_radius", 4);
    let color_selector_preview_corner_radius = parse_u16_from(&content, "color_selector_preview_corner_radius", 4);
    let color_selector_preview_margin = parse_u16_from(&content, "color_selector_preview_margin", 0);
    let textbox_height = parse_u16_from(&content, "textbox_height", 44);
    let textbox_corner_radius = parse_u16_from(&content, "textbox_corner_radius", 4);
    let slider_height = parse_u16_from(&content, "slider_height", 28);
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
    let window_color = parse_surfaces_color(&content, "window_color", [0x0a, 0x1a, 0x0e]);
    let window_opacity = parse_surfaces_opacity(&content);
    let window_corner_radius = parse_surfaces_u16(&content, "window_corner_radius", 12);
    
    InterfaceState {
        windows: read_windows_config(),
        desktop_background_color: bg,
        high_color: border,
        disabled_color: disabled,
        separator_color: separator,
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
        scrollinglist_bg_color: scrollinglist_bg,
        scrollinglist_entry_bg_color: scrollinglist_entry_bg,
        scrollinglist_entry_highlight_color: scrollinglist_entry_highlight,
        breadcrumb_bg_color: breadcrumb_bg,
        popover_bg_color: popover_bg,
        color_selectors: vec![
            ColorSelector::new(page_low).with_label("Low Color").with_font_family(&color_selector_font), // 0: Plate - Low Color
            ColorSelector::new(border).with_label("High Color").with_font_family(&color_selector_font), // 1: Layout - High Color
            ColorSelector::new(visual_guides).with_label("Visual Guides").with_font_family(&color_selector_font), // 2: Layout - Visual Guides
            ColorSelector::new(disabled).with_label("Disabled").with_font_family(&color_selector_font), // 3: Status - Disabled
            ColorSelector::new(separator).with_label("Separators").with_font_family(&color_selector_font), // 4: Status - Separators
            ColorSelector::new(slider_track).with_label("Slider Track").with_font_family(&color_selector_font), // 5: Controls - Slider Track
            ColorSelector::new(color_borders).with_label("Borders").with_font_family(&color_selector_font), // 6: Controls - Borders
            ColorSelector::new(bg).with_label("Low Color").with_font_family(&color_selector_font), // 7: Layout - Low Color
            ColorSelector::new(normal).with_label("Normal").with_font_family(&color_selector_font), // 8: Status - Normal
            ColorSelector::new(paginator_sidebar).with_label("Background").with_font_family(&color_selector_font), // 9: Controls - Paginator Sidebar (now Background)
            ColorSelector::new(primary_highlight).with_label("Primary Highlight").with_font_family(&color_selector_font), // 10: Controls - Primary Highlight
            ColorSelector::new(menubar_tab_label).with_label("Tab Label").with_font_family(&color_selector_font), // 11: Controls - Tab Label
            ColorSelector::new(toggle_enabled).with_label("Enabled").with_font_family(&color_selector_font), // 12: Toggles - Enabled
            ColorSelector::new(toggle_disabled).with_label("Disabled").with_font_family(&color_selector_font), // 13: Toggles - Disabled
            ColorSelector::new(scrollinglist_bg).with_label("Background").with_font_family(&color_selector_font), // 14: ScrollingList - Background
            ColorSelector::new(breadcrumb_bg).with_label("Background").with_font_family(&color_selector_font), // 15: Breadcrumb - Background
            ColorSelector::new(popover_bg).with_label("Background").with_font_family(&color_selector_font), // 16: Popover - Background
            ColorSelector::new(notification_bg_color).with_label("Background").with_font_family(&color_selector_font), // 17: Notification - Background
            ColorSelector::new(window_color).with_label("Color").with_font_family(&color_selector_font), // 18: Surfaces - Window Color
            ColorSelector::new(page_color).with_label("Page Color").with_font_family(&color_selector_font), // 19: Containers - Page Color
            ColorSelector::new(layer_color).with_label("Layer Color").with_font_family(&color_selector_font), // 20: Containers - Layer Color
            ColorSelector::new_rgba(scrollinglist_entry_bg).with_label("Entry Background").with_font_family(&color_selector_font), // 21: ScrollingList - Entry Background
            ColorSelector::new_rgba(scrollinglist_entry_highlight).with_label("Entry Highlight").with_font_family(&color_selector_font), // 22: ScrollingList - Entry Highlight
        ],
        paginator_tab_padding_x,
        paginator_tab_padding_y,
        button_padding,
        button_padding_spinbox: Spinbox::new(button_padding as i32, 0, 100, 1).with_label("Button Padding").with_unit("px"),
        button_strip_spacing,
        button_strip_spacing_spinbox: Spinbox::new(button_strip_spacing as i32, 0, 100, 1).with_label("Spacing").with_unit("px"),
        section_padding,
        section_padding_spinbox: Spinbox::new(section_padding as i32, 0, 100, 1).with_label("Padding").with_unit("px"),
        plate_padding,
        plate_padding_spinbox: Spinbox::new(plate_padding as i32, 0, 100, 1).with_label("Padding").with_unit("px"),
        plate_opacity,
        plate_opacity_spinbox: Spinbox::new((plate_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%"),
        plate_corner_radius,
        plate_corner_radius_spinbox: Spinbox::new(plate_corner_radius as i32, 0, 50, 1).with_label("Corner Radius").with_unit("px"),
        page_color,
        page_opacity,
        page_opacity_spinbox: Spinbox::new((page_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%"),
        layer_color,
        layer_opacity,
        layer_opacity_spinbox: Spinbox::new((layer_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%"),

        page_margin,
        page_margin_spinbox: Spinbox::new(page_margin as i32, 0, 100, 1).with_label("Page Margin").with_unit("px"),
        grid_min_col_width,
        grid_min_col_width_spinbox: Spinbox::new(grid_min_col_width as i32, 100, 1000, 10).with_label("Minimum Width").with_unit("px"),
        spinbox_height,
        spinbox_height_spinbox: Spinbox::new(spinbox_height as i32, 10, 100, 1).with_label("Height").with_unit("px"),
        spinbox_corner_radius,
        spinbox_corner_radius_spinbox: Spinbox::new(spinbox_corner_radius as i32, 0, 50, 1).with_label("Border Radius").with_unit("px"),
        toggle_height,
        toggle_height_spinbox: Spinbox::new(toggle_height as i32, 10, 100, 1).with_label("Height").with_unit("px"),
        toggle_corner_radius,
        toggle_corner_radius_spinbox: Spinbox::new(toggle_corner_radius as i32, 0, 50, 1).with_label("Corner Radius").with_unit("px"),

        color_selector_height,
        color_selector_height_spinbox: Spinbox::new(color_selector_height as i32, 10, 100, 1).with_label("Height").with_unit("px"),
        color_selector_corner_radius,
        color_selector_corner_radius_spinbox: Spinbox::new(color_selector_corner_radius as i32, 0, 50, 1).with_label("Border Radius").with_unit("px"),
        color_selector_preview_corner_radius,
        color_selector_preview_corner_radius_spinbox: Spinbox::new(color_selector_preview_corner_radius as i32, 0, 50, 1).with_label("Preview Corner Radius").with_unit("px"),
        color_selector_preview_margin,
        color_selector_preview_margin_spinbox: Spinbox::new(color_selector_preview_margin as i32, 0, 20, 1).with_label("Preview Margin").with_unit("px"),
        textbox_height,
        textbox_height_spinbox: Spinbox::new(textbox_height as i32, 10, 100, 1).with_label("Height").with_unit("px"),
        textbox_corner_radius,
        textbox_corner_radius_spinbox: Spinbox::new(textbox_corner_radius as i32, 0, 50, 1).with_label("Border Radius").with_unit("px"),
        slider_height,
        slider_height_spinbox: Spinbox::new(slider_height as i32, 10, 100, 1).with_label("Height").with_unit("px"),
        font_selector_height,
        font_selector_height_spinbox: Spinbox::new(font_selector_height as i32, 10, 100, 1).with_label("Height").with_unit("px"),
        font_selector_corner_radius,
        font_selector_corner_radius_spinbox: Spinbox::new(font_selector_corner_radius as i32, 0, 50, 1).with_label("Border Radius").with_unit("px"),
        dropdown_height,
        dropdown_height_spinbox: Spinbox::new(dropdown_height as i32, 10, 100, 1).with_label("Height").with_unit("px"),
        dropdown_corner_radius,
        dropdown_corner_radius_spinbox: Spinbox::new(dropdown_corner_radius as i32, 0, 50, 1).with_label("Radius").with_unit("px"),
        button_corner_radius,
        button_corner_radius_spinbox: Spinbox::new(button_corner_radius as i32, 0, 50, 1).with_label("Radius").with_unit("px"),
        nested_section_label_alignment,
        label_alignment_menu: Dropdown::new(
            vec!["Left".to_string(), "Center".to_string(), "Right".to_string()],
            nested_section_label_alignment as usize,
        ).with_label("Label Alignment"),
        nested_section_label_offset,
        label_offset_spinbox: Spinbox::new(nested_section_label_offset as i32, -100, 100, 1).with_label("Label Offset").with_unit("px"),
        label_margin,
        label_margin_spinbox: Spinbox::new(label_margin as i32, 0, 100, 1).with_label("Label Margin").with_unit("px"),
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
        borders_size_box: Spinbox::new(11, 6, 72, 1),
        status_size_box: Spinbox::new(11, 6, 72, 1),
        fuzzel_size_box: Spinbox::new(14, 6, 72, 1),
        terminal_size_box: Spinbox::new(12, 6, 72, 1),
        color_selector_font: color_selector_font.clone(),
        color_selector_font_selector: FontSelector::new(color_selector_font.clone()).with_label("Value"),
        menubar_font: menubar_font.clone(),
        menubar_font_selector: FontSelector::new(menubar_font.clone()).with_label("Font"),
        section_label_font: section_label_font.clone(),
        section_label_font_selector: FontSelector::new(section_label_font.clone()).with_label("Label"),
        nested_section_label_font: nested_section_label_font.clone(),
        nested_section_label_font_selector: FontSelector::new(nested_section_label_font.clone()).with_label("Label"),
        breadcrumb_font: breadcrumb_font.clone(),
        breadcrumb_font_selector: FontSelector::new(breadcrumb_font.clone()).with_label("Font"),
        graph_show_grid,
        graph_show_grid_toggle: Toggle::new().with_label("Show Grid"),
        graph_snap_enabled,
        graph_snap_enabled_toggle: Toggle::new().with_label("Grid Snapping"),
        graph_uniform_background,
        graph_uniform_background_toggle: Toggle::new().with_label("Uniform Background"),
        graph_cell_opacity,
        graph_cell_opacity_spinbox: Spinbox::new((graph_cell_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Cell Opacity").with_unit("%"),
        graph_gap_opacity,
        graph_gap_opacity_spinbox: Spinbox::new((graph_gap_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Gap Opacity").with_unit("%"),
        graph_gap_width,
        graph_gap_width_spinbox: Spinbox::new(graph_gap_width as i32, 0, 100, 1).with_label("Gap Width").with_unit("px"),
        menubar_opacity,
        menubar_opacity_spinbox: Spinbox::new((menubar_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%"),
        notification_bg_color,
        notification_opacity,
        notification_opacity_spinbox: Spinbox::new((notification_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%"),
        window_color,
        window_opacity,
        window_opacity_spinbox: Spinbox::new((window_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%"),
        window_corner_radius,
        window_corner_radius_spinbox: Spinbox::new(window_corner_radius as i32, 0, 100, 1).with_label("Corner Radius").with_unit("px"),
        custom_multicontrol: MultiControl::new("custom_parameters".to_string()).with_label("custom_parameters"),
    }
}

pub fn read_windows_config() -> WindowsState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let fs = parse_u16_from(&content, "fullscreen_border_width", 0);
    let ca = parse_u16_from(&content, "cascade_border_width", 6);
    let g = parse_u16_from(&content, "grid_border_width", 6);
    let fl = parse_u16_from(&content, "floating_border_width", 6);
    let co = parse_u16_from(&content, "cascade_offset", 20);
    let gl = parse_u16_from(&content, "gap_left", 48);
    let gt = parse_u16_from(&content, "gap_top", 48);
    let gg = parse_u16_from(&content, "grid_gap", 6);
    let sh = parse_u16_from(&content, "bar_height", 24);
    let td = parse_u16_from(&content, "transition_duration", 300);

    let tag_modes = parse_tag_layouts_from_config(&content);
    let dropdown_options = vec![
        "Cascade".to_string(),
        "Grid".to_string(),
        "Fullscreen".to_string(),
        "Floating".to_string(),
        "Popup".to_string(),
    ];
    let tag_layout_menus = (1..=4).map(|i| {
        let mode_str = &tag_modes[i - 1];
        let idx = dropdown_options.iter().position(|opt| opt.to_lowercase() == mode_str.to_lowercase()).unwrap_or(0);
        Dropdown::new(dropdown_options.clone(), idx).with_label(&format!("Tag {}", i))
    }).collect();

    let side_panel_behavior = parse_string_from(&content, "side_panel_behavior", "inline");
    let side_panel_behavior_idx = if side_panel_behavior == "above" { 0 } else { 1 };
    let side_panel_behavior_menu = Dropdown::new(
        vec!["Above".to_string(), "Inline".to_string()],
        side_panel_behavior_idx,
    ).with_label("Behavior");

    let side_panel_position = parse_string_from(&content, "side_panel_position", "left");
    let side_panel_position_idx = if side_panel_position == "right" { 1 } else { 0 };
    let side_panel_position_menu = Dropdown::new(
        vec!["Left".to_string(), "Right".to_string()],
        side_panel_position_idx,
    ).with_label("Position");

    let spw = parse_u16_from(&content, "side_panel_width", 360);
    let spbg = parse_u16_from(&content, "side_panel_border_gap", 0);
    let spbo = parse_u16_from(&content, "side_panel_border_opacity", 100);

    let transparency_enabled = parse_bool_from(&content, "window_opacity", true);

    let window_blur = parse_bool_from(&content, "window_blur", false);
    let border_blur = parse_bool_from(&content, "border_blur", false);
    let blur_enabled = window_blur || border_blur;

    WindowsState {
        fullscreen_border_width: fs,
        cascade_border_width: ca,
        grid_border_width: g,
        floating_border_width: fl,
        cascade_offset: co,
        edge_gap: gl,
        top_gap: gt,
        grid_gap: gg,
        status_height: sh,
        transition_duration: td,
        spinboxes: make_spinboxes(fs, ca, g, fl),
        cascade_offset_spinbox: Spinbox::new(co as i32, 0, 200, 1),
        edge_gap_spinbox: Spinbox::new(gl as i32, 0, 200, 1),
        top_gap_spinbox: Spinbox::new(gt as i32, 0, 200, 1),
        grid_gap_spinbox: Spinbox::new(gg as i32, 0, 200, 1),
        status_height_spinbox: Spinbox::new(sh as i32, 0, 100, 1),
        transition_duration_spinbox: Spinbox::new(td as i32, 0, 2000, 50),
        tag_layout_menus,
        side_panel_behavior_menu,
        side_panel_position_menu,
        side_panel_width: spw,
        side_panel_width_spinbox: Spinbox::new(spw as i32, 0, 2000, 10),
        side_panel_border_gap: spbg,
        side_panel_border_gap_spinbox: Spinbox::new(spbg as i32, 0, 500, 1),
        side_panel_border_opacity: spbo,
        side_panel_border_opacity_spinbox: Spinbox::new(spbo as i32, 0, 100, 5),
        transparency_enabled,
        transparency_toggle: Toggle::new().with_label("Transparency"),
        blur_enabled,
        blur_toggle: Toggle::new().with_label("Blur"),
    }
}

fn parse_tag_layouts_from_config(content: &str) -> Vec<String> {
    let val = parse_json(content);
    let mut modes = vec!["cascade".to_string(); 4];
    if let Some(arr) = val.get("tag_layout").and_then(|t| t.as_array()) {
        for item in arr {
            if let (Some(tag), Some(mode)) = (
                item.get("tag").and_then(|t| t.as_u64()),
                item.get("mode").and_then(|m| m.as_str())
            ) {
                if tag >= 1 && tag <= 4 {
                    modes[tag as usize - 1] = mode.to_string();
                }
            }
        }
    }
    modes
}

fn write_tag_layout(tag_num: usize, mode_str: &str) -> bool {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let mut val = parse_json(&content);
    
    let mut found = false;
    if let Some(arr) = val.get_mut("tag_layout").and_then(|t| t.as_array_mut()) {
        for item in arr.iter_mut() {
            if item.get("tag").and_then(|t| t.as_u64()) == Some(tag_num as u64) {
                if let Some(obj) = item.as_object_mut() {
                    obj.insert("mode".to_string(), serde_json::json!(mode_str.to_lowercase()));
                    found = true;
                    break;
                }
            }
        }
        if !found {
            arr.push(serde_json::json!({
                "tag": tag_num,
                "mode": mode_str.to_lowercase()
            }));
        }
    } else {
        let arr = vec![serde_json::json!({
            "tag": tag_num,
            "mode": mode_str.to_lowercase()
        })];
        if let Some(obj) = val.as_object_mut() {
            obj.insert("tag_layout".to_string(), serde_json::Value::Array(arr));
        }
    }
    
    if let Ok(updated_str) = serde_json::to_string_pretty(&val) {
        return fs::write(CONFIG_PATH, updated_str).is_ok();
    }
    false
}

fn apply_all_widths(s: &WindowsState) {
    let w = |k: &str, v: u16| { write_config_value(k, &v.to_string()); send_ipc_command(&format!("layout {} {}", k, v)); };
    w("fullscreen_border_width", s.fullscreen_border_width);
    w("cascade_border_width", s.cascade_border_width);
    w("grid_border_width", s.grid_border_width);
    w("floating_border_width", s.floating_border_width);
    w("cascade_offset", s.cascade_offset);
    w("gap_left", s.edge_gap);
    w("gap_right", s.edge_gap);
    w("gap_bottom", s.edge_gap);
    w("gap_top", s.top_gap);
    w("transition_duration", s.transition_duration);
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PreviewWindow {
    app_id: String,
    title: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    tags: u32,
    _minimized: bool,
    has_parent: bool,
    layout_mode: String,
}

#[allow(dead_code)]
struct LayoutStatusInfo {
    active_tags: u32,
    focused_tags: u32,
    _num_tags: u32,
    windows: Vec<PreviewWindow>,
    focused_title: String,
    focused_layout_mode: String,
}

fn read_current_layout_status() -> LayoutStatusInfo {
    let mut active_tags = 1;
    let mut focused_tags = 1;
    let mut _num_tags = 4;

    let display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string());
    
    let tags_path = format!("/tmp/cce-tags-{}", display);
    let tags_fallback = "/tmp/cce-tags".to_string();
    let tags_content = fs::read_to_string(&tags_path)
        .or_else(|_| fs::read_to_string(&tags_fallback))
        .unwrap_or_default();

    if let Some(line) = tags_content.lines().next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            active_tags = parts[0].parse().unwrap_or(1);
            focused_tags = parts[1].parse().unwrap_or(1);
            _num_tags = parts[2].parse().unwrap_or(4);
        }
    }

    let title_path = format!("/tmp/cce-title-{}", display);
    let title_fallback = "/tmp/cce-title".to_string();
    let focused_title = fs::read_to_string(&title_path)
        .or_else(|_| fs::read_to_string(&title_fallback))
        .unwrap_or_default()
        .trim()
        .to_string();

    let layout_path = format!("/tmp/cce-layout-{}", display);
    let layout_fallback = "/tmp/cce-layout".to_string();
    let focused_layout_mode = fs::read_to_string(&layout_path)
        .or_else(|_| fs::read_to_string(&layout_fallback))
        .unwrap_or_else(|_| "Cascade".to_string())
        .trim()
        .to_string();

    let windows_path = format!("/tmp/cce-windows-{}", display);
    let windows_fallback = "/tmp/cce-windows".to_string();
    let windows_content = fs::read_to_string(&windows_path)
        .or_else(|_| fs::read_to_string(&windows_fallback))
        .unwrap_or_default();

    let mut windows = Vec::new();
    for line in windows_content.lines() {
        if !line.starts_with("window ") { continue; }
        
        let mut app_id = String::new();
        let mut title = String::new();
        let mut x = 0.0;
        let mut y = 0.0;
        let mut w = 0.0;
        let mut h = 0.0;
        let mut tags = 0;
        let mut minimized = false;
        let mut has_parent = false;
        let mut layout_mode = "Cascade".to_string();
        
        let parts = line.strip_prefix("window ").unwrap_or(line);
        
        let get_val = |p: &str, k: &str| -> Option<String> {
            if let Some(idx) = p.find(k) {
                let start = idx + k.len();
                let mut end = p.len();
                let next_keys = [
                    " app_id=", " title=", " mode=", " decoration=", " presentation=",
                    " tags=", " x=", " y=", " w=", " h=", " has_parent=", " minimized="
                ];
                for nk in next_keys {
                    if nk != k {
                        if let Some(nidx) = p[start..].find(nk) {
                            end = end.min(start + nidx);
                        }
                    }
                }
                Some(p[start..end].trim().to_string())
            } else {
                None
            }
        };

        if let Some(val) = get_val(parts, "app_id=") { app_id = val; }
        if let Some(val) = get_val(parts, "title=") { title = val; }
        if let Some(val) = get_val(parts, "x=") { x = val.parse().unwrap_or(0.0); }
        if let Some(val) = get_val(parts, "y=") { y = val.parse().unwrap_or(0.0); }
        if let Some(val) = get_val(parts, "w=") { w = val.parse().unwrap_or(0.0); }
        if let Some(val) = get_val(parts, "h=") { h = val.parse().unwrap_or(0.0); }
        if let Some(val) = get_val(parts, "tags=") { tags = val.parse().unwrap_or(0); }
        if let Some(val) = get_val(parts, "minimized=") { minimized = val == "true"; }
        if let Some(val) = get_val(parts, "has_parent=") { has_parent = val == "true"; }
        if let Some(val) = get_val(parts, "mode=") { layout_mode = val; }

        windows.push(PreviewWindow {
            app_id,
            title,
            x,
            y,
            w,
            h,
            tags,
            _minimized: minimized,
            has_parent,
            layout_mode,
        });
    }

    LayoutStatusInfo {
        active_tags,
        focused_tags,
        _num_tags,
        windows,
        focused_title,
        focused_layout_mode,
    }
}

fn set_width(state: &mut WindowsState, param: WidthParam, val: u16) {
    let val = val.min(100);
    match param {
        WidthParam::Fullscreen => state.fullscreen_border_width = val,
        WidthParam::Cascade => state.cascade_border_width = val,
        WidthParam::Grid => state.grid_border_width = val,
        WidthParam::Floating => state.floating_border_width = val,
    }
    state.spinboxes[param_idx(param)].value = val as i32;
    apply_all_widths(state);
}

fn param_idx(p: WidthParam) -> usize {
    match p {
        WidthParam::Fullscreen => 0,
        WidthParam::Cascade => 1,
        WidthParam::Grid => 2,
        WidthParam::Floating => 3,
    }
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
    write_config_value_path(CONFIG_PATH, key, value)
}

pub fn get_links() -> Vec<(String, String)> {
    get_links_path(LINKS_PATH)
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

    let set_val = |val_obj: &mut serde_json::Value, k: &str, val_str: &str| {
        let j_val = if let Ok(parsed_val) = serde_json::from_str::<serde_json::Value>(val_str) {
            parsed_val
        } else {
            serde_json::json!(val_str)
        };
        
        let mut updated = false;
        if let Some(obj) = val_obj.as_object_mut() {
            for (_sec_name, sec_val) in obj.iter_mut() {
                if let Some(sec_obj) = sec_val.as_object_mut() {
                    if sec_obj.contains_key(k) {
                        sec_obj.insert(k.to_string(), j_val.clone());
                        updated = true;
                        break;
                    }
                }
            }
            if !updated {
                if let Some(layout_obj) = obj.get_mut("layout").and_then(|l| l.as_object_mut()) {
                    layout_obj.insert(k.to_string(), j_val);
                }
            }
        }
    };

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
        set_val(&mut val, mapped_k, value);
    }

    if let Ok(updated_str) = serde_json::to_string_pretty(&val) {
        if fs::write(path, updated_str).is_ok() {
            return true;
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
}

fn apply_desktop_background(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_surfaces_config_value("desktop_background", &hex);
    send_ipc_command(&format!("layout desktop_background #{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
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
    let _ = std::process::Command::new("pkill")
        .args(["-f", "cce-status-interface"])
        .status();
    std::thread::sleep(std::time::Duration::from_millis(150));
    send_ipc_command("spawn cce-status-interface");
}

fn apply_separator_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("status_separator_color", &hex);
    send_ipc_command(&format!("layout status_separator_color #{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
    status_interface_reload();
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

fn apply_window_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_surfaces_config_value("window_color", &hex);
    send_ipc_command("reload");
}

fn apply_window_opacity(opacity: f32) {
    write_surfaces_config_value("window_opacity", &format!("{:.2}", opacity));
    send_ipc_command("reload");
}

fn apply_window_corner_radius(radius: u16) {
    write_surfaces_config_value("window_corner_radius", &radius.to_string());
    send_ipc_command("reload");
    cce_ui::color::set_window_corner_radius(radius as f32);
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
    let _ = std::process::Command::new("pkill")
        .args(["-f", "cce-status-interface"])
        .spawn();
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

    let mut borders_box = TextBox::new(borders.clone()).with_label("Window Borders");
    borders_box.disabled = borders_idx != 3;

    let mut status_box = TextBox::new(status.clone()).with_label("Status Interface");
    status_box.disabled = status_idx != 3;

    let mut fuzzel_box = TextBox::new(fuzzel_font.clone()).with_label("Fuzzel");
    fuzzel_box.disabled = fuzzel_idx != 3;

    let mut terminal_box = TextBox::new(term.clone()).with_label("Terminal");
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
    state.sans_box = TextBox::new(sans).with_label("Sans-Serif");
    state.serif_box = TextBox::new(serif).with_label("Serif");
    state.mono_box = TextBox::new(mono).with_label("Monospace");
    state.borders_box = borders_box;
    state.status_box = status_box;
    state.fuzzel_box = fuzzel_box;
    state.terminal_box = terminal_box;
    state.borders_menu = Dropdown::new(menu_options.clone(), borders_idx);
    state.status_menu = Dropdown::new(menu_options.clone(), status_idx);
    state.fuzzel_menu = Dropdown::new(menu_options.clone(), fuzzel_idx);
    state.terminal_menu = Dropdown::new(menu_options, terminal_idx);
    state.borders_size_box = Spinbox::new(borders_size as i32, 6, 72, 1);
    state.status_size_box = Spinbox::new(status_size as i32, 6, 72, 1);
    state.fuzzel_size_box = Spinbox::new(fuzzel_size as i32, 6, 72, 1);
    state.terminal_size_box = Spinbox::new(terminal_size as i32, 6, 72, 1);
    state
}

pub fn view(state: &mut InterfaceState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 260.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(10);


    // 1. Custom Parameters Section
    builder.add_section_with_width(&mut final_pc, cw, "Custom Parameters", false, |sec| {
        sec.spacing(8.0);
        let h = state.custom_multicontrol.preferred_height().unwrap_or(100.0);
        sec.widget_full(&mut state.custom_multicontrol, h, ctx);
        sec.spacing(8.0);
    });

    // 2. Layout Section
    builder.add_section_with_width(&mut final_pc, cw, "Layout", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[1].color = state.high_color;
        sec.widget_full(&mut state.color_selectors[1], 40.0, ctx);
        sec.spacing(8.0);
        state.color_selectors[2].color = state.visual_guides_color;
        sec.widget_full(&mut state.color_selectors[2], 40.0, ctx);
        sec.spacing(12.0);

        // Grid Layout child section
        sec.add_section("Adaptive Grid", false, |subsec| {
            subsec.spacing(8.0);
            state.grid_min_col_width_spinbox.value = state.grid_min_col_width as i32;
            subsec.widget_full(&mut state.grid_min_col_width_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);
    });


    // 3. Status Section
    builder.add_section_with_width(&mut final_pc, cw, "Status", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[8].color = state.normal_color;
        sec.widget_full(&mut state.color_selectors[8], 40.0, ctx);
        sec.spacing(8.0);
        state.color_selectors[3].color = state.disabled_color;
        sec.widget_full(&mut state.color_selectors[3], 40.0, ctx);
        sec.spacing(8.0);
        state.color_selectors[4].color = state.separator_color;
        sec.widget_full(&mut state.color_selectors[4], 40.0, ctx);
        sec.spacing(8.0);
    });

    // 4. Controls Section
    builder.add_section_with_width(&mut final_pc, cw, "Controls", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[6].color = state.color_borders_color;
        sec.widget_full(&mut state.color_selectors[6], 40.0, ctx);
        sec.spacing(12.0);

        // Slider Section
        sec.add_section("Slider", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[5].color = state.slider_track_color;
            subsec.widget_full(&mut state.color_selectors[5], 40.0, ctx);
            subsec.spacing(8.0);
            state.slider_height_spinbox.value = state.slider_height as i32;
            subsec.widget_full(&mut state.slider_height_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);


        // MenuBar Section
        sec.add_section("MenuBar", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[9].color = state.paginator_sidebar_color;
            subsec.widget_full(&mut state.color_selectors[9], 40.0, ctx);
            subsec.spacing(8.0);
            state.color_selectors[11].color = state.menubar_tab_label_color;
            subsec.widget_full(&mut state.color_selectors[11], 40.0, ctx);
            state.menubar_font_selector.font_family = state.menubar_font.clone();
            subsec.widget_full(&mut state.menubar_font_selector, 44.0, ctx);
            subsec.spacing(8.0);
            state.menubar_opacity_spinbox.value = (state.menubar_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.menubar_opacity_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Toggle Section
        sec.add_section("Toggle", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[12].color = state.toggle_enabled_color;
            subsec.widget_full(&mut state.color_selectors[12], 40.0, ctx);
            subsec.spacing(8.0);
            state.color_selectors[13].color = state.toggle_disabled_color;
            subsec.widget_full(&mut state.color_selectors[13], 40.0, ctx);
            subsec.spacing(8.0);
            state.toggle_height_spinbox.value = state.toggle_height as i32;
            subsec.widget_full(&mut state.toggle_height_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.toggle_corner_radius_spinbox.value = state.toggle_corner_radius as i32;
            subsec.widget_full(&mut state.toggle_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });

        // Breadcrumb Section
        sec.add_section("Breadcrumb", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[15].color = state.breadcrumb_bg_color;
            subsec.widget_full(&mut state.color_selectors[15], 40.0, ctx);
            subsec.spacing(8.0);
            state.breadcrumb_font_selector.font_family = state.breadcrumb_font.clone();
            subsec.widget_full(&mut state.breadcrumb_font_selector, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);


        // Spinbox Section
        sec.add_section("Spinbox", false, |subsec| {
            subsec.spacing(8.0);
            state.spinbox_height_spinbox.value = state.spinbox_height as i32;
            subsec.widget_full(&mut state.spinbox_height_spinbox, 44.0, ctx);
            subsec.spacing(8.0);

            state.spinbox_corner_radius_spinbox.value = state.spinbox_corner_radius as i32;
            subsec.widget_full(&mut state.spinbox_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // ColorSelector Section
        sec.add_section("ColorSelector", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selector_height_spinbox.value = state.color_selector_height as i32;
            subsec.widget_full(&mut state.color_selector_height_spinbox, 44.0, ctx);
            subsec.spacing(8.0);

            state.color_selector_preview_corner_radius_spinbox.value = state.color_selector_preview_corner_radius as i32;
            subsec.widget_full(&mut state.color_selector_preview_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(12.0);

            state.color_selector_preview_margin_spinbox.value = state.color_selector_preview_margin as i32;
            subsec.widget_full(&mut state.color_selector_preview_margin_spinbox, 44.0, ctx);
            subsec.spacing(12.0);

            state.color_selector_corner_radius_spinbox.value = state.color_selector_corner_radius as i32;
            subsec.widget_full(&mut state.color_selector_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(8.0);

            state.color_selector_font_selector.font_family = state.color_selector_font.clone();
            subsec.widget_full(&mut state.color_selector_font_selector, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Textbox Section
        sec.add_section("Textbox", false, |subsec| {
            subsec.spacing(8.0);
            state.textbox_height_spinbox.value = state.textbox_height as i32;
            subsec.widget_full(&mut state.textbox_height_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.textbox_corner_radius_spinbox.value = state.textbox_corner_radius as i32;
            subsec.widget_full(&mut state.textbox_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // FontSelector Section
        sec.add_section("FontSelector", false, |subsec| {
            subsec.spacing(8.0);
            state.font_selector_height_spinbox.value = state.font_selector_height as i32;
            subsec.widget_full(&mut state.font_selector_height_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.font_selector_corner_radius_spinbox.value = state.font_selector_corner_radius as i32;
            subsec.widget_full(&mut state.font_selector_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Dropdown Section
        sec.add_section("Dropdown", false, |subsec| {
            subsec.spacing(8.0);
            state.dropdown_height_spinbox.value = state.dropdown_height as i32;
            subsec.widget_full(&mut state.dropdown_height_spinbox, 44.0, ctx);
            subsec.spacing(4.0);
            state.dropdown_corner_radius_spinbox.value = state.dropdown_corner_radius as i32;
            subsec.widget_full(&mut state.dropdown_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Button Section
        sec.add_section("Button", false, |subsec| {
            subsec.spacing(8.0);
            state.button_corner_radius_spinbox.value = state.button_corner_radius as i32;
            subsec.widget_full(&mut state.button_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // ButtonStrip Section
        sec.add_section("ButtonStrip", false, |subsec| {
            subsec.spacing(8.0);
            state.button_padding_spinbox.value = state.button_padding as i32;
            subsec.widget_full(&mut state.button_padding_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.button_strip_spacing_spinbox.value = state.button_strip_spacing as i32;
            subsec.widget_full(&mut state.button_strip_spacing_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Labels Section
        sec.add_section("Labels", false, |subsec| {
            subsec.spacing(8.0);
            state.label_margin_spinbox.value = state.label_margin as i32;
            subsec.widget_full(&mut state.label_margin_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(8.0);
    });

    // 5. Indicators Section
    builder.add_section_with_width(&mut final_pc, cw, "Indicators", false, |sec| {
        sec.spacing(8.0);
        sec.add_section("Primary Highlight", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[10].color = state.primary_highlight_color;
            subsec.widget_full(&mut state.color_selectors[10], 40.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(8.0);
    });

    // 5. Notification Section
    builder.add_section_with_width(&mut final_pc, cw, "Notification", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[17].color = state.notification_bg_color;
        sec.widget_full(&mut state.color_selectors[17], 40.0, ctx);
        sec.spacing(8.0);
        state.notification_opacity_spinbox.value = (state.notification_opacity * 100.0).round() as i32;
        sec.widget_full(&mut state.notification_opacity_spinbox, 44.0, ctx);
        sec.spacing(8.0);
    });

    // 6. Surfaces Section
    builder.add_section_with_width(&mut final_pc, cw, "Surfaces", sec_focused.get(6).copied().unwrap_or(false), |sec| {
        sec.spacing(12.0);
        sec.add_section("Window", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[18].color = state.window_color;
            subsec.widget_full(&mut state.color_selectors[18], 40.0, ctx);
            subsec.spacing(8.0);
            state.window_opacity_spinbox.value = (state.window_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.window_opacity_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.window_corner_radius_spinbox.value = state.window_corner_radius as i32;
            subsec.widget_full(&mut state.window_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Plate child section
        sec.add_section("Plate", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[0].color = state.page_low_color;
            subsec.widget_full(&mut state.color_selectors[0], 40.0, ctx);
            subsec.spacing(8.0);
            state.plate_padding_spinbox.value = state.plate_padding as i32;
            subsec.widget_full(&mut state.plate_padding_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.plate_opacity_spinbox.value = (state.plate_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.plate_opacity_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.plate_corner_radius_spinbox.value = state.plate_corner_radius as i32;
            subsec.widget_full(&mut state.plate_corner_radius_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Popover Section
        sec.add_section("Popover", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[16].color = state.popover_bg_color;
            subsec.widget_full(&mut state.color_selectors[16], 40.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Desktop Background Section
        sec.add_section("Desktop Background", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[7].color = state.desktop_background_color;
            subsec.widget_full(&mut state.color_selectors[7], 40.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);
    });


    let widget_h = 26.0;
    const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

    // 7. Fonts Section
    builder.add_section_with_width(&mut final_pc, cw, "Fonts", sec_focused.get(7).copied().unwrap_or(false), |sec| {
        sec.spacing(8.0);

        // System Fonts Section
        sec.add_section("System Fonts", false, |subsec| {
            subsec.spacing(8.0);

            if !state.typeface_loaded {
                subsec.text("Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
                subsec.spacing(18.0);
            } else {
                // Sans-Serif
                subsec.widget_full(&mut state.sans_box, 44.0, ctx);
                subsec.spacing(12.0);

                // Serif
                subsec.widget_full(&mut state.serif_box, 44.0, ctx);
                subsec.spacing(12.0);

                // Monospace
                subsec.widget_full(&mut state.mono_box, 44.0, ctx);
                subsec.spacing(8.0);
            }
        });
        sec.spacing(12.0);

        // Program Fonts Section
        sec.add_section("Program Fonts", false, |subsec| {
            subsec.spacing(8.0);

            if !state.typeface_loaded {
                subsec.text("Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
                subsec.spacing(18.0);
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
                subsec.spacing(widget_h);
                subsec.widget_full(&mut state.borders_box, 44.0, ctx);
                subsec.spacing(16.0);

                // Status Interface
                let start_y = subsec.ay();
                let cols = subsec.row_layout(2, 10.0);
                if cols.len() == 2 {
                    state.status_menu.set_row_rect(cols[0].0, cols[0].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.status_menu, cols[0].0, start_y, cols[0].1, widget_h, ctx);
                    state.status_size_box.set_row_rect(cols[1].0, cols[1].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.status_size_box, cols[1].0, start_y, cols[1].1, widget_h, ctx);
                }
                subsec.spacing(widget_h);
                subsec.widget_full(&mut state.status_box, 44.0, ctx);
                subsec.spacing(16.0);

                // Fuzzel
                let start_y = subsec.ay();
                let cols = subsec.row_layout(2, 10.0);
                if cols.len() == 2 {
                    state.fuzzel_menu.set_row_rect(cols[0].0, cols[0].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.fuzzel_menu, cols[0].0, start_y, cols[0].1, widget_h, ctx);
                    state.fuzzel_size_box.set_row_rect(cols[1].0, cols[1].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.fuzzel_size_box, cols[1].0, start_y, cols[1].1, widget_h, ctx);
                }
                subsec.spacing(widget_h);
                subsec.widget_full(&mut state.fuzzel_box, 44.0, ctx);
                subsec.spacing(16.0);

                // Terminal
                let start_y = subsec.ay();
                let cols = subsec.row_layout(2, 10.0);
                if cols.len() == 2 {
                    state.terminal_menu.set_row_rect(cols[0].0, cols[0].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.terminal_menu, cols[0].0, start_y, cols[0].1, widget_h, ctx);
                    state.terminal_size_box.set_row_rect(cols[1].0, cols[1].1);
                    cce_ui::layout::render_widget(subsec.pc, &mut state.terminal_size_box, cols[1].0, start_y, cols[1].1, widget_h, ctx);
                }
                subsec.spacing(widget_h);
                subsec.widget_full(&mut state.terminal_box, 44.0, ctx);
                subsec.spacing(8.0);
            }
        });
        sec.spacing(8.0);
    });

    builder.add_section_with_width(&mut final_pc, cw, "Containers", sec_focused.get(8).copied().unwrap_or(false), |sec| {
        sec.spacing(12.0);

        // Page child section
        sec.add_section("Page", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[19].color = state.page_color;
            subsec.widget_full(&mut state.color_selectors[19], 40.0, ctx);
            subsec.spacing(8.0);
            state.page_opacity_spinbox.value = (state.page_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.page_opacity_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Layer child section
        sec.add_section("Layer", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[20].color = state.layer_color;
            subsec.widget_full(&mut state.color_selectors[20], 40.0, ctx);
            subsec.spacing(8.0);
            state.layer_opacity_spinbox.value = (state.layer_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.layer_opacity_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Section child section
        sec.add_section("Section", false, |subsec| {
            subsec.spacing(8.0);
            state.section_padding_spinbox.value = state.section_padding as i32;
            subsec.widget_full(&mut state.section_padding_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.page_margin_spinbox.value = state.page_margin as i32;
            subsec.widget_full(&mut state.page_margin_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.section_label_font_selector.font_family = state.section_label_font.clone();
            subsec.widget_full(&mut state.section_label_font_selector, 44.0, ctx);
            subsec.spacing(12.0);

            // Nested Section child section
            subsec.add_section("Nested Section", false, |subsubsec| {
                subsubsec.spacing(8.0);
                subsubsec.widget_full(&mut state.label_alignment_menu, 44.0, ctx);
                subsubsec.spacing(8.0);
                state.label_offset_spinbox.value = state.nested_section_label_offset as i32;
                subsubsec.widget_full(&mut state.label_offset_spinbox, 44.0, ctx);
                subsubsec.spacing(8.0);
                state.nested_section_label_font_selector.font_family = state.nested_section_label_font.clone();
                subsubsec.widget_full(&mut state.nested_section_label_font_selector, 44.0, ctx);
                subsubsec.spacing(8.0);
            });
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // ScrollingList Section
        sec.add_section("ScrollingList", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[14].color = state.scrollinglist_bg_color;
            subsec.widget_full(&mut state.color_selectors[14], 40.0, ctx);
            subsec.spacing(8.0);

            let bg_rgba = state.scrollinglist_entry_bg_color;
            state.color_selectors[21].color = [bg_rgba[0], bg_rgba[1], bg_rgba[2]];
            state.color_selectors[21].alpha = bg_rgba[3];
            subsec.widget_full(&mut state.color_selectors[21], 40.0, ctx);
            subsec.spacing(8.0);

            let highlight_rgba = state.scrollinglist_entry_highlight_color;
            state.color_selectors[22].color = [highlight_rgba[0], highlight_rgba[1], highlight_rgba[2]];
            state.color_selectors[22].alpha = highlight_rgba[3];
            subsec.widget_full(&mut state.color_selectors[22], 40.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Graph child section
        sec.add_section("Graph", false, |subsec| {
            subsec.spacing(8.0);
            state.graph_show_grid_toggle.set_toggled(state.graph_show_grid);
            subsec.widget_full(&mut state.graph_show_grid_toggle, state.toggle_height as f32, ctx);
            subsec.spacing(8.0);
            state.graph_snap_enabled_toggle.set_toggled(state.graph_snap_enabled);
            subsec.widget_full(&mut state.graph_snap_enabled_toggle, state.toggle_height as f32, ctx);
            subsec.spacing(8.0);
            state.graph_uniform_background_toggle.set_toggled(state.graph_uniform_background);
            subsec.widget_full(&mut state.graph_uniform_background_toggle, state.toggle_height as f32, ctx);
            subsec.spacing(8.0);
            state.graph_cell_opacity_spinbox.value = (state.graph_cell_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.graph_cell_opacity_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.graph_gap_opacity_spinbox.value = (state.graph_gap_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.graph_gap_opacity_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.graph_gap_width_spinbox.value = state.graph_gap_width as i32;
            subsec.widget_full(&mut state.graph_gap_width_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);
    });

    // 10. Windows Section
    builder.add_section_with_width(&mut final_pc, cw, "Windows", sec_focused.get(9).copied().unwrap_or(false), |sec| {
        sec.spacing(8.0);
        
        // 1. Current Layout Preview
        sec.add_section("Current Layout", false, |sec_cl| {
            sec_cl.spacing(8.0);
            
            let info = read_current_layout_status();
            
            let card_w = (sec_w - 24.0) / 2.0;
            let card_h = 135.0;
            let rx = sec_cl.left;
            
            for tag_idx in 0..4 {
                let col = tag_idx % 2;
                let row = tag_idx / 2;
                let tx = rx + 8.0 + col as f32 * (card_w + 8.0);
                let ty = sec_cl.ay() + row as f32 * (card_h + 8.0);
                
                let is_active = (info.active_tags & (1 << tag_idx)) != 0;
                let layout_idx = state.windows.tag_layout_menus.get(tag_idx).map(|m| m.selected).unwrap_or(0);
                let mode = match layout_idx {
                    1 => PreviewLayoutMode::Cascade,
                    2 => PreviewLayoutMode::Stack,
                    3 => PreviewLayoutMode::Grid,
                    4 => PreviewLayoutMode::LeftTiled,
                    5 => PreviewLayoutMode::RightTiled,
                    6 => PreviewLayoutMode::Equal,
                    7 => PreviewLayoutMode::Spiral,
                    8 => PreviewLayoutMode::Floating,
                    _ => PreviewLayoutMode::Fullscreen,
                };

                let mut preview = LayoutPreview::new(mode)
                    .with_active(is_active)
                    .with_label(&format!("TAG {}", tag_idx + 1));
                render_widget(sec_cl.pc, &mut preview, tx, ty, card_w, card_h, ctx);
            }
            
            sec_cl.content_y += 2.0 * (card_h + 8.0) + 4.0;
        });

        // 2. Fullscreen Section
        sec.add_section("Fullscreen", false, |subsec| {
            subsec.spacing(8.0);
            state.windows.spinboxes[0].set_label("Border Width");
            subsec.widget_full(&mut state.windows.spinboxes[0], 44.0, ctx);
            subsec.spacing(8.0);
        });

        // 3. Cascade Section
        sec.add_section("Cascade", false, |subsec| {
            subsec.spacing(8.0);
            state.windows.spinboxes[1].set_label("Border Width");
            subsec.widget_full(&mut state.windows.spinboxes[1], 44.0, ctx);
            subsec.spacing(8.0);
            state.windows.cascade_offset_spinbox.set_label("Offset");
            subsec.widget_full(&mut state.windows.cascade_offset_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.windows.edge_gap_spinbox.set_label("Edge Gap");
            subsec.widget_full(&mut state.windows.edge_gap_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.windows.top_gap_spinbox.set_label("Top Gap");
            subsec.widget_full(&mut state.windows.top_gap_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });

        // 4. Grid Section
        sec.add_section("Grid", false, |subsec| {
            subsec.spacing(8.0);
            state.windows.spinboxes[2].set_label("Border Width");
            subsec.widget_full(&mut state.windows.spinboxes[2], 44.0, ctx);
            subsec.spacing(8.0);
            state.windows.grid_gap_spinbox.set_label("Gap");
            subsec.widget_full(&mut state.windows.grid_gap_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });

        // 5. Floating Section
        sec.add_section("Floating", false, |subsec| {
            subsec.spacing(8.0);
            state.windows.spinboxes[3].set_label("Border Width");
            subsec.widget_full(&mut state.windows.spinboxes[3], 44.0, ctx);
            subsec.spacing(8.0);
        });

        // 6. Movement Section
        sec.add_section("Movement", false, |subsec| {
            subsec.spacing(8.0);
            state.windows.transition_duration_spinbox.set_label("Duration (ms)");
            subsec.widget_full(&mut state.windows.transition_duration_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });

        // 7. Default Layouts Section
        sec.add_section("Default Layouts", false, |subsec| {
            subsec.spacing(8.0);
            for i in 0..4 {
                subsec.widget_full(&mut state.windows.tag_layout_menus[i], 44.0, ctx);
                subsec.spacing(8.0);
            }
        });

        // 8. Side Panel Section
        sec.add_section("Side Panel", false, |subsec| {
            subsec.spacing(8.0);
            subsec.widget_full(&mut state.windows.side_panel_behavior_menu, 44.0, ctx);
            subsec.spacing(8.0);
            subsec.widget_full(&mut state.windows.side_panel_position_menu, 44.0, ctx);
            subsec.spacing(8.0);
            state.windows.side_panel_width_spinbox.set_label("Default Width");
            subsec.widget_full(&mut state.windows.side_panel_width_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.windows.side_panel_border_gap_spinbox.set_label("Border Gap");
            subsec.widget_full(&mut state.windows.side_panel_border_gap_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
            state.windows.side_panel_border_opacity_spinbox.set_label("Border Opacity");
            subsec.widget_full(&mut state.windows.side_panel_border_opacity_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });

        // 9. Effects Section
        sec.add_section("Effects", false, |subsec| {
            subsec.spacing(8.0);
            state.windows.transparency_toggle.set_toggled(state.windows.transparency_enabled);
            subsec.widget_full(&mut state.windows.transparency_toggle, cce_ui::layout::toggle_height(), ctx);
            subsec.spacing(8.0);
            state.windows.blur_toggle.set_toggled(state.windows.blur_enabled);
            subsec.widget_full(&mut state.windows.blur_toggle, cce_ui::layout::toggle_height(), ctx);
            subsec.spacing(8.0);
        });

        sec.spacing(8.0);
    });

    final_pc
}


pub fn update(state: &mut InterfaceState, msg: InterfaceMessage) {
    match msg {
        InterfaceMessage::SetDesktopBackground(rgb) => {
            state.desktop_background_color = rgb;
            apply_desktop_background(rgb);
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
        InterfaceMessage::SetSeparatorColor(rgb) => {
            state.separator_color = rgb;
            apply_separator_color(rgb);
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
        InterfaceMessage::SetWindowColor(rgb) => {
            state.window_color = rgb;
            apply_window_color(rgb);
        }
        InterfaceMessage::SetWindowOpacity(opacity) => {
            state.window_opacity = opacity;
            state.window_opacity_spinbox.value = (opacity * 100.0).round() as i32;
            apply_window_opacity(opacity);
        }
        InterfaceMessage::SetWindowCornerRadius(radius) => {
            state.window_corner_radius = radius;
            state.window_corner_radius_spinbox.value = radius as i32;
            apply_window_corner_radius(radius);
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
        InterfaceMessage::PickLowColor | InterfaceMessage::PickHighColor | InterfaceMessage::PickDisabledColor | InterfaceMessage::PickSeparatorColor | InterfaceMessage::PickVisualGuides | InterfaceMessage::PickSliderTrackColor | InterfaceMessage::PickPageLowColor | InterfaceMessage::PickColorBordersColor | InterfaceMessage::PickNormalColor | InterfaceMessage::PickPaginatorSidebarColor | InterfaceMessage::PickPrimaryHighlightColor | InterfaceMessage::PickMenubarTabLabelColor | InterfaceMessage::PickToggleEnabledColor | InterfaceMessage::PickToggleDisabledColor | InterfaceMessage::PickScrollingListBgColor | InterfaceMessage::PickScrollingListEntryBgColor | InterfaceMessage::PickScrollingListEntryHighlightColor | InterfaceMessage::PickBreadcrumbBgColor | InterfaceMessage::PickPopoverBgColor | InterfaceMessage::PickNotificationBgColor | InterfaceMessage::PickWindowColor | InterfaceMessage::PickPageColor | InterfaceMessage::PickLayerColor => {}
        InterfaceMessage::Refreshed(new) => {
            let was_bp_hovered = state.button_padding_spinbox.hovered();
            let was_bss_hovered = state.button_strip_spacing_spinbox.hovered();
            let was_sp_hovered = state.section_padding_spinbox.hovered();
            let was_pp_hovered = state.plate_padding_spinbox.hovered();
            let was_pl_op_hovered = state.plate_opacity_spinbox.hovered();
            let was_pl_cr_hovered = state.plate_corner_radius_spinbox.hovered();
            let was_page_op_hovered = state.page_opacity_spinbox.hovered();
            let was_layer_op_hovered = state.layer_opacity_spinbox.hovered();

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
            let was_lm_hovered = state.label_margin_spinbox.hovered();
            let was_mo_hovered = state.menubar_opacity_spinbox.hovered();
            let was_no_hovered = state.notification_opacity_spinbox.hovered();
            let was_wo_hovered = state.window_opacity_spinbox.hovered();
            let was_wcr_hovered = state.window_corner_radius_spinbox.hovered();
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
            state.label_margin_spinbox.set_hovered(was_lm_hovered);
            state.menubar_opacity_spinbox.set_hovered(was_mo_hovered);
            state.notification_opacity_spinbox.set_hovered(was_no_hovered);
            state.window_opacity_spinbox.set_hovered(was_wo_hovered);
            state.window_corner_radius_spinbox.set_hovered(was_wcr_hovered);

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
        WindowsMessage::SetWidth(p, v) => set_width(state, p, v),
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
        WindowsMessage::SetGridGap(v) => {
            let val = v.min(200);
            state.grid_gap = val;
            state.grid_gap_spinbox.value = val as i32;
            apply_single_layout_param("grid_gap", val);
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
        WindowsMessage::SetTagLayout(tag, idx) => {
            if tag >= 1 && tag <= 4 && idx < 5 {
                state.tag_layout_menus[tag - 1].selected = idx;
                let modes = vec!["cascade", "grid", "fullscreen", "floating", "popup"];
                let mode_str = modes[idx];
                write_tag_layout(tag, mode_str);
                send_ipc_command(&format!("tag-layout {} {}", tag, mode_str));
            }
        }
        WindowsMessage::SetSidePanelBehavior(idx) => {
            if idx < 2 {
                state.side_panel_behavior_menu.selected = idx;
                let val = if idx == 0 { "above" } else { "inline" };
                write_config_value("side_panel_behavior", &format!("\"{}\"", val));
                send_ipc_command(&format!("layout side_panel_behavior {}", val));
            }
        }
        WindowsMessage::SetSidePanelPosition(idx) => {
            if idx < 2 {
                state.side_panel_position_menu.selected = idx;
                let val = if idx == 1 { "right" } else { "left" };
                write_config_value("side_panel_position", &format!("\"{}\"", val));
                send_ipc_command(&format!("layout side_panel_position {}", val));
            }
        }
        WindowsMessage::SetSidePanelWidth(v) => {
            let val = v.min(2000);
            state.side_panel_width = val;
            state.side_panel_width_spinbox.value = val as i32;
            write_config_value("side_panel_width", &val.to_string());
            send_ipc_command(&format!("layout side_panel_width {}", val));
        }
        WindowsMessage::SetSidePanelBorderGap(v) => {
            let val = v.min(500);
            state.side_panel_border_gap = val;
            state.side_panel_border_gap_spinbox.value = val as i32;
            write_config_value("side_panel_border_gap", &val.to_string());
            send_ipc_command(&format!("layout side_panel_border_gap {}", val));
        }
        WindowsMessage::SetSidePanelBorderOpacity(v) => {
            let val = v.min(100);
            state.side_panel_border_opacity = val;
            state.side_panel_border_opacity_spinbox.value = val as i32;
            write_config_value("side_panel_border_opacity", &val.to_string());
            send_ipc_command(&format!("layout side_panel_border_opacity {}", val));
        }
        WindowsMessage::ToggleTransparency => {
            state.transparency_enabled = !state.transparency_enabled;
            write_config_value("window_opacity", &state.transparency_enabled.to_string());
            send_ipc_command("reload");
            status_interface_reload();
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
            let transparency_hover = state.transparency_toggle.hovered();
            let blur_hover = state.blur_toggle.hovered();
            let tag_layout_menus_hover: Vec<bool> = state.tag_layout_menus.iter().map(|m| m.hovered()).collect();
            let side_panel_behavior_hover = state.side_panel_behavior_menu.hovered();
            let side_panel_position_hover = state.side_panel_position_menu.hovered();
            let spinboxes_hover: Vec<bool> = state.spinboxes.iter().map(|sb| sb.hovered()).collect();
            let cascade_offset_hover = state.cascade_offset_spinbox.hovered();
            let edge_gap_hover = state.edge_gap_spinbox.hovered();
            let top_gap_hover = state.top_gap_spinbox.hovered();
            let grid_gap_hover = state.grid_gap_spinbox.hovered();
            let status_height_hover = state.status_height_spinbox.hovered();
            let transition_duration_hover = state.transition_duration_spinbox.hovered();
            let side_panel_width_hover = state.side_panel_width_spinbox.hovered();
            let side_panel_border_gap_hover = state.side_panel_border_gap_spinbox.hovered();
            let side_panel_border_opacity_hover = state.side_panel_border_opacity_spinbox.hovered();

            *state = new;

            state.transparency_toggle.set_hovered(transparency_hover);
            state.blur_toggle.set_hovered(blur_hover);
            for (menu, hover) in state.tag_layout_menus.iter_mut().zip(tag_layout_menus_hover) {
                menu.set_hovered(hover);
            }
            state.side_panel_behavior_menu.set_hovered(side_panel_behavior_hover);
            state.side_panel_position_menu.set_hovered(side_panel_position_hover);
            for (sb, hover) in state.spinboxes.iter_mut().zip(spinboxes_hover) {
                sb.set_hovered(hover);
            }
            state.cascade_offset_spinbox.set_hovered(cascade_offset_hover);
            state.edge_gap_spinbox.set_hovered(edge_gap_hover);
            state.top_gap_spinbox.set_hovered(top_gap_hover);
            state.grid_gap_spinbox.set_hovered(grid_gap_hover);
            state.status_height_spinbox.set_hovered(status_height_hover);
            state.transition_duration_spinbox.set_hovered(transition_duration_hover);
            state.side_panel_width_spinbox.set_hovered(side_panel_width_hover);
            state.side_panel_border_gap_spinbox.set_hovered(side_panel_border_gap_hover);
            state.side_panel_border_opacity_spinbox.set_hovered(side_panel_border_opacity_hover);
        }
    }
}

pub fn parse_transparency_opacity(content: &str) -> f32 {
    let val = parse_json(content);
    val["transparency"]["opacity"].as_f64().map(|v| v as f32).unwrap_or(0.9)
}

pub fn write_transparency_config_value(key: &str, value: &str) {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
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
        let _ = fs::write(CONFIG_PATH, updated_str);
    }
}

fn write_surfaces_config_value(key: &str, value: &str) {
    write_surfaces_config_value_path(CONFIG_PATH, key, value);
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
        let _ = fs::write(path, updated_str);
    }
}

fn parse_surfaces_color(content: &str, key: &str, default: [u8; 3]) -> [u8; 3] {
    let val = parse_json(content);
    if let Some(s) = val["surfaces"].get(key).and_then(|v| v.as_str()) {
        return parse_hex(s);
    }
    default
}

fn parse_surfaces_opacity(content: &str) -> f32 {
    let val = parse_json(content);
    val["surfaces"]["window_opacity"].as_f64().map(|v| v as f32).unwrap_or(0.9)
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
    write_notifications_config_value_path(CONFIG_PATH, key, value);
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
        let _ = fs::write(path, updated_str);
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
        let sb = &mut state.window_corner_radius_spinbox;
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
        let original_links = fs::read_to_string(LINKS_PATH).unwrap_or_default();
        let test_links = r#"{
            "spinbox_height": "textbox_height",
            "textbox_height": "dropdown_height"
        }"#;
        fs::write(LINKS_PATH, test_links).unwrap();

        let dir = std::env::temp_dir();
        let path = dir.join("test_linking_config.toml");
        let path_str = path.to_str().unwrap();

        let initial_content = "{\"layout\": {\"spinbox_height\": 28, \"textbox_height\": 28, \"dropdown_height\": 28, \"button_corner_radius\": 4}}";
        fs::write(path_str, initial_content).unwrap();

        assert!(write_config_value_path(path_str, "spinbox_height", "32"));

        if !original_links.is_empty() {
            let _ = fs::write(LINKS_PATH, original_links);
        } else {
            let _ = fs::remove_file(LINKS_PATH);
        }

        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"spinbox_height\": 32"));
        assert!(updated.contains("\"textbox_height\": 32"));
        assert!(updated.contains("\"dropdown_height\": 32"));
        assert!(updated.contains("\"button_corner_radius\": 4"));

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_propagate_links() {
        let original_config = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
        let original_links = fs::read_to_string(LINKS_PATH).unwrap_or_default();

        let test_config = "{\"layout\": {\"spinbox_height\": 28, \"textbox_height\": 28, \"dropdown_height\": 28}}";
        fs::write(CONFIG_PATH, test_config).unwrap();

        let test_links = r#"{
            "spinbox_height": "textbox_height",
            "textbox_height": "dropdown_height"
        }"#;
        fs::write(LINKS_PATH, test_links).unwrap();

        let mut state = InterfaceState::default();
        state.spinbox_height = 36;
        state.textbox_height = 28;
        state.dropdown_height = 28;
        state.spinbox_height_spinbox.value = 36;
        state.textbox_height_spinbox.value = 28;
        state.dropdown_height_spinbox.value = 28;

        propagate_links(&mut state, "spinbox_height", "36");

        if !original_config.is_empty() {
            let _ = fs::write(CONFIG_PATH, original_config);
        } else {
            let _ = fs::remove_file(CONFIG_PATH);
        }
        if !original_links.is_empty() {
            let _ = fs::write(LINKS_PATH, original_links);
        } else {
            let _ = fs::remove_file(LINKS_PATH);
        }

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
        let opacity = parse_surfaces_opacity(&content);
        assert_eq!(opacity, 0.9);
        let color = parse_surfaces_color(&content, "window_color", [0x0a, 0x1a, 0x0e]);
        assert_eq!(color, [0x0a, 0x1a, 0x0e]);
        let radius = parse_surfaces_u16(&content, "window_corner_radius", 12);
        assert_eq!(radius, 12);

        // 3. Write surfaces opacity config
        write_surfaces_config_value_path(path_str, "window_opacity", "0.75");
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("\"window_opacity\": 0.75"));

        // 4. Parse surfaces opacity when present
        let opacity2 = parse_surfaces_opacity(&updated);
        assert_eq!(opacity2, 0.75);

        // 5. Write surfaces window_color config
        write_surfaces_config_value_path(path_str, "window_color", "\"#112233\"");
        let updated2 = fs::read_to_string(path_str).unwrap();
        assert!(updated2.contains("\"window_color\": \"#112233\""));

        // 6. Parse surfaces window_color when present
        let color2 = parse_surfaces_color(&updated2, "window_color", [0, 0, 0]);
        assert_eq!(color2, [17, 34, 51]);

        // 7. Write surfaces window_corner_radius config
        write_surfaces_config_value_path(path_str, "window_corner_radius", "16");
        let updated3 = fs::read_to_string(path_str).unwrap();
        assert!(updated3.contains("\"window_corner_radius\": 16"));

        // 8. Parse surfaces window_corner_radius when present
        let radius2 = parse_surfaces_u16(&updated3, "window_corner_radius", 12);
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
    fn test_parse_tag_layouts_empty() {
        let content = "";
        let modes = parse_tag_layouts_from_config(content);
        assert_eq!(modes, vec!["cascade", "cascade", "cascade", "cascade"]);
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
    fn test_parse_tag_layouts_single() {
        let content = r#"{
            "layout": {"gap": 10},
            "tag_layout": [
                {"tag": 2, "mode": "grid"}
            ]
        }"#;
        let modes = parse_tag_layouts_from_config(content);
        assert_eq!(modes, vec!["cascade", "grid", "cascade", "cascade"]);
    }

    #[test]
    fn test_parse_tag_layouts_multiple() {
        let content = r#"{
            "tag_layout": [
                {"tag": 1, "mode": "fullscreen"},
                {"tag": 4, "mode": "floating"}
            ]
        }"#;
        let modes = parse_tag_layouts_from_config(content);
        assert_eq!(modes, vec!["fullscreen", "cascade", "cascade", "floating"]);
    }

    #[test]
    fn test_parse_tag_layouts_out_of_bounds() {
        let content = r#"{
            "tag_layout": [
                {"tag": 5, "mode": "grid"},
                {"tag": 0, "mode": "popup"}
            ]
        }"#;
        let modes = parse_tag_layouts_from_config(content);
        assert_eq!(modes, vec!["cascade", "cascade", "cascade", "cascade"]);
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
        let sb = &mut state.windows.spinboxes[0];
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
        let sb = &mut state.windows.spinboxes[0];
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
}



