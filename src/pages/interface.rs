use std::fs;
use std::io::Write;
use crate::app::PageContent;
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{
    ColorSelector, Spinbox, Element, Dropdown, TextBox, FontSelector, Toggle
};

const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.toml";

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/cce-client-{}.sock", display),
        Err(_) => "/tmp/cce-client.sock".to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceState {
    pub low_color: [u8; 3],
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
    pub breadcrumb_bg_color: [u8; 3],
    pub popover_bg_color: [u8; 3],
    pub color_selectors: Vec<ColorSelector>,
    pub menubar_opacity: f32,
    pub menubar_opacity_spinbox: Spinbox,
    pub notification_bg_color: [u8; 3],
    pub notification_opacity: f32,
    pub notification_opacity_spinbox: Spinbox,
    pub paginator_tab_margin_x: u16,
    pub paginator_tab_margin_y: u16,
    pub tab_margin_spinbox_x: Spinbox,
    pub tab_margin_spinbox_y: Spinbox,
    pub paginator_tab_padding_x: u16,
    pub paginator_tab_padding_y: u16,
    pub button_padding: u16,
    pub button_padding_spinbox: Spinbox,
    pub section_padding: u16,
    pub section_padding_spinbox: Spinbox,
    pub plate_padding: u16,
    pub plate_padding_spinbox: Spinbox,
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
    pub dropdown_height: u16,
    pub dropdown_height_spinbox: Spinbox,
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
    pub paginator: String,
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
}

impl Default for InterfaceState {
    fn default() -> Self {
        Self {
            low_color: [0x0a, 0x1a, 0x0e],
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
                ColorSelector::new([0x0a, 0x1a, 0x0e]).with_label("Low Color"), // 7: Layout - Low Color
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
            ],
            paginator_tab_margin_x: 5,
            paginator_tab_margin_y: 10,
            tab_margin_spinbox_x: Spinbox::new(5, 0, 100, 1).with_label("Tab Margin X").with_unit("px"),
            tab_margin_spinbox_y: Spinbox::new(10, 0, 100, 1).with_label("Tab Margin Y").with_unit("px"),
            paginator_tab_padding_x: 10,
            paginator_tab_padding_y: 14,
            button_padding: 14,
            button_padding_spinbox: Spinbox::new(14, 0, 100, 1).with_label("Button Padding").with_unit("px"),
            section_padding: 8,
            section_padding_spinbox: Spinbox::new(8, 0, 100, 1).with_label("Padding").with_unit("px"),
            plate_padding: 20,
            plate_padding_spinbox: Spinbox::new(20, 0, 100, 1).with_label("Padding").with_unit("px"),
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
            dropdown_height: 44,
            dropdown_height_spinbox: Spinbox::new(44, 10, 100, 1).with_label("Height").with_unit("px"),
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
            paginator: String::new(),
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
        }
    }
}

#[derive(Debug, Clone)]
pub enum InterfaceMessage {
    SetLowColor([u8; 3]),
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
    SetBreadcrumbBgColor([u8; 3]),
    SetPopoverBgColor([u8; 3]),
    SetNotificationBgColor([u8; 3]),
    SetNotificationOpacity(f32),
    SetTabMarginX(u16),
    SetTabMarginY(u16),
    SetButtonPadding(u16),
    SetSectionPadding(u16),
    SetPlatePadding(u16),
    SetPageMargin(u16),
    SetGridMinColWidth(u16),
    SetSpinboxHeight(u16),
    SetSpinboxCornerRadius(u16),
    SetToggleHeight(u16),
    SetColorSelectorHeight(u16),
    SetColorSelectorCornerRadius(u16),
    SetColorSelectorPreviewCornerRadius(u16),
    SetColorSelectorPreviewMargin(u16),
    SetTextboxHeight(u16),
    SetTextboxCornerRadius(u16),
    SetSliderHeight(u16),
    SetFontSelectorHeight(u16),
    SetDropdownHeight(u16),
    SetButtonCornerRadius(u16),
    SetColorSelectorFont(String),
    SetMenubarFont(String),
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
    PickBreadcrumbBgColor,
    PickPopoverBgColor,
    PickNotificationBgColor,
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
    let has_low = content.lines().any(|l| l.trim().starts_with("low_color"));
    let bg = if has_low {
        parse_color_from_key(&content, "low_color", [0x0a, 0x1a, 0x0e])
    } else {
        parse_color_from_key(&content, "background_color", [0x0a, 0x1a, 0x0e])
    };
    
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

    let breadcrumb_bg = parse_color_from_key(&content, "breadcrumb_bg_color", scrollinglist_bg);

    let popover_bg = parse_color_from_key(&content, "popover_bg_color", scrollinglist_bg);
    
