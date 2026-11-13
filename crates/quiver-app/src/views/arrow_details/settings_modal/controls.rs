//! The control a variable's type asks for (switch, select, number field, text or secret input).

use super::*;

impl SettingsModal {
    pub(super) fn control(
        &self,
        variable: &ArrowVariable,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let p = cx.quiver().palette;
        let name = variable.name.clone();
        let id = format!("arrow.settings.{}", variable.name.to_lowercase());
        let shown = shown_value(variable, &self.values.read(cx).values);
        let invalid = validate(variable, &shown).is_some();
        let values = self.values.clone();
        match field_kind(variable) {
            FieldKind::Switch => {
                let on = shown == "true";
                div()
                    .relative()
                    .child(
                        Switch::new(SharedString::from(id.clone()))
                            .checked(on)
                            .on_change(move |checked, _, cx| {
                                Self::set(&values, &name, checked.to_string(), cx)
                            }),
                    )
                    .child(probe(
                        id,
                        Kind::Switch,
                        variable.name.clone(),
                        true,
                        if on { "checked" } else { "" },
                    ))
                    .into_any_element()
            }
            FieldKind::Select(options) => div()
                .relative()
                .w(px(160.))
                .child(
                    Select::new(SharedString::from(id.clone()))
                        .options(
                            options
                                .iter()
                                .map(|o| SelectOption::new(o.clone(), o.clone()))
                                .collect(),
                        )
                        .value(Some(SharedString::from(shown.clone())))
                        .invalid(invalid)
                        .on_change(move |v, _, cx| Self::set(&values, &name, v.to_string(), cx)),
                )
                .child(probe(
                    id,
                    Kind::Select,
                    variable.name.clone(),
                    true,
                    format!("value={shown}"),
                ))
                .into_any_element(),
            FieldKind::Number { min, max } => {
                let mut field = NumberField::new(SharedString::from(id.clone()))
                    .value(shown.trim().parse::<f64>().ok())
                    .on_change(move |v, _, cx| {
                        Self::set(&values, &name, v.map(format_number).unwrap_or_default(), cx)
                    });
                if let (Some(min), Some(max)) = (min, max) {
                    field = field.range(min, max);
                }
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(px(4.))
                    .child(field)
                    .when_some(range_hint(min, max), |d, hint| {
                        d.child(
                            div()
                                .text_size(px(11.))
                                .text_color(p.muted_foreground)
                                .child(hint),
                        )
                    })
                    .child(probe(
                        id,
                        Kind::Input,
                        variable.name.clone(),
                        true,
                        format!("value={shown}"),
                    ))
                    .into_any_element()
            }
            kind @ (FieldKind::Text | FieldKind::Secret) => {
                let Some(state) = self.inputs.get(&variable.name) else {
                    return div().into_any_element();
                };
                let secret = kind == FieldKind::Secret;
                let revealed = self.revealed.contains(&variable.name);
                let reveal = secret.then(|| {
                    let (state, name) = (state.clone(), variable.name.clone());
                    Button::new(SharedString::from(format!("{id}.reveal")))
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::IconSm)
                        .child(pict(
                            if revealed { Pict::EyeOff } else { Pict::Eye },
                            16.,
                            p.muted_foreground,
                        ))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            let now = !this.revealed.remove(&name);
                            if now {
                                this.revealed.insert(name.clone());
                            }
                            state.update(cx, |s, cx| s.set_masked(!now, window, cx));
                            cx.notify();
                        }))
                });
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div().w(px(160.)).child(
                            Input::new(SharedString::from(id.clone()), state).invalid(invalid),
                        ),
                    )
                    .children(reveal)
                    .child(probe(
                        id,
                        Kind::Input,
                        variable.name.clone(),
                        true,
                        format!("value={shown}"),
                    ))
                    .into_any_element()
            }
        }
    }
}
