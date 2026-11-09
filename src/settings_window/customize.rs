//! Customize: popup layout, tabs, card presentation and the Usage widget,
//! plus the per-instance popup card table shown on provider pages.

use gpui::{
    AnyElement, AppContext, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use super::kit::{self, Button, Kit, Row};
use super::window::{DraggedProvider, SettingsWindow};
use crate::popup_window::ui::theme::rgb8;
use crate::settings::{
    FloatingCircleColorMode, FloatingCircleColors, PopupPlacement, PopupTabMode, ProviderInstance,
    Settings, TotalSpendPresentation, TrayFixedColor,
};

fn bool_row(
    this: &SettingsWindow,
    k: &mut Kit,
    cx: &mut Context<SettingsWindow>,
    id: &'static str,
    title: &'static str,
    description: Option<&'static str>,
    read: fn(&Settings) -> bool,
    write: fn(&mut Settings, bool),
) -> AnyElement {
    kit::toggle_row(
        k,
        id,
        title,
        description.map(SharedString::from),
        read(&this.settings),
        SettingsWindow::h(cx, move |this, value: bool, _, cx| {
            this.edit(cx, move |settings| write(settings, value))
        }),
    )
}

impl SettingsWindow {
    /// Provider order shared by popup tabs, the Settings sidebar and
    /// floating-circle target choices. Icons support both drag/drop and a
    /// precise click-source, click-destination interaction.
    fn provider_order_row(&mut self, k: &Kit, cx: &mut Context<Self>) -> AnyElement {
        let theme = k.theme.clone();
        let picked = self.provider_order_pick.as_deref();
        let mut icons = div().flex().flex_wrap().items_center().gap(px(6.0));
        for instance in &self.settings.instances {
            let id = instance.id.clone();
            let selected = picked == Some(id.as_str());
            let rest = if selected {
                theme.accent_soft
            } else {
                gpui::transparent_black()
            };
            let tile_id = format!("provider-order-{id}");
            let target_id = id.clone();
            let click_target = id.clone();
            let drag_target = id.clone();
            let drop_color = theme.accent_soft;
            let dragged = DraggedProvider {
                id,
                enabled: instance.enabled,
                label: instance.display_name().into(),
                theme: theme.clone(),
            };
            let tile = kit::hover_bg(
                k,
                div().id(kit::eid(tile_id.clone())),
                kit::hover_key(&tile_id),
                rest,
                theme.subtle_hover,
            )
            .relative()
            .size(px(42.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(8.0))
            .border_1()
            .border_color(if selected {
                theme.accent
            } else {
                theme.divider
            })
            .cursor_pointer()
            .opacity(if instance.enabled { 1.0 } else { 0.52 })
            .on_click(cx.listener(
                move |this, _, _, cx| match this.provider_order_pick.take() {
                    Some(source) if source != click_target => {
                        this.reorder_provider_any(&source, &click_target, cx)
                    }
                    Some(_) => cx.notify(),
                    None => {
                        this.provider_order_pick = Some(click_target.clone());
                        cx.notify();
                    }
                },
            ))
            .child(kit::provider_mark(
                k,
                crate::provider_registry::icon(instance.driver),
                22.0,
                theme.brand(instance.driver),
                None,
            ));
            let tile = kit::with_tooltip(
                k,
                tile,
                format!(
                    "{} · {}",
                    instance.display_name(),
                    crate::i18n::tr("drag-to-reorder")
                ),
            );
            icons = icons.child(
                tile.on_drag(dragged, |dragged, _, _, cx| cx.new(|_| dragged.clone()))
                    .drag_over::<DraggedProvider>(move |style, dragged, _, _| {
                        if dragged.id != target_id {
                            style.bg(drop_color)
                        } else {
                            style
                        }
                    })
                    .on_drop(cx.listener(move |this, dragged: &DraggedProvider, _, cx| {
                        this.reorder_provider_any(&dragged.id, &drag_target, cx);
                    })),
            );
        }
        Row::new(
            "customize-provider-order",
            crate::i18n::tr("provider-tab-order"),
        )
        .description(k, crate::i18n::tr("provider-tab-order-description"))
        .detail(div().pt(px(10.0)).child(icons).into_any_element())
        .render(k)
    }

    #[allow(clippy::too_many_arguments)]
    fn floating_color_input(
        &mut self,
        k: &Kit,
        id: &'static str,
        label: &'static str,
        current: TrayFixedColor,
        set: fn(&mut FloatingCircleColors, TrayFixedColor),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let hex = format!(
            "#{:02X}{:02X}{:02X}",
            current.red, current.green, current.blue
        );
        let input = self.input(
            format!("floating-circle-{id}"),
            &hex,
            "#RRGGBB",
            false,
            false,
            window,
            cx,
            move |this, value, _, _, cx| {
                if let Some(color) = parse_hex(&value)
                    && color != current
                {
                    this.edit(cx, move |settings| {
                        set(&mut settings.floating_circle_colors, color)
                    });
                }
            },
        );
        let preview = div()
            .size(px(kit::CONTROL_HEIGHT))
            .flex_none()
            .rounded(px(6.0))
            .bg(rgb8((current.red, current.green, current.blue)));
        let control = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(preview)
            .child(kit::text_field(k, &input, None, window, cx));
        kit::field(k, label, control.into_any_element())
    }

    pub(super) fn customize_page(
        &mut self,
        k: &mut Kit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let show_edit_button = self.settings.show_edit_home_button;
        let topmost = cfg!(windows) || cx.compositor_name() == "X11";
        let floating = kit::card_of(k, |k| {
            let reset = Button::new(
                "popup-reset-position",
                crate::i18n::tr("popup-reset-position"),
            )
            .on_click(Self::h(cx, |this, (), _, cx| {
                this.edit(cx, |s| s.popup_position = None)
            }))
            .render(k);
            let reset_circle = Button::new(
                "floating-circle-reset-position",
                crate::i18n::tr("popup-reset-position"),
            )
            .on_click(Self::h(cx, |this, (), _, cx| {
                this.edit(cx, |s| s.floating_circle_position = None)
            }))
            .render(k);
            let mut rows = vec![
                bool_row(
                    self,
                    k,
                    cx,
                    "popup-floating",
                    crate::i18n::tr("popup-floating"),
                    Some(crate::i18n::tr("popup-floating-description")),
                    |s| s.popup_floating,
                    |s, v| {
                        s.popup_floating = v;
                        if v {
                            s.popup_pinned = true;
                        }
                    },
                ),
                bool_row(
                    self,
                    k,
                    cx,
                    "popup-always-on-top",
                    crate::i18n::tr(if topmost {
                        "popup-always-on-top"
                    } else {
                        "linux-popup-keep-open"
                    }),
                    Some(crate::i18n::tr(if topmost {
                        "popup-always-on-top-description"
                    } else {
                        "linux-popup-keep-open-description"
                    })),
                    |s| s.popup_pinned,
                    |s, v| s.popup_pinned = v,
                ),
                kit::dropdown_row(
                    k,
                    "popup-placement",
                    crate::i18n::tr("popup-placement"),
                    Some(crate::i18n::tr(if cfg!(windows) {
                        "popup-placement-description"
                    } else if topmost {
                        "linux-popup-placement-description"
                    } else {
                        "linux-popup-placement-wayland"
                    })),
                    PopupPlacement::choices()
                        .iter()
                        .map(|placement| SharedString::from(placement.label()))
                        .collect(),
                    PopupPlacement::choices()
                        .iter()
                        .position(|placement| {
                            *placement == self.settings.popup_placement.resolved()
                        })
                        .map_or(-1, |index| index as i32),
                    !topmost,
                    Self::h(cx, |this, index: usize, _, cx| {
                        let placement = PopupPlacement::choices()[index];
                        // The new spot replaces wherever the panel was dragged.
                        this.edit(cx, move |s| {
                            s.popup_placement = placement;
                            s.popup_position = None;
                        })
                    }),
                ),
                Row::new("popup-position", crate::i18n::tr("popup-position"))
                    .description(k, crate::i18n::tr("popup-position-description"))
                    .trailing(reset)
                    .render(k),
            ];
            if cfg!(windows) {
                rows.push(bool_row(
                    self,
                    k,
                    cx,
                    "floating-circle",
                    crate::i18n::tr("floating-circle"),
                    Some(crate::i18n::tr("floating-circle-description")),
                    |s| s.floating_circle_enabled,
                    |s, v| s.floating_circle_enabled = v,
                ));
                let providers = self.settings.enabled_providers();
                let provider_labels = providers
                    .iter()
                    .map(|provider| SharedString::from(provider.qualified_name()))
                    .collect::<Vec<_>>();
                let primary_provider = self
                    .settings
                    .floating_circle_slots
                    .first()
                    .and_then(|slot| slot.provider())
                    .or_else(|| providers.first().copied());
                let primary_index = primary_provider
                    .and_then(|provider| providers.iter().position(|item| *item == provider))
                    .map_or(-1, |index| index as i32);
                let primary_choices = providers.clone();
                rows.push(kit::dropdown_row(
                    k,
                    "floating-circle-primary-target",
                    crate::i18n::tr("floating-circle-primary-target"),
                    Some(crate::i18n::tr("floating-circle-target-description")),
                    provider_labels.clone(),
                    primary_index,
                    providers.is_empty(),
                    Self::h(cx, move |this, index: usize, _, cx| {
                        let Some(provider) = primary_choices.get(index).copied() else {
                            return;
                        };
                        this.edit(cx, move |settings| {
                            settings.set_floating_circle_provider(0, provider)
                        })
                    }),
                ));
                let second_enabled = self.settings.floating_circle_slots.len() > 1;
                rows.push(kit::toggle_row(
                    k,
                    "floating-circle-second",
                    crate::i18n::tr("floating-circle-second"),
                    Some(crate::i18n::tr("floating-circle-second-description").into()),
                    second_enabled,
                    Self::h(cx, move |this, enabled: bool, _, cx| {
                        this.edit(cx, move |settings| {
                            settings.set_second_floating_circle(enabled)
                        })
                    }),
                ));
                if second_enabled {
                    let second_provider = self.settings.floating_circle_slots[1].provider();
                    let second_index = second_provider
                        .and_then(|provider| providers.iter().position(|item| *item == provider))
                        .map_or(-1, |index| index as i32);
                    let second_choices = providers.clone();
                    rows.push(kit::dropdown_row(
                        k,
                        "floating-circle-secondary-target",
                        crate::i18n::tr("floating-circle-secondary-target"),
                        Some(crate::i18n::tr("floating-circle-target-description")),
                        provider_labels,
                        second_index,
                        providers.is_empty(),
                        Self::h(cx, move |this, index: usize, _, cx| {
                            let Some(provider) = second_choices.get(index).copied() else {
                                return;
                            };
                            this.edit(cx, move |settings| {
                                settings.set_floating_circle_provider(1, provider)
                            })
                        }),
                    ));
                }
                let colors = self.settings.floating_circle_colors;
                rows.push(kit::dropdown_row(
                    k,
                    "floating-circle-color-mode",
                    crate::i18n::tr("floating-circle-color-mode"),
                    None,
                    vec![
                        SharedString::from(crate::i18n::tr("floating-circle-color-gradient")),
                        SharedString::from(crate::i18n::tr("fixed-color")),
                    ],
                    i32::from(colors.mode == FloatingCircleColorMode::Solid),
                    false,
                    Self::h(cx, |this, index: usize, _, cx| {
                        this.edit(cx, move |settings| {
                            settings.floating_circle_colors.mode = if index == 1 {
                                FloatingCircleColorMode::Solid
                            } else {
                                FloatingCircleColorMode::Gradient
                            };
                        })
                    }),
                ));
                let color_fields = if colors.mode == FloatingCircleColorMode::Gradient {
                    let low = self.floating_color_input(
                        k,
                        "color-low",
                        "0%",
                        colors.low,
                        |colors, value| colors.low = value,
                        window,
                        cx,
                    );
                    let warning = self.floating_color_input(
                        k,
                        "color-warning",
                        "50%",
                        colors.warning,
                        |colors, value| colors.warning = value,
                        window,
                        cx,
                    );
                    let safe = self.floating_color_input(
                        k,
                        "color-safe",
                        ">50%",
                        colors.safe,
                        |colors, value| colors.safe = value,
                        window,
                        cx,
                    );
                    let high = self.floating_color_input(
                        k,
                        "color-high",
                        "100%",
                        colors.high,
                        |colors, value| colors.high = value,
                        window,
                        cx,
                    );
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .child(div().flex().gap(px(12.0)).child(low).child(warning))
                        .child(div().flex().gap(px(12.0)).child(safe).child(high))
                        .into_any_element()
                } else {
                    self.floating_color_input(
                        k,
                        "color-solid",
                        crate::i18n::tr("fixed-color"),
                        colors.solid,
                        |colors, value| colors.solid = value,
                        window,
                        cx,
                    )
                };
                rows.push(
                    div()
                        .px(px(16.0))
                        .py(px(12.0))
                        .child(color_fields)
                        .into_any_element(),
                );
                rows.push(
                    Row::new(
                        "floating-circle-position",
                        crate::i18n::tr("floating-circle-position"),
                    )
                    .description(k, crate::i18n::tr("floating-circle-position-description"))
                    .trailing(reset_circle)
                    .render(k),
                );
            }
            if !cfg!(windows) {
                rows.remove(3);
            }
            rows
        });
        let layout = kit::card_of(k, |k| {
            // Starting edit mode here works whether or not the bottom bar
            // shows its Edit button, so that button can be hidden.
            let start = Button::new("customize-start-editing", crate::i18n::tr("start-editing"))
                .with_icon("pencil-simple-fill")
                .on_click(Self::h(cx, |_, (), _, _| {
                    crate::popup_window::start_home_editing()
                }))
                .render(k);
            let set_shown = |this: &mut Self, value: bool, cx: &mut Context<Self>| {
                this.edit(cx, move |settings| settings.show_edit_home_button = value)
            };
            let switch = kit::toggle(
                k,
                "customize-edit-button",
                show_edit_button,
                false,
                Self::h(cx, move |this, value: bool, _, cx| {
                    set_shown(this, value, cx)
                }),
            );
            vec![
                Row::new("customize-edit-button", crate::i18n::tr("show-edit-button"))
                    .description(k, crate::i18n::tr("show-edit-button-description"))
                    .trailing(start)
                    .trailing(switch)
                    .on_click(Self::h(cx, move |this, (), _, cx| {
                        set_shown(this, !show_edit_button, cx)
                    }))
                    .render(k),
                bool_row(
                    self,
                    k,
                    cx,
                    "customize-popup-action-bar",
                    crate::i18n::tr("show-popup-action-bar"),
                    Some(crate::i18n::tr("show-popup-action-bar-description")),
                    |s| s.show_popup_action_bar,
                    |s, v| s.show_popup_action_bar = v,
                ),
                bool_row(
                    self,
                    k,
                    cx,
                    "customize-two-columns",
                    crate::i18n::tr("use-two-columns"),
                    Some(crate::i18n::tr(
                        "widen-home-and-usage-drag-home-blocks-between-columns-provider-ta",
                    )),
                    |s| s.popup_two_columns,
                    |s, v| s.popup_two_columns = v,
                ),
            ]
        });
        let tab_mode = self.settings.popup_tab_mode;
        let tabs = kit::card_of(k, |k| {
            vec![
                self.provider_order_row(k, cx),
                kit::dropdown_row(
                    k,
                    "customize-tab-mode",
                    crate::i18n::tr("several-accounts-of-one-provider"),
                    Some(crate::i18n::tr(
                        "separate-tabs-gives-every-instance-its-own-tab-grouped-shows-one",
                    )),
                    PopupTabMode::ALL
                        .iter()
                        .map(|mode| SharedString::from(mode.label()))
                        .collect(),
                    tab_mode.index(),
                    false,
                    Self::h(cx, |this, index: usize, _, cx| {
                        let mode = PopupTabMode::from_index(index as i32);
                        this.edit(cx, move |settings| settings.popup_tab_mode = mode)
                    }),
                ),
                bool_row(
                    self,
                    k,
                    cx,
                    "customize-mono-icons",
                    crate::i18n::tr("use-monochrome-icons"),
                    Some(crate::i18n::tr(
                        "draw-provider-marks-in-the-popup-without-brand-colors",
                    )),
                    |s| !s.use_colored_provider_icons,
                    |s, v| s.use_colored_provider_icons = !v,
                ),
            ]
        });
        let cards = kit::card_of(k, |k| {
            vec![
                bool_row(
                    self,
                    k,
                    cx,
                    "customize-show-used",
                    crate::i18n::tr("show-used-instead-of-remaining"),
                    None,
                    |s| s.show_used_percentage,
                    |s, v| s.show_used_percentage = v,
                ),
                bool_row(
                    self,
                    k,
                    cx,
                    "customize-usage-values",
                    crate::i18n::tr("show-usage-in-values-when-possible"),
                    Some(crate::i18n::tr(
                        "adds-exact-used-limit-amounts-next-to-percentages-when-a-provider",
                    )),
                    |s| s.show_usage_values,
                    |s, v| s.show_usage_values = v,
                ),
                bool_row(
                    self,
                    k,
                    cx,
                    "customize-usage-pace",
                    crate::i18n::tr("show-usage-pace"),
                    Some(crate::i18n::tr(
                        "marks-whether-you-re-burning-quota-faster-or-slower-than-an-even",
                    )),
                    |s| s.show_usage_pace,
                    |s, v| s.show_usage_pace = v,
                ),
                bool_row(
                    self,
                    k,
                    cx,
                    "customize-account-name",
                    crate::i18n::tr("show-account-name"),
                    None,
                    |s| s.show_account_name,
                    |s, v| s.show_account_name = v,
                ),
            ]
        });
        let presentation = self.settings.total_spend_presentation;
        let widget = kit::card_of(k, |k| {
            vec![
                bool_row(
                    self,
                    k,
                    cx,
                    "customize-spend-home",
                    crate::i18n::tr("show-on-home-tab"),
                    None,
                    |s| s.show_total_spend_on_all_tab,
                    |s, v| s.show_total_spend_on_all_tab = v,
                ),
                Row::new("customize-spend-layout", crate::i18n::tr("layout"))
                    .trailing(kit::segmented(
                        k,
                        "customize-spend-layout",
                        &[crate::i18n::tr("donut"), crate::i18n::tr("cards")],
                        presentation.index().max(0) as usize,
                        !self.settings.show_total_spend_on_all_tab,
                        Self::h(cx, |this, index: usize, _, cx| {
                            let value = TotalSpendPresentation::from_index(index as i32);
                            this.edit(cx, move |settings| {
                                settings.total_spend_presentation = value
                            })
                        }),
                    ))
                    .render(k),
            ]
        });
        vec![
            kit::section_heading(k, crate::i18n::tr("layout")),
            layout,
            kit::section_heading(k, crate::i18n::tr("popup-floating")),
            floating,
            kit::section_heading(k, crate::i18n::tr("tabs")),
            tabs,
            kit::section_heading(k, crate::i18n::tr("cards")),
            cards,
            kit::section_heading(k, crate::i18n::tr("usage-widget")),
            widget,
        ]
    }

    /// Popup card visibility for one instance's page. Card visibility is
    /// per instance; the Home column follows the General "Show on Home".
    pub(super) fn popup_cards_section(
        &mut self,
        instance: &ProviderInstance,
        k: &mut Kit,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let provider = instance.driver;
        let instance_id = instance.provider_id();
        let show_on_home = instance.show_on_home;
        let visibility = self.settings.popup_visibility.clone();
        let extra_ids = visibility
            .bricks
            .keys()
            .chain(self.discovered_bricks.keys())
            .cloned()
            .collect::<Vec<_>>();
        let theme = k.theme.clone();
        let header_cell = |label: &'static str| {
            div()
                .w(px(56.0))
                .flex()
                .justify_center()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(label)
        };
        let mut table = div().flex().flex_col().w_full().child(
            div()
                .flex()
                .items_center()
                .h(px(28.0))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_secondary)
                        .child(crate::i18n::tr("card")),
                )
                .child(header_cell(crate::i18n::tr("home")))
                .child(header_cell(crate::i18n::tr("tab"))),
        );
        for brick_id in crate::provider_registry::settings_brick_ids(provider, &extra_ids) {
            let current = visibility.instance_visibility_for(instance_id.id(), &brick_id);
            let label = crate::provider_registry::settings_brick_label(
                provider,
                &brick_id,
                &self.discovered_bricks,
            );
            let make = |all_tab: Option<bool>| {
                let brick_id = brick_id.clone();
                Self::h(cx, move |this, checked: bool, _, cx| {
                    let id = instance_id.id().to_owned();
                    let brick = brick_id.clone();
                    let now = this
                        .settings
                        .popup_visibility
                        .instance_visibility_for(&id, &brick);
                    let (all, tab) = match all_tab {
                        Some(_) => (checked, now.provider_tab),
                        None => (now.all_tab, checked),
                    };
                    this.edit(cx, move |settings| {
                        settings.popup_visibility.set_instance_brick(
                            id.clone(),
                            brick.clone(),
                            all,
                            tab,
                        )
                    })
                })
            };
            let home = kit::checkbox(
                k,
                format!("brick-home-{}-{brick_id}", instance_id.id()),
                current.all_tab,
                !show_on_home,
                None,
                make(Some(true)),
            );
            let tab = kit::checkbox(
                k,
                format!("brick-tab-{}-{brick_id}", instance_id.id()),
                current.provider_tab,
                false,
                None,
                make(None),
            );
            table = table.child(
                div()
                    .flex()
                    .items_center()
                    .h(px(34.0))
                    .border_t_1()
                    .border_color(theme.divider)
                    .child(div().flex_1().text_size(px(14.0)).child(label))
                    .child(div().w(px(56.0)).flex().justify_center().child(home))
                    .child(div().w(px(56.0)).flex().justify_center().child(tab)),
            );
        }
        let card_id = format!("popup-cards-{}", instance_id.id());
        let expanded = !self.is_expanded(&format!("{card_id}-collapsed"));
        let collapse_id = format!("{card_id}-collapsed");
        let on_toggle = Self::h(cx, move |this, open: bool, _, cx| {
            this.set_expanded(collapse_id.clone(), !open);
            cx.notify();
        });
        let header = Row::new(card_id.clone(), crate::i18n::tr("popup-cards")).description(
            k,
            if show_on_home {
                crate::i18n::tr("choose-which-cards-this-account-shows-on-home-and-on-its-own-tab")
            } else {
                crate::i18n::tr("choose-which-cards-this-account-shows-on-its-own-tab-turn-on-show")
            },
        );
        let table = table.into_any_element();
        kit::expander(k, card_id, header, expanded, on_toggle, move |_| table)
    }
}

fn parse_hex(value: &str) -> Option<TrayFixedColor> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).ok();
    Some(TrayFixedColor {
        red: channel(0..2)?,
        green: channel(2..4)?,
        blue: channel(4..6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floating_hex_colors_parse_with_or_without_hash() {
        assert_eq!(
            parse_hex("#5297F6"),
            Some(TrayFixedColor {
                red: 0x52,
                green: 0x97,
                blue: 0xF6,
            })
        );
        assert_eq!(parse_hex("fce100").unwrap().green, 0xE1);
        assert!(parse_hex("12345").is_none());
        assert!(parse_hex("GGGGGG").is_none());
    }
}
