use eframe::egui::Color32;

// Color Palette
// Primary Colors
pub const PRIMARY_BUTTON_BG: Color32 = Color32::from_rgb(76, 154, 255);  // Vibrant blue for primary actions
pub const SECONDARY_BUTTON_BG: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 0);  // Softer blue for secondary actions

// Background & Surface Colors
pub const WHITE: Color32 = Color32::from_rgb(255, 255, 255);  // Pure white cards
pub const DARK_GRAY: Color32 = Color32::from_rgb(36, 36, 36);  // Dark gray for primary text

// Text Colors
pub const BUTTON_MAIN_TEXT: Color32 = Color32::from_rgb(255, 255, 255);  // White text for buttons
pub const MAIN_TEXT: Color32 = Color32::from_rgb(255,255,255);  // Dark gray for primary text
pub const SECONDARY_TEXT: Color32 = Color32::from_rgb(138, 138, 143);  // Medium gray for secondary text
pub const TEXT_ERROR: Color32 = Color32::from_rgb(255, 0, 0);  // Red for error messages
pub const TEXT_SUCCESS: Color32 = Color32::from_rgb(0, 255, 0);  // Green for success messages

// Input color
pub const INPUT_BG: Color32 = WHITE;
pub const INPUT_TEXT: Color32 = DARK_GRAY;

// UI Elements
pub const BORDER_COLOR: Color32 = Color32::from_rgba_premultiplied(60, 60, 67, 15);  // Subtle border

// Sizing & Spacing
pub const ROUNDING_FRAME: f32 = 2.0;
pub const ROUNDING_BUTTON: f32 = 6.0;
pub const MIN_SIZE_BUTTON: egui::Vec2 = egui::Vec2::new(120.0, 40.0);

// For backward compatibility
pub const PRIMARY_COLOR: Color32 = PRIMARY_BUTTON_BG;

pub const BUTTON_FONT_SIZE: f32 = 16.0;