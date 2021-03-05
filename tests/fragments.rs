use lignin::{Element, Node};
use lignin_html::render_fragment;

#[test]
fn br() {
	let mut fragment = String::new();
	render_fragment(
		&Node::Element {
			element: &Element {
				name: "br",
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
		&Node::Element {
			element: &Element {
				name: "div",
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
