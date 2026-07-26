use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FilterOption {
    pub(crate) label: String,
    pub(crate) url: Option<Url>,
    #[serde(default)]
    pub(crate) selected: bool,
}

impl FilterOption {
    pub(crate) fn new(label: impl Into<String>, url: Option<Url>) -> Self {
        Self {
            label: label.into(),
            url,
            selected: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocationMode {
    #[default]
    Region,
    Subway,
    Commute,
}

impl LocationMode {
    pub(crate) const ALL: [Self; 3] = [Self::Region, Self::Subway, Self::Commute];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Region => "区域",
            Self::Subway => "地铁",
            Self::Commute => "通勤",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FilterCatalog {
    pub(crate) mode: LocationMode,
    pub(crate) districts: Vec<FilterOption>,
    pub(crate) business_areas: Vec<FilterOption>,
    pub(crate) subway_lines: Vec<FilterOption>,
    pub(crate) stations: Vec<FilterOption>,
    pub(crate) rental_types: Vec<FilterOption>,
    pub(crate) rent_presets: Vec<FilterOption>,
    pub(crate) bedrooms: Vec<FilterOption>,
    pub(crate) sort_orders: Vec<FilterOption>,
    pub(crate) selected_labels: Vec<String>,
    pub(crate) current_url: Url,
}

impl Default for FilterCatalog {
    fn default() -> Self {
        Self {
            mode: LocationMode::Region,
            districts: Vec::new(),
            business_areas: Vec::new(),
            subway_lines: Vec::new(),
            stations: Vec::new(),
            rental_types: Vec::new(),
            rent_presets: Vec::new(),
            bedrooms: Vec::new(),
            sort_orders: Vec::new(),
            selected_labels: Vec::new(),
            current_url: Url::parse(crate::ZIROOM_BASE_URL).expect("valid Ziroom base URL"),
        }
    }
}
