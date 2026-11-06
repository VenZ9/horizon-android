use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KeyPickerTab {
    #[default]
    Standard,
    Actions,
    Keys,
}

pub struct Editor {
    pub selected: Option<Control>,
    option: Option<usize>,
    apply_all: bool,
    original: Settings,
    drag: Option<(u64, Drag)>,
    error: Option<String>,
    palette: bool,
    selected_custom: Option<usize>,
    key_picker: bool,
    key_page: usize,
    key_tab: KeyPickerTab,
    confirm_delete: bool,
}

enum Drag {
    Control([f32; 2]),
    Slider(usize),
}

impl TouchControls {
    pub fn open_editor(&mut self) {
        self.clear();
        self.editor = Some(Editor {
            selected: None,
            option: None,
            apply_all: false,
            original: self.settings.clone(),
            drag: None,
            error: None,
            palette: false,
            selected_custom: None,
            key_picker: false,
            key_page: 0,
            key_tab: KeyPickerTab::Standard,
            confirm_delete: false,
        });
    }
    pub fn cancel_editor(&mut self) {
        if let Some(e) = self.editor.take() {
            self.settings = e.original;
        }
        self.clear();
    }
    /// The editor owns every finger: editing cannot move, mine or open menus.
    /// Returns true when Done/Cancel closed the editor.
    pub fn edit_touch(&mut self, id: u64, p: [f32; 2], started: bool, ended: bool) -> bool {
        let Some(mut e) = self.editor.take() else {
            return false;
        };
        self.clicked |= started;
        if self.edit_custom(&mut e, id, p, started, ended) {
            self.editor = Some(e);
            return false;
        }
        if started {
            if layout::contains([0.02, 0.04, 0.105, 0.075], p) {
                match self.settings.save() {
                    Ok(()) => {
                        self.clear();
                        return true;
                    }
                    Err(error) => e.error = Some(error.to_string()),
                }
            } else if layout::contains([0.135, 0.04, 0.105, 0.075], p) {
                self.settings = e.original;
                self.clear();
                return true;
            } else if layout::contains([0.25, 0.04, 0.105, 0.075], p) {
                self.settings.layouts[self.settings.mode.index()] =
                    Settings::default().layouts[self.settings.mode.index()];
                self.settings.button_visibility = [0; 14];
                self.settings.key_overrides = Default::default();
                for (i, button) in self.settings.custom_buttons.iter_mut().enumerate() {
                    button.layouts[self.settings.mode.index()] =
                        CustomButton::new(button.key.clone(), i).layouts
                            [self.settings.mode.index()];
                }
                e.confirm_delete = false;
            } else if layout::contains([0.365, 0.04, 0.125, 0.075], p) {
                if self.settings.custom_buttons.len() < 32 {
                    let index = self.settings.custom_buttons.len();
                    self.settings
                        .custom_buttons
                        .push(CustomButton::new_centered("key.keyboard.unknown".into()));
                    e.selected = None;
                    e.selected_custom = Some(index);
                    e.key_picker = true;
                    e.key_tab = KeyPickerTab::Standard;
                    e.key_page = 0;
                    e.palette = false;
                    e.option = None;
                    e.drag = None;
                    e.confirm_delete = false;
                }
            } else if layout::contains([0.02, 0.13, 0.335, 0.075], p) {
                e.palette = !e.palette;
                e.selected = None;
                e.selected_custom = None;
                e.key_picker = false;
                e.drag = None;
                e.confirm_delete = false;
            } else if e.palette {
                for (index, control) in Control::ALL.into_iter().enumerate() {
                    if layout::contains(palette_rect(index), p) {
                        let show = !self.visible(control);
                        self.settings.button_visibility[index] = if show { 1 } else { 2 };
                    }
                }
            } else if layout::contains([0.32, 0.54, 0.36, 0.07], p) && e.selected.is_some() {
                e.key_picker = true;
                e.key_tab = KeyPickerTab::Standard;
                e.key_page = 0;
                e.drag = None;
                e.confirm_delete = false;
            } else if layout::contains([0.32, 0.88, 0.36, 0.07], p) && e.selected.is_some() {
                if e.confirm_delete {
                    self.settings.button_visibility[e.selected.unwrap() as usize] = 2;
                    e.selected = None;
                    e.confirm_delete = false;
                    e.drag = None;
                } else {
                    e.confirm_delete = true;
                }
            } else if layout::contains([0.32, 0.61, 0.36, 0.06], p)
                && e.selected.is_some()
                && e.option.is_some()
            {
                e.apply_all = !e.apply_all;
            } else if layout::contains([0.32, 0.68, 0.36, 0.07], p)
                && e.selected.is_some()
                && e.option.is_some()
            {
                e.drag = Some((id, Drag::Slider(e.option.unwrap())));
            } else if layout::contains([0.32, 0.79, 0.17, 0.07], p) && e.selected.is_some() {
                e.option = Some(1);
                e.confirm_delete = false;
            } else if layout::contains([0.51, 0.79, 0.17, 0.07], p) && e.selected.is_some() {
                e.option = Some(0);
                e.confirm_delete = false;
            } else if let Some(c) = Control::ALL.into_iter().rev().find(|c| {
                self.visible(*c) && layout::contains(self.placement(*c).rect(self.aspect()), p)
            }) {
                e.selected = Some(c);
                e.selected_custom = None;
                e.option = None;
                e.confirm_delete = false;
                let center = self.placement(c).center;
                e.drag = Some((id, Drag::Control([p[0] - center[0], p[1] - center[1]])));
            } else {
                e.selected = None;
                e.option = None;
                e.confirm_delete = false;
            }
        }
        if let Some((owner, drag)) = &e.drag
            && *owner == id
            && let Some(selected) = e.selected
        {
            let aspect = self.aspect();
            let slot = self.settings.slot(selected);
            let place = &mut self.settings.layouts[self.settings.mode.index()][slot];
            match drag {
                Drag::Control(offset) => {
                    let mut center = [p[0] - offset[0], p[1] - offset[1]];
                    if self.settings.left_handed {
                        center[0] = 1.0 - center[0];
                    }
                    // Match the displayed size on narrow portrait surfaces.
                    // A stored button may be wider than the screen; its raw
                    // size would otherwise produce inverted clamp bounds.
                    let size = (place.size * self.settings.button_scale)
                        .min(aspect)
                        .min(1.0);
                    place.center = [
                        center[0].clamp(size / aspect / 2.0, 1.0 - size / aspect / 2.0),
                        center[1].clamp(size / 2.0, 1.0 - size / 2.0),
                    ];
                }
                Drag::Slider(index) => {
                    let value = ((p[0] - 0.335) / 0.33).clamp(0.0, 1.0);
                    for (i, place) in self.settings.layouts[self.settings.mode.index()]
                        .iter_mut()
                        .enumerate()
                    {
                        if i == slot || e.apply_all {
                            if *index == 0 {
                                place.size = 0.045 + value * 0.405;
                            } else {
                                place.opacity = value;
                            }
                        }
                    }
                }
            }
            if ended {
                e.drag = None;
            }
        }

        self.editor = Some(e);
        false
    }
    pub(super) fn draw_editor_toolbar(&self, b: &mut HudBatch, a: &HudAtlas) {
        for (rect, label) in [
            ([0.02, 0.04, 0.105, 0.075], "Save"),
            ([0.135, 0.04, 0.105, 0.075], "Cancel"),
            ([0.25, 0.04, 0.105, 0.075], "Reset"),
            ([0.365, 0.04, 0.125, 0.075], "+ Button"),
            ([0.02, 0.13, 0.335, 0.075], "Visibility..."),
        ] {
            plate(b, a, rect, label, false);
        }
    }

