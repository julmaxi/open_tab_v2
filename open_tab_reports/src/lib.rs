use std::sync::Arc;

use tera::Tera;

pub mod layout;
pub mod pdf;
pub mod template;

pub use template::{TemplateContext, make_open_office_ballots};
//pub mod pdf;
//pub mod layout;
//mod pdf;
