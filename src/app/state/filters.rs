use crossterm::event::{KeyCode, KeyEvent};

use crate::model::{FilterOption, LocationMode};

use super::AppState;
use crate::app::protocol::{BackendRequest, Effect, InputMode, Screen};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupKind {
    LocationMode,
    District,
    BusinessArea,
    SubwayLine,
    Station,
    Commute,
    RentalType,
    Rent,
    Bedroom,
    Sort,
}

#[derive(Debug, Clone)]
pub(crate) struct FilterGroup {
    pub(crate) title: String,
    pub(crate) kind: GroupKind,
    pub(crate) options: Vec<FilterOption>,
}

impl AppState {
    pub(crate) fn groups(&self) -> Vec<FilterGroup> {
        let Some(catalog) = &self.catalog else {
            return Vec::new();
        };
        let modes = LocationMode::ALL
            .into_iter()
            .map(|mode| FilterOption {
                label: mode.label().into(),
                url: None,
                selected: mode == catalog.mode,
            })
            .collect();
        let mut groups = vec![FilterGroup {
            title: "Location mode".into(),
            kind: GroupKind::LocationMode,
            options: modes,
        }];
        match catalog.mode {
            LocationMode::Region => {
                groups.push(FilterGroup {
                    title: "District / 区域".into(),
                    kind: GroupKind::District,
                    options: catalog.districts.clone(),
                });
                if !catalog.business_areas.is_empty() {
                    groups.push(FilterGroup {
                        title: "Business areas / 商圈".into(),
                        kind: GroupKind::BusinessArea,
                        options: catalog.business_areas.clone(),
                    });
                }
            }
            LocationMode::Subway => {
                groups.push(FilterGroup {
                    title: "Subway line / 地铁".into(),
                    kind: GroupKind::SubwayLine,
                    options: catalog.subway_lines.clone(),
                });
                if !catalog.stations.is_empty() {
                    groups.push(FilterGroup {
                        title: "Stations / 站点".into(),
                        kind: GroupKind::Station,
                        options: catalog.stations.clone(),
                    });
                }
            }
            LocationMode::Commute => groups.push(FilterGroup {
                title: "Commute / 通勤".into(),
                kind: GroupKind::Commute,
                options: vec![FilterOption::new("Press a to enter company address", None)],
            }),
        }
        groups.extend([
            FilterGroup {
                title: "Rental type / 类型".into(),
                kind: GroupKind::RentalType,
                options: catalog.rental_types.clone(),
            },
            FilterGroup {
                title: "Rent / 租金".into(),
                kind: GroupKind::Rent,
                options: catalog.rent_presets.clone(),
            },
            FilterGroup {
                title: "Bedrooms / 户型".into(),
                kind: GroupKind::Bedroom,
                options: catalog.bedrooms.clone(),
            },
            FilterGroup {
                title: "Sort".into(),
                kind: GroupKind::Sort,
                options: catalog.sort_orders.clone(),
            },
        ]);
        groups.retain(|group| !group.options.is_empty());
        groups
    }

    pub(super) fn handle_filter_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        let groups = self.groups();
        if groups.is_empty() {
            return Vec::new();
        }
        match key.code {
            KeyCode::Left | KeyCode::BackTab => {
                self.category_index = self.category_index.saturating_sub(1);
                self.option_index = 0;
            }
            KeyCode::Right | KeyCode::Tab => {
                self.category_index = (self.category_index + 1).min(groups.len() - 1);
                self.option_index = 0;
            }
            KeyCode::Up => self.option_index = self.option_index.saturating_sub(1),
            KeyCode::Down => {
                self.option_index = (self.option_index + 1)
                    .min(groups[self.category_index].options.len().saturating_sub(1));
            }
            KeyCode::Char(' ') | KeyCode::Enter if !self.busy => {
                let group = groups[self.category_index].clone();
                let option = group.options.get(self.option_index).cloned();
                match group.kind {
                    GroupKind::LocationMode => {
                        let mode = LocationMode::ALL[self.option_index.min(2)];
                        self.busy = true;
                        return vec![Effect::Backend(BackendRequest::SetMode(mode))];
                    }
                    GroupKind::Commute => self.open_commute_input(),
                    _ => {
                        if let Some(url) = option.and_then(|value| value.url) {
                            self.busy = true;
                            self.status = "Applying filter…".into();
                            return vec![Effect::Backend(BackendRequest::Navigate(url))];
                        }
                    }
                }
            }
            KeyCode::Char('p') if !self.busy => {
                self.input = Some(InputMode::CustomRent {
                    min: String::new(),
                    max: String::new(),
                    editing_max: false,
                });
                self.input_error = None;
            }
            KeyCode::Char('a') if !self.busy => self.open_commute_input(),
            KeyCode::Char('s') if !self.busy => {
                self.scraping = true;
                self.busy = true;
                self.screen = Screen::Progress;
                self.status = "Starting full scrape…".into();
                return vec![Effect::Backend(BackendRequest::Scrape)];
            }
            KeyCode::Char('r') if !self.busy => {
                self.busy = true;
                self.screen = Screen::Loading;
                self.status = "Resetting filters…".into();
                return vec![Effect::Backend(BackendRequest::Reset)];
            }
            KeyCode::Char('?') => self.show_help(),
            KeyCode::Char('q') => return self.request_quit(),
            _ => {}
        }
        Vec::new()
    }
}
