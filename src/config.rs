//! Project-wide configuration.

use serde::{Deserialize, Serialize};

/// Grid precision lower bound in nanometers (100 nm per requirements).
pub const MIN_GRID_NM: u32 = 100;

pub const DEFAULT_GRID_NM: u32 = 150;
pub const DEFAULT_FONT_SIZE_PT: f32 = 18.0;
pub const DEFAULT_FILL_DENSITY: f32 = 0.35;
pub const DEFAULT_FONT_NAME: &str = "Sarasa Mono SC";

/// GDS layer assignments. Defaults to sky130 met1.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LayerConfig {
    pub text_layer: i16,
    pub text_datatype: i16,
    pub fill_layer: i16,
    pub fill_datatype: i16,
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            text_layer: 68,
            text_datatype: 20,
            fill_layer: 68,
            fill_datatype: 44,
        }
    }
}

/// Sky130-inspired design rule minimums in nanometers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DesignRules {
    pub min_width_nm: u32,
    pub min_spacing_nm: u32,
    pub fill_to_metal_spacing_nm: u32,
}

impl Default for DesignRules {
    fn default() -> Self {
        Self {
            min_width_nm: 200,
            min_spacing_nm: 200,
            fill_to_metal_spacing_nm: 400,
        }
    }
}

/// A single placed text snippet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSnippet {
    pub id: u64,
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    pub rotation_deg: f32,
}

impl TextSnippet {
    pub fn new(id: u64, text: impl Into<String>, x: f32, y: f32) -> Self {
        Self {
            id,
            text: text.into(),
            x,
            y,
            font_size: DEFAULT_FONT_SIZE_PT,
            rotation_deg: 0.0,
        }
    }
}

/// Overall project state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub grid_nm: u32,
    pub fill_density: f32,
    pub font_name: String,
    pub layers: LayerConfig,
    pub rules: DesignRules,
    pub canvas_width_px: u32,
    pub canvas_height_px: u32,
    pub snippets: Vec<TextSnippet>,
    #[serde(skip)]
    pub next_snippet_id: u64,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            grid_nm: DEFAULT_GRID_NM,
            fill_density: DEFAULT_FILL_DENSITY,
            font_name: DEFAULT_FONT_NAME.to_string(),
            layers: LayerConfig::default(),
            rules: DesignRules::default(),
            canvas_width_px: 800,
            canvas_height_px: 500,
            snippets: Vec::new(),
            next_snippet_id: 1,
        }
    }
}

impl ProjectConfig {
    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_snippet_id;
        self.next_snippet_id += 1;
        id
    }
}