    pub(super) fn draw_editor(&self, b: &mut HudBatch, a: &HudAtlas) {
        let e = self.editor.as_ref().unwrap();
        let (w, h) = (b.width, b.height);
        let text = rgba(245, 245, 245, 255);
        if let Some(selected) = e.selected
            && !e.palette
            && !e.key_picker
        {
            plate(
                b,
                a,
                [0.32, 0.54, 0.36, 0.07],
                &format!("Keybind: {}", self.control_label(selected)),
                false,
            );
            plate(
                b,
                a,
                [0.32, 0.79, 0.17, 0.07],
                "Opacity",
                e.option == Some(1),
            );
            plate(b, a, [0.51, 0.79, 0.17, 0.07], "Size", e.option == Some(0));
            plate(
                b,
                a,
                [0.32, 0.88, 0.36, 0.07],
                if e.confirm_delete {
                    "[!] Confirm Remove"
                } else {
                    "Remove button"
                },
                e.confirm_delete,
            );
            let p = self.placement(selected);
            if let Some(option) = e.option {
                b.fill(
                    a,
                    w * 0.31,
                    h * 0.59,
                    w * 0.38,
                    h * 0.18,
                    rgba(15, 15, 15, 235),
                );
                plate(
                    b,
                    a,
                    [0.32, 0.61, 0.36, 0.06],
                    if e.apply_all {
                        "Apply to all: On"
                    } else {
                        "Apply to all: Off"
                    },
                    e.apply_all,
                );
                let value = if option == 0 {
                    (p.size - 0.045) / 0.405
                } else {
                    p.opacity
                };
                let percent = if option == 0 {
                    p.size * 100.0
                } else {
                    p.opacity * 100.0
                };
                b.text(
                    a,
                    &format!("{} {:.0}%", selected.name(), percent),
                    w * 0.325,
                    h * 0.68,
                    text,
                    true,
                );
                nine(
                    b,
                    a,
                    "ui/slider_background",
                    1.0,
                    [w * 0.335, h * 0.72, w * 0.33, h * 0.025],
                    rgba(255, 255, 255, 255),
                );
                nine(
                    b,
                    a,
                    "ui/slider_button_default",
                    2.0,
                    [w * (0.335 + value * 0.33) - 3.0, h * 0.707, 6.0, h * 0.05],
                    rgba(255, 255, 255, 255),
                );
            }
        }
        if e.palette {
            b.fill(
                a,
                w * 0.2,
                h * 0.14,
                w * 0.6,
                h * 0.72,
                rgba(15, 15, 15, 245),
            );
            for (index, control) in Control::ALL.into_iter().enumerate() {
                let shown = self.visible(control);
                plate(
                    b,
                    a,
                    palette_rect(index),
                    &format!("{} {}", if shown { "[-]" } else { "[+]" }, control.name()),
                    shown,
                );
            }
        }
        self.draw_custom_editor(b, a, e);
        if e.error.is_some() {
            b.text(
                a,
                "Save failed. Retry Done.",
                w * 0.33,
                h * 0.88,
                rgba(255, 120, 120, 255),
                true,
            );
        }
    }
}
fn palette_rect(index: usize) -> [f32; 4] {
    [
        0.22 + (index % 2) as f32 * 0.29,
        0.17 + (index / 2) as f32 * 0.11,
        0.27,
        0.09,
    ]
}
fn plate(b: &mut HudBatch, a: &HudAtlas, [x, y, w, h]: [f32; 4], label: &str, on: bool) {
    let (bw, bh) = (b.width, b.height);
    nine(
        b,
        a,
        "ui/button_borderless_light",
        1.0,
        [x * bw, y * bh, w * bw, h * bh],
        if on {
            rgba(100, 210, 90, 255)
        } else {
            rgba(255, 255, 255, 255)
        },
    );
    if let Some((first, second)) = label.split_once(" & ") {
        b.text(
            a,
            &format!("{first} &"),
            x * bw + 3.0,
            y * bh + (h * bh - 20.0) / 2.0,
            rgba(35, 35, 35, 255),
            false,
        );
        b.text(
            a,
            second,
            x * bw + 3.0,
            y * bh + (h * bh - 20.0) / 2.0 + 10.0,
            rgba(35, 35, 35, 255),
            false,
        );
    } else {
        let mut display = label.to_owned();
        while HudBatch::text_width(a, &display) > w * bw - 6.0 && !display.is_empty() {
            display.pop();
        }
        b.text(
            a,
            &display,
            x * bw + 3.0,
            y * bh + (h * bh - 8.0) / 2.0,
            rgba(35, 35, 35, 255),
            false,
        );
    }
}