    let paginator_tab_margin_general = parse_u16_from(&content, "paginator_tab_margin", 999);
    let paginator_tab_margin_x = parse_u16_from(&content, "paginator_tab_margin_x", if paginator_tab_margin_general != 999 { paginator_tab_margin_general } else { 5 });
    let paginator_tab_margin_y = parse_u16_from(&content, "paginator_tab_margin_y", if paginator_tab_margin_general != 999 { paginator_tab_margin_general } else { 10 });
    let paginator_tab_padding_x = parse_u16_from(&content, "paginator_tab_padding_x", 10);
    let paginator_tab_padding_y = parse_u16_from(&content, "paginator_tab_padding_y", 14);
    let button_padding = parse_u16_from(&content, "button_padding", paginator_tab_padding_y);
    let section_padding = parse_u16_from(&content, "section_padding", 8);
    let plate_padding = parse_u16_from(&content, "plate_padding", 20);
    let page_margin = parse_u16_from(&content, "page_margin", 20);
    let grid_min_col_width = parse_u16_from(&content, "grid_min_col_width", 260);
    let spinbox_height = parse_u16_from(&content, "spinbox_height", 26);
    let spinbox_corner_radius = parse_u16_from(&content, "spinbox_corner_radius", 4);
    let toggle_height = parse_u16_from(&content, "toggle_height", 44);
    let color_selector_height = parse_u16_from(&content, "color_selector_height", 22);
    let color_selector_corner_radius = parse_u16_from(&content, "color_selector_corner_radius", 4);
    let color_selector_preview_corner_radius = parse_u16_from(&content, "color_selector_preview_corner_radius", 4);
    let color_selector_preview_margin = parse_u16_from(&content, "color_selector_preview_margin", 0);
    let textbox_height = parse_u16_from(&content, "textbox_height", 44);
    let textbox_corner_radius = parse_u16_from(&content, "textbox_corner_radius", 4);
    let slider_height = parse_u16_from(&content, "slider_height", 28);
    let font_selector_height = parse_u16_from(&content, "font_selector_height", 44);
    let dropdown_height = parse_u16_from(&content, "dropdown_height", 44);
    let button_corner_radius = parse_u16_from(&content, "button_corner_radius", 4);
    let color_selector_font = parse_string_from(&content, "color_selector_font", "monospace");
    let menubar_font = parse_string_from(&content, "menubar_font", "Outfit");
    let section_label_font = parse_string_from(&content, "section_label_font", "Outfit");
    let nested_section_label_font = parse_string_from(&content, "nested_section_label_font", "Outfit");
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
    let opacity = parse_transparency_opacity(&content);
    let notification_bg_color = parse_notifications_color(&content, "bg_color", [0x08, 0x08, 0x0c]);
    let notification_opacity = parse_notifications_opacity(&content);
    
    InterfaceState {
        low_color: bg,
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
        ],
        paginator_tab_margin_x,
        paginator_tab_margin_y,
        tab_margin_spinbox_x: Spinbox::new(paginator_tab_margin_x as i32, 0, 100, 1).with_label("Tab Margin X").with_unit("px"),
        tab_margin_spinbox_y: Spinbox::new(paginator_tab_margin_y as i32, 0, 100, 1).with_label("Tab Margin Y").with_unit("px"),
        paginator_tab_padding_x,
        paginator_tab_padding_y,
        button_padding,
        button_padding_spinbox: Spinbox::new(button_padding as i32, 0, 100, 1).with_label("Button Padding").with_unit("px"),
        section_padding,
        section_padding_spinbox: Spinbox::new(section_padding as i32, 0, 100, 1).with_label("Padding").with_unit("px"),
        plate_padding,
        plate_padding_spinbox: Spinbox::new(plate_padding as i32, 0, 100, 1).with_label("Padding").with_unit("px"),
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
        dropdown_height,
        dropdown_height_spinbox: Spinbox::new(dropdown_height as i32, 10, 100, 1).with_label("Height").with_unit("px"),
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
        paginator: String::new(),
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
        menubar_opacity: opacity,
        menubar_opacity_spinbox: Spinbox::new((opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%"),
        notification_bg_color,
        notification_opacity,
        notification_opacity_spinbox: Spinbox::new((notification_opacity * 100.0).round() as i32, 0, 100, 5).with_label("Opacity").with_unit("%"),
    }
}

pub fn parse_string_from(content: &str, key: &str, default: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=');
            let rest = rest.trim();
            let val_str = if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
                &rest[1..rest.len() - 1]
            } else {
                rest
            };
            return val_str.trim().to_string();
        }
    }
    default.to_string()
}

fn parse_color_from_key(content: &str, key: &str, default: [u8; 3]) -> [u8; 3] {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            let hex = rest.trim_end_matches('"').trim().trim_start_matches('#');
            return parse_hex(hex);
        }
    }
    default
}

fn parse_hex(s: &str) -> [u8; 3] {
    let s = s.trim_start_matches('#');
    if s.len() >= 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0x0a);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0x1a);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0x0e);
        [r, g, b]
    } else { [0x0a, 0x1a, 0x0e] }
}

pub fn write_config_value(key: &str, value: &str) -> bool {
    write_config_value_path(CONFIG_PATH, key, value)
}

pub fn write_config_value_path(path: &str, key: &str, value: &str) -> bool {
    let content = fs::read_to_string(path).unwrap_or_default();
    let old_key = match key {
        "low_color" => "background_color",
        "high_color" => "border_color",
        _ => "",
    };
    let new_line = format!("{} = {}", key, value);
    let mut found = false;
    let updated: String = content.lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with(key) {
                found = true;
                new_line.clone()
            } else if !old_key.is_empty() && trimmed.starts_with(old_key) {
                found = true;
                new_line.clone()
            } else {
                line.to_string()
            }
        }).collect::<Vec<_>>().join("\n");
    if !found {
        let mut result = String::new();
        let mut in_layout = false;
        let mut inserted = false;
        for line in updated.lines() {
            if line.trim() == "[layout]" { in_layout = true; }
            else if line.trim().starts_with('[') && in_layout {
                if !inserted { result.push_str(&new_line); result.push('\n'); inserted = true; }
                in_layout = false;
            }
            result.push_str(line); result.push('\n');
        }
        if in_layout && !inserted { result.push_str(&new_line); result.push('\n'); }
        fs::write(path, result).is_ok()
    } else { fs::write(path, updated).is_ok() }
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

