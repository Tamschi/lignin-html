#![allow(clippy::unneeded_field_pattern)]
#![doc(html_root_url = "https://docs.rs/lignin-html/0.0.4")]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

#[cfg(doctest)]
pub mod readme {
	doc_comment::doctest!("../README.md");
}

use core::fmt::{Error as fmtError, Write};
pub use lignin;
use lignin::{
	bumpalo::Bump,
	remnants::RemnantSite as rRemnantSite,
	Attribute, Element as lElement,
	Node::{self, Comment, Element, Multi, Ref, RemnantSite, Text},
};
use std::error::Error as stdError;
use v_htmlescape::escape;

#[allow(clippy::missing_errors_doc)]
pub fn render<'a>(w: &mut impl Write, vdom: &'a Node<'a>, bump: &'a Bump) -> Result<(), Error<'a>> {
	match vdom {
		&Comment(comment) => {
			write!(w, "<!--{}-->", escape(comment))?;
		}

		&Element(lElement {
			name: element_name,
			ref attributes,
			ref content,
			event_bindings: _,
		}) => {
			write!(w, "<{}", validate_element_name(element_name)?)?;
			for Attribute {
				name: attribute_name,
				value,
			} in &**attributes
			{
				write!(
					w,
					" {}=\"{}\"",
					validate_attribute_name(attribute_name)?,
					escape_attribute_value(value),
				)?;
			}
			w.write_char('>')?;
			for node in *content {
				render(w, node, bump)?;
			}
			//TODO: Fill out the blacklist here.
			if !["BR"].contains(element_name) {
				write!(w, "</{}>", element_name)?;
			}
		}

		Ref(target) => render(w, target, bump)?,
		Multi(nodes) => {
			for node in *nodes {
				render(w, node, bump)?;
			}
		}
		Text(text) => write!(
			w,
			"{}",
			text.replace("<", "&lt;") //TODO: Check if this is enough.
		)?,
		RemnantSite(rRemnantSite { content, .. }) => {
			render(w, content, bump)?;
		}
		other => return Err(Error::Unsupported(other)),
	};
	Ok(())
}

#[non_exhaustive]
pub enum Error<'a> {
	Unsupported(&'a Node<'a>),
	InvalidAttributeName,
	InvalidElementName,
	Format(fmtError),
	Other(Box<dyn stdError>),
}

impl<'a> From<fmtError> for Error<'a> {
	fn from(fmtError: fmtError) -> Self {
		Self::Format(fmtError)
	}
}

impl<'a> From<Box<dyn stdError>> for Error<'a> {
	fn from(std_error: Box<dyn stdError>) -> Self {
		Self::Other(std_error)
	}
}

fn validate_element_name(value: &str) -> Result<&str, Error> {
	if value.contains(|c| c == '>') {
		Err(Error::InvalidElementName) //TODO
	} else {
		Ok(value)
	}
}

fn validate_attribute_name(value: &str) -> Result<&str, Error> {
	if value.contains(|c| c == '>') {
		Err(Error::InvalidAttributeName) //TODO
	} else {
		Ok(value)
	}
}

fn escape_attribute_value(value: &str) -> String {
	if value.contains('"') {
		todo!("Attribute value escapes")
	}
	value.to_owned()
}