impl TouchControls {
    fn edit_custom(
        &mut self,
        e: &mut Editor,
        id: u64,
        p: [f32; 2],
        started: bool,
        ended: bool,
    ) -> bool {
        if p[1] < 0.13 && started {
            return false;
        }
        if e.key_picker {
            if started {
                if layout::contains([0.15, 0.18, 0.22, 0.06], p) {
                    e.key_tab = KeyPickerTab::Standard;
                    e.key_page = 0;
                } else if layout::contains([0.39, 0.18, 0.22, 0.06], p) {
                    e.key_tab = KeyPickerTab::Actions;
                    e.key_page = 0;
                } else if layout::contains([0.63, 0.18, 0.22, 0.06], p) {
                    e.key_tab = KeyPickerTab::Keys;
                    e.key_page = 0;
                } else if layout::contains([0.15, 0.85, 0.24, 0.07], p) {
                    // Reset to Default
                    if let Some(index) = e.selected_custom {
                        self.settings.custom_buttons[index].key = "key.keyboard.unknown".into();
                    } else if let Some(c) = e.selected {
                        self.settings.key_overrides[c as usize] = None;
                    }
                    e.key_picker = false;
                } else if layout::contains([0.41, 0.85, 0.18, 0.07], p) {
                    // Back
                    e.key_picker = false;
                } else if e.key_tab != KeyPickerTab::Standard
                    && layout::contains([0.62, 0.85, 0.12, 0.07], p)
                {
                    e.key_page = e.key_page.saturating_sub(1);
                } else if e.key_tab != KeyPickerTab::Standard
                    && layout::contains([0.76, 0.85, 0.12, 0.07], p)
                {
                    let total = if e.key_tab == KeyPickerTab::Keys {
                        super::custom::choices(true).len()
                    } else {
                        super::custom::choices(false).len()
                    };
                    e.key_page = (e.key_page + 1).min(total.saturating_sub(1) / 8);
                } else {
                    match e.key_tab {
                        KeyPickerTab::Standard => {
                            for (i, (token, _)) in
                                super::custom::STANDARD_ACTIONS.iter().enumerate()
                            {
                                let col = i % 2;
                                let row = i / 2;
                                let rx = if col == 0 { 0.15 } else { 0.51 };
                                let ry = 0.26 + row as f32 * 0.13;
                                if layout::contains([rx, ry, 0.34, 0.10], p) {
                                    if let Some(index) = e.selected_custom {
                                        self.settings.custom_buttons[index].key = (*token).into();
                                    } else if let Some(c) = e.selected {
                                        self.settings.key_overrides[c as usize] =
                                            Some((*token).into());
                                    }
                                    e.key_picker = false;
                                    break;
                                }
                            }
                        }
                        KeyPickerTab::Actions => {
                            for (i, (token, _)) in super::custom::choices(false)
                                .into_iter()
                                .skip(e.key_page * 8)
                                .take(8)
                                .enumerate()
                            {
                                if layout::contains(key_rect(i), p) {
                                    if let Some(index) = e.selected_custom {
                                        self.settings.custom_buttons[index].key = token.clone();
                                    } else if let Some(c) = e.selected {
                                        self.settings.key_overrides[c as usize] = Some(token);
                                    }
                                    e.key_picker = false;
                                    break;
                                }
                            }
                        }
                        KeyPickerTab::Keys => {
                            for (i, (token, _)) in super::custom::choices(true)
                                .into_iter()
                                .skip(e.key_page * 8)
                                .take(8)
                                .enumerate()
                            {
                                if layout::contains(key_rect(i), p) {
                                    if let Some(index) = e.selected_custom {
                                        self.settings.custom_buttons[index].key = token.clone();
                                    } else if let Some(c) = e.selected {
                                        self.settings.key_overrides[c as usize] = Some(token);
                                    }
                                    e.key_picker = false;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            return true;
        }
        if e.palette && started && layout::contains([0.22, 0.84, 0.56, 0.07], p) {
            if self.settings.custom_buttons.len() < 32 {
                let index = self.settings.custom_buttons.len();
                self.settings
                    .custom_buttons
                    .push(CustomButton::new_centered("key.keyboard.unknown".into()));
                e.selected = None;
                e.selected_custom = Some(index);
                e.key_picker = true;
                e.key_tab = KeyPickerTab::Standard;
                e.key_page = 0;
                e.palette = false;
                e.option = None;
                e.drag = None;
                e.confirm_delete = false;
            }
            return true;
        }
        if e.palette {
            return false;
        }
        if started && e.selected_custom.is_some() {
            if layout::contains([0.32, 0.54, 0.36, 0.07], p) {
                e.key_picker = true;
                e.key_tab = KeyPickerTab::Standard;
                e.key_page = 0;
                e.drag = None;
                e.confirm_delete = false;
                return true;
            }
            if layout::contains([0.32, 0.88, 0.36, 0.07], p) {
                if e.confirm_delete {
                    self.settings
                        .custom_buttons
                        .remove(e.selected_custom.unwrap());
                    e.selected_custom = None;
                    e.confirm_delete = false;
                    e.drag = None;
                } else {
                    e.confirm_delete = true;
                }
                return true;
            }
            if layout::contains([0.32, 0.79, 0.17, 0.07], p) {
                e.option = Some(1);
                e.confirm_delete = false;
                return true;
            }
            if layout::contains([0.51, 0.79, 0.17, 0.07], p) {
                e.option = Some(0);
                e.confirm_delete = false;
                return true;
            }
            if layout::contains([0.32, 0.68, 0.36, 0.07], p)
                && let Some(option) = e.option
            {
                e.drag = Some((id, Drag::Slider(option)));
            } else {
                e.selected_custom = None;
                e.confirm_delete = false;
                e.drag = None;
            }
        }
        if started
            && e.drag.is_none()
            && let Some(index) = self.custom_at(p)
        {
            let center = self.custom_placement(index).center;
            e.selected = None;
            e.selected_custom = Some(index);
            e.option = None;
            e.apply_all = false;
            e.confirm_delete = false;
            e.drag = Some((id, Drag::Control([p[0] - center[0], p[1] - center[1]])));
        }
        let Some(index) = e.selected_custom else {
            return false;
        };
        if let Some((owner, drag)) = &e.drag
            && *owner == id
        {
            let aspect = self.aspect();
            let scale = self.settings.button_scale;
            let mirror = self.settings.left_handed;
            let place =
                &mut self.settings.custom_buttons[index].layouts[self.settings.mode.index()];
            match drag {
                Drag::Control(offset) => {
                    let mut center = [p[0] - offset[0], p[1] - offset[1]];
                    if mirror {
                        center[0] = 1.0 - center[0];
                    }
                    let size = (place.size * scale).min(aspect).min(1.0);
                    place.center = [
                        center[0].clamp(size / aspect / 2.0, 1.0 - size / aspect / 2.0),
                        center[1].clamp(size / 2.0, 1.0 - size / 2.0),
                    ];
                }
                Drag::Slider(option) => {
                    let value = ((p[0] - 0.335) / 0.33).clamp(0.0, 1.0);
                    if *option == 0 {
                        place.size = 0.045 + value * 0.405;
                    } else {
                        place.opacity = value;
                    }
                }
            }
            if ended {
                e.drag = None;
            }
        }
        true
    }
    pub fn capture_custom_key(&mut self, name: &str) -> bool {
        if crate::keybinds::key_code(name).is_none() {
            return false;
        }
        if let Some(e) = &mut self.editor
            && e.key_picker
        {
            if let Some(i) = e.selected_custom {
                self.settings.custom_buttons[i].key = name.into();
                e.key_picker = false;
                return true;
            }
            if let Some(c) = e.selected {
                self.settings.key_overrides[c as usize] = Some(name.into());
                e.key_picker = false;
                return true;
            }
        }
        false
    }
    fn draw_custom_editor(&self, b: &mut HudBatch, a: &HudAtlas, e: &Editor) {
        let (w, h) = (b.width, b.height);
        if e.palette {
            plate(b, a, [0.22, 0.84, 0.56, 0.07], "Create button...", false);
        }
        if let Some(i) = e.selected_custom
            && !e.palette
            && !e.key_picker
        {
            let p = self.settings.custom_buttons[i].layouts[self.settings.mode.index()];
            plate(
                b,
                a,
                [0.32, 0.54, 0.36, 0.07],
                &format!("Keybind: {}", self.settings.custom_buttons[i].label()),
                false,
            );
            plate(
                b,
                a,
                [0.32, 0.79, 0.17, 0.07],
                "Opacity",
                e.option == Some(1),
            );
            plate(b, a, [0.51, 0.79, 0.17, 0.07], "Size", e.option == Some(0));
            plate(
                b,
                a,
                [0.32, 0.88, 0.36, 0.07],
                if e.confirm_delete {
                    "[!] Confirm Delete"
                } else {
                    "Delete button"
                },
                e.confirm_delete,
            );
            if let Some(option) = e.option {
                let value = if option == 0 {
                    (p.size - 0.045) / 0.405
                } else {
                    p.opacity
                };
                b.fill(
                    a,
                    w * 0.3,
                    h * 0.66,
                    w * 0.4,
                    h * 0.11,
                    rgba(15, 15, 15, 235),
                );
                let percent = if option == 0 {
                    p.size * 100.0
                } else {
                    p.opacity * 100.0
                };
                b.text(
                    a,
                    &format!(
                        "{} {:.0}%",
                        if option == 0 { "Size" } else { "Opacity" },
                        percent
                    ),
                    w * 0.325,
                    h * 0.68,
                    rgba(245, 245, 245, 255),
                    true,
                );
                nine(
                    b,
                    a,
                    "ui/slider_background",
                    1.0,
                    [w * 0.335, h * 0.72, w * 0.33, h * 0.025],
                    rgba(255, 255, 255, 255),
                );
                nine(
                    b,
                    a,
                    "ui/slider_button_default",
                    2.0,
                    [w * (0.335 + value * 0.33) - 3.0, h * 0.707, 6.0, h * 0.05],
                    rgba(255, 255, 255, 255),
                );
            }
        }
        if e.key_picker {
            b.fill(
                a,
                w * 0.12,
                h * 0.12,
                w * 0.76,
                h * 0.83,
                rgba(15, 15, 15, 250),
            );
            b.text(
                a,
                "Select Keybind",
                w * 0.15,
                h * 0.14,
                rgba(255, 255, 255, 255),
                true,
            );
            plate(
                b,
                a,
                [0.15, 0.18, 0.22, 0.06],
                "Standard",
                e.key_tab == KeyPickerTab::Standard,
            );
            plate(
                b,
                a,
                [0.39, 0.18, 0.22, 0.06],
                "All Actions",
                e.key_tab == KeyPickerTab::Actions,
            );
            plate(
                b,
                a,
                [0.63, 0.18, 0.22, 0.06],
                "Keys",
                e.key_tab == KeyPickerTab::Keys,
            );

            match e.key_tab {
                KeyPickerTab::Standard => {
                    for (i, (token, label)) in super::custom::STANDARD_ACTIONS.iter().enumerate() {
                        let col = i % 2;
                        let row = i / 2;
                        let rx = if col == 0 { 0.15 } else { 0.51 };
                        let ry = 0.26 + row as f32 * 0.13;
                        let is_current = if let Some(idx) = e.selected_custom {
                            self.settings.custom_buttons[idx].key == *token
                        } else if let Some(c) = e.selected {
                            self.settings.key_overrides[c as usize].as_deref() == Some(*token)
                        } else {
                            false
                        };
                        plate(b, a, [rx, ry, 0.34, 0.10], label, is_current);
                    }
                }
                KeyPickerTab::Actions => {
                    for (i, (token, label)) in super::custom::choices(false)
                        .iter()
                        .skip(e.key_page * 8)
                        .take(8)
                        .enumerate()
                    {
                        let is_current = if let Some(idx) = e.selected_custom {
                            self.settings.custom_buttons[idx].key == *token
                        } else if let Some(c) = e.selected {
                            self.settings.key_overrides[c as usize].as_deref() == Some(token)
                        } else {
                            false
                        };
                        plate(b, a, key_rect(i), label, is_current);
                    }
                }
                KeyPickerTab::Keys => {
                    for (i, (token, label)) in super::custom::choices(true)
                        .iter()
                        .skip(e.key_page * 8)
                        .take(8)
                        .enumerate()
                    {
                        let is_current = if let Some(idx) = e.selected_custom {
                            self.settings.custom_buttons[idx].key == *token
                        } else if let Some(c) = e.selected {
                            self.settings.key_overrides[c as usize].as_deref() == Some(token)
                        } else {
                            false
                        };
                        plate(b, a, key_rect(i), label, is_current);
                    }
                }
            }

            plate(b, a, [0.15, 0.85, 0.24, 0.07], "Reset to Default", false);
            plate(b, a, [0.41, 0.85, 0.18, 0.07], "Back", false);
            if e.key_tab != KeyPickerTab::Standard {
                plate(b, a, [0.62, 0.85, 0.12, 0.07], "<", false);
                plate(b, a, [0.76, 0.85, 0.12, 0.07], ">", false);
            }
        }
    }
}
fn key_rect(i: usize) -> [f32; 4] {
    [0.15, 0.26 + i as f32 * 0.07, 0.7, 0.06]
}
