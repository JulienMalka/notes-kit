use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use leptos::prelude::*;

use crate::ast::Element;

#[derive(Clone, Default)]
pub struct RenderContext {
    pub id_map: HashMap<String, String>,
    pub asset_map: HashMap<String, String>,
    pub accessible_ids: HashSet<String>,
    pub config: RenderConfig,
    pub notes_prefix: String,
    pub assets_prefix: String,
    pub note_dir: String,
    pub in_table: bool,
    pub in_item: bool,
}

impl RenderContext {
    pub fn new(id_map: HashMap<String, String>, accessible_ids: HashSet<String>) -> Self {
        Self {
            id_map,
            asset_map: HashMap::new(),
            accessible_ids,
            config: RenderConfig::default(),
            notes_prefix: "/notes".into(),
            assets_prefix: "/assets".into(),
            note_dir: String::new(),
            in_table: false,
            in_item: false,
        }
    }

    pub fn with_config(mut self, config: RenderConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_notes_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.notes_prefix = prefix.into();
        self
    }

    pub fn with_asset_map(mut self, asset_map: HashMap<String, String>, prefix: impl Into<String>) -> Self {
        self.asset_map = asset_map;
        self.assets_prefix = prefix.into();
        self
    }

    pub fn with_assets_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.assets_prefix = prefix.into();
        self
    }

    pub fn with_note_dir(mut self, dir: impl Into<String>) -> Self {
        self.note_dir = dir.into();
        self
    }

    pub fn inside_table(&self) -> Self {
        Self { in_table: true, ..self.clone() }
    }

    pub fn inside_item(&self) -> Self {
        Self { in_item: true, ..self.clone() }
    }
}

pub type RenderFn = Arc<dyn Fn(&Element, &RenderContext) -> AnyView + Send + Sync>;

#[derive(Default, Clone)]
pub struct RenderConfig {
    pub paragraph: Option<RenderFn>,
    pub heading: Option<RenderFn>,
    pub block_quote: Option<RenderFn>,
    pub code_block: Option<RenderFn>,
    pub list: Option<RenderFn>,
    pub list_item: Option<RenderFn>,
    pub table: Option<RenderFn>,
    pub emphasis: Option<RenderFn>,
    pub strong: Option<RenderFn>,
    pub link: Option<RenderFn>,
    pub image: Option<RenderFn>,
}

macro_rules! with_renderer {
    ($name:ident) => {
        pub fn $name<F>(mut self, f: F) -> Self
        where
            F: Fn(&Element, &RenderContext) -> AnyView + Send + Sync + 'static,
        {
            self.$name = Some(Arc::new(f));
            self
        }
    };
}

impl RenderConfig {
    with_renderer!(paragraph);
    with_renderer!(heading);
    with_renderer!(block_quote);
    with_renderer!(code_block);
    with_renderer!(list);
    with_renderer!(list_item);
    with_renderer!(table);
    with_renderer!(emphasis);
    with_renderer!(strong);
    with_renderer!(link);
    with_renderer!(image);
}
