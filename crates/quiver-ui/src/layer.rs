//! `layer`: GPUI's `deferred` (draw this above everything else), safe to nest.
//!
//! GPUI aborts the process when a deferred element draws another one ("cannot call defer_draw during deferred
//! drawing"), and a dropdown inside a dialog is exactly that: the dialog is a deferred layer, the dropdown's
//! list another. A playground session died on the switch dialog's channel select.
//!
//! A layer defers like `deferred` when nothing around it does. Inside another layer it cannot, so it hands its
//! content to the enclosing layer instead, which prepaints and paints it after its own content (in priority
//! order): it still draws above the dialog's later parts (a footer under an open list) and takes the clicks
//! there, as a deferred draw would.

use std::cell::RefCell;

use gpui::{
    AnyElement, App, Bounds, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Point, Window,
};

thread_local! {
    /// One list per layer being drawn in GPUI's deferred pass: the nested layers it found under it.
    static NESTED: RefCell<Vec<Vec<Late>>> = const { RefCell::new(Vec::new()) };
}

struct Late {
    element: AnyElement,
    offset: Point<Pixels>,
    priority: usize,
}

fn inside_a_layer() -> bool {
    NESTED.with(|n| !n.borrow().is_empty())
}

/// `deferred(child)`, safe to nest: see the module docs.
pub fn layer(child: impl IntoElement) -> Layer {
    Layer {
        content: Some(Content::new(child.into_any_element()).into_any_element()),
        priority: 0,
    }
}

pub struct Layer {
    content: Option<AnyElement>,
    priority: usize,
}

impl Layer {
    /// As `Deferred::with_priority`: higher draws on top (among layers at the same level).
    pub fn with_priority(
        mut self,
        priority: usize,
    ) -> Self {
        self.priority = priority;
        self
    }
}

impl IntoElement for Layer {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Layer {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let layout = self.content.as_mut().map(|c| c.request_layout(window, cx));
        (
            layout.unwrap_or_else(|| window.request_layout(gpui::Style::default(), [], cx)),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        let Some(element) = self.content.take() else {
            return;
        };
        let offset = window.element_offset();
        if inside_a_layer() {
            NESTED.with(|n| {
                if let Some(level) = n.borrow_mut().last_mut() {
                    level.push(Late {
                        element,
                        offset,
                        priority: self.priority,
                    });
                }
            });
        } else {
            window.defer_draw(element, offset, self.priority);
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
}

/// A layer's content: while it is prepainted (in GPUI's deferred pass, or late inside an enclosing layer) the
/// layers under it collect here, and are prepainted and painted after it.
struct Content {
    child: AnyElement,
}

impl Content {
    fn new(child: AnyElement) -> Self {
        Self { child }
    }
}

impl IntoElement for Content {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Content {
    type RequestLayoutState = ();
    type PrepaintState = Vec<AnyElement>;

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<AnyElement> {
        NESTED.with(|n| n.borrow_mut().push(Vec::new()));
        self.child.prepaint(window, cx);
        let mut nested = NESTED.with(|n| n.borrow_mut().pop()).unwrap_or_default();
        // Stable: equal priorities keep their tree order, as GPUI's deferred draws do.
        nested.sort_by_key(|l| l.priority);
        nested
            .into_iter()
            .map(|mut late| {
                // Each is itself a `Content`, so what is nested under it is handled the same way.
                window.with_absolute_element_offset(late.offset, |window| {
                    late.element.prepaint(window, cx)
                });
                late.element
            })
            .collect()
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        nested: &mut Vec<AnyElement>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
        for element in nested {
            element.paint(window, cx);
        }
    }
}
