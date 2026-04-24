use leptos::prelude::*;
use orgize::ast::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct RenderContext {
    pub id_map: HashMap<String, String>,
    pub asset_map: HashMap<String, String>,
    pub accessible_ids: HashSet<String>,
    pub config: RenderConfig,
    pub notes_prefix: String,
    pub assets_prefix: String,
}

impl RenderContext {
    pub fn new(
        id_map: HashMap<String, String>,
        accessible_ids: HashSet<String>,
    ) -> Self {
        Self {
            id_map,
            asset_map: HashMap::new(),
            accessible_ids,
            config: RenderConfig::default(),
            notes_prefix: "/notes".into(),
            assets_prefix: "/assets".into(),
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

    pub fn with_asset_map(
        mut self,
        asset_map: HashMap<String, String>,
        prefix: impl Into<String>,
    ) -> Self {
        self.asset_map = asset_map;
        self.assets_prefix = prefix.into();
        self
    }
}

pub type RenderFn<T> = Arc<dyn Fn(&T, &RenderContext) -> AnyView + Send + Sync>;

#[derive(Default, Clone)]
pub struct RenderConfig {
    pub link: Option<RenderFn<Link>>,
    pub source_block: Option<RenderFn<SourceBlock>>,
    pub quote_block: Option<RenderFn<QuoteBlock>>,
    pub example_block: Option<RenderFn<ExampleBlock>>,
    pub headline: Option<RenderFn<Headline>>,
    pub paragraph: Option<RenderFn<Paragraph>>,
    pub code: Option<RenderFn<Code>>,
    pub verbatim: Option<RenderFn<Verbatim>>,
    pub bold: Option<RenderFn<Bold>>,
    pub italic: Option<RenderFn<Italic>>,
    pub strike: Option<RenderFn<Strike>>,
    pub underline: Option<RenderFn<Underline>>,
    pub list: Option<RenderFn<List>>,
    pub list_item: Option<RenderFn<ListItem>>,
    pub org_table: Option<RenderFn<OrgTable>>,
}

macro_rules! with_renderer {
    ($name:ident, $type:ty) => {
        pub fn $name<F>(mut self, f: F) -> Self
        where
            F: Fn(&$type, &RenderContext) -> AnyView + Send + Sync + 'static,
        {
            self.$name = Some(Arc::new(f));
            self
        }
    };
}

impl RenderConfig {
    with_renderer!(link, Link);
    with_renderer!(source_block, SourceBlock);
    with_renderer!(quote_block, QuoteBlock);
    with_renderer!(example_block, ExampleBlock);
    with_renderer!(headline, Headline);
    with_renderer!(paragraph, Paragraph);
    with_renderer!(code, Code);
    with_renderer!(verbatim, Verbatim);
    with_renderer!(bold, Bold);
    with_renderer!(italic, Italic);
    with_renderer!(strike, Strike);
    with_renderer!(underline, Underline);
    with_renderer!(list, List);
    with_renderer!(list_item, ListItem);
    with_renderer!(org_table, OrgTable);
}