fn apply_background(rgb: [u8; 3]) {
    let _ = std::process::Command::new("pkill").args(["-x", "swaybg"]).status();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let hex = format!("{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
    let _ = std::process::Command::new("swaybg").arg("-c").arg(&hex).spawn();
    write_config_value("low_color", &format!("\"#{}\"", hex));
    send_ipc_command(&format!("layout low_color #{}", hex));
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

fn apply_paginator_tab_margin_x(margin: u16) {
    write_config_value("paginator_tab_margin_x", &margin.to_string());
    send_ipc_command(&format!("layout paginator_tab_margin_x {}", margin));
    cce_ui::layout::set_paginator_tab_margin_x(margin as f32);
}

fn apply_paginator_tab_margin_y(margin: u16) {
    write_config_value("paginator_tab_margin_y", &margin.to_string());
    send_ipc_command(&format!("layout paginator_tab_margin_y {}", margin));
    cce_ui::layout::set_paginator_tab_margin_y(margin as f32);
}

fn apply_button_padding(padding: u16) {
    write_config_value("button_padding", &padding.to_string());
    send_ipc_command(&format!("layout button_padding {}", padding));
    cce_ui::layout::set_button_padding(padding as f32);
}

fn apply_plate_padding(padding: u16) {
    write_config_value("plate_padding", &padding.to_string());
    cce_ui::layout::set_plate_padding(padding as f32);
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

fn apply_dropdown_height(height: u16) {
    write_config_value("dropdown_height", &height.to_string());
    cce_ui::layout::set_dropdown_height(height as f32);
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

pub fn read_preferred_fonts() -> (String, String, String, String, String, String, String, String) {
    let content = fs::read_to_string(FONTS_CONF_PATH).unwrap_or_default();
    
    let sans = parse_font_for_alias(&content, "sans-serif").unwrap_or_else(|| "Noto Sans".to_string());
    let serif = parse_font_for_alias(&content, "serif").unwrap_or_else(|| "Noto Serif".to_string());
    let mono = parse_font_for_alias(&content, "monospace").unwrap_or_else(|| "Noto Sans Mono".to_string());
    let borders = parse_font_for_alias(&content, "window-borders").unwrap_or_else(|| "Noto Sans".to_string());
    let status = parse_font_for_alias(&content, "status-interface").unwrap_or_else(|| "Noto Sans".to_string());
    let fuzzel_font = parse_font_for_alias(&content, "fuzzel").unwrap_or_else(|| "Noto Sans".to_string());
    let term = parse_font_for_alias(&content, "terminal").unwrap_or_else(|| "Noto Sans Mono".to_string());
    let paginator = parse_font_for_alias(&content, "paginator-tab-labels").unwrap_or_else(|| "Noto Sans Mono".to_string());
    
    (sans, serif, mono, borders, status, fuzzel_font, term, paginator)
}

pub fn save_preferred_fonts(
    sans: &str,
    serif: &str,
    mono: &str,
    borders: &str,
    status: &str,
    fuzzel: &str,
    terminal: &str,
    paginator: &str,
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
    
    // Paginator Tab Labels
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>paginator-tab-labels</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", paginator));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    new_content.push_str("</fontconfig>\n");
    
    let _ = fs::write(FONTS_CONF_PATH, new_content);
}

pub fn parse_i16_from(content: &str, key: &str, default: i16) -> i16 {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            let val_str = rest.trim_end_matches('"').trim();
            if let Ok(val) = val_str.parse::<i16>() {
                return val;
            }
        }
    }
    default
}

pub fn parse_u16_from(content: &str, key: &str, default: u16) -> u16 {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            let val_str = rest.trim_end_matches('"').trim();
            if let Ok(val) = val_str.parse::<u16>() {
                return val;
            }
        }
    }
    default
}

pub fn parse_bool_from(content: &str, key: &str, default: bool) -> bool {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            let val_str = rest.trim_end_matches('"').trim();
            if let Ok(val) = val_str.parse::<bool>() {
                return val;
            }
        }
    }
    default
}

pub fn parse_f32_from(content: &str, key: &str, default: f32) -> f32 {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            let val_str = rest.trim_end_matches('"').trim();
            if let Ok(val) = val_str.parse::<f32>() {
                return val;
            }
        }
    }
    default
}

fn read_border_font_size() -> Option<u16> {
    let content = fs::read_to_string("/home/lsgalante/.config/cce/config.toml").ok()?;
    Some(parse_u16_from(&content, "border_font_size", 11))
}

