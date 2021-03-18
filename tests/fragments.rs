use lignin::{Element, ElementCreationOptions, Node};
use lignin_html::render_fragment;

#[test]
fn br() {
	let mut fragment = String::new();
	render_fragment(
		&Node::HtmlElement {
			element: &Element {
				name: "br",
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
	assert_eq!(fragment, "<br>");
}

#[test]
fn div() {
	let mut fragment = String::new();
	render_fragment(
		&Node::HtmlElement {
			element: &Element {
				name: "div",
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
	assert_eq!(fragment, "<div></div>");
}

#[test]
fn div_custom() {
	let mut fragment = String::new();
	render_fragment(
		&Node::HtmlElement {
			element: &Element {
				name: "div",
				creation_options: ElementCreationOptions::new().with_is(Some("custom-div")),
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
	assert_eq!(fragment, "<div is=custom-div></div>");
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
