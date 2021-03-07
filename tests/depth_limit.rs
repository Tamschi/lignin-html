use lignin::{Element, Node};
use lignin_html::render_fragment;
use std::fmt::Write;

struct Drain;
impl Write for Drain {
	fn write_str(&mut self, _: &str) -> std::fmt::Result {
		Ok(())
	}
}

#[test]
fn div_pass() {
	render_fragment(
		&Node::HtmlElement {
			element: &Element {
				name: "div",
				attributes: &[],
				content: Node::HtmlElement {
					element: &Element {
						name: "div",
						attributes: &[],
						content: Node::Multi(&[]),
						event_bindings: &[],
					},
					dom_binding: None,
				},
				event_bindings: &[],
			},
			dom_binding: None,
		}
		.prefer_thread_safe(),
		&mut Drain,
		3,
	)
	.unwrap();
}

#[test]
#[should_panic]
fn div_fail() {
	render_fragment(
		&Node::HtmlElement {
			element: &Element {
				name: "div",
				attributes: &[],
				content: Node::HtmlElement {
					element: &Element {
						name: "div",
						attributes: &[],
						content: Node::Multi(&[]),
						event_bindings: &[],
					},
					dom_binding: None,
				},
				event_bindings: &[],
			},
			dom_binding: None,
		}
		.prefer_thread_safe(),
		&mut Drain,
		2,
	)
	.unwrap();
}