fn read_status_size() -> Option<u16> {
    let content = fs::read_to_string("/home/lsgalante/.config/cce/config.toml").ok()?;
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
    let (sans, serif, mono, borders, status, fuzzel_font, term, paginator_font) = read_preferred_fonts();
    
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
    state.paginator = paginator_font;
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
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(6);

    // 1. Layout Section
    builder.add_section(&mut final_pc, "Layout", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[7].color = state.low_color;
        sec.widget_full(&mut state.color_selectors[7], 40.0, ctx);
        sec.spacing(8.0);
        state.color_selectors[1].color = state.high_color;
        sec.widget_full(&mut state.color_selectors[1], 40.0, ctx);
        sec.spacing(8.0);
        state.color_selectors[2].color = state.visual_guides_color;
        sec.widget_full(&mut state.color_selectors[2], 40.0, ctx);
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

        // Grid Layout child section
        sec.add_section("Adaptive Grid", false, |subsec| {
            subsec.spacing(8.0);
            state.grid_min_col_width_spinbox.value = state.grid_min_col_width as i32;
            subsec.widget_full(&mut state.grid_min_col_width_spinbox, 44.0, ctx);
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
        sec.spacing(8.0);
    });

    // 3. Status Section
    builder.add_section(&mut final_pc, "Status", false, |sec| {
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
    builder.add_section(&mut final_pc, "Controls", false, |sec| {
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
            state.tab_margin_spinbox_x.value = state.paginator_tab_margin_x as i32;
            subsec.widget_full(&mut state.tab_margin_spinbox_x, 44.0, ctx);
            subsec.spacing(8.0);
            state.tab_margin_spinbox_y.value = state.paginator_tab_margin_y as i32;
            subsec.widget_full(&mut state.tab_margin_spinbox_y, 44.0, ctx);
            subsec.spacing(8.0);
            state.menubar_font_selector.font_family = state.menubar_font.clone();
            subsec.widget_full(&mut state.menubar_font_selector, 44.0, ctx);
            subsec.spacing(8.0);
            state.menubar_opacity_spinbox.value = (state.menubar_opacity * 100.0).round() as i32;
            subsec.widget_full(&mut state.menubar_opacity_spinbox, 44.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Toggles Section
        sec.add_section("Toggles", false, |subsec| {
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
        });
        sec.spacing(12.0);

        // ScrollingList Section
        sec.add_section("ScrollingList", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[14].color = state.scrollinglist_bg_color;
            subsec.widget_full(&mut state.color_selectors[14], 40.0, ctx);
            subsec.spacing(8.0);
        });
        sec.spacing(12.0);

        // Breadcrumb Section
        sec.add_section("Breadcrumb", false, |subsec| {
            subsec.spacing(8.0);
            state.color_selectors[15].color = state.breadcrumb_bg_color;
            subsec.widget_full(&mut state.color_selectors[15], 40.0, ctx);
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
        });
        sec.spacing(12.0);

        // Dropdown Section
        sec.add_section("Dropdown", false, |subsec| {
            subsec.spacing(8.0);
            state.dropdown_height_spinbox.value = state.dropdown_height as i32;
            subsec.widget_full(&mut state.dropdown_height_spinbox, 44.0, ctx);
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
    builder.add_section(&mut final_pc, "Indicators", false, |sec| {
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
    builder.add_section(&mut final_pc, "Notification", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[17].color = state.notification_bg_color;
        sec.widget_full(&mut state.color_selectors[17], 40.0, ctx);
        sec.spacing(8.0);
        state.notification_opacity_spinbox.value = (state.notification_opacity * 100.0).round() as i32;
        sec.widget_full(&mut state.notification_opacity_spinbox, 44.0, ctx);
        sec.spacing(8.0);
    });

    let widget_h = 26.0;
    const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

    // 6. Fonts Section
    builder.add_section(&mut final_pc, "Fonts", sec_focused.get(5).copied().unwrap_or(false), |sec| {
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

    final_pc
}


pub fn update(state: &mut InterfaceState, msg: InterfaceMessage) {
    match msg {
        InterfaceMessage::SetLowColor(rgb) => {
            state.low_color = rgb;
            apply_background(rgb);
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
        InterfaceMessage::SetTabMarginX(margin) => {
            state.paginator_tab_margin_x = margin;
            apply_paginator_tab_margin_x(margin);
        }
        InterfaceMessage::SetTabMarginY(margin) => {
            state.paginator_tab_margin_y = margin;
            apply_paginator_tab_margin_y(margin);
        }
        InterfaceMessage::SetButtonPadding(padding) => {
            state.button_padding = padding;
            apply_button_padding(padding);
        }
        InterfaceMessage::SetSectionPadding(padding) => {
            state.section_padding = padding;
            apply_section_padding(padding);
        }
        InterfaceMessage::SetPlatePadding(padding) => {
            state.plate_padding = padding;
            apply_plate_padding(padding);
        }
        InterfaceMessage::SetPageMargin(margin) => {
            state.page_margin = margin;
            apply_page_margin(margin);
        }
        InterfaceMessage::SetGridMinColWidth(width) => {
            state.grid_min_col_width = width;
            apply_grid_min_col_width(width);
        }
        InterfaceMessage::SetSpinboxHeight(height) => {
            state.spinbox_height = height;
            apply_spinbox_height(height);
        }
        InterfaceMessage::SetSpinboxCornerRadius(radius) => {
            state.spinbox_corner_radius = radius;
            apply_spinbox_corner_radius(radius);
        }
        InterfaceMessage::SetToggleHeight(height) => {
            state.toggle_height = height;
            apply_toggle_height(height);
        }
        InterfaceMessage::SetColorSelectorHeight(height) => {
            state.color_selector_height = height;
            apply_color_selector_height(height);
        }
        InterfaceMessage::SetColorSelectorCornerRadius(radius) => {
            state.color_selector_corner_radius = radius;
            apply_color_selector_corner_radius(radius);
        }
        InterfaceMessage::SetColorSelectorPreviewCornerRadius(radius) => {
            state.color_selector_preview_corner_radius = radius;
            apply_color_selector_preview_corner_radius(radius);
        }
        InterfaceMessage::SetColorSelectorPreviewMargin(margin) => {
            state.color_selector_preview_margin = margin;
            apply_color_selector_preview_margin(margin);
        }
        InterfaceMessage::SetTextboxHeight(height) => {
            state.textbox_height = height;
            apply_textbox_height(height);
        }
        InterfaceMessage::SetTextboxCornerRadius(radius) => {
            state.textbox_corner_radius = radius;
            apply_textbox_corner_radius(radius);
        }
        InterfaceMessage::SetSliderHeight(height) => {
            state.slider_height = height;
            apply_slider_height(height);
        }
        InterfaceMessage::SetFontSelectorHeight(height) => {
            state.font_selector_height = height;
            apply_font_selector_height(height);
        }
        InterfaceMessage::SetDropdownHeight(height) => {
            state.dropdown_height = height;
            apply_dropdown_height(height);
        }
        InterfaceMessage::SetButtonCornerRadius(radius) => {
            state.button_corner_radius = radius;
            apply_button_corner_radius(radius);
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
        }
        InterfaceMessage::SetLabelMargin(margin) => {
            state.label_margin = margin;
            state.label_margin_spinbox.value = margin as i32;
            apply_label_margin(margin);
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
        }
        InterfaceMessage::SetGraphGapOpacity(opacity) => {
            state.graph_gap_opacity = opacity;
            state.graph_gap_opacity_spinbox.value = (opacity * 100.0).round() as i32;
            write_config_value("graph_gap_opacity", &format!("{:.2}", opacity));
            cce_graph_reload();
        }
        InterfaceMessage::SetGraphGapWidth(gap) => {
            state.graph_gap_width = gap;
            state.graph_gap_width_spinbox.value = gap as i32;
            write_config_value("graph_gap_width", &gap.to_string());
            cce_graph_reload();
        }
        InterfaceMessage::SetMenubarOpacity(opacity) => {
            state.menubar_opacity = opacity;
            state.menubar_opacity_spinbox.value = (opacity * 100.0).round() as i32;
            write_transparency_config_value("opacity", &format!("{:.2}", opacity));
            send_ipc_command("reload");
            status_interface_reload();
        }
        InterfaceMessage::PickLowColor | InterfaceMessage::PickHighColor | InterfaceMessage::PickDisabledColor | InterfaceMessage::PickSeparatorColor | InterfaceMessage::PickVisualGuides | InterfaceMessage::PickSliderTrackColor | InterfaceMessage::PickPageLowColor | InterfaceMessage::PickColorBordersColor | InterfaceMessage::PickNormalColor | InterfaceMessage::PickPaginatorSidebarColor | InterfaceMessage::PickPrimaryHighlightColor | InterfaceMessage::PickMenubarTabLabelColor | InterfaceMessage::PickToggleEnabledColor | InterfaceMessage::PickToggleDisabledColor | InterfaceMessage::PickScrollingListBgColor | InterfaceMessage::PickBreadcrumbBgColor | InterfaceMessage::PickPopoverBgColor | InterfaceMessage::PickNotificationBgColor => {}
        InterfaceMessage::Refreshed(new) => {
            let was_mx_hovered = state.tab_margin_spinbox_x.hovered();
            let was_my_hovered = state.tab_margin_spinbox_y.hovered();
            let was_bp_hovered = state.button_padding_spinbox.hovered();
            let was_sp_hovered = state.section_padding_spinbox.hovered();
            let was_pp_hovered = state.plate_padding_spinbox.hovered();
            let was_pm_hovered = state.page_margin_spinbox.hovered();
            let was_gm_hovered = state.grid_min_col_width_spinbox.hovered();
            let was_sh_hovered = state.spinbox_height_spinbox.hovered();
            let was_scr_hovered = state.spinbox_corner_radius_spinbox.hovered();
            let was_th_hovered = state.toggle_height_spinbox.hovered();
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
            let was_lm_hovered = state.label_margin_spinbox.hovered();
            let was_mo_hovered = state.menubar_opacity_spinbox.hovered();
            let was_no_hovered = state.notification_opacity_spinbox.hovered();
            // Preserve typeface fields
            let typeface_loaded = state.typeface_loaded;
            let sans_serif = state.sans_serif.clone();
            let serif = state.serif.clone();
            let monospace = state.monospace.clone();
            let window_borders = state.window_borders.clone();
            let status_interface = state.status_interface.clone();
            let fuzzel = state.fuzzel.clone();
            let terminal = state.terminal.clone();
            let paginator = state.paginator.clone();
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

            state.tab_margin_spinbox_x.set_hovered(was_mx_hovered);
            state.tab_margin_spinbox_y.set_hovered(was_my_hovered);
            state.button_padding_spinbox.set_hovered(was_bp_hovered);
            state.section_padding_spinbox.set_hovered(was_sp_hovered);
            state.plate_padding_spinbox.set_hovered(was_pp_hovered);
            state.page_margin_spinbox.set_hovered(was_pm_hovered);
            state.grid_min_col_width_spinbox.set_hovered(was_gm_hovered);
            state.spinbox_height_spinbox.set_hovered(was_sh_hovered);
            state.spinbox_corner_radius_spinbox.set_hovered(was_scr_hovered);
            state.toggle_height_spinbox.set_hovered(was_th_hovered);
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
            state.label_margin_spinbox.set_hovered(was_lm_hovered);
            state.menubar_opacity_spinbox.set_hovered(was_mo_hovered);
            state.notification_opacity_spinbox.set_hovered(was_no_hovered);

            if typeface_loaded {
                state.typeface_loaded = typeface_loaded;
                state.sans_serif = sans_serif;
                state.serif = serif;
                state.monospace = monospace;
                state.window_borders = window_borders;
                state.status_interface = status_interface;
                state.fuzzel = fuzzel;
                state.terminal = terminal;
                state.paginator = paginator;
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
                &state.paginator,
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
                &state.paginator,
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
                &state.paginator,
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
                &state.paginator,
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
                &state.paginator,
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
                &state.paginator,
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
                &state.paginator,
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
                &state.paginator,
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
                &state.paginator,
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
                &state.paginator,
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
                &state.paginator,
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
    }
}

pub fn parse_transparency_opacity(content: &str) -> f32 {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[transparency]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("opacity") {
            if let Some(val) = trimmed.split('=').nth(1) {
                if let Ok(o) = val.trim().parse::<f32>() {
                    return o.clamp(0.0, 1.0);
                }
            }
        }
    }
    0.9 // default to 0.9
}

pub fn write_transparency_config_value(key: &str, value: &str) {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let new_line = format!("{} = {}", key, value);

    let mut found = false;
    let mut updated_lines = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[transparency]" {
            in_section = true;
            updated_lines.push(line.to_string());
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            in_section = false;
        }
        if in_section && trimmed.starts_with(key) {
            found = true;
            updated_lines.push(new_line.clone());
        } else {
            updated_lines.push(line.to_string());
        }
    }

    let mut updated = updated_lines.join("\n");

    if !found {
        let mut result = String::new();
        let has_section = content.lines().any(|l| l.trim() == "[transparency]");
        if has_section {
            let mut in_section = false;
            let mut inserted = false;
            for line in updated.lines() {
                if line.trim() == "[transparency]" {
                    in_section = true;
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }
                if line.trim().starts_with('[') && in_section {
                    if !inserted {
                        result.push_str(&new_line);
                        result.push('\n');
                        inserted = true;
                    }
                    in_section = false;
                }
                result.push_str(line);
                result.push('\n');
            }
            if !inserted {
                result.push_str(&new_line);
                result.push('\n');
            }
            updated = result;
        } else {
            updated.push_str("\n[transparency]\n");
            updated.push_str(&new_line);
            updated.push_str("\n");
        }
    }
    let _ = fs::write(CONFIG_PATH, updated);
}

fn parse_notifications_color(content: &str, key: &str, default: [u8; 3]) -> [u8; 3] {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with(key) {
            if let Some(rest) = trimmed.strip_prefix(key) {
                let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
                let hex = rest.trim_end_matches('"').trim().trim_start_matches('#');
                return parse_hex(hex);
            }
        }
    }
    default
}

fn parse_notifications_opacity(content: &str) -> f32 {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("opacity") {
            if let Some(val) = trimmed.split('=').nth(1) {
                if let Ok(o) = val.trim().parse::<f32>() {
                    return o.clamp(0.0, 1.0);
                }
            }
        }
    }
    0.9 // default to 0.9
}

fn write_notifications_config_value(key: &str, value: &str) {
    write_notifications_config_value_path(CONFIG_PATH, key, value);
}

fn write_notifications_config_value_path(path: &str, key: &str, value: &str) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let new_line = format!("{} = {}", key, value);

    let mut found = false;
    let mut updated_lines = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
            in_section = true;
            updated_lines.push(line.to_string());
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            in_section = false;
        }
        if in_section && trimmed.starts_with(key) {
            found = true;
            updated_lines.push(new_line.clone());
        } else {
            updated_lines.push(line.to_string());
        }
    }

    let mut updated = updated_lines.join("\n");

    if !found {
        let mut result = String::new();
        let has_section = content.lines().any(|l| l.trim() == "[notifications]");
        if has_section {
            let mut in_section = false;
            let mut inserted = false;
            for line in updated.lines() {
                if line.trim() == "[notifications]" {
                    in_section = true;
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }
                if line.trim().starts_with('[') && in_section {
                    if !inserted {
                        result.push_str(&new_line);
                        result.push('\n');
                        inserted = true;
                    }
                    in_section = false;
                }
                result.push_str(line);
                result.push('\n');
            }
            if !inserted {
                result.push_str(&new_line);
                result.push('\n');
            }
            updated = result;
        } else {
            updated.push_str("\n[notifications]\n");
            updated.push_str(&new_line);
            updated.push_str("\n");
        }
    }
    let _ = fs::write(path, updated);
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
    fn test_parse_color_from_key() {
        let content = "\n[layout]\nlow_color = \"#112233\"\nhigh_color = \"#445566\"\ndisabled_color = \"#778899\"\nstatus_separator_color = \"#aabbcc\"\nvisual_guides_color = \"#ddeeff\"\nslider_track_color = \"#123456\"\npage_low_color = \"#474751\"\ncolor_borders_color = \"#abcdef\"\nstatus_normal_color = \"#ccccd8\"\npaginator_sidebar_color = \"#5a5a65\"\nprimary_highlight_color = \"#ffffff\"\nmenubar_tab_label_color = \"#e6e6f2\"\ntoggle_enabled_color = \"#68d8a5\"\ntoggle_disabled_color = \"#878794\"\nscrollinglist_bg_color = \"#515161\"\nbreadcrumb_bg_color = \"#515161\"\n";
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
        assert_eq!(parse_color_from_key(content, "non_existent", [1, 2, 3]), [1, 2, 3]);
    }

    #[test]
    fn test_write_config_value_path() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Write initial file content with [layout] and other keys
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n\n[output]\nscale = 2\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Write disabled_color which does not exist yet (key not found case)
        assert!(write_config_value_path(path_str, "disabled_color", "\"#555555\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("disabled_color = \"#555555\""));
        // Check it was inserted before [output]
        assert!(updated.find("disabled_color = \"#555555\"").unwrap() < updated.find("[output]").unwrap());

        // 3. Update disabled_color (key found case)
        assert!(write_config_value_path(path_str, "disabled_color", "\"#666666\""));
        let updated2 = fs::read_to_string(path_str).unwrap();
        assert!(updated2.contains("disabled_color = \"#666666\""));
        assert!(!updated2.contains("disabled_color = \"#555555\""));

        // 4. Write visual_guides_color which does not exist yet
        assert!(write_config_value_path(path_str, "visual_guides_color", "\"#ff8c00\""));
        let updated3 = fs::read_to_string(path_str).unwrap();
        assert!(updated3.contains("visual_guides_color = \"#ff8c00\""));

        // 5. Write slider_track_color which does not exist yet
        assert!(write_config_value_path(path_str, "slider_track_color", "\"#123456\""));
        let updated4 = fs::read_to_string(path_str).unwrap();
        assert!(updated4.contains("slider_track_color = \"#123456\""));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse section_padding when missing (should return default 8)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "section_padding", 8);
        assert_eq!(val, 8);

        // 3. Write section_padding config
        assert!(write_config_value_path(path_str, "section_padding", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("section_padding = 12"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse plate_padding when missing (should return default 20)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "plate_padding", 20);
        assert_eq!(val, 20);

        // 3. Write plate_padding config
        assert!(write_config_value_path(path_str, "plate_padding", "15"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("plate_padding = 15"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse page_margin when missing (should return default 20)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "page_margin", 20);
        assert_eq!(val, 20);

        // 3. Write page_margin config
        assert!(write_config_value_path(path_str, "page_margin", "15"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("page_margin = 15"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse spinbox_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "spinbox_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write spinbox_corner_radius config
        assert!(write_config_value_path(path_str, "spinbox_corner_radius", "8"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("spinbox_corner_radius = 8"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse spinbox_height when missing (should return default 26)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "spinbox_height", 26);
        assert_eq!(val, 26);

        // 3. Write spinbox_height config
        assert!(write_config_value_path(path_str, "spinbox_height", "30"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("spinbox_height = 30"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse toggle_height when missing (should return default 44)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "toggle_height", 44);
        assert_eq!(val, 44);

        // 3. Write toggle_height config
        assert!(write_config_value_path(path_str, "toggle_height", "52"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("toggle_height = 52"));

        // 4. Parse toggle_height when present (should return written value 52)
        let val2 = parse_u16_from(&updated, "toggle_height", 44);
        assert_eq!(val2, 52);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_color_selector_height() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_color_selector_height_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_height when missing (should return default 22)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "color_selector_height", 22);
        assert_eq!(val, 22);

        // 3. Write color_selector_height config
        assert!(write_config_value_path(path_str, "color_selector_height", "28"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("color_selector_height = 28"));

        // 4. Parse color_selector_height when present (should return written value 28)
        let val2 = parse_u16_from(&updated, "color_selector_height", 22);
        assert_eq!(val2, 28);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_textbox_corner_radius() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_textbox_corner_radius_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse textbox_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "textbox_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write textbox_corner_radius config
        assert!(write_config_value_path(path_str, "textbox_corner_radius", "8"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("textbox_corner_radius = 8"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse textbox_height when missing (should return default 44)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "textbox_height", 44);
        assert_eq!(val, 44);

        // 3. Write textbox_height config
        assert!(write_config_value_path(path_str, "textbox_height", "48"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("textbox_height = 48"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_font when missing (should return default "monospace")
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "color_selector_font", "monospace");
        assert_eq!(val, "monospace");

        // 3. Write color_selector_font config
        assert!(write_config_value_path(path_str, "color_selector_font", "\"Berkeley Mono\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("color_selector_font = \"Berkeley Mono\""));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse menubar_font when missing (should return default "Outfit")
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_string_from(&content, "menubar_font", "Outfit");
        assert_eq!(val, "Outfit");

        // 3. Write menubar_font config
        assert!(write_config_value_path(path_str, "menubar_font", "\"Inter\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("menubar_font = \"Inter\""));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse font_selector_height when missing (should return default 44)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "font_selector_height", 44);
        assert_eq!(val, 44);

        // 3. Write font_selector_height config
        assert!(write_config_value_path(path_str, "font_selector_height", "48"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("font_selector_height = 48"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse grid_min_col_width when missing (should return default 260)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "grid_min_col_width", 260);
        assert_eq!(val, 260);

        // 3. Write grid_min_col_width config
        assert!(write_config_value_path(path_str, "grid_min_col_width", "280"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("grid_min_col_width = 280"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_preview_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "color_selector_preview_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write color_selector_preview_corner_radius config
        assert!(write_config_value_path(path_str, "color_selector_preview_corner_radius", "8"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("color_selector_preview_corner_radius = 8"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_corner_radius when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "color_selector_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write color_selector_corner_radius config
        assert!(write_config_value_path(path_str, "color_selector_corner_radius", "6"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("color_selector_corner_radius = 6"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse color_selector_preview_margin when missing (should return default 0)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "color_selector_preview_margin", 0);
        assert_eq!(val, 0);

        // 3. Write color_selector_preview_margin config
        assert!(write_config_value_path(path_str, "color_selector_preview_margin", "3"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("color_selector_preview_margin = 3"));

        // 4. Parse color_selector_preview_margin when present (should return written value 3)
        let val2 = parse_u16_from(&updated, "color_selector_preview_margin", 0);
        assert_eq!(val2, 3);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_paginator_tab_margin_x() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_paginator_tab_margin_x_config.toml");
        let path_str = path.to_str().unwrap();

        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "paginator_tab_margin_x", 5);
        assert_eq!(val, 5);

        assert!(write_config_value_path(path_str, "paginator_tab_margin_x", "8"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("paginator_tab_margin_x = 8"));

        let val2 = parse_u16_from(&updated, "paginator_tab_margin_x", 5);
        assert_eq!(val2, 8);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_paginator_tab_margin_y() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_paginator_tab_margin_y_config.toml");
        let path_str = path.to_str().unwrap();

        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "paginator_tab_margin_y", 10);
        assert_eq!(val, 10);

        assert!(write_config_value_path(path_str, "paginator_tab_margin_y", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("paginator_tab_margin_y = 12"));

        let val2 = parse_u16_from(&updated, "paginator_tab_margin_y", 10);
        assert_eq!(val2, 12);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_button_padding() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_button_padding_config.toml");
        let path_str = path.to_str().unwrap();

        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "button_padding", 14);
        assert_eq!(val, 14);

        assert!(write_config_value_path(path_str, "button_padding", "20"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("button_padding = 20"));

        let val2 = parse_u16_from(&updated, "button_padding", 14);
        assert_eq!(val2, 20);

        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_slider_height() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_slider_height_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse slider_height when missing (should return default 28)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "slider_height", 28);
        assert_eq!(val, 28);

        // 3. Write slider_height config
        assert!(write_config_value_path(path_str, "slider_height", "32"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("slider_height = 32"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when missing (should return default 0)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "nested_section_label_alignment", 0);
        assert_eq!(val, 0);

        // 3. Write alignment config
        assert!(write_config_value_path(path_str, "nested_section_label_alignment", "2"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("nested_section_label_alignment = 2"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when missing (should return default 0)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_i16_from(&content, "nested_section_label_offset", 0);
        assert_eq!(val, 0);

        // 3. Write alignment config
        assert!(write_config_value_path(path_str, "nested_section_label_offset", "-15"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("nested_section_label_offset = -15"));

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
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse dropdown_height when missing (should return default 44)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "dropdown_height", 44);
        assert_eq!(val, 44);

        // 3. Write dropdown_height config
        assert!(write_config_value_path(path_str, "dropdown_height", "48"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("dropdown_height = 48"));

        // 4. Parse dropdown_height when present (should return written value 48)
        let val2 = parse_u16_from(&updated, "dropdown_height", 44);
        assert_eq!(val2, 48);

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_read_write_label_margin() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_label_margin_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Initial configuration
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when missing (should return default 6)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "label_margin", 6);
        assert_eq!(val, 6);

        // 3. Write label_margin config
        assert!(write_config_value_path(path_str, "label_margin", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("label_margin = 12"));

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
        let initial_content = "[transparency]\nopacity = 0.85\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when present (should return 0.85)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_transparency_opacity(&content);
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
        let initial_content = "[layout]\ngap = 18\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse when missing (should return default 4)
        let content = fs::read_to_string(path_str).unwrap();
        let val = parse_u16_from(&content, "button_corner_radius", 4);
        assert_eq!(val, 4);

        // 3. Write button_corner_radius config
        assert!(write_config_value_path(path_str, "button_corner_radius", "12"));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("button_corner_radius = 12"));

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
        let initial_content = "[notifications]\nenable = true\n\n[layout]\ngap = 18\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Parse opacity when missing (should return default 0.9)
        let content = fs::read_to_string(path_str).unwrap();
        let opacity = parse_notifications_opacity(&content);
        assert_eq!(opacity, 0.9);

        // 3. Write opacity config
        write_notifications_config_value_path(path_str, "opacity", "0.85");
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("opacity = 0.85"));

        // 4. Parse opacity when present (should return 0.85)
        let opacity2 = parse_notifications_opacity(&updated);
        assert_eq!(opacity2, 0.85);

        // 5. Write bg_color config
        write_notifications_config_value_path(path_str, "bg_color", "\"#112233\"");
        let updated2 = fs::read_to_string(path_str).unwrap();
        assert!(updated2.contains("bg_color = \"#112233\""));

        // 6. Parse bg_color when present
        let bg_color = parse_notifications_color(&updated2, "bg_color", [0, 0, 0]);
        assert_eq!(bg_color, [17, 34, 51]);

        // Clean up
        let _ = fs::remove_file(path_str);
    }
}



