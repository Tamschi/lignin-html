use lignin::{Element, ElementCreationOptions, Node};
use lignin_html::render_fragment;

#[test]
fn br() {
	let mut fragment = String::new();
	render_fragment(
		&Node::HtmlElement {
			element: &Element {
				name: "BR",
				creation_options: ElementCreationOptions::new(),
				attributes: &[],
				content: Node::Multi(&[]),
				event_bindings: &[],
			},
			dom_binding: None,
		}
		.prefer_thread_safe(),
		&mut fragment,
		1,
	)
	.unwrap();
	assert_eq!(fragment, "<BR>");
}

#[test]
fn div() {
	let mut fragment = String::new();
	render_fragment(
		&Node::HtmlElement {
			element: &Element {
				name: "DIV",
				creation_options: ElementCreationOptions::new(),
				attributes: &[],
				content: Node::Multi(&[]),
				event_bindings: &[],
			},
			dom_binding: None,
		}
		.prefer_thread_safe(),
		&mut fragment,
		2,
	)
	.unwrap();
	assert_eq!(fragment, "<DIV></DIV>");
}

#[test]
fn div_custom() {
	let mut fragment = String::new();
	render_fragment(
		&Node::HtmlElement {
			element: &Element {
				name: "DIV",
				creation_options: ElementCreationOptions::new().with_is(Some("CUSTOM-DIV")),
				attributes: &[],
				content: Node::Multi(&[]),
				event_bindings: &[],
			},
			dom_binding: None,
		}
		.prefer_thread_safe(),
		&mut fragment,
		2,
	)
	.unwrap();
	assert_eq!(fragment, "<DIV is=CUSTOM-DIV></DIV>");
}

#[test]
fn text() {
	let mut fragment = String::new();
	render_fragment(
		&Node::Comment {
			comment: "><!-- Hello! --!> --><!-",
			dom_binding: None,
		}
		.prefer_thread_safe(),
		&mut fragment,
		1,
	)
	.unwrap();
	assert_eq!(fragment, "<!--|><!== Hello! ==!> ==><!-|-->");
}
