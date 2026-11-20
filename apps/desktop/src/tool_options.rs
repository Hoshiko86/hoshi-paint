//! Typed bindings for the context bar's new shape and picker choices.
use crate::{
    AppWindow, PickerAfterClickChoice, PickerSamplingChoice, PickerSizeChoice, ShapeKindChoice,
    SharedController, update, workspace::Workspace,
};
use hoshi_controller::{AfterClick, PickerSampling, PickerSettings, SampleSize, ToolSettings};
use hoshi_core::ShapeKind;
use slint::ComponentHandle;

/// Project controller settings while retaining the session's screen source.
pub(super) fn refresh(ui: &AppWindow, state: &Workspace) {
    ui.set_shape_kind(shape_choice(state.shape_kind()));
    let picker = state.settings().picker;
    ui.set_picker_sampling(if ui.get_picker_screen_sampling() {
        PickerSamplingChoice::Screen
    } else {
        sampling_choice(picker.sampling)
    });
    ui.set_picker_size(size_choice(picker.size));
    ui.set_picker_after_click(after_choice(picker.after_click));
}

/// Write new context-bar choices through the existing update boundary.
pub(super) fn bind(ui: &AppWindow, state: SharedController) {
    let (weak, controller) = (ui.as_weak(), state.clone());
    ui.on_select_shape_kind(move |value| {
        update(&weak, &controller, |c| c.set_shape_kind(shape(value)));
    });
    let (weak, controller) = (ui.as_weak(), state.clone());
    ui.on_select_picker_sampling(move |value| {
        if let Some(ui) = weak.upgrade() {
            ui.set_picker_screen_sampling(value == PickerSamplingChoice::Screen);
        }
        update(&weak, &controller, |c| {
            let s = c.settings();
            c.set_settings(ToolSettings {
                picker: PickerSettings {
                    sampling: sampling(value),
                    ..s.picker
                },
                ..s
            })
        });
    });
    let (weak, controller) = (ui.as_weak(), state.clone());
    ui.on_select_picker_size(move |value| {
        update(&weak, &controller, |c| {
            let s = c.settings();
            c.set_settings(ToolSettings {
                picker: PickerSettings {
                    size: size(value),
                    ..s.picker
                },
                ..s
            })
        });
    });
    let weak = ui.as_weak();
    ui.on_select_picker_after_click(move |value| {
        update(&weak, &state, |c| {
            let s = c.settings();
            c.set_settings(ToolSettings {
                picker: PickerSettings {
                    after_click: after(value),
                    ..s.picker
                },
                ..s
            })
        });
    });
}
fn shape(value: ShapeKindChoice) -> ShapeKind {
    match value {
        ShapeKindChoice::Rectangle => ShapeKind::Rectangle,
        ShapeKindChoice::Ellipse => ShapeKind::Ellipse,
    }
}
fn shape_choice(value: ShapeKind) -> ShapeKindChoice {
    match value {
        ShapeKind::Rectangle => ShapeKindChoice::Rectangle,
        ShapeKind::Ellipse => ShapeKindChoice::Ellipse,
    }
}
fn sampling(value: PickerSamplingChoice) -> PickerSampling {
    match value {
        PickerSamplingChoice::Layer => PickerSampling::Layer,
        PickerSamplingChoice::Image | PickerSamplingChoice::Screen => PickerSampling::Image,
    }
}
fn sampling_choice(value: PickerSampling) -> PickerSamplingChoice {
    match value {
        PickerSampling::Layer => PickerSamplingChoice::Layer,
        PickerSampling::Image => PickerSamplingChoice::Image,
    }
}
fn size(value: PickerSizeChoice) -> SampleSize {
    match value {
        PickerSizeChoice::Single => SampleSize::Single,
        PickerSizeChoice::Square3 => SampleSize::Square3,
        PickerSizeChoice::Square5 => SampleSize::Square5,
        PickerSizeChoice::Square11 => SampleSize::Square11,
        PickerSizeChoice::Square31 => SampleSize::Square31,
        PickerSizeChoice::Square51 => SampleSize::Square51,
    }
}
fn size_choice(value: SampleSize) -> PickerSizeChoice {
    match value {
        SampleSize::Single => PickerSizeChoice::Single,
        SampleSize::Square3 => PickerSizeChoice::Square3,
        SampleSize::Square5 => PickerSizeChoice::Square5,
        SampleSize::Square11 => PickerSizeChoice::Square11,
        SampleSize::Square31 => PickerSizeChoice::Square31,
        SampleSize::Square51 => PickerSizeChoice::Square51,
    }
}
fn after(value: PickerAfterClickChoice) -> AfterClick {
    match value {
        PickerAfterClickChoice::DoNotSwitch => AfterClick::DoNotSwitch,
        PickerAfterClickChoice::PreviousTool => AfterClick::PreviousTool,
    }
}
fn after_choice(value: AfterClick) -> PickerAfterClickChoice {
    match value {
        AfterClick::DoNotSwitch => PickerAfterClickChoice::DoNotSwitch,
        AfterClick::PreviousTool => PickerAfterClickChoice::PreviousTool,
    }
}
